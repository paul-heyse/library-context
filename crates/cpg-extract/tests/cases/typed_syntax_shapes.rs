//! The `pyrefly` stage's typed syntax (cutover plan A4) over the pinned parse: fixtures are
//! acquired as trees and run through `acquire`, `pyrefly` and `assemble` into a stage-bound memory
//! generation the model validates. Every known answer is written from the fixture's source.
use cpg_extract::{pyrefly_stage::Pyrefly, typed_syntax::SyntaxLimits};
use lctx_model::domain::{
    attribution::*, calls::*, obligation::ObligationKind, source::*, syntax::*, value::*, *,
};
use std::{collections::BTreeMap, path::Path};

/// Everything the stages wrote, and the captured bytes to read texts from.
#[derive(Default, Clone)]
struct Facts {
    files: BTreeMap<String, Vec<u8>>,
    artifacts: Vec<SourceArtifact>,
    modules: Vec<Module>,
    occurrences: Vec<Occurrence>,
    placements: Vec<SyntaxPlacement>,
    details: Vec<SyntaxDetailObservation>,
    syntax_details: Vec<SyntaxDetail>,
    declarations: Vec<DeclarationObservation>,
    decorators: Vec<DeclarationDecorator>,
    imports: Vec<ImportAliasObservation>,
    dunder_all: Vec<DunderAllObservation>,
    parameters: Vec<ParameterSyntaxObservation>,
    calls: Vec<CallSyntax>,
    symbols: Vec<ProviderSymbol>,
    symbol_declarations: Vec<lctx_model::domain::declarations::SymbolDeclaration>,
    signatures: Vec<Signature>,
    qualifications: Vec<lctx_model::domain::assertion::AssertionQualification>,
    specializations: Vec<lctx_model::domain::types::GenericSpecializationObservation>,
    arguments: Vec<CallArgument>,
    literals: Vec<Literal>,
    members: Vec<LiteralSetMember>,
    boundaries: Vec<SubjectBoundary>,
    coverage: Vec<ProviderCoverage>,
}
impl Facts {
    fn occurrence(&self, id: Id<Occurrence>) -> &Occurrence {
        self.occurrences
            .iter()
            .find(|o| o.id() == id)
            .expect("a stated occurrence")
    }
    fn path(&self, source: Id<SourceArtifact>) -> &str {
        &self
            .artifacts
            .iter()
            .find(|a| a.id() == source)
            .unwrap()
            .path
    }
    /// The captured bytes at an occurrence, as text.
    fn text(&self, id: Id<Occurrence>) -> String {
        let o = self.occurrence(id);
        String::from_utf8(
            self.files[self.path(o.source)][o.start as usize..o.end as usize].to_vec(),
        )
        .unwrap()
    }
    fn in_file<'a, T>(
        &'a self,
        rows: &'a [T],
        path: &'a str,
        subject: impl Fn(&T) -> Id<Occurrence> + 'a,
    ) -> impl Iterator<Item = &'a T> + 'a {
        rows.iter()
            .filter(move |row| self.path(self.occurrence(subject(row)).source) == path)
    }
    /// Import aliases of one file, in source order.
    fn imports(&self, path: &str) -> Vec<(String, i64, Option<String>)> {
        let mut rows: Vec<&ImportAliasObservation> =
            self.in_file(&self.imports, path, |i| i.alias).collect();
        rows.sort_by_key(|i| self.occurrence(i.alias).start);
        rows.into_iter()
            .map(|i| (self.text(i.alias), i.level, i.resolved_module.clone()))
            .collect()
    }
    fn names(&self, set: Id<LiteralSet>) -> Vec<String> {
        let mut names: Vec<String> = self
            .members
            .iter()
            .filter(|m| m.set == set)
            .map(|m| match self.literals.iter().find(|l| l.id() == m.value) {
                Some(Literal::String { value }) => value.to_string(),
                other => panic!("not a string: {other:?}"),
            })
            .collect();
        names.sort();
        names
    }
    fn literal(&self, id: Id<Literal>) -> &Literal {
        self.literals.iter().find(|l| l.id() == id).unwrap()
    }
}

