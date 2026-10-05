#!/usr/bin/env python3
"""Synthetic/model-shaped probes: P1 journey closure and P2 snapshot mechanics on a persistent
SurrealDB 3.3 server (RocksDB). Writes raw/synthetic-results.json and raw/synthetic-run.log."""
import copy
import json
import os
import random
import threading
import time
import traceback

import dataset
import journeys
import pgref
import sdb
import snapshot
from synth import Synth

HERE = os.path.dirname(os.path.abspath(__file__))
LOG = open(os.path.join(HERE, "raw/synthetic-run.log"), "w")
RESULTS = {}


def log(*a):
    line = " ".join(str(x) for x in a)
    print(line)
    LOG.write(line + "\n")
    LOG.flush()


def outcome(name, status, **detail):
    RESULTS[name] = {"status": status, **detail}
    log(f"[{status}] {name} {json.dumps(detail, default=str)[:600]}")


def reset(model):
    sdb.sql("DEFINE NAMESPACE IF NOT EXISTS w1;", db="x")
    sdb.sql("REMOVE DATABASE IF EXISTS content; DEFINE DATABASE content STRICT;", db="x")
    sdb.sql(open(os.path.join(sdb.BUILD, "schema.surql")).read())
    sdb.sql("CREATE reader:a SET name = 'a', pass = crypto::argon2::generate('pw-a');"
            "CREATE reader:b SET name = 'b', pass = crypto::argon2::generate('pw-b');"
            "CREATE reader:c SET name = 'c', pass = crypto::argon2::generate('pw-c');")


def surreal_journeys(token, cases):
    out = {}
    for case, m in cases.items():
        res = sdb.sql(journeys.surreal_query(m), token=token)
        out[case] = journeys.normalise_surreal(res[-1]["result"])
    return out


def pg_journeys(schema, cases):
    return {case: journeys.run_pg(pgref.DB, schema, m, read_only=True) for case, m in cases.items()}


def compare(name, sur, pg):
    diffs = {c: {"surreal": sur[c], "pg": pg[c]} for c in sur if sur[c] != pg[c]}
    json.dump({"surreal": sur, "pg": pg}, open(os.path.join(HERE, f"raw/{name}.json"), "w"), indent=1, sort_keys=True)
    return diffs


def live_count(table, s):
    return sdb.sql(f"SELECT VALUE count() FROM {table} WHERE fn::live(spans, {s}) GROUP ALL;")[0]["result"][0]


