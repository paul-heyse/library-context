"""Compare current catalog content while preserving run-qualified citation identities separately."""

from collections import Counter
import json
import re
from pathlib import Path
import sys

import pyarrow.ipc as ipc


def main():
    output = Path(sys.argv[1])
    rows = json.loads((output / "receipt.json").read_text())
    live = {
        row["profile"]: Path(row["generation"])
        for row in rows
        if row["embedder"] == "none"
    }
    results = {}
    for path in sorted(live["catalog"].glob("catalog_*.arrow")):
        catalog = ipc.open_file(path).read_all()
        behavioral = ipc.open_file(live["behavioral"] / path.name).read_all()
        # FactSink::fact includes run_id; these two identities derive from fact IDs.
        excluded = [
            name for name in catalog.column_names
            if (name == "fact_id" or name.endswith("_fact_id"))
            or name in {"binding_id", "link_id", "brief_status", "brief_reason", "association_id", "domain_id"}
            or (path.stem == "catalog_evidence" and name == "evidence_id")
        ]
        names = [name for name in catalog.column_names if name not in excluded]

        def canonical(table):
            def clean(value, key=""):
                if isinstance(value, dict):
                    if value.get("kind") == "fact":
                        return {"kind": "fact", "id": "<run-qualified>"}
                    return {k: clean(v, k) for k, v in value.items() if not (k == "fact_id" or k.endswith("_fact_id")) and k not in {"edge_id", "binding"}}
                if isinstance(value, list):
                    values = [clean(v) for v in value]
                    # Context/evidence sets are ordered by run-qualified identities in storage.
                    # Removing those identities requires reordering the comparison, preserving
                    # every occurrence and all semantic fields rather than dropping a domain.
                    if path.stem == "catalog_selection_domains" and key in {"domains", "contexts", "evidence", "releases"}:
                        values.sort(key=lambda item: json.dumps(item, sort_keys=True))
                    return values
                if isinstance(value, str):
                    if value.startswith(("{", "[")):
                        try:
                            return clean(json.loads(value))
                        except ValueError:
                            pass
                    return re.sub(r"source fact [a-f0-9]{32}", "source fact <run-qualified>", value)
                return value
            return Counter(
                json.dumps(clean(row), sort_keys=True, default=lambda value: value.hex())
                for row in table.select(names).to_pylist()
            )

        results[path.stem] = {
            "catalog_rows": catalog.num_rows,
            "behavioral_rows": behavioral.num_rows,
            "same_contract_multiset": canonical(catalog) == canonical(behavioral),
            "excluded": excluded,
        }
    assert len(results) == 18, "expected eighteen catalog projection relations"
    print(json.dumps(results, indent=2))
    assert all(row["same_contract_multiset"] for row in results.values())


if __name__ == "__main__":
    main()
