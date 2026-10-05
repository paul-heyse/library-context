"""Three served journeys, as SurrealQL over a pinned reader session and as PostgreSQL reference SQL.

J1 briefs   (operation_sections.rs brief_section):  member -> member invocations -> selected seeds
            (seed.member is a payload field) -> briefs; first page of at most 100 by id with
            omitted/truncated, as section_page computes it.
J2 options  (catalog member -> options -> subject / evidence / default): evidence must exist (the
            service raises Contract otherwise); a missing default is absent, not unknown.
J3 behavior (operation_sections.rs behavior_section): member invocations -> summary facets; a facet
            without qualification is `unavailable` (unknown), distinct from no facet (absent);
            summary-analysis invocations of the member's input must each have an outcome
            (Contract otherwise).
Reverse lookups name the referencing field (`<~(t FIELD f)`), as read_for(field) does: an
unqualified `<~t` unions every field of t that references the record.
Results are normalised to plain JSON (hex ids, sorted) so both sides compare exactly.
"""
import json
import subprocess

PAGE = 100


def surreal_query(m):
    return f"""
LET $m = catalog_members:`{m}`;
LET $inv = $m<~(catalog_member_invocations FIELD member);
LET $seeds = array::distinct(array::flatten($inv.map(|$i| ($i<~(synthesis_selected_seeds__p FIELD member)).identity_of)));
LET $briefs = array::sort(array::flatten($seeds.map(|$s| $s<~(synthesis_briefs FIELD seed))).map(|$b| record::id($b)));
LET $opts = $m<~(catalog_options FIELD member);
LET $facets = array::flatten($inv.map(|$i| $i<~(synthesis_summary_facets FIELD member)));
LET $sinv = IF $m.input = NONE {{ [] }} ELSE {{ $m.input<~(summary_analysis_invocations FIELD input) }};
RETURN {{
  member_visible: $m.id != NONE,
  invocations: array::sort($inv.map(|$i| record::id($i))),
  j1: {{ total: array::len($briefs), page: array::slice($briefs, 0, {PAGE}),
         omitted: math::max([0, array::len($briefs) - {PAGE}]), truncated: array::len($briefs) > {PAGE},
         seeds: array::sort($seeds.map(|$s| record::id($s))) }},
  j2: $opts.map(|$o| {{
         id: record::id($o.id),
         subject: record::id($o.subject), subject_kind: $o.subject.kind,
         evidence: record::id($o.evidence), evidence_present: $o.evidence.id != NONE, evidence_kind: $o.evidence.kind,
         default: IF ($o<~(catalog_options__p FIELD identity_of))[0].`default` = NONE {{ NONE }} ELSE {{ record::id(($o<~(catalog_options__p FIELD identity_of))[0].`default`) }},
         default_kind: ($o<~(catalog_options__p FIELD identity_of))[0].`default`.kind
       }}),
  j3: {{ facets: $facets.map(|$f| {{ id: record::id($f), unavailable: ($f<~(synthesis_summary_facets__p FIELD identity_of))[0].qualification = NONE }}),
         invocations: array::len($sinv),
         outcomes: array::len(array::flatten($sinv.map(|$x| $x<~(summary_analysis_outcomes FIELD invocation)))) }}
}};
"""


def normalise_surreal(r):
    j2 = []
    for o in r["j2"]:
        j2.append({"id": o["id"], "subject": o["subject"], "subject_kind": o.get("subject_kind"),
                   "evidence": o["evidence"], "evidence_kind": o.get("evidence_kind") if o["evidence_present"] else None,
                   "default": o.get("default"), "default_kind": o.get("default_kind")})
    facets = sorted(r["j3"]["facets"], key=lambda f: f["id"])
    return {"member_visible": r["member_visible"], "invocations": r["invocations"],
            "j1": r["j1"], "j2": sorted(j2, key=lambda o: o["id"]),
            "j3": {"facets": facets, "unavailable": sum(f["unavailable"] for f in facets),
                   "invocations": r["j3"]["invocations"], "outcomes": r["j3"]["outcomes"],
                   "contract_ok": r["j3"]["invocations"] == r["j3"]["outcomes"]},
            "j2_contract_ok": all(o["evidence_kind"] is not None for o in j2)}


