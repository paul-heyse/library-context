//! Independent finite serving declaration controls; no mocked store admission is claimed.
fn snapshot_for(byte: u8) -> lctx_model::domain::serving::SnapshotHandle {
    use lctx_model::domain::serving::{DatabaseIdentity, Name, SnapshotHandle};
    let mut handle = SnapshotHandle {
        publication: lctx_model::domain::ContentHash([0; 32]),
        view: lctx_model::domain::ContentHash([byte; 32]),
        service_generation: lctx_model::domain::ContentHash([byte; 32]),
        definition_epoch: lctx_model::domain::ContentHash([byte; 32]),
        semantic: lctx_model::domain::ContentHash([byte; 32]),
        realization: lctx_model::domain::ContentHash([byte; 32]),
        database: DatabaseIdentity {
            namespace: Name::new("lctx").unwrap(),
            database: Name::new(format!("snapshot_{byte}")).unwrap(),
        },
    };
    handle.publication = handle.expected_publication();
    handle
}
use lctx_model::{
    Domain,
    domain::{
        self,
        serving::{self, identity::*, *},
        *,
    },
};
use serde_json::{Value, json};

fn decode(tool: &str, value: Value) -> Result<Request, WireError> {
    decode_request(
        tool,
        &serde_json::to_string(&value).unwrap(),
        &ResourceLimits::default(),
    )
}
fn request_cases() -> Vec<(&'static str, Value)> {
    let id = vec![1u8; 16];
    vec![
        (
            "search_operations",
            json!({"library":"fastmcp","query":"request context"}),
        ),
        ("find_operations", json!({"library":"fastmcp"})),
        (
            "get_operation",
            json!({"library":"fastmcp","operation":{"kind":"member","member":id}}),
        ),
        ("browse_library", json!({"library":"fastmcp"})),
        (
            "get_evidence",
            json!({"source":{"kind":"catalog","source":id}}),
        ),
        (
            "search_evidence",
            json!({"library":"fastmcp","query":"deployment","families":[1]}),
        ),
        (
            "compare_operations",
            json!({"library":"fastmcp","operations":[{"kind":"public_path","path":["FastMCP","run"]}]}),
        ),
        (
            "search_capabilities",
            json!({"library":"fastmcp","query":"context"}),
        ),
        ("get_capability", json!({"capability":id})),
        (
            "inspect_value_paths",
            json!({"member":id,"analysis":id,"inputs":[{"formal":id,"value":{"kind":"integer","decimal":"1000000000000000000000000000000000000000000"}}],"assumptions":{"builtin_namespace":"unknown"}}),
        ),
    ]
}
#[test]
fn all_ten_routes_decode_and_emit_distinct_schemas() {
    assert_eq!(Tool::ALL.len(), 10);
    let declarations = tools();
    assert_eq!(declarations.len(), 10);
    for (tool, input) in request_cases() {
        let request = decode(tool, input).unwrap();
        assert_eq!(request.tool().name(), tool);
        let normalized = request.to_json().unwrap();
        assert_eq!(
            decode_request(tool, &normalized, &ResourceLimits::default()).unwrap(),
            request
        );
        assert_eq!(request.page().size, 20);
        let declaration = declarations.iter().find(|d| d.name == tool).unwrap();
        assert_eq!(declaration.request_schema["additionalProperties"], false);
        assert_eq!(declaration.response_schema["additionalProperties"], false);
        assert!(declaration.read_only && declaration.idempotent);
        assert_ne!(declaration.request_schema, declaration.response_schema);
    }
    assert_eq!(wire_identity(), wire_identity());
}
#[test]
fn schema_null_tag_type_and_unknown_field_controls() {
    assert!(
        decode(
            "get_evidence",
            json!({"source":{"kind":"catalog","source":vec![1;15]}})
        )
        .is_err()
    );
    assert!(
        decode(
            "get_evidence",
            json!({"source":{"kind":"catalog","source":vec![256;16]}})
        )
        .is_err()
    );
    assert!(
        decode(
            "get_operation",
            json!({"library":"fastmcp","operation":{"kind":"invented","member":vec![0;16]}})
        )
        .is_err()
    );
    assert!(
        decode(
            "get_operation",
            json!({"library":"fastmcp","operation":{"kind":"public_path","path":[]}})
        )
        .is_err()
    );
    assert!(
        decode(
            "search_operations",
            json!({"library":"fastmcp","query":false})
        )
        .is_err()
    );
    assert!(
        decode(
            "find_operations",
            json!({"library":"fastmcp","legacy_snapshot":"bad"})
        )
        .is_err()
    );
    assert!(
        decode(
            "find_operations",
            json!({"library":"fastmcp","page":{"cursor":null}})
        )
        .is_err()
    );
    assert!(
        decode(
            "find_operations",
            json!({"library":"fastmcp","selection":{"requirements":[],"mode":25,"joint":1}})
        )
        .is_err()
    );
    assert!(decode("find_operations",json!({"library":"fastmcp","selection":{"requirements":[{"predicate":{"DeclaresParameter":{"name":"x","ignored":true}},"quantifier":0}],"mode":0,"joint":1}})).is_err());
    assert!(decode("find_operations",json!({"library":"fastmcp","selection":{"requirements":[],"mode":0,"joint":1,"ignored":true}})).is_err());
    let s = schema("get_evidence", false).unwrap();
    assert_eq!(
        s["$defs"]["catalog_original_sourcesId"]["x-nominal-relation"],
        "catalog_original_sources"
    );
    assert_eq!(s["$defs"]["catalog_original_sourcesId"]["minItems"], 16);
    assert!(
        serde_json::from_value::<BrowseScope>(json!({"kind":"library","invented":true})).is_err()
    );
    assert!(
        serde_json::from_value::<VectorChannel>(json!({"status":"disabled","invented":true}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<Availability>(json!({"status":"not_requested","invented":true}))
            .is_err()
    );
    let d = schema_for::<PageRequest>(false);
    let e = schema_for::<PageRequest>(true);
    assert_ne!(d["required"], e["required"]);
    let cursor_def = &d["$defs"]["Text1To8192Bytes"];
    assert_eq!(cursor_def["type"], "string");
    let default_schema = schema_for::<DefaultValue>(true);
    assert!(default_schema.to_string().contains("factory"));
    assert!(
        serde_json::from_value::<DefaultValue>(json!({"kind":"literal","literal":null})).is_err()
    );
    assert!(
        serde_json::from_value::<DefaultValue>(json!({"kind":"unknown","invented":true})).is_err()
    );
}
#[test]
fn canonical_identity_uses_decoded_selection_and_excludes_page_budgets() {
    let a = decode("find_operations", json!({"library":"fastmcp"})).unwrap();
    let b=decode_request("find_operations",r#"{"page":{"expanded":true,"cursor":"recomputed","size":100},"selection":{"joint":0,"mode":0,"requirements":[]},"library":"fastmcp"}"#,&ResourceLimits::default()).unwrap();
    assert_eq!(
        a.canonical_identity().unwrap(),
        b.canonical_identity().unwrap()
    );
    let c=decode("find_operations",json!({"library":"fastmcp","selection":{"requirements":[{"predicate":{"DeclaresParameter":{"name":"ctx"}},"quantifier":0}],"mode":0,"joint":1}})).unwrap();
    assert_ne!(
        a.canonical_identity().unwrap(),
        c.canonical_identity().unwrap()
    );
    let a=decode("get_operation",json!({"library":"fastmcp","operation":{"kind":"public_path","path":["FastMCP","run"]},"sections":["briefs","scenarios"]})).unwrap();
    let b=decode("get_operation",json!({"library":"fastmcp","operation":{"kind":"public_path","path":["FastMCP","run"]},"sections":["scenarios","briefs","scenarios"]})).unwrap();
    assert_eq!(
        a.canonical_identity().unwrap(),
        b.canonical_identity().unwrap()
    );
}
fn binding() -> CursorBinding {
    CursorBinding {
        snapshot: snapshot_for(1),
        request: RequestIdentity(ContentHash::of(b"request")),
        policy: PolicyIdentity(ContentHash::of(b"policy")),
        wire: WireIdentity(ContentHash::of(b"wire")),
        channels: ChannelState {
            lexical: true,
            vector: VectorChannel::Disabled {},
        }
        .identity(),
        group: Name::new("supported").unwrap(),
        section: Name::new("members").unwrap(),
        member: None,
        ordering: ContentHash::of(b"order"),
    }
}
#[test]
fn continuation_rejects_each_invalidated_boundary() {
    let original = binding();
    let cursor = Cursor {
        binding: original.clone(),
        after: CursorPosition::Key {
            key: ContentHash([20; 32]),
        },
    };
    let token = cursor.encode().unwrap();
    assert_eq!(Cursor::decode(&token, &original).unwrap(), cursor);
    let signed = CursorToken::new(format!("+9{}", token.as_str())).unwrap();
    assert!(
        Cursor::decode(&signed, &original).is_err(),
        "signed byte pairs cannot encode cursor whitespace"
    );
    let mut changed = original.clone();
    changed.snapshot = snapshot_for(2);
    assert!(Cursor::decode(&token, &changed).is_err());
    for snapshot in [
        SnapshotHandle {
            semantic: ContentHash::of(b"other content"),
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            realization: ContentHash::of(b"other operations"),
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            view: ContentHash::of(b"other completed views"),
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            service_generation: ContentHash::of(b"other installed generation"),
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            definition_epoch: ContentHash::of(b"other executable definitions"),
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            database: DatabaseIdentity {
                namespace: Name::new("other namespace").unwrap(),
                ..original.snapshot.database.clone()
            },
            ..original.snapshot.clone()
        },
        SnapshotHandle {
            database: DatabaseIdentity {
                database: Name::new("other database").unwrap(),
                ..original.snapshot.database.clone()
            },
            ..original.snapshot.clone()
        },
    ] {
        let changed = CursorBinding {
            snapshot,
            ..original.clone()
        };
        assert!(Cursor::decode(&token, &changed).is_err());
    }
    let mut changed = original.clone();
    changed.request = RequestIdentity(ContentHash::of(b"changed"));
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.policy = PolicyIdentity(ContentHash::of(b"changed"));
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.wire = WireIdentity(ContentHash::of(b"changed"));
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.channels = ChannelState {
        lexical: true,
        vector: VectorChannel::Degraded {
            reason: Name::new("embedding unavailable").unwrap(),
        },
    }
    .identity();
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.group = Name::new("unresolved").unwrap();
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.section = Name::new("signatures").unwrap();
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.ordering = ContentHash::of(b"changed");
    assert!(Cursor::decode(&token, &changed).is_err());
    let mut changed = original.clone();
    changed.member = Some(serde_json::from_value(json!(vec![1; 16])).unwrap());
    assert!(Cursor::decode(&token, &changed).is_err());
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "serving_control_basic")]
struct Basic {
    #[model(key)]
    name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "serving_control_extended")]
struct Extended {
    #[model(key)]
    name: String,
    mapped_field: Option<Id<catalog::CatalogMember>>,
}
#[test]
fn mapping_field_propagation_and_safe_lookup_indexes() {
    let basic = mappings::Mapping::of::<Basic>("control", &[mappings::Capability::Catalog]);
    let extended = mappings::Mapping::of::<Extended>("control", &[mappings::Capability::Catalog]);
    assert_eq!(
        extended.source.schema().fields().len(),
        basic.source.schema().fields().len() + 1
    );
    let field = extended
        .fields()
        .iter()
        .find(|f| f.name() == "mapped_field")
        .unwrap();
    assert!(field.nullable());
    assert_eq!(field.target().unwrap().1, catalog::CatalogMember::NAME);
    assert!(
        extended
            .dependencies
            .contains(&catalog::CatalogMember::NAME)
    );
    assert_ne!(
        mappings::identity_for(&[basic]),
        mappings::identity_for(&[extended])
    );
    let all = mappings::inventory();
    let declared = domain::catalog_frontier_relations();
    for mapping in &all {
        assert_eq!(mapping.minimum_frontier, admission::Frontier::Catalog);
        assert!(declared.iter().any(|r| r.name() == mapping.source.name()));
        for dependency in &mapping.dependencies {
            assert!(
                declared.iter().any(|r| r.name() == *dependency),
                "{dependency}"
            );
        }
        for key in &mapping.lookup_keys {
            let field = mapping.fields().iter().find(|f| f.name() == *key).unwrap();
            assert!(!field.list());
            assert!(!matches!(field.scalar(), Scalar::Text | Scalar::Binary));
        }
    }
    assert_eq!(mappings::identity(), mappings::identity());
}
#[test]
fn policy_wire_and_consumer_change_without_reextracting_canonical_facts() {
    let canonical = Basic {
        name: "canonical".into(),
    };
    let before = canonical.content_digest();
    let policy_a = policy_identity(&ResourceLimits::default()).unwrap();
    let limits = ResourceLimits {
        cpu_jobs: 3,
        ..ResourceLimits::default()
    };
    let policy_b = policy_identity(&limits).unwrap();
    assert_ne!(policy_a, policy_b);
    assert_eq!(canonical.content_digest(), before);
    let consumed = vec![OperationDependency {
        operation: Name::new("catalog_members").unwrap(),
        definition: ContentHash::of(b"content"),
    }];
    let snapshot = snapshot_for(1);
    let mapping = mappings::identity();
    let wire = wire_identity();
    let a = consumer_identity(&snapshot, &consumed, mapping, policy_a, wire, None).unwrap();
    let b = consumer_identity(&snapshot, &consumed, mapping, policy_b, wire, None).unwrap();
    assert_ne!(a, b);
    let c = consumer_identity(
        &snapshot,
        &consumed,
        mapping,
        policy_a,
        WireIdentity(ContentHash::of(b"new wire")),
        None,
    )
    .unwrap();
    assert_ne!(a, c);
    let mut changed = consumed.clone();
    changed[0].definition = ContentHash::of(b"changed");
    let d = consumer_identity(&snapshot, &changed, mapping, policy_a, wire, None).unwrap();
    assert_ne!(a, d);
    assert_eq!(canonical.content_digest(), before);
}
#[test]
fn resource_refusals_and_exact_scalar_boundaries() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.query_connections, 2);
    assert_eq!(limits.cpu_jobs, 2);
    assert_eq!(limits.request_deadline_ms, 30_000);
    assert_eq!(limits.admission_wait_ms, 1_000);
    assert_eq!(limits.shared_bytes, 256 * 1024 * 1024);
    assert_eq!(limits.preparation_bytes, 128 * 1024 * 1024);
    assert_eq!(limits.request_bytes, 64 * 1024 * 1024);
    assert_eq!(limits.default_response_bytes, 32 * 1024);
    assert_eq!(limits.expanded_response_bytes, 256 * 1024);
    limits.validate().unwrap();
    assert!(matches!(
        decode(
            "find_operations",
            json!({"library":"fastmcp","page":{"size":101}})
        ),
        Err(WireError::ResourceRefused(_))
    ));
    let requirements: Vec<_> = (0..17)
        .map(|_| json!({"predicate":{"DeclaresParameter":{"name":"ctx"}},"quantifier":0}))
        .collect();
    assert!(matches!(
        decode(
            "find_operations",
            json!({"library":"fastmcp","selection":{"requirements":requirements,"mode":0,"joint":1}})
        ),
        Err(WireError::ResourceRefused(_))
    ));
    let mut scalar = request_cases().pop().unwrap().1;
    scalar["inputs"][0]["value"] = json!({"kind":"integer","decimal":"+1"});
    assert!(decode("inspect_value_paths", scalar.clone()).is_err());
    scalar["inputs"][0]["value"] = json!({"kind":"bool","value":1});
    assert!(decode("inspect_value_paths", scalar).is_err());
    let mut original = request_cases().pop().unwrap().1;
    original["inputs"][0]["value"] = json!({"kind":"string","value":"\0unicode λ"});
    decode("inspect_value_paths", original).unwrap();
}
fn operation_response(parameters: usize) -> Value {
    let id = json!(vec![1u8; 16]);
    let absent =
        json!({"availability":{"status":"not_requested"},"items":[],"omitted":0,"truncated":false});
    let parameter = json!({"parameter":id,"slot":null,"formals":[],"ordinal":0,"name":"x".repeat(500),"kind":1,"required":true,"types":[],"type_evidence":[],"default":{"kind":"absent"}});
    json!({"snapshot":snapshot_for(1),"domains":[],"operation":{"resolution":"unique","packet":{
        "core":{"member":id,"name":"FastMCP.run",
            "release":{"input":id,"release":id,"distribution":"fastmcp","version":"4.0.5"},
            "access":{"module":id,"path":["FastMCP","run"],"exposures":[id],"candidates":[id],"basis":null},
            "invocations":[],"signatures":[{"signature":id,"role":0,"native":null,"variant":id,"analysis":id,"form":0,"adjustment":0,"parameters":vec![parameter.clone();parameters],"effective_parameters":vec![parameter;parameters],"return_types":[],"return_evidence":[],"typing":[],"complete":true}],
            "signature_knowledge":0,"options":[],"literal_values":[],"type_presentations":[],"interpretation":{"contexts":[],"defaults":[],"qualifications":[],"availability":{"status":"unavailable","reason":"controlled_fixture"}},"limits":{"maximum_page_rows":100,"maximum_response_bytes":32768,"signature_indivisible":true}},
        "callable_comparison":absent,"contextual_typing":absent,"incoming_references":absent,"access_routes":absent,"scenarios":absent,"deployment":absent,"relationships":absent,"conflicts":absent,"briefs":absent,"behavior":absent
    }}})
}
#[test]
fn signature_is_mandatory_and_optional_sections_have_explicit_status() {
    let limits = ResourceLimits::default();
    let packet = operation_response(1);
    decode_response("get_operation", &packet.to_string(), false, &limits).unwrap();
    let mut missing_domains = packet.clone();
    missing_domains.as_object_mut().unwrap().remove("domains");
    assert!(
        decode_response(
            "get_operation",
            &missing_domains.to_string(),
            false,
            &limits
        )
        .is_err()
    );
    let mut missing = packet.clone();
    missing["operation"]["packet"]["core"]
        .as_object_mut()
        .unwrap()
        .remove("signatures");
    assert!(decode_response("get_operation", &missing.to_string(), false, &limits).is_err());
    let mut missing_nullable = packet.clone();
    missing_nullable["operation"]["packet"]["core"]["access"]
        .as_object_mut()
        .unwrap()
        .remove("basis");
    assert!(
        decode_response(
            "get_operation",
            &missing_nullable.to_string(),
            false,
            &limits
        )
        .is_err()
    );
    let mut null_name = packet.clone();
    null_name["operation"]["packet"]["core"]["signatures"][0]["parameters"][0]["name"] =
        Value::Null;
    decode_response("get_operation", &null_name.to_string(), false, &limits).unwrap();
    let mut missing_name = null_name;
    missing_name["operation"]["packet"]["core"]["signatures"][0]["parameters"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    assert!(decode_response("get_operation", &missing_name.to_string(), false, &limits).is_err());
    let mut incomplete_section = packet.clone();
    incomplete_section["operation"]["packet"]["briefs"]
        .as_object_mut()
        .unwrap()
        .remove("availability");
    assert!(
        decode_response(
            "get_operation",
            &incomplete_section.to_string(),
            false,
            &limits
        )
        .is_err()
    );
    let large = operation_response(60).to_string();
    assert!(matches!(
        decode_response("get_operation", &large, false, &limits),
        Err(WireError::ResourceRefused(_))
    ));
    let admitted = decode_response("get_operation", &large, true, &limits).unwrap();
    let Response::GetOperation(response) = admitted else {
        panic!("wrong route")
    };
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("wrong resolution")
    };
    assert_eq!(packet.core.signatures[0].parameters.len(), 60);
    assert!(packet.core.signatures[0].complete);
    let schema = schema_for::<OperationCore>(true);
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("signatures"))
    );
}
#[test]
fn packet_mapping_and_snapshot_schema_have_current_nominal_owners() {
    use std::collections::BTreeSet;
    fn direct_closure(
        kind: mappings::PacketKind,
        visited: &mut BTreeSet<&'static str>,
        sources: &mut BTreeSet<&'static str>,
    ) -> bool {
        let binding = kind.binding();
        if !visited.insert(binding.mapping.name) {
            return false;
        }
        sources.extend(binding.mapping.sources.iter().map(Relation::name));
        let mut canonical_proof = binding
            .prepared
            .contains(&mappings::PreparedDependency::CanonicalProof);
        for child in binding.children {
            canonical_proof |= direct_closure(*child, visited, sources);
        }
        canonical_proof
    }
    let declared = domain::catalog_frontier_relations()
        .iter()
        .map(Relation::name)
        .collect::<BTreeSet<_>>();
    let mut graph_sources = BTreeSet::new();
    macro_rules! graph_sources {
        ($($variant:ident:$ty:ty,)*) => {
            $(graph_sources.insert(<$ty as Record>::NAME);)*
        };
    }
    lctx_model::graph_entity_records!(graph_sources);
    lctx_model::graph_assertion_records!(graph_sources);
    let analytic_premise = analysis::analytic::AnalysisDerivationPremise::NAME;
    assert!(graph_sources.contains(analytic_premise));
    assert!(!declared.contains(analytic_premise));
    let mut proof_bindings = 0;
    let mut ordinary_bindings = 0;
    for kind in mappings::PacketKind::ALL {
        let binding = kind.binding();
        assert_eq!(
            binding.mapping.minimum_frontier,
            admission::Frontier::Catalog
        );
        assert!(
            binding
                .mapping
                .required_capabilities
                .contains(&mappings::Capability::Catalog)
        );
        for source in &binding.mapping.sources {
            assert!(
                declared.contains(source.name()),
                "{}: {}",
                binding.mapping.name,
                source.name()
            );
        }
        let mut expected = BTreeSet::new();
        let canonical_proof = direct_closure(kind, &mut BTreeSet::new(), &mut expected);
        if canonical_proof {
            proof_bindings += 1;
            expected.extend(graph_sources.iter().copied());
        } else {
            ordinary_bindings += 1;
            assert!(expected.is_subset(&declared));
        }
        let lowered = binding.lowered();
        assert_eq!(lowered.minimum_frontier, admission::Frontier::Catalog);
        assert_eq!(
            lowered.required_capabilities,
            binding.mapping.required_capabilities
        );
        let actual = lowered
            .sources
            .iter()
            .map(Relation::name)
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{}", binding.mapping.name);
        assert_eq!(
            binding.permits_relation(analytic_premise),
            canonical_proof,
            "{}",
            binding.mapping.name
        );
    }
    assert!(proof_bindings > 0);
    assert!(ordinary_bindings > 0);
    let snapshot = schema_for::<SnapshotHandle>(true);
    assert_eq!(snapshot["additionalProperties"], false);
    assert_eq!(
        snapshot["required"],
        json!(["publication", "semantic", "realization", "view", "service_generation", "definition_epoch", "database"])
    );
    let packet = operation_response(1);
    let raw = decode_response(
        "get_operation",
        &packet.to_string(),
        false,
        &ResourceLimits::default(),
    )
    .unwrap()
    .to_json()
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&raw).unwrap()["snapshot"],
        json!(snapshot_for(1))
    );
}

