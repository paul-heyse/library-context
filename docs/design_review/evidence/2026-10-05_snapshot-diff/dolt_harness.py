"""Dolt 2.4.1 as a snapshot store: load A, commit/tag, full rebuild to B, commit/tag, dolt_diff.

Server:
  docker run -d --name lctx-probe-dolt -e DOLT_ROOT_PASSWORD=root -e DOLT_ROOT_HOST=% \
    -p 127.0.0.1:13306:3306 -v lctx-probe-dolt:/var/lib/dolt dolthub/dolt-sql-server:2.4.1
  uv run --no-project --with pymysql python .../dolt_harness.py <data_dir> [db]
Cleanup: docker rm -f lctx-probe-dolt; docker volume rm lctx-probe-dolt

B is loaded as a *full rebuild* (delete every row, insert every B row), as a producer that
rebuilds from pinned inputs would; Dolt's content-addressed prolly trees are expected to share
the unchanged chunks without being told what changed. Checks: FK and CHECK enforcement on load,
FOREIGN_KEY_CHECKS=0 plus DOLT_VERIFY_CONSTRAINTS, dolt_diff correctness, storage growth, and the
keyed semantic diff with AS OF.
"""
import csv, pathlib, subprocess, sys, pymysql
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import load, compare

D = pathlib.Path(sys.argv[1]).resolve()
DB = sys.argv[2] if len(sys.argv) > 2 else "snap"
schema, ref = load(D)
SQLT = {"id": "CHAR(32) CHARACTER SET ascii", "int": "BIGINT", "text": "TEXT"}
con = pymysql.connect(host="127.0.0.1", port=13306, user="root", password="root", autocommit=True)
cur = con.cursor()
def x(sql, args=None):
    cur.execute(sql, args); return cur.fetchall()
def order():
    done, out = set(), []
    while len(out) < len(schema):
        for rel, s in schema.items():
            if rel not in done and all(t in done or t == rel for t in s["refs"].values()):
                done.add(rel); out.append(rel)
    return out
def du():
    return int(subprocess.run(["docker", "exec", "lctx-probe-dolt", "du", "-sb", f"/var/lib/dolt/{DB}/.dolt"],
                              capture_output=True, text=True).stdout.split()[0])

SKIP = "--skip-load" in sys.argv
if not SKIP:
    x(f"DROP DATABASE IF EXISTS {DB}"); x(f"CREATE DATABASE {DB}")
x(f"USE {DB}")
for rel in ([] if SKIP else order()):
    s = schema[rel]; t = s["types"]
    cols = ", ".join(f"`{c}` {SQLT[t[c]]}{' NULL' if c in s['refs'] else ' NOT NULL'}" for c in s["columns"] + ["pdig"])
    fks = "".join(f", FOREIGN KEY (`{c}`) REFERENCES {tr}(id)" for c, tr in s["refs"].items())
    checks = "".join(f", CHECK (`{c}` >= 0)" for c in s.get("nonneg", [c for c, ty in t.items() if ty == "int"]))
    x(f"CREATE TABLE {rel} ({cols}, PRIMARY KEY (id){fks}{checks})")

def load_state(state):
    x("START TRANSACTION")
    for rel in reversed(order()):
        x(f"DELETE FROM {rel}")
    for rel in order():
        cols = schema[rel]["columns"] + ["pdig"]
        sql = f"INSERT INTO {rel} ({', '.join('`'+c+'`' for c in cols)}) VALUES ({', '.join(['%s'] * len(cols))})"
        batch = []
        with open(D / state / f"{rel}.csv") as fh:
            for r in csv.DictReader(fh):
                batch.append([None if (c in schema[rel]["refs"] and r[c] == "") else r[c] for c in cols])
                if len(batch) == 5000:
                    cur.executemany(sql, batch); batch.clear()
        if batch:
            cur.executemany(sql, batch)
    x("COMMIT")
    x("CALL DOLT_COMMIT('-Am', %s)", (f"state {state}",))
    x("CALL DOLT_TAG(%s, 'HEAD')", (f"s{state}",))