def pg_query(schema, m):
    s = schema
    return f"""
WITH mem AS (SELECT id, input FROM {s}.catalog_members WHERE id = decode('{m}','hex')),
inv AS (SELECT id FROM {s}.catalog_member_invocations WHERE member IN (SELECT id FROM mem)),
seeds AS (SELECT id FROM {s}.synthesis_selected_seeds WHERE member IN (SELECT id FROM inv)),
briefs AS (SELECT id FROM {s}.synthesis_briefs WHERE seed IN (SELECT id FROM seeds)),
opts AS (
  SELECT o.id, o.subject, s.kind AS subject_kind, o.evidence, e.kind AS evidence_kind, o."default", d.kind AS default_kind
  FROM {s}.catalog_options o
  LEFT JOIN {s}.catalog_option_subjects s ON s.id = o.subject
  LEFT JOIN {s}.catalog_option_evidence e ON e.id = o.evidence
  LEFT JOIN {s}.catalog_defaults d ON d.id = o."default"
  WHERE o.member IN (SELECT id FROM mem)),
facets AS (SELECT id, qualification IS NULL AS unavailable FROM {s}.synthesis_summary_facets WHERE member IN (SELECT id FROM inv)),
sinv AS (SELECT id FROM {s}.summary_analysis_invocations WHERE input IN (SELECT input FROM mem)),
outc AS (SELECT id FROM {s}.summary_analysis_outcomes WHERE invocation IN (SELECT id FROM sinv))
SELECT json_build_object(
  'member_visible', EXISTS (SELECT 1 FROM mem),
  'invocations', COALESCE((SELECT json_agg(encode(id,'hex') ORDER BY id) FROM inv), '[]'),
  'briefs', COALESCE((SELECT json_agg(encode(id,'hex') ORDER BY id) FROM briefs), '[]'),
  'seeds', COALESCE((SELECT json_agg(encode(id,'hex') ORDER BY id) FROM seeds), '[]'),
  'opts', COALESCE((SELECT json_agg(json_build_object('id', encode(id,'hex'), 'subject', encode(subject,'hex'), 'subject_kind', subject_kind,
          'evidence', encode(evidence,'hex'), 'evidence_kind', evidence_kind, 'default', encode("default",'hex'), 'default_kind', default_kind)) FROM opts), '[]'),
  'facets', COALESCE((SELECT json_agg(json_build_object('id', encode(id,'hex'), 'unavailable', unavailable)) FROM facets), '[]'),
  'sinv', (SELECT count(*) FROM sinv), 'outc', (SELECT count(*) FROM outc));
"""


def normalise_pg(r):
    b = r["briefs"]
    j2 = sorted(r["opts"], key=lambda o: o["id"])
    facets = sorted(r["facets"], key=lambda f: f["id"])
    return {"member_visible": r["member_visible"], "invocations": r["invocations"],
            "j1": {"total": len(b), "page": b[:PAGE], "omitted": max(0, len(b) - PAGE), "truncated": len(b) > PAGE, "seeds": r["seeds"]},
            "j2": j2,
            "j3": {"facets": facets, "unavailable": sum(f["unavailable"] for f in facets),
                   "invocations": r["sinv"], "outcomes": r["outc"], "contract_ok": r["sinv"] == r["outc"]},
            "j2_contract_ok": all(o["evidence_kind"] is not None for o in j2)}


def psql(db, sql, extra=()):
    out = subprocess.run(["psql", "-h", "127.0.0.1", "-U", "lctx_superuser", "-d", db, "-At", "-v", "ON_ERROR_STOP=1", "-q", *extra, "-f", "-"],
                         input=sql, check=True, capture_output=True, text=True).stdout
    return out


def run_pg(db, schema, m, read_only=True):
    pre = "SET default_transaction_read_only = on; " if read_only else ""
    out = psql(db, pre + pg_query(schema, m))
    line = [l for l in out.split("\n") if l.startswith("{")][0]
    return normalise_pg(json.loads(line))


# ---------------------------------------------------------------------------------------------
# J4 (facts-only evidence closure; NOT a served journey -- the served catalog journeys need
# catalog relations, which the staging generation does not have): occurrence -> type
# observations about it (page of 100 by id, total/omitted/truncated) -> term kind, qualification
# (context/scope/condition/modality/approximation) and every support with run, surface and
# evidence (kind, anchored occurrence). Absent occurrence vs occurrence without observations.