#[test]
fn final_wire_budget_counts_text_and_structured_duplication() {
    let structured = json!({"body":"s".repeat(17000)});
    let envelope = json!({"content":[{"type":"text","text":"t".repeat(17000)}],"structuredContent":structured});
    let bytes = serde_json::to_vec(&envelope).unwrap();
    let limits = ResourceLimits::default();
    assert!(matches!(
        serving::resources::admit_final_mcp_bytes(&bytes, false, &limits),
        Err(WireError::ResourceRefused(_))
    ));
    serving::resources::admit_final_mcp_bytes(&bytes, true, &limits).unwrap();
}

#[test]
fn literal_rendering_preserves_nominal_identity_and_exact_payloads() {
    let rows = [
        value::Literal::None,
        value::Literal::Bool { value: true },
        value::Literal::Integer {
            decimal: "100000000000000000000000000000000000".into(),
        },
        value::Literal::String {
            value: Utf8Text::from("\0λ"),
        },
        value::Literal::Bytes {
            value: EvidenceBytes(vec![0, 255, 128]),
        },
        value::Literal::Float {
            bits: f64::NAN.to_bits() as i64,
        },
    ];
    for row in rows {
        let packet = LiteralPacket::from_canonical(&row).unwrap();
        assert_eq!(packet.literal, row.id());
        assert_eq!(
            serde_json::from_str::<LiteralPacket>(&serde_json::to_string(&packet).unwrap())
                .unwrap(),
            packet
        );
    }
}
#[test]
fn original_reference_arms_keep_actual_nominal_ids_and_span_subtype() {
    let id = vec![7u8; 16];
    for (kind, field) in [
        ("catalog", "source"),
        ("anchor", "anchor"),
        ("prose", "slice"),
        ("artifact", "artifact"),
        ("occurrence", "occurrence"),
        ("span", "span"),
    ] {
        let mut source = serde_json::json!({"kind":kind});
        source[field] = serde_json::json!(id);
        let request = decode("get_evidence", serde_json::json!({"source":source})).unwrap();
        let Request::GetEvidence(request) = request else {
            panic!("wrong retained route")
        };
        let encoded = serde_json::to_value(request.source).unwrap();
        assert_eq!(encoded["kind"], kind);
        assert_eq!(encoded[field], serde_json::json!(id));
        let mut extra = encoded;
        extra["invented_original"] = serde_json::json!(true);
        assert!(serde_json::from_value::<OriginalReference>(extra).is_err());
    }
    let schema = schema_for::<OriginalReference>(false);
    assert_eq!(schema["$defs"]["evidenceArm1Id"]["x-nominal-arm"], 1);
    assert!(
        serde_json::from_value::<OriginalReference>(
            serde_json::json!({"kind":"artifact","source":id})
        )
        .is_err()
    );
}

