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
