import json, time, urllib.request
def q(sql, db="g1"):
    r = urllib.request.Request("http://127.0.0.1:18765/sql", data=sql.encode(), headers={"Accept":"application/json","Surreal-NS":"a3","Surreal-DB":db})
    return json.load(urllib.request.urlopen(r, timeout=600))
N, B = 100_000, 2_000
t0 = time.time()
for b in range(0, N, B):
    rows = [{"id": [i, f"k{i}"], **{f"f{f}": (i if f%2 else f"s{i}_{f}") for f in range(10)}} for i in range(b, b+B)]
    res = q("INSERT INTO t1 " + json.dumps(rows) + " RETURN NONE;")
    assert res[0]["status"] == "OK", res[0]
t1 = time.time()
print("insert rows/s", round(N/(t1-t0)))
print(q("SELECT count() FROM t1 GROUP ALL;")[0]["result"])
t2=time.time(); print(q("RETURN array::len(SELECT VALUE id FROM t1 WHERE f1 = 4242 AND f0 = 's4242_0');")[0]); print("unique-index lookup s", round(time.time()-t2,3))
t3=time.time(); print(q("REMOVE DATABASE g1;", db="g1")[0]["status"]); print("remove db s", round(time.time()-t3,2))