use crate::typed_driver;
use crate::inspector;
inspector!(Inspect, SourceArtifact, Occurrence);
async fn run(files: BTreeMap<String, Vec<u8>>, limits: SyntaxLimits) -> Facts {
    let tables = typed_driver::Tables::default();
    typed_driver::run_with(
        typed_driver::capture(
            &files,
            "syntax shapes",
            lctx_model::domain::stages::Profile::Catalog,
        ),
        Pyrefly::new(limits),
        Inspect(tables.clone()),
    )
    .await
    .unwrap();
    Facts {
        files,
        artifacts: typed_driver::rows(&tables),
        modules: typed_driver::rows(&tables),
        occurrences: typed_driver::rows(&tables),
        placements: typed_driver::rows(&tables),
        details: typed_driver::rows(&tables),
        syntax_details: typed_driver::rows(&tables),
        declarations: typed_driver::rows(&tables),
        decorators: typed_driver::rows(&tables),
        imports: typed_driver::rows(&tables),
        dunder_all: typed_driver::rows(&tables),
        parameters: typed_driver::rows(&tables),
        calls: typed_driver::rows(&tables),
        symbols: typed_driver::rows(&tables),
        symbol_declarations: typed_driver::rows(&tables),
        signatures: typed_driver::rows(&tables),
        qualifications: typed_driver::rows(&tables),
        specializations: typed_driver::rows(&tables),
        arguments: typed_driver::rows(&tables),
        literals: typed_driver::rows(&tables),
        members: typed_driver::rows(&tables),
        boundaries: typed_driver::rows(&tables),
        coverage: typed_driver::rows(&tables),
    }
}
fn fixture(case: &str) -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python")
        .join(case);
    walkdir::WalkDir::new(&root)
        .sort_by_file_name()
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path()
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn syntax_shapes_states_declarations_parameters_calls_and_operators() {
    let f = run(fixture("syntax_shapes"), SyntaxLimits::default()).await;
    let path = "syn/__init__.py";
    assert_eq!(
        f.modules
            .iter()
            .map(|m| m.qualified_name.as_str())
            .collect::<Vec<_>>(),
        ["syn"]
    );
    let mut declared: Vec<(String, DeclarationKind)> = f
        .declarations
        .iter()
        .map(|d| (f.text(d.name), d.kind))
        .collect();
    declared.sort_by_key(|d| {
        f.declarations
            .iter()
            .find(|x| f.text(x.name) == d.0)
            .map(|x| f.occurrence(x.declaration).start)
    });
    use DeclarationKind::*;
    assert_eq!(
        declared,
        [
            ("ConfigError".into(), Class),
            ("guarded".into(), Function),
            ("handled".into(), Function),
            ("matched".into(), Function),
            ("looped".into(), Function),
            ("decorated".into(), Function)
        ]
    );
    assert!(
        f.declarations
            .iter()
            .all(|d| d.parent.is_none() && d.docstring.is_none() && !d.overload)
    );
    assert_eq!(
        f.decorators
            .iter()
            .map(|d| f.text(d.decorator))
            .collect::<Vec<_>>(),
        ["@functools.lru_cache(maxsize=8)"]
    );
    assert_eq!(
        f.imports
            .iter()
            .map(|i| (f.text(i.alias), i.level, i.resolved_module.clone()))
            .collect::<Vec<_>>(),
        [("functools".into(), 0, Some("functools".into()))]
    );
    let guarded = f
        .declarations
        .iter()
        .find(|d| f.text(d.name) == "guarded")
        .unwrap()
        .declaration;
    let mut parameters: Vec<_> = f
        .parameters
        .iter()
        .filter(|p| p.function == guarded)
        .collect();
    parameters.sort_by_key(|p| p.ordinal);
    let shown: Vec<(String, ParameterKind, Option<Literal>)> = parameters
        .iter()
        .map(|p| {
            (
                f.text(p.parameter),
                p.kind,
                p.default_literal.map(|l| f.literal(l).clone()),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("x".into(), ParameterKind::PositionalOrKeyword, None),
            (
                "mode=\"fast\"".into(),
                ParameterKind::PositionalOrKeyword,
                Some(Literal::String {
                    value: "fast".into()
                })
            ),
            (
                "limit=0".into(),
                ParameterKind::PositionalOrKeyword,
                Some(Literal::Integer {
                    decimal: "0".into()
                })
            )
        ]
    );
    let mut callees: Vec<String> = f.calls.iter().map(|c| f.text(c.callee)).collect();
    callees.sort();
    assert_eq!(
        callees,
        [
            "ConfigError",
            "ConfigError",
            "ValueError",
            "fh.read",
            "functools.lru_cache",
            "key.upper",
            "len",
            "len",
            "open",
            "open",
            "open(path).read"
        ]
    );
    let cache = f
        .calls
        .iter()
        .find(|c| f.text(c.callee) == "functools.lru_cache")
        .unwrap();
    let arguments: Vec<_> = f
        .arguments
        .iter()
        .filter(|a| a.call == cache.id())
        .map(|a| (a.kind, a.keyword.clone(), f.text(a.value)))
        .collect();
    assert_eq!(
        arguments,
        [(ArgumentKind::Keyword, Some("maxsize".into()), "8".into())]
    );
    // `0 <= limit < 10` states two comparison operators in order.
    let chain = f
        .occurrences
        .iter()
        .find(|o| o.syntax_kind == SyntaxKind::ExprCompare && f.text(o.id()) == "0 <= limit < 10")
        .unwrap();
    let mut chained: Vec<_> = f
        .details
        .iter()
        .filter(|d| d.occurrence == chain.id())
        .collect();
    chained.sort_by_key(|d| d.ordinal);
    let operator = |d: &&SyntaxDetailObservation| {
        f.syntax_details
            .iter()
            .find(|s| s.id() == d.detail)
            .cloned()
            .unwrap()
    };
    assert_eq!(
        chained.iter().map(operator).collect::<Vec<_>>(),
        [
            SyntaxDetail::Operator {
                operator: OperatorKind::LtE
            },
            SyntaxDetail::Operator {
                operator: OperatorKind::Lt
            }
        ]
    );
    // Every occurrence is placed once, under its parse parent; only the module has none.
    assert_eq!(f.placements.len(), f.occurrences.len());
    let roots: Vec<_> = f.placements.iter().filter(|p| p.parent.is_none()).collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(
        f.occurrence(roots[0].occurrence).syntax_kind,
        SyntaxKind::ModModule
    );
    let raise = f
        .occurrences
        .iter()
        .find(|o| {
            o.syntax_kind == SyntaxKind::StmtRaise
                && f.text(o.id()).starts_with("raise ConfigError(\"cannot")
        })
        .unwrap();
    let cause = f
        .placements
        .iter()
        .find(|p| {
            p.parent == Some(raise.id())
                && p.field == lctx_model::domain::lexical::SyntaxField::Cause
        })
        .unwrap();
    assert_eq!(f.text(cause.occurrence), "err");
    let syntax = f
        .coverage
        .iter()
        .find(|c| c.family == FactFamily::Syntax)
        .unwrap();
    assert_eq!(
        syntax.status,
        CoverageStatus::CompleteUnderStatedModel,
        "{syntax:?}"
    );
    assert!(f.coverage.iter().any(|c| c.family == FactFamily::Types));
    // Source declarations stay located even when a native callable shape is unavailable.
    assert!(
        f.coverage
            .iter()
            .filter(|c| c.family == FactFamily::Exports)
            .all(|c| c.status == CoverageStatus::CompleteUnderStatedModel)
    );
    for declaration in &f.declarations {
        assert!(
            f.symbol_declarations
                .iter()
                .any(|d| d.declaration == declaration.declaration),
            "native declaration remains attached: {}",
            f.text(declaration.name)
        );
    }
    for coverage in f
        .coverage
        .iter()
        .filter(|c| c.family == FactFamily::Signatures)
    {
        match coverage.status {
            CoverageStatus::CompleteUnderStatedModel => {}
            CoverageStatus::Partial => {
                assert_eq!(
                    coverage.reason,
                    Some(ObligationKind::OutsideProviderModel),
                    "{coverage:?}"
                );
                let disclosed: Vec<_> = f
                    .boundaries
                    .iter()
                    .filter(|b| {
                        b.scope == coverage.scope
                            && Some(b.provider) == coverage.provider
                            && b.context == coverage.context
                            && b.family == coverage.family
                            && Some(b.reason) == coverage.reason
                    })
                    .collect();
                let unavailable = f.signatures.iter().any(|s| {
                    s.form == SignatureForm::NativeUnavailable
                        && s.scope == coverage.scope
                        && f.qualifications
                            .iter()
                            .any(|q| q.id() == s.qualification && q.context == coverage.context)
                        && f.symbols.iter().any(|symbol| {
                            symbol.id() == s.symbol && Some(symbol.provider) == coverage.provider
                        })
                });
                assert!(
                    !disclosed.is_empty() || unavailable,
                    "partial signatures retain a same-frame native fallback or attributed boundary: {coverage:?}"
                );
                for boundary in disclosed {
                    let detail = boundary.detail.as_deref().unwrap();
                    if detail.starts_with("native signature variant") {
                        assert!(
                            f.signatures
                                .iter()
                                .any(|s| s.form == SignatureForm::NativeUnavailable),
                            "unavailable native binding shape remains represented: {:?}",
                            f.symbols
                        );
                    } else {
                        assert!(
                            detail.starts_with("model-context "),
                            "unexpected native signature boundary: {boundary:?}"
                        );
                    }
                }
            }
            other => {
                panic!("source signature inventory must remain available: {other:?}, {coverage:?}")
            }
        }
    }
    let _ = path;
}

#[tokio::test]
async fn unicode_names_a_bom_and_crlf_keep_byte_spans() {
    let f = run(fixture("unicode_bom"), SyntaxLimits::default()).await;
    let mut modules: Vec<&str> = f
        .modules
        .iter()
        .map(|m| m.qualified_name.as_str())
        .collect();
    modules.sort();
    assert_eq!(modules, ["lcfix", "lcfix.core"]);
    let core = "lcfix/core.py";
    let names: Vec<String> = f
        .in_file(&f.declarations, core, |d| d.declaration)
        .map(|d| f.text(d.name))
        .collect();
    for expected in [
        "Base",
        "méthode",
        "Écrivain",
        "__new__",
        "__init__",
        "étiquette",
        "build",
        "λ_helper",
        "utilise",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "{expected} in {names:?}"
        );
    }
    let builds: Vec<bool> = f
        .in_file(&f.declarations, core, |d| d.declaration)
        .filter(|d| f.text(d.name) == "build")
        .map(|d| d.overload)
        .collect();
    assert_eq!(
        (builds.len(), builds.iter().filter(|o| **o).count()),
        (3, 2),
        "two @overload signatures and the implementation"
    );
    let ecrivain = f
        .declarations
        .iter()
        .find(|d| f.text(d.name) == "Écrivain")
        .unwrap();
    assert_eq!(
        f.text(ecrivain.docstring.unwrap()),
        "\"\"\"Écrit des données — “quoted”.\"\"\""
    );
    let methods: Vec<_> = f
        .declarations
        .iter()
        .filter(|d| d.parent == Some(ecrivain.declaration))
        .map(|d| f.text(d.name))
        .collect();
    assert_eq!(methods.len(), 4, "{methods:?}");
    let property = f
        .declarations
        .iter()
        .find(|d| f.text(d.name) == "étiquette")
        .unwrap();
    assert_eq!(
        f.decorators
            .iter()
            .filter(|d| d.declaration == property.declaration)
            .map(|d| f.text(d.decorator))
            .collect::<Vec<_>>(),
        ["@property"]
    );
    let new = f
        .declarations
        .iter()
        .find(|d| f.text(d.name) == "__new__")
        .unwrap();
    let mut kinds: Vec<_> = f
        .parameters
        .iter()
        .filter(|p| p.function == new.declaration)
        .map(|p| (p.ordinal, p.kind, f.text(p.parameter)))
        .collect();
    kinds.sort_by_key(|k| k.0);
    // Ruff's variadic parameter node spans its star.
    assert_eq!(
        kinds,
        [
            (0, ParameterKind::PositionalOrKeyword, "cls".into()),
            (1, ParameterKind::VarPositional, "*args: object".into()),
            (2, ParameterKind::VarKeyword, "**kwargs: object".into())
        ]
    );
    let init = f
        .declarations
        .iter()
        .find(|d| f.text(d.name) == "__init__")
        .unwrap();
    let nom = f
        .parameters
        .iter()
        .find(|p| p.function == init.declaration && p.ordinal == 1)
        .unwrap();
    assert_eq!(
        f.literal(nom.default_literal.unwrap()),
        &Literal::String {
            value: "défaut".into()
        }
    );
    let lambda = f
        .parameters
        .iter()
        .find(|p| f.occurrence(p.function).syntax_kind == SyntaxKind::ExprLambda)
        .unwrap();
    assert_eq!(f.text(lambda.parameter), "z");
    // The package's BOM and CRLF lines: spans are byte offsets in the captured bytes.
    let init_py = "lcfix/__init__.py";
    let imports = f.imports(init_py);
    assert_eq!(
        imports,
        [
            ("Écrivain".into(), 0, Some("lcfix.core".into())),
            ("build".into(), 0, Some("lcfix.core".into())),
            ("λ_helper as helper".into(), 0, Some("lcfix.core".into())),
            ("core as core".into(), 0, Some("lcfix".into()))
        ]
    );
    let all = f
        .in_file(&f.dunder_all, init_py, |d| d.statement)
        .next()
        .unwrap();
    assert_eq!(
        (f.text(all.statement).as_str(), f.names(all.names.unwrap())),
        (
            "__all__ = [\"Écrivain\", \"build\", \"helper\"]",
            vec!["build".into(), "helper".into(), "Écrivain".into()]
        )
    );
}