def facts_surreal_query(o):
    return f"""
LET $o = occurrences:`{o}`;
LET $all = array::sort(($o<~(type_observations FIELD subject)).map(|$x| record::id($x)));
LET $page = array::slice($all, 0, {PAGE}).map(|$k| type::record('type_observations', $k));
RETURN {{
  visible: $o.id != NONE, total: array::len($all), omitted: math::max([0, array::len($all) - {PAGE}]),
  truncated: array::len($all) > {PAGE},
  page: $page.map(|$t| {{
    id: record::id($t.id), role: $t.role, declared: $t.declared, term: record::id($t.term), term_kind: $t.term.kind,
    q: {{ id: record::id($t.qualification), context: record::id($t.qualification.context), scope: record::id($t.qualification.scope),
          condition: record::id($t.qualification.condition), modality: $t.qualification.modality, approximation: $t.qualification.approximation }},
    supports: ($t<~(type_supports FIELD assertion)).map(|$s| {{
      id: record::id($s.id), run: record::id($s.run), surface: $s.surface.name, family: $s.surface.family,
      origin: $s.origin, mode: $s.mode, fidelity: $s.fidelity,
      evidence: record::id($s.evidence), evidence_kind: $s.evidence.kind,
      evidence_occurrence: IF $s.evidence.occurrence_occurrence = NONE {{ NONE }} ELSE {{ record::id($s.evidence.occurrence_occurrence) }}
    }})
  }})
}};
"""


def facts_normalise_surreal(r):
    page = []
    for t in r["page"]:
        t = dict(t)
        t["supports"] = sorted((dict(s, evidence_occurrence=s.get("evidence_occurrence")) for s in t["supports"]), key=lambda s: s["id"])
        page.append(t)
    return {"visible": r["visible"], "total": r["total"], "omitted": r["omitted"], "truncated": r["truncated"], "page": page}


def facts_pg_query(schema, phys, o):
    t = lambda n: f"{schema}.{phys.get(n, n)}"
    return f"""
WITH occ AS (SELECT id FROM {t('occurrences')} WHERE id = decode('{o}','hex')),
obs AS (SELECT * FROM {t('type_observations')} WHERE subject IN (SELECT id FROM occ)),
page AS (SELECT * FROM obs ORDER BY id LIMIT {PAGE})
SELECT json_build_object(
  'visible', EXISTS (SELECT 1 FROM occ), 'total', (SELECT count(*) FROM obs),
  'page', COALESCE((SELECT json_agg(json_build_object(
     'id', encode(p.id,'hex'), 'role', p.role, 'declared', p.declared, 'term', encode(p.term,'hex'), 'term_kind', tt.kind,
     'q', json_build_object('id', encode(p.qualification,'hex'), 'context', encode(q.context,'hex'), 'scope', encode(q.scope,'hex'),
                            'condition', encode(q.condition,'hex'), 'modality', q.modality, 'approximation', q.approximation),
     'supports', COALESCE((SELECT json_agg(json_build_object(
         'id', encode(s.id,'hex'), 'run', encode(s.run,'hex'), 'surface', sf.name, 'family', sf.family,
         'origin', s.origin, 'mode', s.mode, 'fidelity', s.fidelity,
         'evidence', encode(s.evidence,'hex'), 'evidence_kind', e.kind, 'evidence_occurrence', encode(e.occurrence_occurrence,'hex')))
       FROM {t('type_supports')} s LEFT JOIN {t('provider_surfaces')} sf ON sf.id = s.surface
       LEFT JOIN {t('evidence')} e ON e.id = s.evidence WHERE s.assertion = p.id), '[]'))
     ORDER BY p.id)
   FROM page p LEFT JOIN {t('type_terms')} tt ON tt.id = p.term LEFT JOIN {t('assertion_qualifications')} q ON q.id = p.qualification), '[]'));
"""


def facts_run_pg(db, schema, phys, o):
    out = psql(db, "SET default_transaction_read_only = on;\n" + facts_pg_query(schema, phys, o))
    r = json.loads([l for l in out.split("\n") if l.startswith("{")][0])
    for p in r["page"]:
        p["supports"] = sorted(p["supports"], key=lambda s: s["id"])
    return {"visible": r["visible"], "total": r["total"], "omitted": max(0, r["total"] - PAGE), "truncated": r["total"] > PAGE, "page": r["page"]}
