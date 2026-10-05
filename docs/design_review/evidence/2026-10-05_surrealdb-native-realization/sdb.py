"""Minimal stdlib HTTP client for the probe server (SurrealDB 3.3 /sql and /signin)."""
import base64
import json
import os
import urllib.error
import urllib.request

URL = os.environ.get("W1_SURREAL_URL", "http://127.0.0.1:18800")
BUILD = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../build/review-probes/surrealdb-native-realization")
NS, DB = "w1", "content"


def _root_auth():
    pw = open(os.path.join(BUILD, "root.pass")).read().strip()
    return "Basic " + base64.b64encode(f"root:{pw}".encode()).decode()


def sql(text, token=None, ns=NS, db=DB, check=True):
    """Run SurrealQL; return list of statement results. check=True raises on any ERR."""
    req = urllib.request.Request(URL + "/sql", data=text.encode(), method="POST", headers={
        "Accept": "application/json", "Surreal-NS": ns, "Surreal-DB": db,
        "Authorization": ("Bearer " + token) if token else _root_auth()})
    try:
        res = json.load(urllib.request.urlopen(req, timeout=3600))
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")
        raise RuntimeError(f"HTTP {e.code}: {body[:2000]}") from None
    if check:
        errs = [(i, r["result"]) for i, r in enumerate(res) if r["status"] != "OK"]
        if errs:
            raise RuntimeError(f"statement errors: {errs[:5]}")
    return res


def signin(name, password, access="agent", ns=NS, db=DB):
    body = json.dumps({"ns": ns, "db": db, "ac": access, "name": name, "pass": password}).encode()
    req = urllib.request.Request(URL + "/signin", data=body, method="POST",
                                 headers={"Accept": "application/json", "Content-Type": "application/json"})
    res = json.load(urllib.request.urlopen(req, timeout=60))
    return res["token"]
