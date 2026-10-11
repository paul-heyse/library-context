"""Reproduce the dated failure inventory from the retained JUnit report.

Run with Python 3.14.7. This is review evidence, not a test runner or product
failure classifier; classifications retain the scope and limitations in README.md.
"""

from collections import Counter
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parent
REPORT = ROOT / "raw" / "junit.xml"
EXPECTED_SHA256 = "9cab4f4bfff5d2d46f01c8cc282da226f371e1a93b8a6dad5fe09ac0ec0c8ad2"
# These messages omit the underlying VarError. The source call sites read the
# required runtime variable with expect(), as described in the source map.
SETUP_LABELS = {
    "stable validation runtime",
    "owned persistent native fixture",
    "owned native fixture required",
    "installed validation runtime",
    "owned compiler native fixture",
    "stable validation service",
    "owned compiler fixture",
    "owned native fixture",
}
MASKED_REFUSALS = {
    "typed_documents::corrupt_materialized_block_is_refused_before_publication",
    "native_model_context::driver_refuses_a_profile_mismatch_before_the_first_provider_can_write",
    "native_model_context::driver_refuses_a_foreign_catalog_allocation_pool_before_the_first_write",
}
COMPLETION_CASES = {
    "compiler::completion_tests::cancelled_native_failure_drain_retains_typed_primary_and_late_error",
    "compiler::completion_tests::post_closure_state_requires_a_live_same_store_finalization_parent",
}


def main() -> None:
    raw = REPORT.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != EXPECTED_SHA256:
        raise SystemExit("Retained report differs from the reviewed input")
    report = ET.fromstring(raw)
    failures = []
    suites = []
    groups: Counter[str] = Counter()
    classes: Counter[str] = Counter()
    observed: Counter[str] = Counter()
    for suite in report.findall("testsuite"):
        suites.append(dict(suite.attrib))
        for case in suite.findall("testcase"):
            failure = case.find("failure")
            if failure is None:
                observed["skipped" if case.find("skipped") is not None else "passed"] += 1
                continue
            observed["failed"] += 1
            lines = (failure.text or "").splitlines()
            reason = lines[1] if len(lines) > 1 else failure.get("message", "")
            output = "\n".join(node.text or "" for node in case)
            name = case.get("name", "")
            if "environment variable not found" in output or "NotPresent" in output or reason in SETUP_LABELS:
                classification = "missing_runtime_prerequisite"
            elif name in MASKED_REFUSALS:
                classification = "refusal_assertion_with_prerequisite_masking_candidate"
            elif name in COMPLETION_CASES:
                classification = "completion_contract_expectation"
            else:
                classification = "unresolved"
            classes[classification] += 1
            groups[reason] += 1
            failures.append({
                "suite": suite.get("name"),
                "name": name,
                "seconds": case.get("time"),
                "reported_message": failure.get("message"),
                "reported_type": failure.get("type"),
                "immediate_reason": reason,
                "review_classification": classification,
            })
    if observed["failed"] != int(report.get("failures", "0")):
        raise SystemExit("Failure inventory is incomplete")
    inventory = {
        "report_sha256": digest,
        "report": dict(report.attrib),
        "observed_outcomes": dict(observed),
        "review_classifications": dict(classes),
        "classification_limit": "Cause hypotheses and test-boundary observations, not native behavioral qualification; original failures are not reclassified as passes or skipped tests.",
        "suites": suites,
        "immediate_reason_counts": dict(groups.most_common()),
        "failed_cases": failures,
    }
    (ROOT / "inventory.json").write_text(json.dumps(inventory, indent=2) + "\n")
    rows = ["# Retained failure inventory", "", f"Report SHA-256: `{digest}`.", "",
            "Derived from the retained raw XML by `classify.py`; see README for causal limitations.", "",
            "| Review classification | Cases |", "|---|---:|"]
    rows.extend(f"| {name} | {count} |" for name, count in classes.items())
    rows += ["", "The complete failed-case list and per-suite totals are in `inventory.json`.", "",
             "| Immediate reported reason | Cases |", "|---|---:|"]
    rows.extend(f"| {reason.replace('|', '&#124;')} | {count} |" for reason, count in groups.most_common())
    (ROOT / "inventory.md").write_text("\n".join(rows) + "\n")
    print(json.dumps({"outcomes": dict(observed), "classifications": dict(classes), "failed_cases": len(failures)}))


if __name__ == "__main__":
    main()
