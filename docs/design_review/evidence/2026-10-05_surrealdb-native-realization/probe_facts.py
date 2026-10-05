#!/usr/bin/env python3
"""P2 at real scale on the staging generation's facts frontier (no published catalog exists).

Usage: probe_facts.py <generation-id-hex> [--workers N]
Data: ordinary facts relations from their main tables (stage receipts) plus vocabulary relations
sealed by `assemble` but never merged, read from their private delta tables (real_source.py).
Read-only on database lctx. Writes only the probe SurrealDB server and w1_probe.
Outputs raw/facts-*.json/.log/.tsv. No timings beyond qualitative completion; container memory is
sampled every 15 s as the qualitative growth signal."""
import argparse
import copy
import json
import os
import random
import subprocess
import threading
import time
import traceback

import journeys
import real_source
import sdb
import snapshot
from synth import Synth

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS, PHASE = {}, ["start"]
LOG = None


def log(*a):
    line = " ".join(str(x) for x in a)
    print(line, flush=True)
    LOG.write(line + "\n")
    LOG.flush()


def outcome(name, status, **detail):
    RESULTS[name] = {"status": status, **detail}
    log(f"[{status}] {name} {json.dumps(detail, default=str)[:900]}")
    json.dump(RESULTS, open(os.path.join(HERE, "raw/facts-results.json"), "w"), indent=1, sort_keys=True, default=str)


