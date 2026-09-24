"""Cheap prevalence measurements over the fastmcp package source (stdlib ast only).

Usage: uv run python prevalence.py <path-to-fastmcp-package-dir>
Never imports fastmcp.
"""

from __future__ import annotations

import ast
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(sys.argv[1])
FILES = sorted(p for p in ROOT.rglob("*.py") if "__pycache__" not in p.parts)

CALLABLE_HINTS = ("Callable", "Handler", "Middleware", "Fn", "Hook", "Lifespan", "Awaitable")
CALLABLE_NAMES = {
    "fn",
    "func",
    "function",
    "handler",
    "callback",
    "middleware",
    "hook",
    "lifespan",
    "call_next",
}
CONTAINER_METHODS = {"append", "add", "insert", "extend", "appendleft", "setdefault", "update"}


def rel(p: Path) -> str:
    return str(p.relative_to(ROOT))


def params_of(fn: ast.FunctionDef | ast.AsyncFunctionDef) -> list[ast.arg]:
    a = fn.args
    out = [*a.posonlyargs, *a.args, *a.kwonlyargs]
    if a.vararg:
        out.append(a.vararg)
    if a.kwarg:
        out.append(a.kwarg)
    return out


def ann_text(arg: ast.arg) -> str:
    return ast.unparse(arg.annotation) if arg.annotation is not None else ""


def is_callable_param(arg: ast.arg) -> bool:
    t = ann_text(arg)
    return any(h in t for h in CALLABLE_HINTS) or arg.arg in CALLABLE_NAMES


def names_in(node: ast.AST) -> set[str]:
    return {n.id for n in ast.walk(node) if isinstance(n, ast.Name)}


def own_body_nodes(fn: ast.AST):
    """Walk fn's body but do not descend into nested defs/classes/lambdas."""
    stack = list(ast.iter_child_nodes(fn))
    while stack:
        n = stack.pop()
        yield n
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Lambda)):
            continue
        stack.extend(ast.iter_child_nodes(n))


def all_body_nodes(fn: ast.AST):
    """Walk including nested closures (a **kwargs captured by a closure still counts)."""
    for n in ast.walk(fn):
        if n is not fn:
            yield n


# ---------------------------------------------------------------- collection
trees: dict[str, ast.Module] = {}
for p in FILES:
    trees[rel(p)] = ast.parse(p.read_text(encoding="utf-8"), filename=str(p))

# function records: (module, qualname, node, class_name|None, nested_in_function)
funcs: list[tuple[str, str, ast.AST, str | None, bool]] = []
classes: dict[tuple[str, str], ast.ClassDef] = {}


def collect(mod: str, node: ast.AST, prefix: str, cls: str | None, in_fn: bool) -> None:
    for ch in ast.iter_child_nodes(node):
        if isinstance(ch, ast.ClassDef):
            classes[(mod, ch.name)] = ch
            collect(mod, ch, f"{prefix}{ch.name}.", ch.name, in_fn)
        elif isinstance(ch, (ast.FunctionDef, ast.AsyncFunctionDef)):
            funcs.append((mod, f"{prefix}{ch.name}", ch, cls, in_fn))
            collect(mod, ch, f"{prefix}{ch.name}.<locals>.", None, True)
        else:
            collect(mod, ch, prefix, cls, in_fn)


for mod, tree in trees.items():
    collect(mod, tree, "", None, False)

print(
    f"files={len(FILES)}  function/method defs (incl. nested)={len(funcs)}  classes={len(classes)}"
)
print()


def is_stub(f) -> bool:
    """@overload / Protocol / abstract stubs: body is only docstring, ..., pass, or raise NotImplementedError."""
    if any(
        ast.unparse(d).split(".")[-1] in ("overload", "abstractmethod") for d in f.decorator_list
    ):
        return True
    for s in f.body:
        if isinstance(s, ast.Expr) and isinstance(s.value, ast.Constant):
            continue
        if isinstance(s, ast.Pass):
            continue
        if (
            isinstance(s, ast.Raise)
            and s.exc is not None
            and "NotImplementedError" in ast.unparse(s.exc)
        ):
            continue
        return False
    return True


