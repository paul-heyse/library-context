"""PostgreSQL 18 snapshot representations and identity/semantic diff, checked against ref.json.

uv run --no-project --with 'psycopg[binary]' python .../pg_harness.py <data_dir> [dbname]
Uses a disposable database (default lctx_probe_snapshot_diff) created and dropped by the caller:
  createdb -h 127.0.0.1 -U lctx_superuser lctx_probe_snapshot_diff ; dropdb ... at the end.

Representations
  M0  full copy per state: schema g_a / g_b, PK(id), FK to the same schema, CHECK on ints.
  M1  shared content c.<rel> keyed (id, pdig) + membership m.member(snap, rel, id, pdig).
      Referential integrity in a snapshot cannot be a plain FK (a target id may be present in
      the store but not in this snapshot): checked by a membership-scoped anti-join.
  M2  shared content with a validity interval live int4range over linear state ordinals.
"""
import pathlib, sys, psycopg
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import load, compare

D = pathlib.Path(sys.argv[1]).resolve()
DB = sys.argv[2] if len(sys.argv) > 2 else "lctx_probe_snapshot_diff"
schema, ref = load(D)
PGT = {"id": "uuid", "int": "bigint", "text": "text"}
con = psycopg.connect(host="127.0.0.1", user="lctx_superuser", dbname=DB, autocommit=True)
x = con.execute
def ids(sql):
    return {r[0].hex for r in x(sql).fetchall()}
def coldefs(rel):
    t = schema[rel]["types"]
    return ", ".join(f'"{c}" {PGT[t[c]]}' for c in schema[rel]["columns"] + ["pdig"])
def order():  # referenced relations first
    done, out = set(), []
    while len(out) < len(schema):
        for rel, s in schema.items():
            if rel not in done and all(t in done or t == rel for t in s["refs"].values()):
                done.add(rel); out.append(rel)
    return out

# ---------- M0 full copies ----------
for g, st in (("g_a", "A"), ("g_b", "B")):
    x(f"drop schema if exists {g} cascade; create schema {g}")
    for rel in order():
        s = schema[rel]
        fks = "".join(f', foreign key ("{c}") references {g}.{t}(id)' for c, t in s["refs"].items())
        checks = "".join(f', check ("{c}" >= 0)' for c in s.get("nonneg", [c for c, ty in s["types"].items() if ty == "int"]))
        x(f"create table {g}.{rel} ({coldefs(rel)}, primary key (id){fks}{checks})")
        with con.cursor().copy(f"copy {g}.{rel} from stdin (format csv, header true)") as cp:
            cp.write((D / st / f"{rel}.csv").read_bytes())
got0 = {rel: {"added": ids(f"select id from g_b.{rel} except select id from g_a.{rel}"),
              "removed": ids(f"select id from g_a.{rel} except select id from g_b.{rel}"),
              "changed": ids(f"select a.id from g_a.{rel} a join g_b.{rel} b using (id) where a.pdig <> b.pdig")}
        for rel in schema}
ok0 = compare("pg-M0-except", ref, got0)
# full outer join form, one pass per relation
gotj = {}
for rel in schema:
    rows = x(f"""select coalesce(a.id,b.id), case when a.id is null then 'added' when b.id is null then 'removed'
                 else 'changed' end from g_a.{rel} a full join g_b.{rel} b using (id)
                 where a.id is null or b.id is null or a.pdig <> b.pdig""").fetchall()
    gotj[rel] = {k: {i.hex for i, kk in rows if kk == k} for k in ("added", "removed", "changed")}
okj = compare("pg-M0-fullouter", ref, gotj)
# FK / CHECK enforcement on load: a dangling reference and a negative int must be refused
neg = []
for sql in ("insert into g_b.occurrences select gen_random_uuid(), gen_random_uuid(), 0,1,1,0,'0',gen_random_uuid()",
            "update g_b.source_artifacts set byte_len = -1 where id = (select id from g_b.source_artifacts limit 1)"):
    try:
        x(sql); neg.append(f"ACCEPTED (unexpected): {sql[:60]}")
    except psycopg.Error as e:
        neg.append(f"refused {e.sqlstate}: {sql[:60]}")
print("pg-M0 enforcement:", *neg, sep="\n  ")

# ---------- M1 shared content + membership ----------
x("drop schema if exists c cascade; drop schema if exists m cascade; create schema c; create schema m")
x("create table m.member (snap smallint, rel text, id uuid, pdig uuid, primary key (snap, rel, id))")
for rel in order():
    x(f"create table c.{rel} ({coldefs(rel)}, primary key (id, pdig))")
    for snap, g in ((1, "g_a"), (2, "g_b")):
        x(f"insert into c.{rel} select * from {g}.{rel} on conflict do nothing")
        x(f"insert into m.member select {snap}, '{rel}', id, pdig from {g}.{rel}")
x("alter table m.member add constraint member_rel_check check (snap > 0)")
stored = x("select sum(n) from (" + " union all ".join(f"select count(*) n from c.{r}" for r in schema) + ") t").fetchone()[0]
full = x("select count(*) from m.member").fetchone()[0]
print(f"pg-M1 sharing: content rows stored {stored} for {full} membership rows "
      f"(two full copies would store {full}); shared fraction of state B = "
      f"{x('select count(*) from m.member b join m.member a on a.snap=1 and a.rel=b.rel and a.id=b.id and a.pdig=b.pdig where b.snap=2').fetchone()[0]}"
      f"/{x('select count(*) from m.member where snap=2').fetchone()[0]}")