def mem_sampler(stop):
    with open(os.path.join(HERE, "raw/facts-memory.tsv"), "w") as f:
        f.write("utc\tcontainer_mem_usage\tphase\n")
        while not stop.is_set():
            u = subprocess.run(["docker", "stats", "--no-stream", "--format", "{{.MemUsage}}", "w1sdb"],
                               capture_output=True, text=True).stdout.strip()
            f.write(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\t{u}\t{PHASE[0]}\n")
            f.flush()
            stop.wait(15)


def pgq(sql):
    out = journeys.psql("lctx", "SET default_transaction_read_only = on;\n" + sql)
    return [l for l in out.splitlines() if l and l != "SET"]


def live(table, s, token=None):
    return sdb.sql(f"SELECT VALUE count() FROM {table} WHERE fn::live(spans, {s}) GROUP ALL;", token=token)[0]["result"]


def main():
    global LOG
    ap = argparse.ArgumentParser()
    ap.add_argument("generation")
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--skip-load", action="store_true")
    a = ap.parse_args()
    LOG = open(os.path.join(HERE, "raw/facts-run.log"), "a")
    schema = real_source.schema_of(a.generation)
    model = snapshot.Model(os.path.join(sdb.BUILD, "model-describe.json"))
    stop = threading.Event()
    threading.Thread(target=mem_sampler, args=(stop,), daemon=True).start()
    phys, counts = real_source.facts_frontier(a.generation)
    rels = set(counts)
    prov = real_source.provider(model, schema, physical=phys)
    state = pgq(f"SELECT state || '|' || frontier || '|' || profile FROM lctx_model_store.generations WHERE id = decode('{a.generation}','hex');")
    log("generation", a.generation, "state|frontier|profile", state, "relations", len(rels), "nonempty", sum(1 for c in counts.values() if c),
        "receipted rows", sum(counts.values()), "unmerged vocabulary deltas", len(phys))
    order = sorted(rels, key=lambda n: -counts[n])

    # ---- S1: full facts frontier
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
            stats1, man1 = snapshot.load(model, 1, prov, source=f"pg:{a.generation}:facts-frontier(staging,unvalidated)",
                                         log=log, workers=a.workers, verify_shared=False, order=order)
        except Exception as e:
            outcome("p2.facts.load-s1", "failed", error=str(e)[:1500])
            raise
        mism = {n: (m["rows"], counts[n]) for n, m in man1.items() if m["rows"] != counts[n]}
        stored = {n: live(n, 1) for n in order[:5]}
        outcome("p2.facts.load-s1", "passed" if not mism else "failed", stats=stats1, rows=sum(m["rows"] for m in man1.values()),
                receipt_count_mismatches=mism, stored_live_counts_top5=stored)
        PHASE[0] = "validate-s1"
        fails = snapshot.validate(model, 1, log=log, workers=a.workers, relations=rels)
        outcome("p2.facts.validate-s1", "passed" if not fails else "failed", checks=len(snapshot.validation_queries(model, 1, rels)), failures=fails[:50])
        snapshot.publish(1) if not fails else None
        if fails:
            return

    # ---- J4 facts evidence-closure journey vs PostgreSQL joins over the same staging data
    PHASE[0] = "journey-j4"
    t = lambda n: f"{schema}.{phys.get(n, n)}"
    cases = {}
    r = pgq(f"SELECT encode(subject,'hex') FROM {t('type_observations')} GROUP BY subject ORDER BY count(*) DESC, subject LIMIT 1;")
    cases["most_observations"] = r[0]
    r = pgq(f"SELECT encode(o.id,'hex') FROM {t('occurrences')} o WHERE NOT EXISTS (SELECT 1 FROM {t('type_observations')} x WHERE x.subject = o.id) ORDER BY o.id LIMIT 1;")
    cases["no_observations"] = r[0]
    for i, h in enumerate(pgq(f"SELECT DISTINCT ON (md5(subject::text)) encode(subject,'hex') FROM {t('type_observations')} ORDER BY md5(subject::text) LIMIT 30;")):
        cases[f"sample{i}"] = h
    cases["missing"] = "0" * 31 + "1"
    tok_a = sdb.signin("a", "pw-a")
    surj = lambda tok, cs: {c: journeys.facts_normalise_surreal(sdb.sql(journeys.facts_surreal_query(o), token=tok)[-1]["result"]) for c, o in cs.items()}
    sur = surj(tok_a, cases)
    ref = {c: journeys.facts_run_pg("lctx", schema, phys, o) for c, o in cases.items()}
    diffs = [c for c in cases if sur[c] != ref[c]]
    json.dump({"cases": cases, "surreal": sur, "pg": ref}, open(os.path.join(HERE, "raw/facts-journey-j4.json"), "w"), indent=1, sort_keys=True)
    outcome("p1.facts.j4-evidence-closure-equal", "passed" if not diffs else "failed", cases=len(cases), diff_cases=diffs,
            most_observations=(sur["most_observations"]["total"], len(sur["most_observations"]["page"]), sur["most_observations"]["truncated"]),
            no_observations=(sur["no_observations"]["visible"], sur["no_observations"]["total"]), missing_visible=sur["missing"]["visible"],
            supports_checked=sum(len(p["supports"]) for v in sur.values() for p in v["page"]))

    # ---- seeded violations (staging S2 sharing unchanged relations with S1; journal abort)
    PHASE[0] = "seeds"
    sy = Synth(model, seed=99)
    probe_tables = ["type_supports", "occurrences", "evidence", "type_observations"]
    before = {x: live(x, 1) for x in probe_tables}
    base_rows = {n: prov(n) for n in ("type_supports", "occurrences")}

    def staged(label, override, expect, verify=False):
        journal = {}
        p2 = lambda n: override[n] if n in override else prov(n)
        refused, fails = None, None
        try:
            _, man = snapshot.load(model, 2, p2, source="seed:" + label, log=log, workers=a.workers,
                                   verify_shared=set(override) if verify else False, relations=rels, base=1, journal=journal, order=order,
                                   carry=rels - set(override))
            changed = {n for n, m in man.items() if not m["unchanged"]}
            fails = snapshot.validate(model, 2, log=log, workers=a.workers, relations=rels, changed=changed)
        except Exception as e:
            refused = str(e)[:600]
            changed = None
        try:
            snapshot.publish(2); published = True
        except Exception:
            published = False
        snapshot.abort(model, 2, journal=journal)
        sdb.sql("DELETE snapshot:2;")
        after = {x: live(x, 1) for x in probe_tables}
        leftover = (sdb.sql("SELECT VALUE count() FROM type_supports WHERE spans CONTAINS [2] GROUP ALL;")[0]["result"] or [0])[0]
        caught = (refused is not None) if expect == "refused" else bool(fails)
        outcome(f"p2.facts.seed.{label}", "passed" if caught and not published and after == before and not leftover else "failed",
                expected=expect, refused=refused, validation_failures=(fails or [])[:10], delta_changed_relations=sorted(changed or []),
                published=published, s1_intact=after == before, staging_leftovers=leftover)

    ts = copy.deepcopy(base_rows["type_supports"])
    ts.append(dict(ts[0], id=sy.hexid(), evidence=sy.hexid()))
    staged("dangling-reference", {"type_supports": ts}, "validation")
    oc = copy.deepcopy(base_rows["occurrences"])
    oc[0]["start"] = oc[0]["start"] + 1
    staged("duplicate-id-different-content", {"occurrences": oc}, "refused", verify=True)
    ts = copy.deepcopy(base_rows["type_supports"])
    ts.append(dict(ts[0], id=sy.hexid(), fidelity="'not-an-int'"))
    staged("type-violation", {"type_supports": ts}, "refused")
    ts = copy.deepcopy(base_rows["type_supports"])
    ts.append(dict(ts[0], id=sy.hexid(), fidelity=9999))
    staged("out-of-codebook", {"type_supports": ts}, "refused")
    # the digest shortcut alone does not see same-id-different-content (identity keys unchanged)
    oc = copy.deepcopy(base_rows["occurrences"])
    oc[0]["start"] = oc[0]["start"] + 1
    j = {}
    _, man = snapshot.load(model, 2, lambda n: oc if n == "occurrences" else prov(n), source="seed:unverified-content-change",
                           log=log, workers=a.workers, verify_shared=False, relations=rels, base=1, journal=j, order=order,
                           carry=rels - {"occurrences"})
    outcome("p2.facts.seed.content-change-unverified-path", "passed",
            note="expected blind spot: without re-assertion a same-id content change is invisible to key digests; the model's BLAKE3 id recomputation (external) is what refuses it",
            occurrences_unchanged_flag=man["occurrences"]["unchanged"])
    snapshot.abort(model, 2, journal=j)
    sdb.sql("DELETE snapshot:2;")

    # ---- S2: perturbed real state with structural sharing; pinned readers
    PHASE[0] = "s2"
    referenced = {f["target"] for n in rels for f in model.rel[n]["fields"] if f["type"] == "id" and counts[n]}
    payload_rel = next(n for n in order if counts[n] and model.pay(n) and n not in referenced
                       and any(f["type"] in ("int16", "bool") or "codes" in f for f in model.pay(n)))
    leaf_rel = next(n for n in order if counts[n] > 50 and n not in referenced and n != payload_rel)
    rows_p = copy.deepcopy(prov(payload_rel))
    pf = next(f for f in model.pay(payload_rel) if f["type"] in ("int16", "bool") or "codes" in f)
    changed_ids = []
    for r in rows_p:
        v = r.get(pf["name"])
        if "codes" in pf:
            alts = [c[0] for c in pf["codes"] if c[0] != v]
            if not alts:
                continue
            r[pf["name"]] = alts[0]
        elif pf["type"] == "bool":
            r[pf["name"]] = not v
        else:
            continue
        changed_ids.append(r["id"])
        if len(changed_ids) == 10:
            break

    class Lazy(dict):
        def __missing__(self, k):
            self[k] = prov(k)
            return self[k]
    sy2 = Synth(model, seed=123, reuse=1.0)
    sy2.rows = Lazy()
    sy2.rows[payload_rel] = rows_p
    leaf_rows = sy2.rows[leaf_rel]
    removed = [r["id"] for r in sorted(leaf_rows, key=lambda r: r["id"])[:5]]
    sy2.rows[leaf_rel] = [r for r in leaf_rows if r["id"] not in set(removed)]
    added = [sy2.make(leaf_rel)["id"] for _ in range(3)]
    log("perturbation:", json.dumps({"payload_relation": payload_rel, "payload_field": pf["name"], "changed": len(changed_ids),
                                     "leaf_relation": leaf_rel, "removed": len(removed), "added": len(added),
                                     "relations_touched_by_synth": sorted(k for k in sy2.rows if k not in (payload_rel, leaf_rel))}))
    probe_sql = lambda s: (f"SELECT VALUE [record::id(id), `{pf['name']}`] FROM {payload_rel}__p WHERE record::id(identity_of) INSIDE {json.dumps(changed_ids)};"
                           f"SELECT VALUE record::id(id) FROM {leaf_rel} WHERE record::id(id) INSIDE {json.dumps(removed + added)};")

    def view(tok):
        r = sdb.sql(probe_sql(0), token=tok)
        return {"payload": sorted(map(tuple, r[0]["result"])), "leaf": sorted(r[1]["result"])}
    s1_probe = view(tok_a)
    poll_cases = dict(list(cases.items())[:4])
    s1_j4 = {c: sur[c] for c in poll_cases}
    obs, stop2 = [], threading.Event()

    def poll():
        while not stop2.is_set():
            try:
                same = view(tok_a) == s1_probe and surj(tok_a, poll_cases) == s1_j4
                tb = sdb.signin("b", "pw-b")
                mid = view(tb) == s1_probe
                obs.append({"pinned_same_as_s1": same, "new_signin_sees_s1": mid})
            except Exception as e:
                obs.append({"error": str(e)[:300]})
            stop2.wait(2.0)
    th = threading.Thread(target=poll, daemon=True)
    th.start()
    journal2 = {}
    p2 = lambda n: sy2.rows[n] if n in sy2.rows else prov(n)
    stats2, man2 = snapshot.load(model, 2, p2, source=f"pg:{a.generation}:facts+perturbation", log=log, workers=a.workers,
                                 verify_shared=False, relations=rels, base=1, journal=journal2, order=order)
    changed = {n for n, m in man2.items() if not m["unchanged"]}
    fails2 = snapshot.validate(model, 2, log=log, workers=a.workers, relations=rels, changed=changed)
    stop2.set(); th.join()
    bad = [o for o in obs if o != {"pinned_same_as_s1": True, "new_signin_sees_s1": True}]
    outcome("p2.facts.no-partial-visibility", "passed" if obs and not bad else "failed", polls=len(obs), deviations=bad[:5])
    outcome("p2.facts.validate-s2-delta", "passed" if not fails2 else "failed", changed_relations=sorted(changed),
            checks=len(snapshot.validation_queries(model, 2, rels, changed)), failures=fails2[:20])
    if fails2:
        return
    snapshot.publish(2)
    a_after = view(tok_a)
    tok_c = sdb.signin("c", "pw-c")
    c_view = view(tok_c)
    exp_c = {"payload": sorted((r["id"], r[pf["name"]]) for r in rows_p if r["id"] in set(changed_ids)),
             "leaf": sorted(added)}
    c_cmp = {"payload": sorted((k[0], v) for k, v in c_view["payload"]), "leaf": c_view["leaf"]}  # payload key is [id, payload hash]
    outcome("p2.facts.pinned-reader-after-publish", "passed" if a_after == s1_probe and surj(tok_a, poll_cases) == s1_j4 else "failed",
            s1_view=s1_probe, after=a_after)
    outcome("p2.facts.s2-reader-sees-s2", "passed" if c_cmp == exp_c else "failed", s2_view=c_view, expected=exp_c)
    outcome("p2.facts.structural-sharing", "passed" if stats2["identity"]["shared"] > 0.99 * sum(counts.values()) else "failed",
            load_stats=stats2, relations_written=sorted(changed), relations_shared_by_digest=len(rels) - len(changed))
    # retirement of S1-only records (closed at 2), targeted by the S2 journal, after leases expire
    PHASE[0] = "retire"
    live_leases = sdb.sql("SELECT VALUE count() FROM lease WHERE snap = 1 AND expires > time::now() GROUP ALL;")[0]["result"]
    sdb.sql("UPDATE lease SET expires = time::now() - 1s WHERE snap = 1;")
    sdb.sql("UPDATE snapshot:1 SET state = 'retired';")
    gone = 0
    for table, jj in journal2.items():
        for k, _old in jj["spans"]:
            gone += len(sdb.sql(f"DELETE type::record('{table}', {json.dumps(k)}) WHERE spans.all(|$p| array::len($p) = 2 AND $p[1] <= 2) RETURN BEFORE;")[0]["result"] or [])
    outcome("p2.facts.retire-gc", "passed" if live_leases and gone >= len(removed) + len(changed_ids) and view(tok_c) == c_view else "failed",
            leases_blocking_before_expiry=live_leases, records_deleted=gone, expected=len(removed) + len(changed_ids))
    PHASE[0] = "done"
    stop.set()


if __name__ == "__main__":
    try:
        main()
    except Exception:
        if LOG:
            log(traceback.format_exc())
        raise