#[tokio::test]
async fn literal_all_is_stated_and_computed_all_is_a_boundary() {
    let f = run(fixture("dunder_all"), SyntaxLimits::default()).await;
    let lit: Vec<Vec<String>> = f
        .in_file(&f.dunder_all, "dunder/lit.py", |d| d.statement)
        .map(|d| f.names(d.names.unwrap()))
        .collect();
    let mut lit = lit;
    lit.sort();
    assert_eq!(
        lit,
        [
            vec!["also".to_owned()],
            vec!["exported".to_owned()],
            vec!["third".to_owned()]
        ],
        "assign, += and .append state literal names"
    );
    assert_eq!(
        f.in_file(&f.dunder_all, "dunder/sub/__init__.py", |d| d.statement)
            .map(|d| f.names(d.names.unwrap()))
            .collect::<Vec<_>>(),
        [vec!["alpha".to_owned(), "beta".to_owned()]]
    );
    let mut computed: Vec<String> = f
        .dunder_all
        .iter()
        .filter(|d| !d.literal)
        .map(|d| f.text(d.statement))
        .collect();
    computed.sort();
    assert_eq!(
        computed,
        ["__all__ = _names()", "__all__ = sub.__all__ + [\"top\"]"]
    );
    let mut boundaries: Vec<(String, ObligationKind, FactFamily)> = f
        .boundaries
        .iter()
        .map(|b| (f.text(b.subject.unwrap()), b.reason, b.family))
        .collect();
    boundaries.sort();
    assert_eq!(
        boundaries,
        [
            (
                "__all__ = _names()".into(),
                ObligationKind::OutsideProviderModel,
                FactFamily::Exports
            ),
            (
                "__all__ = sub.__all__ + [\"top\"]".into(),
                ObligationKind::OutsideProviderModel,
                FactFamily::Exports
            ),
            (
                "__all__.append".into(),
                ObligationKind::MissingEvidence,
                FactFamily::Types
            )
        ]
    );
    let boundary = f
        .boundaries
        .iter()
        .find(|b| {
            b.family == FactFamily::Types
                && b.subject.is_some_and(|s| f.text(s) == "__all__.append")
        })
        .unwrap();
    assert_eq!(
        boundary.detail.as_deref(),
        Some("native generic receiver lacks emitted signature origin")
    );
    assert!(
        !f.specializations
            .iter()
            .any(|s| Some(s.site) == boundary.subject),
        "missing native signature origin cannot invent a receiver substitution"
    );
    assert!(f.coverage.iter().any(|c| c.family == boundary.family
        && c.scope == boundary.scope
        && c.context == boundary.context
        && c.provider == Some(boundary.provider)
        && c.status == CoverageStatus::Partial));
    let imports = f.imports("dunder/__init__.py");
    assert_eq!(
        imports,
        [
            ("sub".into(), 1, Some("dunder".into())),
            ("*".into(), 1, Some("dunder.sub".into())),
            ("dyn_public".into(), 1, Some("dunder.dyn".into()))
        ]
    );
}

