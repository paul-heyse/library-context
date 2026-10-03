"""Check finite review inventory and consumer routes; no analyzer semantic claims."""

import json
from pathlib import Path
import re


def main() -> None:
    root = Path(__file__).parent
    inventory = json.loads((root / "coverage-inventory.json").read_text())
    concepts = inventory["concepts"]
    facts = inventory["provider_facts"]
    concept_ids = {row["id"] for row in concepts}
    fact_ids = {row["id"] for row in facts}
    assert len(concepts) == len(concept_ids) == 137
    assert len(facts) == len(fact_ids) == 340
    assert len({item.split(".")[0] for item in concept_ids}) == 26
    assert {row["concept"] for row in facts} <= concept_ids
    ledger = (root / "consumer-ledger.md").read_text()
    for identifier in concept_ids | fact_ids:
        assert re.fullmatch(r"[a-z0-9.-]+", identifier)
    for identifier in concept_ids:
        assert ledger.count(f"| `{identifier}` |") >= 1, identifier
    links = 0
    for fact in facts:
        assert fact["decision"] in {
            "recommend_payload",
            "competing_or_existing_payload",
            "defer_no_selected_consumer",
        }
        assert fact["reason"] and fact["first_consumer_candidate"]
        for reference in fact["source_checks"]:
            assert reference["file_present"], reference["url"]
            assert reference["pin"] in reference["url"]
            links += 1
        if fact["decision"] == "recommend_payload":
            assert f"| `{fact['id']}` |" in ledger, fact["id"]
    assert links == 488
    assert sum(row["decision"] == "recommend_payload" for row in facts) == 41
    for concept in concepts:
        assert set(concept["facts"]) <= fact_ids
    print("passed: 26 areas, 137 concepts, 340 facts, 41 candidate routes; 488 recorded pin references")


if __name__ == "__main__":
    main()
