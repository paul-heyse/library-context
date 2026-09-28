"""Compare live catalog content while preserving run-qualified citation identities separately."""

from collections import Counter
import json
from pathlib import Path
import sys

import pyarrow.ipc as ipc


def main():
    output = Path(sys.argv[1])
    rows = json.loads((output / "receipt.json").read_text())
    live = {
        row["profile"]: Path(row["generation"])
        for row in rows
        if row["embedder"] == "vllm"
    }
    results = {}
    for path in sorted(live["catalog"].glob("catalog_*.arrow")):
        catalog = ipc.open_file(path).read_all()
        behavioral = ipc.open_file(live["behavioral"] / path.name).read_all()
        # FactSink::fact includes run_id; these two identities derive from fact IDs.
        excluded = [
            name for name in catalog.column_names
            if name.endswith("fact_id")
            or name in {"evidence_id", "binding_id", "link_id", "brief_status", "brief_reason"}
        ]
        names = [name for name in catalog.column_names if name not in excluded]

        def canonical(table):
            return Counter(
                json.dumps(row, sort_keys=True, default=lambda value: value.hex())
                for row in table.select(names).to_pylist()
            )

        results[path.stem] = {
            "catalog_rows": catalog.num_rows,
            "behavioral_rows": behavioral.num_rows,
            "same_contract_multiset": canonical(catalog) == canonical(behavioral),
            "excluded": excluded,
        }
    assert len(results) == 12, "expected twelve catalog projection relations"
    print(json.dumps(results, indent=2))
    assert all(row["same_contract_multiset"] for row in results.values())


if __name__ == "__main__":
    main()