n_stubs = sum(1 for (_, _, f, _, _) in funcs if is_stub(f))
print(f"stub defs (@overload/Protocol/abstract, excluded from (a)): {n_stubs}")

# ------------------------------------------------ (a) **kwargs and forwarding
kw_funcs = [(m, q, f) for (m, q, f, c, n) in funcs if f.args.kwarg is not None and not is_stub(f)]
kw_funcs_all = [f for (m, q, f, c, n) in funcs if f.args.kwarg is not None]
print(f"(a0) defs with **kwargs incl. stubs: {len(kw_funcs_all)}")
fwd_kind_defs = Counter()
fwd, consumed_only, unused = [], [], []
fwd_target_kinds = Counter()
for m, q, f in kw_funcs:
    kname = f.args.kwarg.arg
    forwarded = False
    referenced = False
    kinds_here = set()
    for n in all_body_nodes(f):
        if isinstance(n, ast.Call):
            for kw in n.keywords:
                if kw.arg is None and isinstance(kw.value, ast.Name) and kw.value.id == kname:
                    forwarded = True
                    fn_txt = ast.unparse(n.func)
                    if fn_txt in ("partial", "functools.partial"):
                        k = "functools.partial(...)"
                    elif fn_txt.startswith("super()"):
                        k = "super().<m>(...)"
                    elif fn_txt.startswith("self.") or fn_txt.startswith("cls."):
                        k = "self./cls.<m>(...)"
                    elif fn_txt[:1].isupper() or fn_txt.split(".")[-1][:1].isupper():
                        k = "Constructor(...)"
                    elif isinstance(n.func, ast.Name) and n.func.id in {
                        a.arg for a in params_of(f)
                    } | {"fn", "func"}:
                        k = "call of a parameter/closure var (wrapper)"
                    else:
                        k = "other call"
                    fwd_target_kinds[k] += 1
                    kinds_here.add(k)
        if isinstance(n, ast.Name) and n.id == kname:
            referenced = True
    for k in kinds_here:
        fwd_kind_defs[k] += 1
    if forwarded:
        fwd.append((m, q))
    elif referenced:
        consumed_only.append((m, q))
    else:
        unused.append((m, q))

va_funcs = [(m, q, f) for (m, q, f, c, n) in funcs if f.args.vararg is not None]
va_fwd = 0
for m, q, f in va_funcs:
    vname = f.args.vararg.arg
    if any(
        isinstance(n, ast.Starred) and isinstance(n.value, ast.Name) and n.value.id == vname
        for n in all_body_nodes(f)
    ):
        va_fwd += 1

print("(a) **kwargs")
print(f"  defs with **kwargs: {len(kw_funcs)}")
print(f"    forward it as **name to a call:        {len(fwd)}")
print(f"      forwarding call sites by kind: {dict(fwd_target_kinds)}")
print(f"      forwarding defs by kind:       {dict(fwd_kind_defs)}")
for m, q in fwd:
    print(f"        fwd: {m}::{q}")
for m, q in consumed_only:
    print(f"        consumed-only: {m}::{q}")
print(f"    reference it but never **-forward:      {len(consumed_only)}")
print(f"    never reference it (accepted, ignored): {len(unused)}")
for m, q in unused:
    print(f"        unused **kwargs: {m}::{q}")
print(f"  defs with *args: {len(va_funcs)}; forward *args: {va_fwd}")

# Named-parameter pass-through at call sites: keyword arg `x=<param>` or positional <param>
total_kw_args = 0
passthrough_kw = 0
for m, q, f, c, n_ in funcs:
    pnames = {a.arg for a in params_of(f)} - {"self", "cls"}
    for n in own_body_nodes(f):
        if isinstance(n, ast.Call):
            for kw in n.keywords:
                if kw.arg is None:
                    continue
                total_kw_args += 1
                if isinstance(kw.value, ast.Name) and kw.value.id in pnames:
                    passthrough_kw += 1