#[test]
fn final_transport_admission_counts_actual_utf8_envelope_and_metadata_bytes() {
    let limit = ResourceLimits::default().response_bytes(false) as usize;
    let exact = format!("\"{}\"", "a".repeat(limit - 2));
    assert_eq!(exact.len(), limit);
    admit_envelope(&exact, false).unwrap();
    assert!(matches!(
        admit_envelope(&(exact.clone() + "\n"), false),
        Err(WireError::ResourceRefused(_))
    ));
    let unicode = format!("\"{}aa\"", "é".repeat((limit - 4) / 2));
    assert_eq!(unicode.len(), limit);
    admit_envelope(&unicode, false).unwrap();
    let escaped = unicode.replace('é', "\\u00e9");
    assert!(escaped.len() > limit);
    assert!(matches!(
        admit_envelope(&escaped, false),
        Err(WireError::ResourceRefused(_))
    ));
    admit_envelope(&escaped, true).unwrap();
    let prefix = r#"{"jsonrpc":"2.0","id":""#;
    let suffix = r#"","result":{"content":[],"structuredContent":{},"isError":false}}"#;
    let rpc = format!(
        "{prefix}{}{suffix}",
        "r".repeat(limit - prefix.len() - suffix.len())
    );
    assert_eq!(rpc.len(), limit);
    admit_envelope(&rpc, false).unwrap();
    assert!(admit_envelope(&(rpc + "\n"), false).is_err());
    let prefix = format!(
        r#"{{"snapshot":{},"capability":{{"capability":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"title":"x","rendered":""#,
        serde_json::to_string(&snapshot_for(0)).unwrap()
    );
    let suffix = r#"","assertions":[],"originals":[],"availability":{"status":"available"},"unreviewed":true,"documentation_only":true}}"#;
    let raw = format!(
        "{prefix}{}{suffix}",
        "a".repeat(limit - prefix.len() - suffix.len())
    );
    assert_eq!(raw.len(), limit);
    decode_response("get_capability", &raw, false, &ResourceLimits::default()).unwrap();
    assert!(matches!(
        tool_result("get_capability", &raw, false),
        Err(WireError::ResourceRefused(_))
    ));
    let expanded = tool_result("get_capability", &raw, true).unwrap();
    let envelope: Value = serde_json::from_str(&expanded).unwrap();
    assert_eq!(envelope["isError"], false);
    assert_eq!(envelope["content"][0]["type"], "text");
    assert!(envelope["structuredContent"]["capability"].is_object());
}
#[test]
fn optional_packets_retain_their_canonical_question_and_typed_condition() {
    let schema = schema_for::<RelationshipPacket>(true);
    let arms = schema["oneOf"].as_array().unwrap();
    assert_eq!(arms.len(), 3);
    for arm in arms {
        let required = arm["required"].as_array().unwrap();
        assert!(required.contains(&json!("analysis")));
        assert!(required.contains(&json!("proof")));
    }
    assert!(
        schema_for::<ConflictPacket>(true)["required"]
            .as_array()
            .unwrap()
            .contains(&json!("requirement"))
    );
    let behavior = schema_for::<BehaviorPacket>(true);
    assert!(behavior.to_string().contains("RenderedConditionPacket"));
    assert!(
        serde_json::from_value::<RenderedConditionPacket>(
            json!({"terms":[],"truncated":false,"verdict":0})
        )
        .is_err()
    );
}

