"""Synthetic journey dataset with the edge cases the journeys must keep apart."""
from synth import Synth


def build(model, seed=7):
    sy = Synth(model, seed=seed)
    cases = {}

    def member(**kw):
        return sy.make("catalog_members", **kw)

    # M_none: a member with no invocations, options or facets (absent everywhere)
    cases["none"] = member()["id"]
    # M_trunc: one invocation, 130 briefs through 3 seeds -> page 100, omitted 30, truncated
    m = member(); cases["truncated"] = m["id"]
    inv = sy.make("catalog_member_invocations", member=m["id"])
    for i in range(3):
        seed = sy.make("synthesis_selected_seeds", member=inv["id"])
        for _ in range([50, 50, 30][i]):
            sy.make("synthesis_briefs", seed=seed["id"])
    # M_mixed: two invocations, a few briefs, options with and without default, facets with and
    # without qualification, input with matching invocations/outcomes
    m = member(); cases["mixed"] = m["id"]
    invs = [sy.make("catalog_member_invocations", member=m["id"]) for _ in range(2)]
    seed = sy.make("synthesis_selected_seeds", member=invs[0]["id"])
    for _ in range(4):
        sy.make("synthesis_briefs", seed=seed["id"])
    # catalog_options.default is non-nullable: "no default" vs "unknown default" are default kinds
    for kind in (0, 1, 2, 3):
        sy.make("catalog_options", member=m["id"], default=sy.make("catalog_defaults", tag=kind)["id"])
    sy.make("synthesis_summary_facets", member=invs[0]["id"], qualification=None)
    sy.make("synthesis_summary_facets", member=invs[1]["id"], qualification=sy.make("assertion_qualifications")["id"])
    for _ in range(2):
        si = sy.make("summary_analysis_invocations", input=m["input"])
        sy.make("summary_analysis_outcomes", invocation=si["id"])
    # M_contract: summary invocation without outcome -> the journey must report a contract failure
    m = member(); cases["contract"] = m["id"]
    sy.make("summary_analysis_invocations", input=m["input"])
    # a few random members
    for i in range(3):
        m = member(); cases[f"random{i}"] = m["id"]
        inv = sy.make("catalog_member_invocations", member=m["id"])
        seed = sy.make("synthesis_selected_seeds", member=inv["id"])
        sy.make("synthesis_briefs", seed=seed["id"])
        sy.make("catalog_options", member=m["id"])
        sy.make("synthesis_summary_facets", member=inv["id"])
    cases["missing"] = sy.hexid()  # never created: absent member
    return sy.rows, cases
