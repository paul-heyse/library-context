//! Independent raw type answers over the pinned native provider, through shared model validation.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    attribution::*, calls::*, declarations::ParameterDeclaration, source::*, types::*, *,
};
use typed_driver::{files, rows, run};
inspector!(
    Types,
    TypeTerm,
    TypeObservation,
    TypeSupport,
    TypePresentation,
    TypePresentationSupport,
    TypeVariable,
    TypeVariableRestriction,
    CallableParameterList,
    CallableParameter,
    TypedDictFieldList,
    TypedDictField,
    FunctionBodyObservation,
    RecordFieldObservation,
    lctx_model::domain::class_metadata::ClassMemberObservation,
    lctx_model::domain::class_metadata::ClassMemberSupport,
    ProviderSymbol,
    ParameterDeclaration,
    Occurrence,
    ProviderCoverage,
    Signature,
    SignatureParameter,
    ParameterShape,
    lctx_model::domain::value::Literal
);

#[tokio::test]
async fn native_types_keep_structural_terms_record_flags_and_declared_async_return() {
    let tables = typed_driver::Tables::default();
    run(&files("type_shapes"), Types(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let fields = rows::<RecordFieldObservation>(&tables);
    let named = |name: &str| {
        symbols
            .iter()
            .find(|s| s.name == name && s.kind == SymbolKind::Class)
            .unwrap()
            .id()
    };
    let movie: Vec<_> = fields
        .iter()
        .filter(|f| f.class == named("Movie"))
        .collect();
    assert_eq!(movie.len(), 2);
    assert!(
        movie
            .iter()
            .all(|f| f.record == RecordKind::TypedDict && f.read_only == Some(false))
    );
    assert_eq!(
        movie.iter().find(|f| f.name == "year").unwrap().required,
        Some(false)
    );
    let config: Vec<_> = fields
        .iter()
        .filter(|f| f.class == named("Config"))
        .collect();
    assert_eq!(config.len(), 5);
    assert_eq!(
        config.iter().find(|f| f.name == "internal").unwrap().init,
        Some(false)
    );
    assert_eq!(
        config.iter().find(|f| f.name == "token").unwrap().kw_only,
        Some(true)
    );
    let terms = rows::<TypeTerm>(&tables);
    assert!(
        terms
            .iter()
            .any(|t| matches!(t, TypeTerm::ParamSpec { .. }))
    );
    assert!(
        terms
            .iter()
            .any(|t| matches!(t, TypeTerm::EnumLiteral { .. }))
    );
    let occurrences = rows::<Occurrence>(&tables);
    let artifacts = rows::<SourceArtifact>(&tables);
    let source = artifacts
        .iter()
        .find(|a| a.path == "ts/__init__.py")
        .unwrap();
    let bytes = &files("type_shapes")["ts/__init__.py"];
    let start = bytes
        .windows(b"async def fetch".len())
        .position(|s| s == b"async def fetch")
        .unwrap() as i64;
    let fetch = occurrences
        .iter()
        .find(|o| {
            o.source == source.id()
                && o.syntax_kind == SyntaxKind::StmtFunctionDef
                && o.start == start
        })
        .unwrap();
    let observation = rows::<TypeObservation>(&tables)
        .into_iter()
        .find(|o| o.subject == fetch.id() && o.role == TypeRole::Return)
        .unwrap();
    assert!(observation.declared);
    let term = terms.iter().find(|t| t.id() == observation.term).unwrap();
    let TypeTerm::ClassInstance { class, .. } = term else {
        panic!("declared async return must be str: {term:?}");
    };
    assert_eq!(
        symbols.iter().find(|s| s.id() == *class).unwrap().name,
        "str"
    );
}

#[tokio::test]
async fn declared_def_parameters_have_typed_observations_at_their_formal_declarations() {
    let input=std::collections::BTreeMap::from([("example.py".into(),b"def f(a: int, /, b: str = '', *args: bytes, c: bool = False, **kwargs: float) -> str:\n    if type(a) is int and b:\n        return b\n    return ''\n".to_vec())]);
    let tables = typed_driver::Tables::default();
    run(&input, Types(tables.clone())).await.unwrap();
    let links = rows::<ParameterDeclaration>(&tables);
    let observations = rows::<TypeObservation>(&tables);
    assert_eq!(links.len(), 5);
    for link in links {
        assert!(
            observations.iter().any(|o| o.subject == link.declaration
                && o.role == TypeRole::Parameter
                && o.declared)
        );
    }
    assert!(observations.iter().any(|o| o.role == TypeRole::TestOperand));
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.family == FactFamily::Types)
    );
}