#[test]
fn bounded_final_response_and_distinct_mcp_envelope_keep_exact_byte_contracts() {
    let response = Response::GetCapability(GetCapabilityResponse {
        delivery: Optional::default(),
        snapshot: snapshot_for(7),
        capability: serde_json::from_value(json!({
            "capability": vec![6u8; 16],
            "title": "API",
            "rendered": "Original é🦀\n\t\"\\ body.",
            "assertions": [],
            "originals": [],
            "availability": {"status": "available"},
            "unreviewed": true,
            "documentation_only": true,
        }))
        .unwrap(),
    });
    let raw = response.to_json().unwrap();
    let envelope = tool_result("get_capability", &raw, false).unwrap();
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(envelope.len() * 4).unwrap();
    for limit in [raw.len(), raw.len() + 1] {
        let encoded = response.encode_json(&budget, limit).unwrap();
        assert_eq!(encoded.as_str(), raw);
        assert_eq!(
            serde_json::from_str::<Value>(encoded.as_str()).unwrap(),
            serde_json::from_str::<Value>(&raw).unwrap()
        );
        drop(encoded);
        assert_eq!(budget.reserved(), 0);
    }
    assert!(matches!(
        response.encode_json(&budget, raw.len() - 1),
        Err(WireError::ResourceRefused(_))
    ));
    for limit in [envelope.len(), envelope.len() + 1] {
        let encoded = response.encode_mcp_result(&budget, limit).unwrap();
        assert_eq!(encoded.as_str(), envelope);
        assert_eq!(encoded.as_str().len(), response.mcp_result_len().unwrap());
        let value: Value = serde_json::from_str(encoded.as_str()).unwrap();
        assert_eq!(
            value["structuredContent"],
            serde_json::from_str::<Value>(&raw).unwrap()
        );
        assert_eq!(value["isError"], false);
        drop(encoded);
        assert_eq!(budget.reserved(), 0);
    }
    assert!(matches!(
        response.encode_mcp_result(&budget, envelope.len() - 1),
        Err(WireError::ResourceRefused(_))
    ));
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn enrichment_sections_require_exact_comparison_selection_and_closed_shapes() {
    let base = json!({"library":"demo","operation":{"kind":"member","member":vec![1u8;16]},"sections":["callable_comparison"]});
    assert!(decode("get_operation", base.clone()).is_err());
    let mut request = base;
    request["comparison"] =
        json!({"analysis":vec![2u8;16],"left":vec![3u8;16],"right":vec![4u8;16]});
    decode("get_operation", request.clone()).unwrap();
    request["comparison"]["inferred_role"] = "first_effective".into();
    assert!(decode("get_operation", request).is_err());
    let schema = schema_for::<GetOperationRequest>(false).to_string();
    for section in [
        "callable_comparison",
        "contextual_typing",
        "incoming_references",
    ] {
        assert!(schema.contains(section));
    }
    let fields = schema_for::<OperationPacket>(true).to_string();
    assert!(fields.contains("ReferenceSearchScope"));
    assert!(
        fields.contains("DifferentRetainedStructure")
            || fields.contains("different_retained_structure")
    );
}

#[test]
fn evidence_body_pages_omit_terminal_continuation_and_reject_null() {
    let terminal = EvidenceBodyPage {
        start: 3,
        end: 6,
        bytes: vec![0, 128, 255],
        continuation: Optional::default(),
        omitted: 0,
        truncated: false,
    };
    let encoded = serde_json::to_value(&terminal).unwrap();
    assert!(encoded.get("continuation").is_none());
    assert_eq!(
        serde_json::from_value::<EvidenceBodyPage>(encoded.clone()).unwrap(),
        terminal
    );
    let continued = EvidenceBodyPage {
        continuation: Optional(Some(CursorToken::new("ab").unwrap())),
        omitted: 9,
        truncated: true,
        ..terminal.clone()
    };
    let encoded_continued = serde_json::to_value(&continued).unwrap();
    assert_eq!(encoded_continued["continuation"], "ab");
    assert_eq!(
        serde_json::from_value::<EvidenceBodyPage>(encoded_continued).unwrap(),
        continued
    );
    let mut null = encoded.clone();
    null["continuation"] = Value::Null;
    assert!(serde_json::from_value::<EvidenceBodyPage>(null).is_err());
    let mut unknown = encoded;
    unknown["invented"] = json!(true);
    assert!(serde_json::from_value::<EvidenceBodyPage>(unknown).is_err());
    for serialize in [true, false] {
        let schema = schema_for::<EvidenceBodyPage>(serialize);
        assert_eq!(schema["additionalProperties"], false);
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("continuation"))
        );
        assert!(
            !schema["properties"]["continuation"]
                .to_string()
                .contains("null")
        );
    }
}