print(
    f"  keyword arguments at call sites (non-**): {total_kw_args}; value is a bare parameter of the enclosing def: {passthrough_kw} ({passthrough_kw / total_kw_args:.0%})"
)
print()

# ----------------------------------- (b) self.attr = <param> in __init__, read elsewhere
# Build per-class: attr -> ('strict'|'derived') from __init__
init_attrs: dict[tuple[str, str], dict[str, str]] = defaultdict(dict)
for (mod, cname), cnode in classes.items():
    for item in cnode.body:
        if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)) and item.name == "__init__":
            pnames = {a.arg for a in params_of(item)} - {"self"}
            for n in own_body_nodes(item):
                targets: list[ast.AST] = []
                value = None
                if isinstance(n, ast.Assign):
                    targets, value = n.targets, n.value
                elif isinstance(n, ast.AnnAssign) and n.value is not None:
                    targets, value = [n.target], n.value
                for t in targets:
                    if (
                        isinstance(t, ast.Attribute)
                        and isinstance(t.value, ast.Name)
                        and t.value.id == "self"
                    ):
                        if isinstance(value, ast.Name) and value.id in pnames:
                            init_attrs[(mod, cname)][t.attr] = "strict"
                        elif names_in(value) & pnames:
                            init_attrs[(mod, cname)].setdefault(t.attr, "derived")

# reads: self.<attr> Load in methods of class (other than __init__), and in mixin methods
# whose `self` parameter is annotated with the class name (e.g. `self: FastMCP`).
reads_same_class: dict[tuple[str, str], set[str]] = defaultdict(set)
reads_typed_self: dict[str, set[str]] = defaultdict(set)  # class name -> attrs
for m, q, f, c, nested in funcs:
    args = f.args.args
    self_ann = ""
    if args and args[0].arg == "self":
        self_ann = ann_text(args[0]).strip("'\"")
    for n in all_body_nodes(f):
        if (
            isinstance(n, ast.Attribute)
            and isinstance(n.ctx, ast.Load)
            and isinstance(n.value, ast.Name)
            and n.value.id == "self"
        ):
            if c is not None and f.name != "__init__":
                reads_same_class[(m, c)].add(n.attr)
            if self_ann:
                reads_typed_self[self_ann].add(n.attr)

# global attribute-name loads anywhere (x.attr), excluding the defining __init__ – upper bound
global_attr_loads = Counter()
for mod, tree in trees.items():
    for n in ast.walk(tree):
        if isinstance(n, ast.Attribute) and isinstance(n.ctx, ast.Load):
            global_attr_loads[n.attr] += 1

strict_total = derived_total = 0
strict_read_same = derived_read_same = 0
strict_read_mixin = derived_read_mixin = 0
strict_read_elsewhere_only = derived_read_elsewhere_only = 0
strict_never = derived_never = 0
never_examples = []
for key, attrs in init_attrs.items():
    mod, cname = key
    same = reads_same_class.get(key, set())
    mix = reads_typed_self.get(cname, set())
    for attr, kind in attrs.items():
        in_same = attr in same
        in_mix = (attr in mix) and not in_same
        # loads named .attr anywhere, minus loads inside own class that we already counted
        elsewhere = global_attr_loads[attr] > 0
        if kind == "strict":
            strict_total += 1
            if in_same:
                strict_read_same += 1
            elif in_mix:
                strict_read_mixin += 1
            elif elsewhere:
                strict_read_elsewhere_only += 1
            else:
                strict_never += 1
                never_examples.append(f"{mod}::{cname}.{attr}")
        else:
            derived_total += 1
            if in_same:
                derived_read_same += 1
            elif in_mix:
                derived_read_mixin += 1
            elif elsewhere:
                derived_read_elsewhere_only += 1
            else:
                derived_never += 1
                never_examples.append(f"{mod}::{cname}.{attr} (derived)")