#[tokio::test]
async fn inherited_fields_refer_to_the_declaring_artifact_and_ignore_method_assignments() {
    let input=std::collections::BTreeMap::from([
        ("base.py".into(),b"from dataclasses import dataclass\n@dataclass\nclass Base:\n    x: int = 0\n".to_vec()),
        ("child.py".into(),b"from dataclasses import dataclass\nfrom base import Base\n@dataclass\nclass Child(Base):\n    def __post_init__(self):\n        self.x = 1\n".to_vec()),
    ]);
    let tables = typed_driver::Tables::default();
    run(&input, Types(tables.clone())).await.unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let fields = rows::<RecordFieldObservation>(&tables);
    let class = |name: &str| {
        symbols
            .iter()
            .find(|s| s.name == name && s.kind == SymbolKind::Class)
            .unwrap()
            .id()
    };
    let base = fields
        .iter()
        .find(|f| f.class == class("Base") && f.name == "x")
        .unwrap();
    let child = fields
        .iter()
        .find(|f| f.class == class("Child") && f.name == "x")
        .unwrap();
    assert!(base.declaration.is_some());
    assert_eq!(base.declaration, child.declaration);
    assert_eq!(base.term, child.term);
    assert!(child.declared);
    let declarations = rows::<Occurrence>(&tables);
    let sources = rows::<SourceArtifact>(&tables);
    let occurrence = declarations
        .iter()
        .find(|o| Some(o.id()) == child.declaration)
        .unwrap();
    assert_eq!(
        sources
            .iter()
            .find(|a| a.id() == occurrence.source)
            .unwrap()
            .path,
        "base.py"
    );
    let tables = typed_driver::Tables::default();
    run(&files("type_shapes"), Types(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let fields = rows::<RecordFieldObservation>(&tables);
    let class = |name: &str| {
        symbols
            .iter()
            .find(|s| s.name == name && s.kind == SymbolKind::Class)
            .unwrap()
            .id()
    };
    let base = fields
        .iter()
        .find(|f| f.class == class("BaseRec") && f.name == "x")
        .unwrap();
    let child = fields
        .iter()
        .find(|f| f.class == class("SubRec") && f.name == "x")
        .unwrap();
    assert_eq!(base.declaration, child.declaration);
    assert!(child.declaration.is_some());
}

#[tokio::test]
async fn typed_dictionary_keys_are_not_parameter_names() {
    let tables = typed_driver::Tables::default();
    run(&files("dictionary_keys"), Types(tables.clone()))
        .await
        .unwrap();
    let fields = rows::<TypedDictField>(&tables);
    let record_fields = rows::<RecordFieldObservation>(&tables);
    let names: std::collections::BTreeSet<_> = fields
        .iter()
        .map(|f| f.name.as_str())
        .chain(
            record_fields
                .iter()
                .filter(|f| f.record == RecordKind::TypedDict)
                .map(|f| f.name.as_str()),
        )
        .collect();
    let members = rows::<lctx_model::domain::class_metadata::ClassMemberObservation>(&tables);
    let supports = rows::<lctx_model::domain::class_metadata::ClassMemberSupport>(&tables);
    let symbols = rows::<ProviderSymbol>(&tables);
    for name in ["", " ", "not-an-identifier"] {
        let member = members
            .iter()
            .find(|m| {
                m.name == name
                    && symbols
                        .iter()
                        .any(|s| s.id() == m.class && s.name == "Keys")
            })
            .expect("raw native TypedDict key remains a member candidate");
        member.validate().unwrap();
        assert!(
            supports.iter().any(|s| s.assertion == member.id()),
            "native member keeps attributed support"
        );
    }
    assert!(names.contains(""), "{names:?}");
    assert!(names.contains(" "), "{names:?}");
    assert!(names.contains("not-an-identifier"), "{names:?}");
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.family == FactFamily::Signatures
                && c.status == CoverageStatus::Partial
                && c.reason == Some(ObligationKind::OutsideProviderModel))
    );
    // Dictionary keys remain record fields. The former collector-derived generated source
    // signature is retired; it cannot manufacture source formals from mapping keys.
    let signatures = rows::<Signature>(&tables);
    let parameters = rows::<SignatureParameter>(&tables);
    let shapes = rows::<ParameterShape>(&tables);
    assert!(
        signatures
            .iter()
            .filter(|s| s.role.runtime_source())
            .all(|s| parameters
                .iter()
                .filter(|p| p.signature == s.id())
                .all(|p| shapes
                    .iter()
                    .find(|shape| shape.id() == p.shape)
                    .unwrap()
                    .name
                    .as_deref()
                    != Some("")))
    );
}

#[tokio::test]
async fn native_unicode_values_and_dictionary_keys_preserve_nul() {
    use lctx_model::domain::value::Literal;
    let tables = typed_driver::Tables::default();
    run(&files("string_values"), Types(tables.clone()))
        .await
        .unwrap();
    let literals = rows::<Literal>(&tables);
    for expected in ["\0", "é\0終"] {
        assert!(
            literals
                .iter()
                .any(|l| matches!(l, Literal::String { value } if value.as_str() == expected)),
            "{literals:?}"
        );
    }
    assert!(
        rows::<RecordFieldObservation>(&tables)
            .iter()
            .any(|f| f.name.as_str() == "\0")
    );
}