#[test]
fn capability_assertion_status_is_structured_and_visible_without_rewriting_authored_bytes() {
    let claim = json!({"assertion":vec![1u8;16],"kind":0,"section":0,"status":1,"qualification":vec![2u8;16],"claim_basis":{"set":lctx_model::domain::assumptions::AssumptionSet::empty_id(),"members_digest":lctx_model::domain::assumptions::AssumptionSet::empty().members,"definitions":[]},"terminal_question":null,"text":"Authored result.","supports":[{"support":vec![3u8;16],"role":0,"source":vec![4u8;16],"proof":[{"kind":"entity","entity":vec![4u8;32]}]}]});
    let schema = schema_for::<AssertionPacket>(true);
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&claim));
    let mut missing = claim.clone();
    missing.as_object_mut().unwrap().remove("terminal_question");
    assert!(!validator.is_valid(&missing));
    assert!(serde_json::from_value::<AssertionPacket>(missing).is_err());
    let mut invalid = claim.clone();
    invalid["status"] = json!(99);
    assert!(!validator.is_valid(&invalid));
    assert!(serde_json::from_value::<AssertionPacket>(invalid).is_err());
    let mut observed = claim.clone();
    observed["assertion"] = json!(vec![5u8; 16]);
    observed["status"] = json!(0);
    observed["kind"] = json!(1);
    observed["section"] = json!(1);
    observed["text"] = json!("Observed access.");
    let capability:CapabilityPacket=serde_json::from_value(json!({"capability":vec![6u8;16],"title":"API","rendered":"Original authored body.\n","assertions":[claim,observed],"originals":[],"availability":{"status":"available"},"unreviewed":true,"documentation_only":true})).unwrap();
    let response = GetCapabilityResponse {
        delivery: Optional::default(),
        snapshot: snapshot_for(7),
        capability,
    };
    let resource = response.resource_text().unwrap();
    assert!(resource.starts_with("Original authored body.\n"));
    assert_eq!(
        response.capability.rendered.as_str(),
        "Original authored body.\n"
    );
    let snapshot_text = resource
        .split("## Snapshot metadata\n\n```json\n")
        .nth(1)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(snapshot_text).unwrap();
    assert_eq!(snapshot["snapshot"], json!(snapshot_for(7)));
    assert_eq!(snapshot["capability"], json!(vec![6u8; 16]));
    assert_eq!(snapshot["uri_scope"], "process");
    let mut other = response.clone();
    other.snapshot = snapshot_for(8);
    assert_ne!(other.resource_text().unwrap(), resource);
    assert_eq!(other.capability.rendered, response.capability.rendered);
    assert!(resource.contains("\"status_name\":\"Documented\""));
    assert!(resource.contains("\"status_name\":\"StructurallyObserved\""));
    assert!(resource.contains("Authored result."));
    assert!(resource.contains("Observed access."));
    let response = Response::GetCapability(response);
    let encoded = response.to_json().unwrap();
    assert!(tool_result("get_capability", &encoded, false).is_ok());
}

