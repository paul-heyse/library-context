//! Pysa's definitions of one analyzed module as the model's symbol records (cutover plan A9): the
//! symbols it defines and where they nest, their function and class traits, bases and method
//! resolution orders, undecorated signatures with each parameter's displayed annotation, and the
//! declaration links from each symbol and signature parameter to the occurrence that declares it.
//! A link is made only at an exact name span; any other outcome is a boundary, never a name match.
#![deny(clippy::wildcard_enum_match_arm)]
use crate::{natives::Natives, syntax_records::Spans};
use lctx_model::domain::{
    Id, ModelError, Record,
    assertion::AssertionQualification,
    calls::*,
    declarations::{ParameterDeclaration, SymbolDeclaration},
    source::{Occurrence, SyntaxKind},
    symbols::*,
    syntax::{DeclarationObservation, ParameterSyntaxObservation},
};
use pyrefly::report::pysa::{
    PysaModuleDefinitions,
    class::{ClassRef, PysaClassMro},
    function::{FunctionParameter, FunctionParameters, FunctionRef},
    location::PysaLocation,
    module::ModuleId,
    scope::ScopeParent,
};
use pyrefly_python::module_name::ModuleName;
use ruff_source_file::{LineIndex, OneIndexed, PositionEncoding, SourceLocation};
use ruff_text_size::{TextRange, TextSize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn invalid(message: String) -> ModelError {
    ModelError::Invalid(message)
}

/// Pysa locations (1-based line, 1-based UTF-8 byte column) as byte ranges of the module's text:
/// the exact inverse of `PysaLocation::from_text_range`.
pub struct Locator<'a> {
    pub line_index: &'a LineIndex,
    pub text: &'a str,
}
impl Locator<'_> {
    fn offset(&self, line: u32, column: u32) -> TextSize {
        self.line_index.offset(
            SourceLocation {
                line: OneIndexed::new(line as usize).unwrap_or(OneIndexed::MIN),
                character_offset: OneIndexed::new(column as usize).unwrap_or(OneIndexed::MIN),
            },
            self.text,
            PositionEncoding::Utf8,
        )
    }
    pub fn range(&self, location: &PysaLocation) -> TextRange {
        TextRange::new(
            self.offset(location.line(), location.col()),
            self.offset(location.end_line(), location.end_col()),
        )
    }
}

/// What the module's syntax records declare: each declaration's name occurrence, and each `def`'s
/// parameters in order with their kinds.
#[derive(Default)]
pub struct Declared {
    names: HashMap<Id<Occurrence>, Id<Occurrence>>,
    parameters: ParameterIndex,
}
impl Declared {
    /// `formals` pairs each parameter occurrence with the `Parameter` node a link names.
    pub fn new(
        declarations: &[DeclarationObservation],
        parameters: &[ParameterSyntaxObservation],
        formals: &[(Id<Occurrence>, Id<Occurrence>)],
    ) -> Result<Self, ModelError> {
        let formals: HashMap<_, _> = formals.iter().copied().collect();
        let mut declared = Self {
            names: declarations
                .iter()
                .map(|d| (d.name, d.declaration))
                .collect(),
            parameters: HashMap::new(),
        };
        for p in parameters {
            let formal = *formals
                .get(&p.parameter)
                .ok_or_else(|| invalid("a parameter occurrence has no formal".into()))?;
            declared
                .parameters
                .entry(p.function)
                .or_default()
                .push((p.ordinal, formal, p.kind));
        }
        for list in declared.parameters.values_mut() {
            list.sort_by_key(|(ordinal, _, _)| *ordinal);
        }
        Ok(declared)
    }
}

