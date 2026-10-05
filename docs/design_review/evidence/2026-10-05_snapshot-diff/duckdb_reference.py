"""Independent check of ref.json: recompute the identity diff from the CSVs with DuckDB.
uv run --no-project --with duckdb python .../duckdb_reference.py <data_dir>"""
import duckdb, sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import load, compare
d = sys.argv[1]; schema, ref = load(d); con = duckdb.connect(); got = {}
for rel in schema:
    a = f"read_csv('{d}/A/{rel}.csv', header=true, all_varchar=true)"
    b = f"read_csv('{d}/B/{rel}.csv', header=true, all_varchar=true)"
    q = lambda s: {r[0] for r in con.execute(s).fetchall()}
    got[rel] = {"added": q(f"select id from {b} except select id from {a}"),
                "removed": q(f"select id from {a} except select id from {b}"),
                "changed": q(f"select a.id from {a} a join {b} b using (id) where a.pdig <> b.pdig")}
sys.exit(0 if compare("duckdb", ref, got) else 1)
