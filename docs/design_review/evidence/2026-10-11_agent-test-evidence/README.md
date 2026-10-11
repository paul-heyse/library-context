# Agent test-evidence capability checks

Date: 2026-10-11. Consumer: [agent test-evidence and causal-analysis review](../../reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md).

Question: can existing Python reporting preserve independent failures and completed work after
interruption, without running project tests or installing a reporting plugin into its environment?

## Method and versions

Linux x86_64, checkout `/home/paul/library-context`, source baseline `48088a95`.
Python 3.14.7 and the installed pytest 9.1.1. The probe downloads the official
[`pytest-reportlog==1.0.0` wheel](https://pypi.org/project/pytest-reportlog/1.0.0/), verifies
SHA-256 `3fc837ef3be6e50f33b52aaf99f88bfbb2ec525febbc3990389d24bbfba28753`, and explicitly
loads its plugin from a temporary directory. It disables plugin auto-discovery and isolates pytest
configuration and synthetic inputs from the project. No dependency, project test, service, fixture
or operator configuration changes. Only the probe's own child is terminated in the kill scenario.

```sh
just run --label 'pytest evidence capability probe' -- uv run --no-sync python \
  docs/design_review/evidence/2026-10-11_agent-test-evidence/probe_pytest_reports.py \
  --output docs/design_review/evidence/2026-10-11_agent-test-evidence/results.json
```

**passed**, 2026-10-11, run `20261011T052916.677Z-80b055`, exit0. The intentional synthetic
pytest failures/exits below are inputs to the probe, not repository test failures. The
[derived results](results.json) and [source](probe_pytest_reports.py) are the retained evidence.
Raw reports, downloaded plugin and synthetic inputs are owned by the temporary-directory context
and removed when the probe exits. No raw captures or environments are committed. The existing
run owner retains its small receipt/log; this review is its named evidence consumer.

## Results and limits

| Scenario | Observed result |
|---|---|
| Seven ordinary items, including setup failure, execution plus teardown failure, skip, xfail, strict xpass and warning | JSONL retains separate phase reports and exception chain, passing stdout/fd stderr/log sections and warnings. JUnit contains eight testcase rows because execution plus teardown failure creates two rows for one item. |
| Syntax failure during collection | Collection error appears in JSONL and JUnit; no test-phase reports. |
| Graceful `KeyboardInterrupt` after a completed test | JSONL retains the completed test and interrupted item's setup; session finish records exit2. There is no completed call report for the interrupted item. JUnit exists and includes an unnamed partial row; its existence does not establish completed-case coverage. |
| SIGTERM during a second test's call | Previously flushed JSONL reports remain, including that item's setup. No session-finish record or JUnit exists. Captured stdout from the unfinished call is absent from JSONL. |

These controls qualify this plugin/version composition for completed-phase reporting; they do not
establish universal crash durability, fsync guarantees, live capture of unfinished calls, xdist
compatibility or repository-wide acceptance. `pytest-reportlog`1.0.0 emits no test-start event;
minimal lifecycle hooks are needed if exact active-item tracking is selected. Repeated captured
sections in phase reports must not be mistaken for repeated execution. A surviving event prefix
is explicitly partial until the relevant terminal and owner-cleanup evidence exists.

No quantitative performance, production bug diagnosis or full test-suite pass claim is made.
