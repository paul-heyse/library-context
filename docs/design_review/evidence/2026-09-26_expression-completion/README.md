# Expression and completion controls

**Tested, 2026-09-26.** The isolated command
`HYPOTHESIS_STORAGE_DIRECTORY=/tmp/lctx-stage3-completion-hypothesis timeout 30s uv run --no-sync python docs/design_review/evidence/2026-09-26_expression-completion/probe.py`
passed 120 generated CPython 3.14.7 / Hypothesis 6.168.1 controls. The probe owns its generated
programs; it never imports compiler fixtures, facts or evaluation references. Hypothesis uses
deterministic generation, no example database, no deadline and a maximum of 120 examples.

[`raw/results.json`](raw/results.json) records 28 normal finalizers, 33 overriding returns,
30 exceptional unwinds and 29 bare-handler controls. `sys.monitoring` observed the generated
function's return or exceptional unwind. Expected SyntaxWarnings about deliberate overriding
returns are retained in [`raw/stderr.txt`](raw/stderr.txt).

Two independent child processes observed the start of implicit exception-class construction
and old-value finalization on local rebinding, then failed to complete before a two-second
timeout. A timeout is an observation, not a proof of divergence. These counter-controls explain
why a normal read cannot certify `raise arbitrary_value`, and why an arbitrary overwrite cannot
certify normal assignment. Production withholds those cases; only a unique lexical assignment
outside any loop can supply the current local-initialization certificate.

This challenges the language semantics of the focused completion cases. It is not a complete
generated-program comparison against compiler output, CrossHair equivalence, Pysa coverage,
or integrated Stage 3 acceptance. Those obligations remain in the forward plan S7/S8.

The Hypothesis settings were checked against its [upstream settings documentation](https://github.com/HypothesisWorks/hypothesis/blob/master/hypothesis/docs/tutorial/settings.rst)
and the repository's pinned oracle index; the executed dependency version is recorded above.