#[tokio::test]
async fn with_items_are_distinct_occurrences_and_limits_disclose() {
    let source = b"with open(a) as x, open(b) as y:\n    pass\n".to_vec();
    let f = run(
        BTreeMap::from([("w.py".to_owned(), source.clone())]),
        SyntaxLimits::default(),
    )
    .await;
    let items: Vec<String> = f
        .occurrences
        .iter()
        .filter(|o| o.syntax_kind == SyntaxKind::WithItem)
        .map(|o| f.text(o.id()))
        .collect();
    assert_eq!(items, ["open(a) as x", "open(b) as y"]);
    assert!(
        f.occurrences
            .iter()
            .filter(|o| o.syntax_kind == SyntaxKind::WithItem)
            .all(|o| o.role == OccurrenceRole::WithItem)
    );
    // A traversal bound keeps the emitted prefix and discloses Partial Syntax coverage.
    let bounded = run(
        BTreeMap::from([("w.py".to_owned(), source)]),
        SyntaxLimits {
            nodes: 4,
            ..SyntaxLimits::default()
        },
    )
    .await;
    assert_eq!(bounded.occurrences.len(), 4);
    let syntax = bounded
        .coverage
        .iter()
        .find(|c| c.family == FactFamily::Syntax)
        .unwrap();
    assert_eq!(
        (syntax.status, syntax.reason),
        (
            CoverageStatus::Partial,
            Some(ObligationKind::ResourceRefused)
        )
    );
    assert!(
        bounded.calls.is_empty(),
        "records are stated only from a complete traversal"
    );
}