#[test]
fn discovery_explains_codes_and_closed_packet_bindings_cover_composition() {
    for declaration in tools() {
        assert!(!declaration.description.is_empty());
    }
    let selection = schema_for::<selection::Selection>(false).to_string();
    assert!(selection.contains("Discovery"));
    assert!(selection.contains("Strict"));
    assert!(selection.contains("integer"));
    assert!(selection.contains("RequireCompatible"));
    let native = mappings::PacketKind::NativeAssessmentPacket.binding();
    assert!(native.permits::<conditions::Condition>());
    assert!(
        !mappings::PacketKind::OperationCore
            .binding()
            .permits::<retrieval::consumption::RetrievalEmbeddingUse>()
    );
    let operation = mappings::PacketKind::OperationPacket.binding();
    assert!(operation.permits::<catalog::evidence::ScenarioSpan>());
    assert!(operation.permits::<source::SourceArtifact>());
    assert!(
        mappings::PacketKind::EvidenceHit
            .binding()
            .permits::<retrieval::OriginalAnchor>()
    );
    for kind in FailureKind::ALL {
        let failure = PublicFailure::new(kind);
        assert_eq!(failure.kind.name(), kind.name());
        assert_eq!(failure.message, kind.message());
        assert!(failure.message.len() < 100);
    }
    assert!(FailureKind::from_name("driver-secret").is_none());
}