/// One undecorated signature's parameters: shapes and displayed annotations.
struct Formal {
    shape: ParameterShape,
    display: Option<String>,
}
fn formals(parameters: &FunctionParameters) -> (SignatureForm, Vec<Formal>) {
    match parameters {
        FunctionParameters::List(list) => (
            SignatureForm::List,
            list.iter()
                .map(|p| {
                    let (kind, name, required, annotation) = match p {
                        FunctionParameter::PosOnly {
                            name,
                            annotation,
                            required,
                        } => (
                            ParameterKind::PositionalOnly,
                            name.as_ref().map(ToString::to_string),
                            *required,
                            annotation,
                        ),
                        FunctionParameter::Pos {
                            name,
                            annotation,
                            required,
                        } => (
                            ParameterKind::PositionalOrKeyword,
                            Some(name.to_string()),
                            *required,
                            annotation,
                        ),
                        FunctionParameter::VarArg { name, annotation } => (
                            ParameterKind::VarPositional,
                            name.as_ref().map(ToString::to_string),
                            false,
                            annotation,
                        ),
                        FunctionParameter::KwOnly {
                            name,
                            annotation,
                            required,
                        } => (
                            ParameterKind::KeywordOnly,
                            Some(name.to_string()),
                            *required,
                            annotation,
                        ),
                        FunctionParameter::Kwargs { name, annotation } => (
                            ParameterKind::VarKeyword,
                            name.as_ref().map(ToString::to_string),
                            false,
                            annotation,
                        ),
                    };
                    Formal {
                        shape: ParameterShape {
                            name: name.map(Into::into),
                            kind,
                            required,
                        },
                        display: Some(annotation.string.clone()).filter(|d| !d.is_empty()),
                    }
                })
                .collect(),
        ),
        FunctionParameters::Ellipsis => (SignatureForm::Ellipsis, vec![]),
        FunctionParameters::ParamSpec => (SignatureForm::ParamSpec, vec![]),
    }
}

/// One module's symbol records, all under the module's qualification.
#[derive(Default)]
pub struct SymbolRecords {
    pub symbols: Vec<SymbolObservation>,
    pub functions: Vec<FunctionTraitObservation>,
    pub classes: Vec<ClassTraitObservation>,
    pub ancestry: Vec<ClassAncestryObservation>,
    pub sequences: Vec<(SymbolSequence, Vec<SymbolSequenceMember>)>,
    pub shapes: Vec<ParameterShape>,
    pub signatures: Vec<(Signature, Vec<SignatureParameter>)>,
    pub annotations: Vec<ParameterAnnotationObservation>,
    pub declarations: Vec<SymbolDeclaration>,
    pub parameter_declarations: Vec<ParameterDeclaration>,
    /// Symbols (with a name span) whose declaration no exact span attaches, and why.
    pub unattached: Vec<(Id<ProviderSymbol>, String)>,
    pub native_unavailable: Vec<(Id<ProviderSymbol>, String)>,
}

/// Resolves a module a reference names by Pysa's id and the name it carries.
pub type Resolve<'r> =
    dyn FnMut(&mut Natives, ModuleId, &ModuleName) -> Result<Id<ProviderModule>, ModelError> + 'r;
/// How an analyzed module's records link to its occurrences.
pub struct Linking<'a> {
    pub locator: &'a Locator<'a>,
    pub spans: &'a Spans,
    pub declared: &'a Declared,
}

/// Pysa's key of a definition's enclosing scope.
fn parent_key(scope: &ScopeParent) -> Option<String> {
    match scope {
        ScopeParent::TopLevel => None,
        ScopeParent::Class { class_id } => Some(class_id.to_int().to_string()),
        ScopeParent::Function { func_def_index } => Some(format!("F:{}", func_def_index.0)),
    }
}
/// The keys of the module's top-level definitions named `name`.
pub fn top_level(definitions: &PysaModuleDefinitions, name: &str) -> Vec<String> {
    let classes = definitions
        .class_definitions
        .iter()
        .filter(|(_, c)| c.name == name && matches!(c.parent, ScopeParent::TopLevel))
        .map(|(id, _)| id.to_int().to_string());
    let functions = definitions
        .function_definitions
        .as_map()
        .iter()
        .filter(|(_, f)| {
            f.base.name.as_str() == name && matches!(f.base.parent, ScopeParent::TopLevel)
        })
        .map(|(id, _)| id.serialize_to_string());
    classes.chain(functions).collect()
}

