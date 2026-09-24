//! The condition language's known answers (ADR-0022 §Conditions; DESIGN §3.9): normal forms and
//! their encodings. Compatibility waits for its first question (Stage 3; the Stage 2 review's F11).

use cpg_schema::condition::Condition;

const NORMAL: &[(&str, &str)] = &[
    ("true", "true"),
    ("false", "false"),
    (
        "equals(mode,\"http\") & !is_none(x)",
        "!is_none(x) & equals(mode,\"http\")",
    ),
    ("is_none(x) & !is_none(x)", "false"),
    ("truthy(a) | truthy(a) & truthy(b)", "truthy(a)"),
    (
        "truthy(a) | !truthy(a) & truthy(b)",
        "truthy(a) | truthy(b)",
    ),
    // Self-subsuming resolution: the other conjunction holds `y` and only literals this one
    // holds too.
    (
        "truthy(a) & truthy(r) & !truthy(y) | truthy(r) & truthy(y)",
        "truthy(a) & truthy(r) | truthy(r) & truthy(y)",
    ),
    (
        "!truthy(y) | truthy(r) & truthy(y)",
        "!truthy(y) | truthy(r)",
    ),
    (
        "truthy(a) & !truthy(y) | truthy(a) & truthy(y)",
        "truthy(a)",
    ),
    (
        "truthy(a) & !truthy(y) | truthy(b) & truthy(y)",
        "!truthy(y) & truthy(a) | truthy(b) & truthy(y)",
    ),
    // A normal path met in two differently nested forms (`Client.__init__`'s `verify` guard).
    (
        "!truthy(i) & !truthy(m) & is_none(t) & is_none(v) | !truthy(m) & is_none(t) & truthy(i)",
        "!truthy(m) & is_none(t) & is_none(v) | !truthy(m) & is_none(t) & truthy(i)",
    ),
    (
        "member_of(t,{\"sse\",\"http\",\"sse\"})",
        "member_of(t,{\"http\",\"sse\"})",
    ),
    ("member_of(t,{\"x\"})", "equals(t,\"x\")"),
    (
        "opaque(\"f(x)  and g\") & truthy(y)",
        "opaque(\"f(x) and g\") & truthy(y)",
    ),
    ("equals(s,\"é\")", "equals(s,\"é\")"),
    ("equals(n,-1)", "equals(n,-1)"),
];

#[test]
fn every_known_normal_form_holds() {
    for (input, normal) in NORMAL {
        let c = Condition::parse(input).unwrap_or_else(|e| panic!("{input}: {e}"));
        assert_eq!(c.encode(), *normal, "{input}");
        assert_eq!(
            Condition::parse(&c.encode()).unwrap(),
            c,
            "{input} round-trips"
        );
    }
    assert!(Condition::parse("self._x.y | settings.debug").is_err());
}
