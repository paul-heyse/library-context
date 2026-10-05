#!/usr/bin/env python3
"""Nested-number record-id collision with digest-based array keys (3.2->3.3 upgrade guide: ids
differing only in a nested number's type share one doc id in standard/HNSW/DiskANN indexes).
Probe: [digest, 1] vs [digest, 1dec] vs [digest, 1f]; control: the same keys as strings."""
import json
import os

import sdb

HERE = os.path.dirname(os.path.abspath(__file__))
D = "9" * 64


def run():
    sdb.sql("DEFINE DATABASE IF NOT EXISTS collide;", db="x")
    q = lambda t: sdb.sql(t, db="collide", check=False)
    q("REMOVE TABLE IF EXISTS num; REMOVE TABLE IF EXISTS str;")
    out = {}
    for tab, keys in (("num", ["1", "1dec", "1f"]), ("str", ["'1'", "'1dec'", "'1f'"])):
        q(f"DEFINE TABLE {tab} SCHEMAFULL; DEFINE FIELD g ON {tab} TYPE string; DEFINE FIELD e ON {tab} TYPE array<float>;"
          f"DEFINE INDEX gi ON {tab} FIELDS g; DEFINE INDEX ei ON {tab} FIELDS e HNSW DIMENSION 3 DIST EUCLIDEAN;")
        for i, k in enumerate(keys):
            q(f"CREATE {tab}:['{D}', {k}] SET g = 'x', e = [1.0, {i}.0, 0.0];")
        res = q(f"SELECT VALUE id FROM {tab};"
                f"SELECT VALUE id FROM {tab} WHERE g = 'x';"
                f"SELECT VALUE id FROM {tab} WITH NOINDEX WHERE g = 'x';"
                f"SELECT VALUE id FROM {tab} WHERE e <|3,40|> [1.0, 0.0, 0.0];"
                f"EXPLAIN SELECT VALUE id FROM {tab} WHERE g = 'x';")
        out[tab] = {"all": res[0]["result"], "std_index": res[1]["result"], "noindex": res[2]["result"],
                    "hnsw_knn3": res[3]["result"], "explain": res[4]["result"]}
    n = out["num"]
    collided = len(set(map(str, n["std_index"]))) < 3 or len(set(map(str, n["hnsw_knn3"]))) < 3
    ok_ctrl = len(set(map(str, out["str"]["std_index"]))) == 3 and len(set(map(str, out["str"]["hnsw_knn3"]))) == 3
    out["verdict"] = {"numeric_keys_collide_in_index": collided, "string_keys_control_distinct": ok_ctrl}
    json.dump(out, open(os.path.join(HERE, "raw/idcollision.json"), "w"), indent=1, default=str)
    print(json.dumps(out["verdict"]), json.dumps({k: {kk: v[kk] for kk in ("all", "std_index", "hnsw_knn3")} for k, v in out.items() if k != "verdict"}))


if __name__ == "__main__":
    run()