print("(b) self.<attr> = <param> in __init__")
print(f"  classes with such assignments: {len(init_attrs)}")
print(f"  strict (RHS is exactly a parameter): {strict_total}")
print(f"    read via self.<attr> in another method of the same class: {strict_read_same}")
print(f"    read only in a mixin method typed `self: <Class>`:          {strict_read_mixin}")
print(
    f"    read only as <expr>.<attr> elsewhere (needs types/aliasing): {strict_read_elsewhere_only}"
)
print(f"    no load of that attribute name anywhere in the package:     {strict_never}")
print(
    f"  derived (RHS expression mentions a parameter, e.g. `p or default`, `p if p is not None else settings.x`): {derived_total}"
)
print(
    f"    same-class read: {derived_read_same}; mixin read: {derived_read_mixin}; elsewhere only: {derived_read_elsewhere_only}; never: {derived_never}"
)
for e in never_examples[:40]:
    print(f"      never-read: {e}")

# declarative field storage (pydantic / dataclass / settings) - not visible as __init__ assignments
base_names: dict[str, set[str]] = {}
for (mod, cname), cnode in classes.items():
    base_names[cname] = {ast.unparse(b).split("[")[0].split(".")[-1] for b in cnode.bases}
PYD_ROOTS = {"BaseModel", "BaseSettings", "FastMCPBaseModel", "RootModel", "FastMCPComponent"}


def is_pydantic(cname: str, seen: set[str] | None = None) -> bool:
    seen = seen or set()
    if cname in seen:
        return False
    seen.add(cname)
    bases = base_names.get(cname, set())
    if bases & PYD_ROOTS:
        return True
    return any(is_pydantic(b, seen) for b in bases if b in base_names)


pyd_classes = dataclass_classes = 0
pyd_fields = dc_fields = 0
for (mod, cname), cnode in classes.items():
    decos = {ast.unparse(d).split("(")[0].split(".")[-1] for d in cnode.decorator_list}
    nfields = sum(
        1 for s in cnode.body if isinstance(s, ast.AnnAssign) and isinstance(s.target, ast.Name)
    )
    if "dataclass" in decos:
        dataclass_classes += 1
        dc_fields += nfields
    elif is_pydantic(cname):
        pyd_classes += 1
        pyd_fields += nfields
print(
    f"  declarative storage: pydantic-model classes={pyd_classes} (annotated fields={pyd_fields}); @dataclass classes={dataclass_classes} (fields={dc_fields})"
)
print()

# ------------------------------------------ (c) callables stored into containers/registries
store_container = []  # (mod, qual, how)
store_attr = []
attach_meta = []
for m, q, f, c, nested in funcs:
    cparams = {a.arg for a in params_of(f) if is_callable_param(a)}
    allp = {a.arg for a in params_of(f)} - {"self", "cls"}
    hows = set()
    for n in own_body_nodes(f):
        # X.append(p) / self.x.add(p) ...
        if (
            isinstance(n, ast.Call)
            and isinstance(n.func, ast.Attribute)
            and n.func.attr in CONTAINER_METHODS
        ):
            argnames = set().union(*(names_in(a) for a in n.args)) if n.args else set()
            if argnames & cparams:
                hows.add(f"container.{n.func.attr}(callable param)")
        # X[k] = p
        if isinstance(n, ast.Assign):
            for t in n.targets:
                if (
                    isinstance(t, ast.Subscript)
                    and isinstance(n.value, ast.Name)
                    and n.value.id in cparams
                ):
                    hows.add("container[key] = callable param")
                if (
                    isinstance(t, ast.Attribute)
                    and isinstance(t.value, ast.Name)
                    and t.value.id == "self"
                    and isinstance(n.value, ast.Name)
                    and n.value.id in cparams
                ):
                    store_attr.append((m, q, t.attr))
                # fn.__dunder__ = metadata   (decorator attaching metadata to the function object)
                if (
                    isinstance(t, ast.Attribute)
                    and t.attr.startswith("__")
                    and t.attr.endswith("__")
                    and t.attr
                    not in {
                        "__signature__",
                        "__annotations__",
                        "__name__",
                        "__doc__",
                        "__module__",
                        "__qualname__",
                        "__wrapped__",
                        "__dict__",
                    }
                    and not (isinstance(t.value, ast.Name) and t.value.id == "self")
                ):
                    attach_meta.append((m, q, t.attr))
        if isinstance(n, ast.Call) and ast.unparse(n.func) == "setattr" and n.args:
            if (
                names_in(n.args[0]) & allp
                and len(n.args) > 1
                and isinstance(n.args[1], ast.Constant)
                and str(n.args[1].value).startswith("__")
            ):
                attach_meta.append((m, q, f"setattr {n.args[1].value}"))
    if hows:
        store_container.append((m, q, sorted(hows)))

