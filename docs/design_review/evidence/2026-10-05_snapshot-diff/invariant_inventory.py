"""Static inventory of lctx-model invariants: name, inputs (relation, order, prefix), check type,
check state field types and finish-body features. Read-only; run from the repository root:
    python3 docs/design_review/evidence/2026-10-05_snapshot-diff/invariant_inventory.py > raw/invariants.tsv
Features are lexical heuristics; the README records the manual classification built on them."""
import re, sys, pathlib
ROOT = pathlib.Path("crates/lctx-model/src/domain")
files = sorted(ROOT.rglob("*.rs"))
src = {f: f.read_text() for f in files}

def block(text, start):
    """Return the brace-balanced block starting at the first '{' at/after start."""
    i = text.index("{", start); d = 0
    for j in range(i, len(text)):
        d += {"{": 1, "}": -1}.get(text[j], 0)
        if d == 0:
            return text[i:j + 1]
    return text[i:]

# struct name -> (file, field text); impl InvariantCheck for X -> finish/visit body
structs, impls = {}, {}
for f, t in src.items():
    for m in re.finditer(r"\bstruct (\w+)(<[^>{]*>)?\s*\{", t):
        structs.setdefault(m.group(1), (f, block(t, m.start())))
    for m in re.finditer(r"impl(?:<[^>]*>)?\s+(?:super::)*(?:\w+::)*InvariantCheck for (\w+)", t):
        impls[m.group(1)] = (f, block(t, m.start()))

STATE = ["ChargedMap", "ChargedSet", "ChargedVec", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
         "Vec<", "previous", "Option<"]
rows = []
for f, t in src.items():
    for m in re.finditer(r"\bInvariant\s*\{\s*\n?\s*(revision|name|inputs)", t):
        b = block(t, m.start())
        name = re.search(r'name:\s*"([^"]+)"', b)
        name = name.group(1) if name else re.search(r"name:\s*([^,\n]+)", b).group(1).strip()
        inputs = []
        for im in re.finditer(r"ValidationInput::(?:of::<\s*([\w:]+)\s*>|of_relation\(([^,]+),)\s*\(?&\[([^\]]*)\]\)?(\s*\.at_epoch\([^)]*\))?", b):
            rel = (im.group(1) or im.group(2)).split("::")[-1]
            order = ",".join(x.strip().strip('"') for x in im.group(3).split(",") if x.strip())
            inputs.append(f"{rel}[{order}]{'@epoch' if im.group(4) else ''}")
        check = re.search(r"Box::new\(\s*(?:[\w:]+::)?(\w+)\s*(?:\{|::new|\()", b)
        check = check.group(1) if check else "?"
        fields = structs.get(check, (None, ""))[1]
        body = impls.get(check, (None, ""))[1]
        feats = sorted({s.rstrip("<") for s in STATE if s in fields})
        lead = {i.split("[")[1].split(",")[0].rstrip("]") for i in inputs}
        rows.append([str(f.relative_to(ROOT)), name, str(len(inputs)), ";".join(inputs), check,
                     ",".join(feats) or "-", "shared-lead" if len(lead) == 1 and len(inputs) > 1 else
                     ("single" if len(inputs) == 1 else "mixed-lead"),
                     "finish-scans-state" if re.search(r"fn finish[\s\S]*(\.iter\(\)|\.values\(\)|\.keys\(\)|is_empty\(\)|!=|\.len\(\))", body) else "-"])
print("\t".join(["file", "name", "n_inputs", "inputs", "check", "state", "order_lead", "finish"]))
for r in rows:
    print("\t".join(r))
print(f"# {len(rows)} definitions; {sum(1 for r in rows if r[4]=='?')} without a resolved check type", file=sys.stderr)