if not SKIP:
    load_state("A"); x("CALL DOLT_GC()"); size_a = du()
    load_state("B"); x("CALL DOLT_GC()"); size_ab = du()
    print(f"dolt storage (.dolt, after dolt_gc): A only {size_a} bytes; A+B {size_ab} bytes; "
          f"growth for B = {100 * (size_ab - size_a) / size_a:.1f}% of A")

got, stats = {}, []
for rel in schema:
    rows = x(f"SELECT diff_type, COALESCE(to_id, from_id) FROM dolt_diff('sA', 'sB', '{rel}')")
    got[rel] = {"added": {i for k, i in rows if k == "added"}, "removed": {i for k, i in rows if k == "removed"},
                "changed": {i for k, i in rows if k == "modified"}}
ok = compare("dolt-diff", ref, got)
print("dolt_diff_stat:", x("SELECT table_name, rows_added, rows_deleted, rows_modified FROM dolt_diff_stat('sA', 'sB') ORDER BY 1"))

# Enforcement on load
enf = []
for sql in ("INSERT INTO occurrences VALUES (REPEAT('a',32), REPEAT('b',32), 0, 1, 1, 0, '0', REPEAT('c',32))",
            "UPDATE source_artifacts SET byte_len = -1 LIMIT 1"):
    try:
        x(sql); enf.append(f"ACCEPTED: {sql[:60]}")
    except pymysql.MySQLError as e:
        enf.append(f"refused {e.args[0]}: {sql[:60]} -> {str(e.args[1])[:80]}")
BAD = "INSERT INTO occurrences VALUES (REPEAT('a',32), REPEAT('b',32), 0, 1, 1, 0, '0', REPEAT('c',32))"
COMMIT_SQL, VERIFY_SQL = "CALL DOLT_COMMIT('-am', 'dangling row')", "CALL DOLT_VERIFY_CONSTRAINTS('--all')"
def r(sql):
    try:
        return f"OK {x(sql)}"
    except pymysql.MySQLError as e:
        return f"ERR {e.args[0]} {str(e.args[1])[:110]}"
x("SET FOREIGN_KEY_CHECKS = 0")
enf.append(f"FOREIGN_KEY_CHECKS=0 dangling insert: {r(BAD)}")
x("SET FOREIGN_KEY_CHECKS = 1")
enf.append(f"DOLT_COMMIT of the dangling working set: {r(COMMIT_SQL)}")
enf.append(f"DOLT_VERIFY_CONSTRAINTS('--all'): {r(VERIFY_SQL)}")
x("SET @@dolt_force_transaction_commit = 1")
enf.append(f"same with dolt_force_transaction_commit=1: {r(VERIFY_SQL)} "
           f"violations {r('SELECT * FROM dolt_constraint_violations')}")
x("SET @@dolt_force_transaction_commit = 0")
x("CALL DOLT_RESET('--hard', 'sB')")
enf.append(f"after DOLT_RESET --hard sB: status {x('SELECT * FROM dolt_status')} head {x('SELECT message FROM dolt_log LIMIT 1')}")
print("dolt enforcement:", *enf, sep="\n  ")

# Keyed semantic diff across tags (correspondence key from ref["semantic_spec"])
okS = True
sp = ref.get("semantic_spec")
if sp and "semantic" in ref:
    def sem(tag):
        k = ", '|', ".join([f"COALESCE(p.`{c}`, '')" for c in sp["key_sym"]] + [f"COALESCE(s.`{c}`, '')" for c in sp["key_sig"]])
        return dict(x(f"SELECT CONCAT({k}), s.`{sp['digest']}` FROM {sp['sig']} AS OF '{tag}' s "
                      f"JOIN {sp['sym']} AS OF '{tag}' p ON p.id = s.`{sp['sym_ref']}`"))
    sa, sb = sem("sA"), sem("sB")
    semantic = {"added": sb.keys() - sa.keys(), "removed": sa.keys() - sb.keys(),
                "changed": {k for k in sa.keys() & sb.keys() if sa[k] != sb[k]}}
    okS = compare("dolt-semantic", {"relations": {}, "semantic": ref["semantic"]}, {}, semantic)
print("dolt OVERALL", "passed" if ok and okS else "failed")