# broadened: any parameter (not only callable-typed) put into a self.<container>
reg_defs = []
for m, q, f, c, nested in funcs:
    allp = {a.arg for a in params_of(f)} - {"self", "cls"}
    hit = set()
    for n in all_body_nodes(f):
        if (
            isinstance(n, ast.Call)
            and isinstance(n.func, ast.Attribute)
            and n.func.attr in CONTAINER_METHODS
        ):
            tgt = n.func.value
            if (
                isinstance(tgt, ast.Attribute)
                and isinstance(tgt.value, ast.Name)
                and tgt.value.id == "self"
            ):
                argnames = set().union(*(names_in(a) for a in n.args)) if n.args else set()
                if argnames & allp:
                    hit.add(f"self.{tgt.attr}.{n.func.attr}(param)")
        if isinstance(n, ast.Assign):
            for t in n.targets:
                if (
                    isinstance(t, ast.Subscript)
                    and isinstance(t.value, ast.Attribute)
                    and isinstance(t.value.value, ast.Name)
                    and t.value.value.id == "self"
                    and names_in(n.value) & allp
                ):
                    hit.add(f"self.{t.value.attr}[k] = param")
    if hit:
        reg_defs.append((m, q, sorted(hit)))

# decorator factories: a def whose body defines an inner def and returns it (or partial of self)
decorator_factories = 0
for m, q, f, c, nested in funcs:
    inner = {
        ch.name
        for ch in own_body_nodes(f)
        if isinstance(ch, (ast.FunctionDef, ast.AsyncFunctionDef))
    }
    rets = [n for n in own_body_nodes(f) if isinstance(n, ast.Return) and n.value is not None]
    if inner and any(isinstance(r.value, ast.Name) and r.value.id in inner for r in rets):
        decorator_factories += 1

# registry-like attributes of FastMCP-ish objects written via container ops on self.<x>
print("(c) callable storage")
print(
    f"  defs that put a callable-typed parameter into a container (append/add/insert/extend/update/[k]=): {len(store_container)}"
)
for m, q, h in store_container:
    print(f"      {m}::{q}  {h}")
print(
    f"  defs that store a callable-typed parameter as self.<attr> (callback kept for later invocation): {len(store_attr)} sites in {len({(m, q) for m, q, _ in store_attr})} defs"
)
for m, q, a in store_attr[:60]:
    print(f"      {m}::{q}  self.{a}")
print(f"  metadata attached to a function object (fn.__x__ = ... / setattr): {len(attach_meta)}")
for m, q, a in attach_meta:
    print(f"      {m}::{q}  {a}")
print(
    f"  defs inserting a parameter-derived value into a self.<container> (registry writes, any type): {len(reg_defs)}"
)
for m, q, h in reg_defs:
    print(f"      reg: {m}::{q}  {h}")
print(
    f"  closures returned by their enclosing def (decorator/wrapper factories): {decorator_factories}"
)
print()

