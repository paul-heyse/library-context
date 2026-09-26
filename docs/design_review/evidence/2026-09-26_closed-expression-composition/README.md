# Closed expression controls

**Tested, 2026-09-26.** `uv run --no-project --offline --no-python-downloads python docs/design_review/evidence/2026-09-26_closed-expression-composition/probe.py` passed nine independent CPython 3.14.7 controls; [results](raw/results.json) record actual values/types and exceptions. The probe executes its own literal examples, never compiler fixtures, and is not a compiler input.

The pure evaluator's unit tests mirror the nested Boolean/conditional, skipped/evaluated operand, selected non-Boolean value and bounded numeric cases. The numeric counter-control establishes why arbitrary Python integers cannot silently become Rust floats: Python conversion may raise `OverflowError`. Unsupported arithmetic remains unknown; these controls do not establish completeness or prove that every refusal raises.

This is a focused language-semantics observation, not a Hypothesis/sys.monitoring campaign, CrossHair equivalence, Pysa comparison, or integrated Stage 3 qualification. Those remain in the forward plan S7/S8.
