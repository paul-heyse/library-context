# Modeled return identity

**Tested, 2026-09-27; bounded independent runtime challenge.**
`runtime_oracle.py` runs seven independently authored generated programs in isolated CPython
3.14.7 workers. Each receives an opaque object and checks returned object identity, separately
from normal completion. Single/nested pass finalizers and keyword binding retain that object;
rebinding, deletion, overriding finalizers and invoked nonlocal mutation do not. Source, harness
and pinned `typing` digests are retained in `raw/runtime_receipt.json` (Git LFS).

Run: `uv run --no-sync python docs/design_review/evidence/2026-09-27_modeled-identity/runtime_oracle.py`.
Outcome: **passed**, seven programs. Workers have two-second CPU, 512 MiB address-space and
five-second wall bounds and prohibit network sockets. No analyzer fixture is executed. A finite
runtime pass challenges these examples; it does not establish complete source or model coverage.

The compiler/native test `finite_depth_and_unsupported_refusals_reach_the_native_response`
compares the published `summary_caps` controls with this independent output and challenges
certificate deletion/substitution and model-group omission. Its current receipt and the bounded
review belong in the [active plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md)
and [channel review](../../reviews/design_review_stage3-channel-contracts_2026-09-26.md).
Full source semantics in served support, additional modeled-chain/assignment certificates and
integrated Stage 3 acceptance remain outside this evidence.