# ------------------------------------------- (d) except handlers that raise a different type
tries = 0
handlers = 0
rewrap = rewrap_from_e = rewrap_from_none = rewrap_bare = 0
reraise_same = swallow = 0
rewrap_pairs = Counter()
raise_var = [0]
for mod, tree in trees.items():
    for n in ast.walk(tree):
        if isinstance(n, (ast.Try, getattr(ast, "TryStar", ast.Try))):
            tries += 1
            for h in n.handlers:
                handlers += 1
                caught = ast.unparse(h.type) if h.type is not None else "BaseException(bare)"
                caught_names = {
                    x.split(".")[-1]
                    for x in caught.replace("(", " ")
                    .replace(")", " ")
                    .replace(",", " ")
                    .replace("|", " ")
                    .split()
                }
                raises = [r for r in own_body_nodes(h) if isinstance(r, ast.Raise)]
                raises += [r for r in h.body if isinstance(r, ast.Raise)]
                raises = list({id(r): r for r in raises}.values())
                diff = []
                for r in raises:
                    if r.exc is None:
                        continue
                    exc_node = r.exc.func if isinstance(r.exc, ast.Call) else r.exc
                    ename = ast.unparse(exc_node).split(".")[-1]
                    if h.name and isinstance(r.exc, ast.Name) and r.exc.id == h.name:
                        continue  # `raise e` = same object
                    if isinstance(r.exc, ast.Name) and not r.exc.id[:1].isupper():
                        raise_var[0] += 1  # `raise some_var`: type needs inference
                        continue
                    if ename in caught_names:
                        continue
                    diff.append((r, ename))
                if diff:
                    rewrap += 1
                    for r, ename in diff:
                        rewrap_pairs[(caught, ename)] += 1
                    r0 = diff[0][0]
                    if r0.cause is None:
                        rewrap_bare += 1
                    elif isinstance(r0.cause, ast.Constant) and r0.cause.value is None:
                        rewrap_from_none += 1
                    else:
                        rewrap_from_e += 1
                elif any(r.exc is None for r in raises) or any(
                    isinstance(r.exc, ast.Name) and r.exc.id == h.name for r in raises
                ):
                    reraise_same += 1
                elif not raises:
                    swallow += 1
print("(d) exception handling")
print(f"  try statements: {tries}; except handlers: {handlers}")
print(
    f"  handlers raising a different exception type: {rewrap}  (`from e`: {rewrap_from_e}, `from None`: {rewrap_from_none}, no cause: {rewrap_bare})"
)
print(f"  handlers that only re-raise the same exception: {reraise_same}")
print(
    f"  `raise <lowercase variable>` inside handlers (exception type needs inference): {raise_var[0]}"
)
print(f"  handlers with no raise at all (swallow / convert to value / log): {swallow}")
print("  most common (caught -> raised) pairs:")
for (c_, r_), k in rewrap_pairs.most_common(15):
    print(f"      {k:3d}  {c_} -> {r_}")
print()


# ------------------------------------------------ (e) public defs and async fraction
def is_private_mod(mod: str) -> bool:
    return any(part.startswith("_") and part != "__init__.py" for part in Path(mod).parts)