/// The records of `definitions`, the module `module`. `complete` states, per class id, whether its
/// resolved MRO is the complete C3 linearization. An analyzed module is `linked` to its
/// occurrences and states every definition; a dependency module states only the definitions
/// `keep` names, with their enclosing definitions.
#[allow(
    clippy::too_many_arguments,
    reason = "the module's definitions and the indices that place them, each distinct"
)]
pub fn records(
    definitions: &PysaModuleDefinitions,
    module: Id<ProviderModule>,
    qualification: &AssertionQualification,
    natives: &mut Natives,
    resolve: &mut Resolve<'_>,
    complete: &HashMap<u32, bool>,
    origins: &HashMap<String, FunctionOrigin>,
    linking: Option<Linking<'_>>,
    keep: Option<&BTreeSet<String>>,
) -> Result<SymbolRecords, ModelError> {
    let q = qualification.id();
    let mut out = SymbolRecords::default();
    // Which definitions are stated: all, or the kept ones and every definition enclosing them.
    let mut parents: HashMap<String, Option<String>> = HashMap::new();
    for (id, class) in &definitions.class_definitions {
        parents.insert(id.to_int().to_string(), parent_key(&class.parent));
    }
    for (id, function) in definitions.function_definitions.as_map() {
        parents.insert(id.serialize_to_string(), parent_key(&function.base.parent));
    }
    let stated: BTreeSet<String> = match keep {
        None => parents.keys().cloned().collect(),
        Some(keep) => {
            let mut stated = BTreeSet::new();
            for key in keep {
                let mut current = parents.contains_key(key).then(|| key.clone());
                while let Some(key) = current {
                    if !stated.insert(key.clone()) {
                        break;
                    }
                    current = parents.get(&key).cloned().flatten();
                }
            }
            stated
        }
    };
    let mut classes: BTreeMap<u32, Id<ProviderSymbol>> = BTreeMap::new();
    let mut functions: BTreeMap<String, Id<ProviderSymbol>> = BTreeMap::new();
    for (id, class) in &definitions.class_definitions {
        if stated.contains(&id.to_int().to_string()) {
            classes.insert(
                id.to_int(),
                natives.symbol(
                    module,
                    id.to_int().to_string(),
                    class.name.clone(),
                    SymbolKind::Class,
                )?,
            );
        }
    }
    for (id, function) in definitions.function_definitions.as_map() {
        if stated.contains(&id.serialize_to_string()) {
            let kind = if function.base.defining_class.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            functions.insert(
                id.serialize_to_string(),
                natives.symbol(
                    module,
                    id.serialize_to_string(),
                    function.base.name.to_string(),
                    kind,
                )?,
            );
        }
    }
    let parent = |scope: &ScopeParent| -> Result<Option<Id<ProviderSymbol>>, ModelError> {
        Ok(match scope {
            ScopeParent::TopLevel => None,
            ScopeParent::Class { class_id } => {
                Some(*classes.get(&class_id.to_int()).ok_or_else(|| {
                    invalid(format!(
                        "Pysa nests a symbol in an undefined class {}",
                        class_id.to_int()
                    ))
                })?)
            }
            ScopeParent::Function { func_def_index } => {
                let key = format!("F:{}", func_def_index.0);
                Some(*functions.get(&key).ok_or_else(|| {
                    invalid(format!(
                        "Pysa nests a symbol in an undefined function {key}"
                    ))
                })?)
            }
        })
    };
    let class_ref =
        |natives: &mut Natives, class: &ClassRef| -> Result<Id<ProviderSymbol>, ModelError> {
            let owner = class.class.module();
            let owner = natives.module(&owner.name().to_string(), owner.path())?;
            natives.symbol(
                owner,
                class.class_id.to_int().to_string(),
                class.class.name().to_string(),
                SymbolKind::Class,
            )
        };
    let function_ref = |natives: &mut Natives,
                        resolve: &mut Resolve<'_>,
                        function: &FunctionRef,
                        kind: SymbolKind|
     -> Result<Id<ProviderSymbol>, ModelError> {
        let owner = resolve(natives, function.module_id, &function.module_name)?;
        natives.symbol(
            owner,
            function.function_id.serialize_to_string(),
            function.function_name.to_string(),
            kind,
        )
    };
    // A declaration link at an exact name span, or the reason there is none.
    let attach = |out: &mut SymbolRecords,
                  symbol: Id<ProviderSymbol>,
                  location: Option<&PysaLocation>|
     -> Option<Id<Occurrence>> {
        let (Some(linking), Some(location)) = (linking.as_ref(), location) else {
            return None;
        };
        let range = linking.locator.range(location);
        let found = linking
            .spans
            .get(range, SyntaxKind::Identifier)
            .ok()
            .and_then(|name| linking.declared.names.get(&name).copied());
        match found {
            Some(declaration) => {
                out.declarations.push(SymbolDeclaration {
                    qualification: q,
                    symbol,
                    declaration,
                });
                Some(declaration)
            }
            None => {
                out.unattached
                    .push((symbol, format!("no declaration names the span {range:?}")));
                None
            }
        }
    };
    for (id, class) in &definitions.class_definitions {
        let Some(symbol) = classes.get(&id.to_int()).copied() else {
            continue;
        };
        out.symbols.push(SymbolObservation {
            qualification: q,
            symbol,
            parent: parent(&class.parent)?,
        });
        out.classes.push(ClassTraitObservation {
            qualification: q,
            symbol,
            synthesized: class.is_synthesized,
            dataclass: class.is_dataclass,
            named_tuple: class.is_named_tuple,
            typed_dict: class.is_typed_dict,
        });
        if !class.is_synthesized {
            attach(&mut out, symbol, Some(&class.name_location));
        }
        let bases = class
            .bases
            .iter()
            .map(|base| class_ref(natives, base))
            .collect::<Result<Vec<_>, _>>()?;
        let (mro, linearization) = match &class.mro {
            PysaClassMro::Resolved(ancestors) => {
                let linearization = if complete.get(&id.to_int()).copied().unwrap_or(false) {
                    Linearization::Complete
                } else {
                    Linearization::Prefix
                };
                (
                    ancestors
                        .iter()
                        .map(|ancestor| class_ref(natives, ancestor))
                        .collect::<Result<Vec<_>, _>>()?,
                    linearization,
                )
            }
            PysaClassMro::Cyclic => (vec![], Linearization::Cyclic),
        };
        for (relation, ancestors, linearization) in [
            (AncestryRelation::Bases, bases, None),
            (AncestryRelation::Mro, mro, Some(linearization)),
        ] {
            let sequence = SymbolSequence::new(&ancestors)?;
            out.ancestry.push(ClassAncestryObservation {
                qualification: q,
                class: symbol,
                relation,
                ancestors: sequence.0.id(),
                linearization,
            });
            out.sequences.push(sequence);
        }
    }
    for (id, function) in definitions.function_definitions.as_map() {
        let Some(symbol) = functions.get(&id.serialize_to_string()).copied() else {
            continue;
        };
        let base = &function.base;
        out.symbols.push(SymbolObservation {
            qualification: q,
            symbol,
            parent: parent(&base.parent)?,
        });
        let defining_class = base
            .defining_class
            .as_ref()
            .map(|class| class_ref(natives, class))
            .transpose()?;
        // Pysa reports the root class's methods in the bundled stubs (`object.__init__`) as overriding
        // themselves; a self-override states nothing, so none is stated.
        let overrides = function
            .overridden_base_method
            .as_ref()
            .map(|base| function_ref(natives, resolve, base, SymbolKind::Method))
            .transpose()?
            .filter(|base| *base != symbol);
        if defining_class.is_none()
            && (base.is_staticmethod
                || base.is_classmethod
                || base.is_property_getter
                || base.is_property_setter
                || overrides.is_some())
        {
            // Pysa can propagate a surrounding method's flags into a nested function.
            // Keep its native evidence and declaration, but do not assert inconsistent traits.
            out.unattached
                .push((symbol, "native method traits have no defining class".into()));
        } else {
            out.functions.push(FunctionTraitObservation {
                qualification: q,
                symbol,
                overload: base.is_overload,
                staticmethod: base.is_staticmethod,
                classmethod: base.is_classmethod,
                property_getter: base.is_property_getter,
                property_setter: base.is_property_setter,
                stub: base.is_stub,
                origin: *origins.get(&id.serialize_to_string()).ok_or_else(|| invalid("native function origin absent".into()))?,
                defining_class,
                overrides,
            });
        }
        let declaration = if base.is_def_statement {
            attach(&mut out, symbol, base.name_location.as_ref())
        } else {
            None
        };
        for (variant, signature) in function.undecorated_signatures.iter().enumerate() {
            let (form, formals) = formals(&signature.parameters);
            let shapes: Vec<ParameterShape> = formals.iter().map(|f| f.shape.clone()).collect();
            let (row, members) = match Signature::new(
                qualification,
                symbol,
                variant as i64,
                form,
                &shapes,
            ) {
                Ok(value) => value,
                Err(ModelError::Invalid(detail)) if form == SignatureForm::List => {
                    out.native_unavailable.push((symbol, format!("native signature variant {variant} is unavailable for binding: {detail}")));
                    Signature::new(
                        qualification,
                        symbol,
                        variant as i64,
                        SignatureForm::NativeUnavailable,
                        &shapes,
                    )?
                }
                Err(error) => return Err(error),
            };
            for (member, formal) in members.iter().zip(&formals) {
                if let Some(display) = &formal.display {
                    out.annotations.push(ParameterAnnotationObservation {
                        qualification: q,
                        scope: qualification.scope,
                        parameter: member.id(),
                        display: display.clone(),
                    });
                }
            }
            // A signature's parameters link to the `def`'s own parameters only when the lists
            // agree slot by slot in kind; otherwise the symbol's declaration stands alone.
            if let (Some(declaration), Some(linking), SignatureForm::List) =
                (declaration, linking.as_ref(), row.form)
            {
                let written = linking
                    .declared
                    .parameters
                    .get(&declaration)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                if written.len() == shapes.len()
                    && written
                        .iter()
                        .zip(&shapes)
                        .all(|((_, _, kind), shape)| *kind == shape.kind)
                {
                    out.parameter_declarations
                        .extend(
                            members
                                .iter()
                                .zip(written)
                                .map(|(member, (_, parameter, _))| ParameterDeclaration {
                                    qualification: q,
                                    parameter: member.id(),
                                    declaration: *parameter,
                                }),
                        );
                } else if function.undecorated_signatures.len() == 1 {
                    out.unattached.push((
                        symbol,
                        format!(
                            "the signature's {} parameters differ from the def's {}",
                            shapes.len(),
                            written.len()
                        ),
                    ));
                }
            }
            out.shapes.extend(shapes);
            out.signatures.push((row, members));
        }
    }
    Ok(out)
}