got1 = {rel: {"added": set(), "removed": set(), "changed": set()} for rel in schema}
for rel, i, k in x("""select coalesce(a.rel,b.rel), coalesce(a.id,b.id), case when a.id is null then 'added'
        when b.id is null then 'removed' else 'changed' end
        from (select * from m.member where snap=1) a full join (select * from m.member where snap=2) b using (rel, id)
        where a.id is null or b.id is null or a.pdig <> b.pdig""").fetchall():
    got1[rel][k].add(i.hex)
ok1 = compare("pg-M1-membership", ref, got1)
# snapshot-scoped referential integrity: membership anti-join (a plain FK cannot express it)
def dangling(snap):
    n = 0
    for rel, s in schema.items():
        for c, t in s["refs"].items():
            n += x(f"""select count(*) from m.member r join c.{rel} cr on cr.id=r.id and cr.pdig=r.pdig
                where r.snap={snap} and r.rel='{rel}' and cr."{c}" is not null and not exists
                (select 1 from m.member t where t.snap={snap} and t.rel='{t}' and t.id=cr."{c}")""").fetchone()[0]
    return n
x("insert into m.member select 3, rel, id, pdig from m.member where snap=2")
rrel, (rcol, trel) = next((r, next(iter(s["refs"].items()))) for r, s in schema.items() if s["refs"])
tid = x(f"""select cr."{rcol}" from m.member r join c.{rrel} cr on cr.id=r.id and cr.pdig=r.pdig
            where r.snap=3 and r.rel='{rrel}' and cr."{rcol}" is not null limit 1""").fetchone()[0]
x("delete from m.member where snap=3 and rel=%s and id=%s", (trel, tid))
print(f"pg-M1 integrity probe: removed one {trel} member referenced from {rrel}.{rcol}")
print(f"pg-M1 snapshot-scoped refs: dangling(A)={dangling(1)} dangling(B)={dangling(2)} "
      f"dangling(B minus one referenced occurrence)={dangling(3)} (expected 0,0,>=1; a FK on c.* would not see it)")
x("delete from m.member where snap=3")

# ---------- M2 validity intervals (linear history only) ----------
x("drop schema if exists v cascade; create schema v")
for rel in order():
    x(f"create table v.{rel} ({coldefs(rel)}, live int4range not null, primary key (id, pdig))")
    x(f"""insert into v.{rel} select c.*, int4range(min(m.snap), max(m.snap)+1) from c.{rel} c
          join m.member m on m.rel='{rel}' and m.id=c.id and m.pdig=c.pdig group by {", ".join('c."'+k+'"' for k in schema[rel]["columns"]+["pdig"])}""")
gaps = x("select count(*) from (select rel,id,pdig from m.member group by 1,2,3 having max(snap)-min(snap)+1 <> count(*)) t").fetchone()[0]
got2 = {}
for rel in schema:
    add = ids(f"select id from v.{rel} where lower(live)=2 except select id from v.{rel} where live @> 1")
    rem = ids(f"select id from v.{rel} where upper(live)=2 except select id from v.{rel} where live @> 2")
    chg = ids(f"select a.id from v.{rel} a join v.{rel} b using (id) where a.live @> 1 and not a.live @> 2 and b.live @> 2 and not b.live @> 1")
    got2[rel] = {"added": add, "removed": rem, "changed": chg}
ok2 = compare("pg-M2-intervals", ref, got2)
print(f"pg-M2 non-contiguous memberships (would need a second interval) = {gaps}")

# ---------- semantic keyed diff (correspondence key from ref["semantic_spec"]; the model declares none) ----------
okS = True
sp = ref.get("semantic_spec")
if sp and "semantic" in ref:
    def keyexpr():
        def t(alias, rel, c):
            e = f'{alias}."{c}"::text'
            e = f"replace({e}, '-', '')" if schema[rel]["types"][c] == "id" else e
            return f"coalesce({e}, '')"
        parts = [t("p", sp["sym"], c) for c in sp["key_sym"]] + [t("s", sp["sig"], c) for c in sp["key_sig"]]
        return " || '|' || ".join(parts)
    def side(g):
        return (f'select {keyexpr()} k, replace(s."{sp["digest"]}"::text, \'-\', \'\') d from {g}.{sp["sig"]} s '
                f'join {g}.{sp["sym"]} p on p.id = s."{sp["sym_ref"]}"')
    coll = {g: x(f"select count(*) - count(distinct k) from ({side(g)}) t").fetchone()[0] for g in ("g_a", "g_b")}
    rows = x(f"""with a as ({side('g_a')}), b as ({side('g_b')})
            select coalesce(a.k,b.k), case when a.k is null then 'added' when b.k is null then 'removed' else 'changed' end
            from a full join b using (k) where a.k is null or b.k is null or a.d <> b.d""").fetchall()
    semantic = {k: {r[0] for r in rows if r[1] == k} for k in ("added", "removed", "changed")}
    okS = compare("pg-semantic", {"relations": {}, "semantic": ref["semantic"]}, {}, semantic)
    print(f"pg-semantic key collisions per state: {coll}")
if "modules" in schema:
    mq = x("select count(*) - count(distinct qualified_name) from g_a.modules").fetchone()[0]
    print(f"pg modules: qualified_name duplicates in A = {mq} (.py/.pyi pairs; key needs the stub flag)")
print("pg OVERALL", "passed" if all((ok0, okj, ok1, ok2, okS)) else "failed")
