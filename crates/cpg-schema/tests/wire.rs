//! Shared protocol fixtures and complete schema snapshots, independent of Python reflection.
use cpg_schema::wire;
#[test]
fn shared_request_fixtures() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../specs/wire/requests.json")).unwrap();
    for case in cases {
        let name = case["contract"].as_str().unwrap();
        let expected = case["valid"].as_bool().unwrap();
        assert_eq!(
            wire::decode(name, &case["input"].to_string()).is_ok(),
            expected,
            "{case}"
        );
        let schema = wire::schema(name, false).unwrap();
        let validator = jsonschema::options()
            .offline()
            .should_validate_formats(false)
            .build(&schema)
            .unwrap();
        assert_eq!(
            validator.is_valid(&case["input"]),
            case["schema_valid"].as_bool().unwrap_or(expected),
            "{case}"
        );
    }
}
#[test]
fn generated_wire_schemas_snapshot() {
    let mut schemas = std::collections::BTreeMap::new();
    for name in [
        "search_capabilities",
        "get_capability",
        "get_operation",
        "get_evidence",
        "find_operations",
        "search_operations",
        "inspect_value_paths",
        "browse_library",
        "search_evidence",
        "compare_operations",
    ] {
        let (input, output) = wire::tool_schemas(name).unwrap();
        schemas.insert(format!("{name}.input"), input);
        schemas.insert(format!("{name}.output"), output);
    }
    insta::assert_snapshot!(serde_json::to_string_pretty(&schemas).unwrap());
}