/// Each `def`'s documented parameters: Pyrefly's parse of its docstring, each description located
/// in the literal's bytes. A documented name that is not one of the `def`'s parameters is not
/// stated; a description whose bytes are not located is disclosed, never guessed.
#[derive(Default)]
pub struct ParameterDocs {
    pub docs: Vec<(
        ParameterDocObservation,
        lctx_model::domain::assertion::Evidence,
    )>,
    pub unlocated: Vec<String>,
}
pub fn parameter_docs(
    ast: &ruff_python_ast::ModModule,
    text: &str,
    source: Id<lctx_model::domain::source::SourceArtifact>,
    spans: &Spans,
    qualification: Id<AssertionQualification>,
) -> Result<ParameterDocs, ModelError> {
    use crate::docstrings::{Located, docstring, locate_description};
    use lctx_model::domain::assertion::{Evidence, EvidenceSourceSpanId};
    use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
    use ruff_text_size::Ranged;
    struct Defs<'a>(Vec<&'a ruff_python_ast::StmtFunctionDef>);
    impl<'a> StatementVisitor<'a> for Defs<'a> {
        fn visit_stmt(&mut self, stmt: &'a ruff_python_ast::Stmt) {
            if let ruff_python_ast::Stmt::FunctionDef(def) = stmt {
                self.0.push(def);
            }
            walk_stmt(self, stmt);
        }
    }
    let mut defs = Defs(vec![]);
    defs.visit_body(&ast.body);
    let mut out = ParameterDocs::default();
    for def in defs.0 {
        let Some((value, range)) = docstring(&def.body) else {
            continue;
        };
        let declaration = spans.get(def.range(), SyntaxKind::StmtFunctionDef)?;
        let names: BTreeSet<String> = def
            .parameters
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        let base = usize::from(range.start());
        let literal = text
            .get(base..usize::from(range.end()))
            .ok_or_else(|| invalid("a docstring lies outside the module's text".into()))?;
        let mut documented: Vec<(String, String)> =
            pyrefly_python::docstring::parse_parameter_documentation(&value)
                .into_iter()
                .map(|(name, text)| (name.trim_start_matches('*').to_owned(), text))
                .filter(|(name, text)| names.contains(name) && !text.trim().is_empty())
                .collect();
        documented.sort();
        for (name, text) in documented {
            let (start, end, text) = match locate_description(literal, &name, &text) {
                Some(Located::Exact(start, end)) => (start, end, text),
                Some(Located::Extended(start, end, whole)) => (start, end, whole),
                None => {
                    out.unlocated.push(format!("the description of parameter `{name}` is not located in its docstring's bytes"));
                    continue;
                }
            };
            let evidence = Evidence::SourceSpan {
                source,
                start: (base + start) as i64,
                end: (base + end) as i64,
            };
            let description = EvidenceSourceSpanId::of(&evidence)?;
            out.docs.push((
                ParameterDocObservation {
                    qualification,
                    declaration,
                    name,
                    text,
                    description,
                },
                evidence,
            ));
        }
    }
    Ok(out)
}

type ParameterIndex = HashMap<Id<Occurrence>, Vec<(i64, Id<Occurrence>, ParameterKind)>>;
