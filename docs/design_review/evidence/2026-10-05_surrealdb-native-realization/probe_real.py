#!/usr/bin/env python3
"""Real-generation probes (P1 journeys, P2 full load / integrity / visibility / pinning / sharing).

Usage: probe_real.py <generation-id-hex> [--workers N]
Reads the published generation read-only (database lctx, schema lctx_g<id>); writes only the
probe SurrealDB server and the disposable PostgreSQL database w1_probe. Outputs raw/real-*.
No timings are recorded beyond qualitative completion; container memory is sampled to
raw/real-memory.tsv as the qualitative growth signal."""
import argparse
import copy
import json
import os
import subprocess
import threading
import time
import traceback

import journeys
import pgref
import real_source
import sdb
import snapshot
from synth import Synth

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = {}
PREFIX = ["real"]
LOG = None
JOURNEY_RELS = ["catalog_members", "catalog_member_invocations", "synthesis_selected_seeds", "synthesis_briefs",
                "catalog_options", "catalog_option_subjects", "catalog_option_evidence", "catalog_defaults",
                "synthesis_summary_facets", "summary_analysis_invocations", "summary_analysis_outcomes"]


def log(*a):
    line = " ".join(str(x) for x in a)
    print(line, flush=True)
    LOG.write(line + "\n")
    LOG.flush()


def outcome(name, status, **detail):
    RESULTS[name] = {"status": status, **detail}
    log(f"[{status}] {name} {json.dumps(detail, default=str)[:800]}")
    json.dump(RESULTS, open(os.path.join(HERE, f"raw/{PREFIX[0]}-results.json"), "w"), indent=1, sort_keys=True, default=str)