pub_all = [(m, q, f) for (m, q, f, c, nested) in funcs if not f.name.startswith("_") and not nested]
pub_async = [x for x in pub_all if isinstance(x[2], ast.AsyncFunctionDef)]
pub_strict = [
    (m, q, f)
    for (m, q, f, c, nested) in funcs
    if not f.name.startswith("_")
    and not nested
    and not is_private_mod(m)
    and not (c is not None and c.startswith("_"))
]
pub_strict_async = [x for x in pub_strict if isinstance(x[2], ast.AsyncFunctionDef)]
methods = [x for x in pub_strict if "." in x[1]]
methods_async = [x for x in methods if isinstance(x[2], ast.AsyncFunctionDef)]
tops = [x for x in pub_strict if "." not in x[1]]
tops_async = [x for x in tops if isinstance(x[2], ast.AsyncFunctionDef)]
acm = sum(
    1
    for (m, q, f) in pub_strict
    if any("asynccontextmanager" in ast.unparse(d) for d in f.decorator_list)
)
agen = sum(
    1
    for (m, q, f) in pub_strict
    if isinstance(f, ast.AsyncFunctionDef)
    and any(isinstance(n, (ast.Yield, ast.YieldFrom)) for n in own_body_nodes(f))
)
all_defs_async = sum(1 for (_, _, f, _, _) in funcs if isinstance(f, ast.AsyncFunctionDef))
print("(e) public defs")
print(
    f"  non-nested defs whose name does not start with '_': {len(pub_all)}; async: {len(pub_async)} ({len(pub_async) / len(pub_all):.1%})"
)
print(
    f"  ... also excluding private modules and private classes: {len(pub_strict)}; async: {len(pub_strict_async)} ({len(pub_strict_async) / len(pub_strict):.1%})"
)
print(
    f"      methods: {len(methods)} (async {len(methods_async)}, {len(methods_async) / len(methods):.1%}); module functions: {len(tops)} (async {len(tops_async)}, {len(tops_async) / len(tops):.1%})"
)
print(
    f"      of which @asynccontextmanager: {acm}; async generators/CMs (async def with yield): {agen}"
)
print(
    f"  all defs incl. private/nested: {len(funcs)}; async: {all_defs_async} ({all_defs_async / len(funcs):.1%})"
)
by_top = Counter()
by_top_async = Counter()
for m, q, f in pub_strict:
    top = Path(m).parts[0] if len(Path(m).parts) > 1 else "(root)"
    by_top[top] += 1
    if isinstance(f, ast.AsyncFunctionDef):
        by_top_async[top] += 1
print(
    "  public (strict) by top-level subpackage: "
    + ", ".join(f"{k}={v} ({by_top_async[k]} async)" for k, v in by_top.most_common())
)
print()

# ------------------------------------------------ extra: dynamic / ambient features
extra = Counter()
for mod, tree in trees.items():
    for n in ast.walk(tree):
        if isinstance(n, ast.Call):
            t = ast.unparse(n.func)
            if t in ("getattr", "setattr", "hasattr", "delattr"):
                extra[t] += 1
            elif t.endswith("import_module"):
                extra["importlib.import_module"] += 1
            elif t in ("partial", "functools.partial"):
                extra["functools.partial"] += 1
            elif t == "isinstance":
                extra["isinstance"] += 1
            elif t in ("ContextVar", "contextvars.ContextVar"):
                extra["ContextVar(...) created"] += 1
            elif t == "cast":
                extra["typing.cast"] += 1
            for kw in n.keywords:
                if kw.arg is None and not isinstance(kw.value, ast.Name):
                    extra["**<non-name expr> at call site"] += 1
                elif kw.arg is None:
                    extra["**name at call site"] += 1
        elif (
            isinstance(n, ast.Attribute)
            and n.attr in ("get", "set", "reset")
            and isinstance(n.value, ast.Name)
            and n.value.id.startswith("_current")
        ):
            extra[f"_current*.{n.attr}() ContextVar access"] += 1
        elif isinstance(n, (ast.AsyncWith,)):
            extra["async with"] += 1
        elif isinstance(n, ast.Await):
            extra["await"] += 1
        elif (
            isinstance(n, ast.Attribute)
            and isinstance(n.ctx, ast.Load)
            and n.attr == "settings"
            and isinstance(n.value, ast.Name)
            and n.value.id == "fastmcp"
        ):
            extra["fastmcp.settings.<x> reads"] += 1
        elif isinstance(n, ast.FunctionDef) and n.name == "__getattr__" and n in tree.body:
            extra["module-level __getattr__"] += 1
    for n in ast.walk(tree):
        if isinstance(n, (ast.Import, ast.ImportFrom)):
            # imports inside a function body (lazy imports) are harder for call resolution
            pass
lazy_imports = 0
for m, q, f, c, nested in funcs:
    lazy_imports += sum(1 for n in own_body_nodes(f) if isinstance(n, (ast.Import, ast.ImportFrom)))
extra["function-local import statements"] = lazy_imports
print("extra: dynamic / ambient features")
for k, v in sorted(extra.items()):
    print(f"  {k}: {v}")
