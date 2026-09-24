# Stage 3 design notes (working draft, not repo)

## Core: resolve `call_transfer` by summaries

Today: a use inside a call within a sink value is `through_call`; its claims are unknown
(`call_transfer`). Stage 3 resolves them.

### Call results as intermediate values
- cpg-flow: a value source may be a **call result**: when `sources()` meets `Expr::Call` in a value
  position, record `ValueSource { call: Some(call span), .. }` (the call's result is (part of) the
  value, identity when the call IS the value, derived otherwise). Keep `through_call` uses for the
  args as today (fallback), or drop them once call results exist (decide: keep, marked, so an
  unresolved call still yields an unknown claim without a second path).
- The call's own arguments are already Argument sinks (per argument span); the CPG maps argument
  node -> (call node, formal) (`argument_at` in flow_model).
- Composition: sink S <- call result(c) <- callee T summary (formal q -> ReturnValue, kind k,
  condition) <- Argument sink (c, q) <- sources ... recursively (nested calls).
- Receiver: `x.m()` -> Parameter[self] of m -> ReturnValue.

### Summary relation (L3), per callable
- inputs: Parameter[name], Parameter[self].Field[f] (receiver fields)
- outputs: ReturnValue, Parameter[self].Field[f] (stores), Argument[formal]@Call[target] (external
  sinks), Raise[T]
- kind: value (identity) / transform (derived) / constant
- verdict: established / conditional / unknown(reason)
- Complete summary => absence of q->ReturnValue means "does not reach" (refuted_under_model only
  with a complete region: the callee's behavior_status established and every nested call resolved).

### Order
- SCCs of the release call graph (definite + candidate arcs), tarjan_scc (callees first).
- Within an SCC: fixpoint; bounds k (path depth) and c (condition size = budget); widening ->
  unknown(budget_reached).
- Override-open calls: join candidates' summaries, mark open (unknown override_dispatch stays).

### Models (L4)
- `crates/cpg-schema/models/external.toml`, `frameworks.toml`
- entry: id (append-only), callable access path (e.g. `typing.cast`), transfers
  (`Argument[1] -> ReturnValue value`), effects (`log`, `validate(...)`), raises.
- digest joins compiler_digest; rule: no model cites `.claude/skills`.
- v1 list (plan 3.1) + `typing.cast`, `str`, `dict`, `list`, `isinstance` (no transfer),
  `logging.Logger.*` (effect log, no ReturnValue).

### Exceptions (Q05)
- `handlers`: per try statement: caught types, action (re-raise, convert to T, swallow, value).
- Summary Raise[T] outputs propagate to callers through handlers.

### ContextVar (Q03.e)
- ambient reads extended to ContextVar `.get()` (a module-global ContextVar object).

### Compatibility (Q09, F11)
- `Condition::compatible(&self, assignment)`: evaluate literals under a partial assignment
  (place = literal); unknown atoms free; SAT over DNF is trivial (any conjunction consistent).
- "silently ignored under transport='sse'": a parameter with no fate whose condition is
  compatible with `equals(transport,"sse")`.

### Exit: behavior_shapes part 2 + Q01, Q03, Q05, Q09 answered (stage-3 exit rule).