def mem_sampler(stop):
    with open(os.path.join(HERE, f"raw/{PREFIX[0]}-memory.tsv"), "w") as f:
        f.write("utc\tmem_usage\tphase\n")
        while not stop.is_set():
            u = subprocess.run(["docker", "stats", "--no-stream", "--format", "{{.MemUsage}}", "w1sdb"],
                               capture_output=True, text=True).stdout.strip()
            f.write(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\t{u}\t{PHASE[0]}\n")
            f.flush()
            stop.wait(15)


PHASE = ["start"]


DBNAME = ["lctx"]


def pg(schema, sql):
    return journeys.psql(DBNAME[0], "SET default_transaction_read_only = on;\n" + sql.replace("{s}", schema))


def pick_cases(schema):
    q = lambda sql: [l for l in pg(schema, sql).splitlines() if l and l != "SET"]
    cases = {}
    r = q("SELECT encode(m.id,'hex') FROM {s}.catalog_members m JOIN {s}.catalog_member_invocations i ON i.member = m.id "
          "JOIN {s}.synthesis_selected_seeds s ON s.member = i.id JOIN {s}.synthesis_briefs b ON b.seed = s.id "
          "GROUP BY m.id ORDER BY count(*) DESC, m.id LIMIT 1;")
    if r: cases["most_briefs"] = r[0]
    r = q("SELECT encode(m.id,'hex') FROM {s}.catalog_members m WHERE NOT EXISTS (SELECT 1 FROM {s}.catalog_member_invocations i WHERE i.member = m.id) ORDER BY m.id LIMIT 1;")
    if r: cases["no_invocations"] = r[0]
    r = q("SELECT encode(i.member,'hex') FROM {s}.synthesis_summary_facets f JOIN {s}.catalog_member_invocations i ON i.id = f.member WHERE f.qualification IS NULL ORDER BY i.member LIMIT 1;")
    if r: cases["unavailable_facet"] = r[0]
    r = q("SELECT encode(member,'hex') FROM {s}.catalog_options GROUP BY member ORDER BY count(*) DESC, member LIMIT 1;")
    if r: cases["most_options"] = r[0]
    r = q("SELECT encode(m.id,'hex') FROM {s}.catalog_members m WHERE EXISTS (SELECT 1 FROM {s}.summary_analysis_invocations x WHERE x.input = m.input) ORDER BY m.id LIMIT 1;")
    if r: cases["with_summary_invocations"] = r[0]
    for i, h in enumerate(q("SELECT encode(id,'hex') FROM {s}.catalog_members ORDER BY md5(id::text) LIMIT 30;")):
        cases[f"sample{i}"] = h
    cases["missing"] = "0" * 31 + "1"
    return cases


def surreal_journeys(token, cases):
    return {c: journeys.normalise_surreal(sdb.sql(journeys.surreal_query(m), token=token)[-1]["result"]) for c, m in cases.items()}


def main():
    global LOG
    ap = argparse.ArgumentParser()
    ap.add_argument("generation")
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--skip-load", action="store_true", help="reuse an already loaded S1")
    ap.add_argument("--dry-schema", help="harness check: read this schema of database w1_probe instead of a generation")
    a = ap.parse_args()
    if a.dry_schema:
        DBNAME[0] = pgref.DB
        PREFIX[0] = "real-dryrun"
    LOG = open(os.path.join(HERE, "raw/real-run.log" if not a.dry_schema else "raw/real-dryrun.log"), "a")
    schema = a.dry_schema or real_source.schema_of(a.generation)
    model = snapshot.Model(os.path.join(sdb.BUILD, "model-describe.json"))
    stop = threading.Event()
    threading.Thread(target=mem_sampler, args=(stop,), daemon=True).start()
    prov = real_source.provider(model, schema, db=DBNAME[0])
    log("generation", a.generation, "schema", schema)
    counts = {l.split("|")[0]: int(l.split("|")[1]) for l in pg(schema, "SELECT relname || '|' || n_live_tup FROM pg_stat_user_tables WHERE schemaname = '" + schema + "';").splitlines() if "|" in l}
    log("source rows (pg_stat estimate, model relations):", sum(v for k, v in counts.items() if k in model.rel))

    if not a.skip_load:
        PHASE[0] = "reset"
        sdb.sql("DEFINE NAMESPACE IF NOT EXISTS w1;", db="x")
        sdb.sql("REMOVE DATABASE IF EXISTS content; DEFINE DATABASE content STRICT;", db="x")
        sdb.sql(open(os.path.join(sdb.BUILD, "schema.surql")).read())
        sdb.sql("CREATE reader:a SET name = 'a', pass = crypto::argon2::generate('pw-a');"
                "CREATE reader:b SET name = 'b', pass = crypto::argon2::generate('pw-b');"
                "CREATE reader:c SET name = 'c', pass = crypto::argon2::generate('pw-c');")
        PHASE[0] = "load-s1"
        try:
            stats1, man1 = snapshot.load(model, 1, prov, source=f"pg:{a.generation}", log=log, workers=a.workers, verify_shared=False)
        except Exception as e:
            outcome("p2.real.load-s1", "failed", error=str(e)[:1000])
            raise
        mism = {n: (m["rows"], counts.get(n)) for n, m in man1.items() if counts.get(n) is not None and m["rows"] != counts[n]}
        outcome("p2.real.load-s1", "passed", stats=stats1, rows=sum(m["rows"] for m in man1.values()), count_vs_pg_stat_estimate_mismatches=len(mism))
        PHASE[0] = "validate-s1"
        fails = snapshot.validate(model, 1, log=log, workers=a.workers)
        outcome("p2.real.validate-s1", "passed" if not fails else "failed", failures=fails[:50])
        if fails:
            return
        snapshot.publish(1)
    PHASE[0] = "journeys-s1"
    cases = pick_cases(schema)
    log("cases", json.dumps(cases))
    tok_a = sdb.signin("a", "pw-a")
    sur = surreal_journeys(tok_a, cases)
    ref = {c: journeys.run_pg(DBNAME[0], schema, m) for c, m in cases.items()}
    diffs = {c: {"surreal": sur[c], "pg": ref[c]} for c in cases if sur[c] != ref[c]}
    json.dump({"surreal": sur, "pg": ref}, open(os.path.join(HERE, f"raw/{PREFIX[0]}-journeys-s1.json"), "w"), indent=1, sort_keys=True)
    shape = {c: {"briefs": sur[c]["j1"]["total"], "truncated": sur[c]["j1"]["truncated"], "options": len(sur[c]["j2"]),
                 "facets": len(sur[c]["j3"]["facets"]), "unavailable": sur[c]["j3"]["unavailable"],
                 "j2_contract_ok": sur[c]["j2_contract_ok"], "j3_contract_ok": sur[c]["j3"]["contract_ok"]} for c in cases if not c.startswith("sample")}
    outcome("p1.real.journeys-closure-equal", "passed" if not diffs else "failed", cases=len(cases), shapes=shape, diff_cases=list(diffs))

    # ---- seeded violations at scale (staging S2, abort)
    PHASE[0] = "seeds"
    sy = Synth(model, seed=99)
    first = {n: prov(n) for n in ("catalog_member_invocations", "catalog_members", "summary_analysis_outcomes")}
    before = {t: sdb.sql(f"SELECT VALUE count() FROM {t} WHERE fn::live(spans, 1) GROUP ALL;")[0]["result"] for t in first}

    def staged(label, override, expect, verify):
        p2 = lambda n: override[n] if n in override else prov(n)
        refused, fails = None, None
        try:
            snapshot.load(model, 2, p2, source="seed:" + label, log=log, workers=a.workers, verify_shared=verify)
            fails = snapshot.validate(model, 2, log=log, workers=a.workers)
        except Exception as e:
            refused = str(e)[:500]
        try:
            snapshot.publish(2); published = True
        except Exception:
            published = False
        snapshot.abort(model, 2)
        sdb.sql("DELETE snapshot:2;")
        after = {t: sdb.sql(f"SELECT VALUE count() FROM {t} WHERE fn::live(spans, 1) GROUP ALL;")[0]["result"] for t in first}
        caught = (refused is not None) if expect == "refused" else bool(fails)
        outcome(f"p2.real.seed.{label}", "passed" if caught and not published and after == before else "failed",
                refused=refused, validation_failures=(fails or [])[:10], published=published, s1_intact=after == before)

    cmi = copy.deepcopy(first["catalog_member_invocations"])
    cmi.append(dict(cmi[0], id=sy.hexid(), member=sy.hexid()))
    staged("dangling-reference", {"catalog_member_invocations": cmi}, "validation", False)
    cm = copy.deepcopy(first["catalog_members"])
    cm[0]["path"] = list(cm[0]["path"]) + ["tampered"]
    staged("duplicate-id-different-content", {"catalog_members": cm}, "refused", True)
    so = copy.deepcopy(first["summary_analysis_outcomes"])
    if so:
        so[0] = dict(so[0], id=sy.hexid(), status="'not-an-int'")
        staged("type-violation", {"summary_analysis_outcomes": so}, "refused", False)
        so = copy.deepcopy(first["summary_analysis_outcomes"])
        so[0] = dict(so[0], id=sy.hexid(), status=9999)
        staged("out-of-codebook", {"summary_analysis_outcomes": so}, "refused", False)

    # ---- S2: perturbed with sharing; concurrent pinned reader
    PHASE[0] = "s2"

    class Lazy(dict):
        def __missing__(self, k):
            self[k] = prov(k)
            return self[k]
    sy2 = Synth(model, seed=123, reuse=1.0)
    sy2.rows = Lazy()
    briefs = sy2.rows["synthesis_briefs"]
    referenced = set()
    for n, rel in model.rel.items():
        for f in rel["fields"]:
            if f["type"] == "id" and f["target"] == "synthesis_briefs" and counts.get(n):
                referenced |= {r[f["name"]] for r in sy2.rows[n]}
    for b in briefs[:10]:
        b["title"] = b"changed title"
    removed = {b["id"] for b in briefs[10:40] if b["id"] not in referenced}
    removed = set(sorted(removed)[:5])
    sy2.rows["synthesis_briefs"] = [b for b in briefs if b["id"] not in removed]
    seed_id = sy2.rows["synthesis_selected_seeds"][0]["id"] if sy2.rows["synthesis_selected_seeds"] else None
    added = [sy2.make("synthesis_briefs", seed=seed_id)["id"] for _ in range(3)] if seed_id else []
    newm = sy2.make("catalog_members")
    sy2.make("catalog_options", member=newm["id"])
    cases2 = dict(cases, new_member=newm["id"])
    s1_view = surreal_journeys(tok_a, cases2)
    obs, stop2 = [], threading.Event()

    def poll():
        while not stop2.is_set():
            try:
                v = surreal_journeys(tok_a, {k: cases2[k] for k in list(cases2)[:6] + ["new_member"]})
                same = all(v[k] == s1_view[k] for k in v)
                tb = sdb.signin("b", "pw-b")
                seen = bool(sdb.sql(f"SELECT VALUE id FROM catalog_members:`{newm['id']}`;", token=tb)[0]["result"])
                obs.append({"same_as_s1": same, "mid_load_reader_sees_new": seen})
            except Exception as e:
                obs.append({"error": str(e)[:300]})
            stop2.wait(1.0)
    t = threading.Thread(target=poll, daemon=True)
    t.start()
    p2 = lambda n: sy2.rows[n] if n in sy2.rows else prov(n)
    stats2, _ = snapshot.load(model, 2, p2, source=f"pg:{a.generation}+perturbation", log=log, workers=a.workers, verify_shared=False)
    fails2 = snapshot.validate(model, 2, log=log, workers=a.workers)
    stop2.set(); t.join()
    bad = [o for o in obs if o != {"same_as_s1": True, "mid_load_reader_sees_new": False}]
    outcome("p2.real.no-partial-visibility", "passed" if obs and not bad else "failed", polls=len(obs), deviations=bad[:5])
    outcome("p2.real.validate-s2", "passed" if not fails2 else "failed", failures=fails2[:20])
    if fails2:
        return
    snapshot.publish(2)
    a_after = surreal_journeys(tok_a, cases2)
    ref1 = dict(ref, new_member=journeys.run_pg(DBNAME[0], schema, newm["id"]))
    d1 = {c for c in cases2 if a_after[c] != ref1[c]}
    outcome("p2.real.pinned-reader-after-publish", "passed" if not d1 else "failed", diff_cases=sorted(d1))
    tok_c = sdb.signin("c", "pw-c")
    c_view = surreal_journeys(tok_c, cases2)
    pgref.ensure_db()
    pgref.load(model, "real_s2", {n: p2(n) for n in JOURNEY_RELS})
    ref2 = {c: journeys.run_pg(pgref.DB, "real_s2", m) for c, m in cases2.items()}
    d2 = {c for c in cases2 if c_view[c] != ref2[c]}
    json.dump({"surreal": c_view, "pg": ref2}, open(os.path.join(HERE, f"raw/{PREFIX[0]}-journeys-s2.json"), "w"), indent=1, sort_keys=True)
    outcome("p2.real.s2-reader-equals-reference", "passed" if not d2 else "failed", diff_cases=sorted(d2))
    outcome("p2.real.structural-sharing", "passed" if stats2["identity"]["shared"] > 0 else "failed",
            load_stats=stats2, changed_payloads=10, removed=len(removed), added_briefs=len(added))
    PHASE[0] = "done"
    stop.set()


if __name__ == "__main__":
    try:
        main()
    except Exception:
        if LOG:
            log(traceback.format_exc())
        raise