#[test]
fn unsupported_facets_are_refused_by_every_selection_wire_route() {
    let selection = json!({"requirements":[{"predicate":{"FacetMembership":{"facet":6,"value":{"ParameterName":{"name":"timeout"}}}},"quantifier":0}],"mode":0,"joint":1});
    for tool in [
        "search_operations",
        "find_operations",
        "browse_library",
        "compare_operations",
    ] {
        let mut request = json!({"library":"control", "selection":selection});
        if tool.starts_with("search_") {
            request["query"] = json!("timeout");
        }
        if tool == "compare_operations" {
            request["operations"] = json!([{"kind":"public_path","path":["control","api"]}]);
        }
        assert!(
            matches!(decode(tool, request), Err(WireError::Invalid(message)) if message.contains("facet membership")),
            "{tool} must reject before service admission"
        );
    }
}

#[test]
fn behavior_requires_explicit_direct_capture_provenance_even_when_empty() {
    use lctx_model::domain::{assumptions::AssumptionSet, conditions::Diagram};
    let empty = AssumptionSet::empty();
    let mut wire = json!({"captures":[],"claim_basis":{"set":empty.id(),"members_digest":empty.members,"definitions":[]},
        "condition":Diagram::always().id(),"verdict":0,"model":vec![1u8;16],"proof":[],
        "presentation":{"terms":[],"truncated":false},"presentation_truncated":false});
    let packet: BehaviorPacket = serde_json::from_value(wire.clone()).unwrap();
    assert!(packet.captures.is_empty());
    assert_eq!(packet.claim_basis.set, AssumptionSet::empty_id());
    wire.as_object_mut().unwrap().remove("captures");
    assert!(serde_json::from_value::<BehaviorPacket>(wire).is_err());
    let schema = schema_for::<BehaviorPacket>(true);
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("captures"))
    );
    let timing = schema_for::<CaptureTimingPacket>(true);
    assert!(
        timing["required"]
            .as_array()
            .unwrap()
            .contains(&json!("characterization_only"))
    );
    let correspondence = schema_for::<CaptureSourceDeclarationPacket>(true);
    for field in [
        "source_correspondence_only",
        "qualification",
        "claim_basis",
        "support",
    ] {
        assert!(
            correspondence["required"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }
    let source_support = schema_for::<CaptureSourceSupportPacket>(true);
    for field in ["origin", "mode", "fidelity", "provider_id", "evidence"] {
        assert!(
            source_support["required"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }
    let native = schema_for::<CaptureNativeProofPacket>(true);
    for field in [
        "qualification",
        "modality",
        "approximation",
        "claim_basis",
        "support",
    ] {
        assert!(
            native["required"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }
}

#[test]
fn original_flow_inventory_schema_preserves_native_closure_and_value_boundary() {
    let value = serde_json::to_value(schemars::schema_for!(EvidencePacket))
        .unwrap()
        .to_string();
    for field in [
        "flow_inventory",
        "native_count",
        "mapped_count",
        "complete",
        "reachability",
        "narrowing",
        "narrowing_unavailable",
        "narrowing_precision_lost",
        "condition_unavailable",
        "reachability_lost",
        "entry_value_reason",
        "renamed_type_checking",
        "view_content",
    ] {
        assert!(value.contains(field), "original evidence loses {field}");
    }
    let binding =
        <EvidencePacket as lctx_model::domain::serving::mappings::PacketOutput>::binding();
    assert!(binding.permits::<lctx_model::domain::flow_inventory::FlowUseCandidate>());
    assert!(binding.permits::<lctx_model::domain::flow_inventory::FlowUseInventoryMember>());
    assert!(binding.permits::<lctx_model::domain::flow::FlowReachingSupport>());
}

#[test]
fn packet_proofs_address_selected_graph_owners_and_refuse_old_wire_rows() {
    let member = serde_json::from_value::<Id<catalog::CatalogMember>>(json!(vec![19; 16])).unwrap();
    let exposure =
        serde_json::from_value::<Id<catalog::CatalogExposure>>(json!(vec![23; 16])).unwrap();
    let entity = ProofReference::from_canonical(derivation::RowRef::of(member)).unwrap();
    let exposure_proof = ProofReference::from_canonical(derivation::RowRef::of(exposure)).unwrap();
    let support =
        serde_json::from_value::<Id<symbols::ClassTraitSupport>>(json!(vec![29; 16])).unwrap();
    let assertion = ProofReference::from_canonical(derivation::RowRef::of(support)).unwrap();
    assert_eq!(
        entity,
        ProofReference::Entity {
            entity: graph::EntityId::of(member).0.0
        }
    );
    assert_eq!(
        exposure_proof,
        ProofReference::Entity {
            entity: graph::EntityId::of(exposure).0.0
        }
    );
    assert_eq!(
        assertion,
        ProofReference::Assertion {
            assertion: graph::AssertionId::of(support).0.0
        }
    );
    assert_eq!(
        assertion.target(),
        graph::Target::Assertion(graph::AssertionId::of(support))
    );
    assert_eq!(
        entity.target(),
        graph::Target::Entity(graph::EntityId::of(member))
    );
    assert_ne!(entity, assertion);
    assert!(
        serde_json::from_value::<ProofReference>(
            json!({"relation":"catalog_members","row":vec![19;16]})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ProofReference>(json!({"kind":"entity","entity":vec![19;16]}))
            .is_err()
    );
}