def main():
    model = snapshot.Model(os.path.join(sdb.BUILD, "model-describe.json"))
    reset(model)
    rows1, cases = dataset.build(model)
    log("dataset S1:", sum(len(v) for v in rows1.values()), "rows in", len(rows1), "relations; cases", json.dumps(cases))

    # ---- S1: load, validate, publish
    snapshot.load(model, 1, rows1, log=log)
    fails = snapshot.validate(model, 1, log=log)
    outcome("p2.s1.validate", "passed" if not fails else "failed", failures=fails)
    snapshot.publish(1)

    # ---- P1 journeys on S1 vs PostgreSQL reference
    pgref.create_db()
    pgref.load(model, "s1", rows1)
    tok_a = sdb.signin("a", "pw-a")
    sur = surreal_journeys(tok_a, cases)
    pg = pg_journeys("s1", cases)
    diffs = compare("journeys-s1", sur, pg)
    outcome("p1.journeys.s1.closure-equal", "passed" if not diffs else "failed", cases=list(cases), diffs=diffs)
    flags = {
        "truncated_page": (sur["truncated"]["j1"]["total"], len(sur["truncated"]["j1"]["page"]), sur["truncated"]["j1"]["omitted"], sur["truncated"]["j1"]["truncated"]),
        "absent_member_visible": sur["missing"]["member_visible"],
        "none_member": (sur["none"]["member_visible"], sur["none"]["j1"]["total"], len(sur["none"]["j2"]), len(sur["none"]["j3"]["facets"])),
        "mixed_unavailable_vs_facets": (sur["mixed"]["j3"]["unavailable"], len(sur["mixed"]["j3"]["facets"])),
        "mixed_default_kinds": sorted(o["default_kind"] for o in sur["mixed"]["j2"]),
        "contract_case_ok": sur["contract"]["j3"]["contract_ok"],
    }
    ok = (flags["truncated_page"] == (130, 100, 30, True) and flags["absent_member_visible"] is False
          and flags["none_member"] == (True, 0, 0, 0) and flags["mixed_unavailable_vs_facets"] == (1, 2)
          and flags["mixed_default_kinds"] == [0, 1, 2, 3] and flags["contract_case_ok"] is False)
    outcome("p1.journeys.edge-cases", "passed" if ok else "failed", **flags)

    # ---- P2 seeded integrity violations under the load path (staging snapshot 2, then abort)
    before = {t: live_count(t, 1) for t in ("catalog_members", "catalog_member_invocations", "synthesis_briefs", "summary_analysis_outcomes")}
    sy = Synth(model, seed=99)

    def staged(label, mutate, expect):
        rows = copy.deepcopy(rows1)
        mutate(rows)
        refused, fails = None, None
        try:
            snapshot.load(model, 2, rows, log=log)
            fails = snapshot.validate(model, 2, log=log)
        except Exception as e:  # server refusal surfaces as a statement error
            refused = str(e)[:500]
        state = sdb.sql("SELECT VALUE state FROM ONLY snapshot:2;")[0]["result"]
        try:
            snapshot.publish(2)
            published = True
        except Exception:
            published = False
        snapshot.abort(model, 2)
        sdb.sql("DELETE snapshot:2;")
        after = {t: live_count(t, 1) for t in before}
        caught = (refused is not None) if expect == "refused" else bool(fails)
        outcome(f"p2.seed.{label}", "passed" if caught and not published and after == before else "failed",
                expected=expect, refused=refused, validation_failures=fails, staging_state=state,
                published=published, s1_intact=after == before)

    def dangling(rows):
        rows["catalog_member_invocations"].append(dict(rows["catalog_member_invocations"][0], id=sy.hexid(), member=sy.hexid()))
    staged("dangling-reference", dangling, "validation")

    def dup_changed(rows):  # same content id as an S1 record, different key content
        r = rows["catalog_members"][1]
        r["path"] = list(r["path"]) + ["tampered"]
    staged("duplicate-id-different-content", dup_changed, "refused")

    def type_violation(rows):
        f = next(f for f in model.rel["summary_analysis_outcomes"]["fields"] if f["name"] == "status")
        rows["summary_analysis_outcomes"][0] = dict(rows["summary_analysis_outcomes"][0], id=sy.hexid(), status="'not-an-int'")
    staged("type-violation", type_violation, "refused")

    def codebook(rows):
        rows["summary_analysis_outcomes"][0] = dict(rows["summary_analysis_outcomes"][0], id=sy.hexid(), status=9999)
    staged("out-of-codebook", codebook, "refused")

    # duplicate id inside one INSERT statement (server-level, independent of the loader's own check)
    try:
        r = rows1["input_revisions"][0]
        sdb.sql(f"INSERT INTO input_revisions [{{id: input_revisions:`ffff{r['id'][4:]}`, manifest: '{r['manifest']}', spans: [[9]]}}, {{id: input_revisions:`ffff{r['id'][4:]}`, manifest: '{r['manifest']}', spans: [[9]]}}];")
        outcome("p2.seed.duplicate-id-one-statement", "failed", note="accepted")
    except Exception as e:
        cnt = sdb.sql(f"SELECT VALUE count() FROM input_revisions WHERE spans = [[9]] GROUP ALL;")[0]["result"]
        outcome("p2.seed.duplicate-id-one-statement", "passed" if not cnt or cnt == [0] else "failed", refused=str(e)[:300], stored=cnt)

    # loader-side duplicate (two rows with one id in one snapshot)
    try:
        rows = copy.deepcopy(rows1)
        rows["catalog_members"].append(dict(rows["catalog_members"][0], name="other"))
        snapshot.load(model, 2, rows, log=log)
        outcome("p2.seed.duplicate-id-in-snapshot", "failed")
    except ValueError as e:
        snapshot.abort(model, 2)
        sdb.sql("DELETE snapshot:2;")
        outcome("p2.seed.duplicate-id-in-snapshot", "passed", refused_by="loader", detail=str(e))

    # ---- immutability
    imm = {}
    for label, q, tok in [
        ("reader-update", f"UPDATE catalog_members:`{cases['mixed']}` SET spans = [];", tok_a),
        ("reader-create", "CREATE input_revisions:`00` SET manifest = '" + "0" * 64 + "', spans = [[1]];", tok_a),
        ("reader-delete", f"DELETE catalog_members:`{cases['mixed']}`;", tok_a),
        ("owner-content-update", f"UPDATE catalog_members:`{cases['mixed']}` SET path = ['x'];", None)]:
        res = sdb.sql(q, token=tok, check=False)[0]
        imm[label] = (res["status"], str(res["result"])[:200])
    still = sdb.sql(f"SELECT VALUE path FROM ONLY catalog_members:`{cases['mixed']}`;")[0]["result"]
    imm["content_unchanged"] = still == next(r for r in rows1["catalog_members"] if r["id"] == cases["mixed"])["path"]
    imm["reader_create_had_no_effect"] = sdb.sql("SELECT VALUE id FROM input_revisions:`00`;")[0]["result"] == []
    imm["reader_delete_had_no_effect"] = sdb.sql(f"SELECT VALUE id FROM catalog_members:`{cases['mixed']}`;")[0]["result"] != []
    imm["reader_update_had_no_effect"] = sdb.sql(f"SELECT VALUE spans FROM ONLY catalog_members:`{cases['mixed']}`;")[0]["result"] == [[1]]
    outcome("p2.immutability", "passed" if imm["owner-content-update"][0] == "ERR" and imm["content_unchanged"]
            and imm["reader_create_had_no_effect"] and imm["reader_delete_had_no_effect"] and imm["reader_update_had_no_effect"] else "failed",
            note="reader writes return OK with [] (silently filtered by PERMISSIONS NONE), not an error", **imm)

    # ---- S2: perturbed, structurally shared; concurrent pinned reader
    rows2 = copy.deepcopy(rows1)
    briefs = rows2["synthesis_briefs"]
    referenced = set()
    for k, v in rows2.items():
        for f in model.rel[k]["fields"]:
            if f["type"] == "id" and f["target"] == "synthesis_briefs":
                referenced |= {r[f["name"]] for r in v}
    trunc_seeds = set(sur["truncated"]["j1"]["seeds"])
    tb = [b for b in briefs if b["seed"] in trunc_seeds]
    for b in tb[:10]:
        b["title"] = b"changed title"
    removed = {b["id"] for b in tb[10:15] if b["id"] not in referenced}
    rows2["synthesis_briefs"] = [b for b in briefs if b["id"] not in removed]
    sy2 = Synth(model, seed=123)
    sy2.rows = rows2
    mixed_seed = sur["mixed"]["j1"]["seeds"][0]
    for _ in range(3):
        sy2.make("synthesis_briefs", seed=mixed_seed)
    newm = sy2.make("catalog_members")
    sy2.make("catalog_options", member=newm["id"])
    cases2 = dict(cases, new_member=newm["id"])

    s1_view = surreal_journeys(tok_a, cases2)
    observations, stop = [], threading.Event()

    def poll():
        while not stop.is_set():
            try:
                view = surreal_journeys(tok_a, cases2)
                snap2 = sdb.sql("SELECT * FROM snapshot:2;", token=tok_a)[0]["result"]
                tok_b = sdb.signin("b", "pw-b")
                b_new = sdb.sql(f"SELECT VALUE id FROM catalog_members:`{newm['id']}`;", token=tok_b)[0]["result"]
                observations.append({"same_as_s1": view == s1_view, "snapshot2_visible": bool(snap2), "mid_load_reader_sees_new": bool(b_new)})
            except Exception as e:
                observations.append({"error": str(e)[:300]})
            time.sleep(0.05)

    t = threading.Thread(target=poll, daemon=True)
    t.start()
    stats2, _ = snapshot.load(model, 2, rows2, log=log)
    fails2 = snapshot.validate(model, 2, log=log)
    stop.set()
    t.join()
    bad = [o for o in observations if o != {"same_as_s1": True, "snapshot2_visible": False, "mid_load_reader_sees_new": False}]
    outcome("p2.no-partial-visibility", "passed" if observations and not bad else "failed",
            polls=len(observations), deviations=bad[:5])
    snapshot.publish(2)
    a_after = surreal_journeys(tok_a, cases2)
    tok_c = sdb.signin("c", "pw-c")
    c_view = surreal_journeys(tok_c, cases2)
    pgref.load(model, "s2", rows2)
    pg2 = pg_journeys("s2", cases2)
    pg1 = pg_journeys("s1", cases2)
    d1 = compare("journeys-s1-pinned-after-publish", a_after, pg1)
    d2 = compare("journeys-s2", c_view, pg2)
    outcome("p2.pinned-reader-after-publish", "passed" if not d1 and a_after == s1_view else "failed", diffs=d1)
    outcome("p2.s2-reader-equals-reference", "passed" if not d2 and not fails2 else "failed", diffs=d2, validation_failures=fails2)
    outcome("p2.structural-sharing", "passed" if stats2["identity"]["shared"] > 0 and stats2["payload"]["new"] >= 10 else "failed",
            load_stats=stats2, changed_payloads=10, removed=len(removed), added_briefs=3)

    # ---- retirement with leases
    leases = sdb.sql("SELECT snap, reader, count() AS n FROM lease GROUP BY snap, reader;")[0]["result"]
    live_leases_s1 = sdb.sql("SELECT VALUE count() FROM lease WHERE snap = 1 AND expires > time::now() GROUP ALL;")[0]["result"]
    gc_q = "DELETE {t} WHERE spans.all(|$p| array::len($p) = 2 AND $p[1] <= 2) RETURN BEFORE;"
    blocked = bool(live_leases_s1)
    sdb.sql("UPDATE lease SET expires = time::now() - 1s WHERE snap = 1;")
    sdb.sql("UPDATE snapshot:1 SET state = 'retired';")
    deleted = 0
    for name in model.rel:
        for table in [name] + ([name + "__p"] if model.pay(name) else []):
            deleted += len(sdb.sql(gc_q.format(t=table))[0]["result"] or [])
    c_after_gc = surreal_journeys(tok_c, cases2)
    outcome("p2.retire-gc", "passed" if blocked and deleted and c_after_gc == c_view else "failed",
            leases=leases, gc_blocked_while_lease=blocked, records_deleted=deleted, s2_unchanged=c_after_gc == c_view)
    json.dump(RESULTS, open(os.path.join(HERE, "raw/synthetic-results.json"), "w"), indent=1, sort_keys=True, default=str)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        log(traceback.format_exc())
        json.dump(RESULTS, open(os.path.join(HERE, "raw/synthetic-results.json"), "w"), indent=1, sort_keys=True, default=str)
        raise
