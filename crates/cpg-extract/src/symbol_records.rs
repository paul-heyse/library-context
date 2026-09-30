//! Pysa's definitions of one analyzed module as the model's symbol records (cutover plan A9): the
//! symbols it defines and where they nest, their function and class traits, bases and method
//! resolution orders, undecorated signatures with each parameter's displayed annotation, and the
//! declaration links from each symbol and signature parameter to the occurrence that declares it.
//! A link is made only at an exact name span; any other outcome is a boundary, never a name match.
#![deny(clippy::wildcard_enum_match_arm)]
use std::collections::{BTreeMap, HashMap};
use lctx_model::domain::{Id, ModelError, Record, assertion::AssertionQualification, calls::*, declarations::{ParameterDeclaration, SymbolDeclaration},
    source::{Occurrence, SyntaxKind}, symbols::*, syntax::{DeclarationObservation, ParameterSyntaxObservation}};
use pyrefly::report::pysa::{PysaModuleDefinitions, class::{PysaClassMro, ClassRef}, function::{FunctionParameter, FunctionParameters, FunctionRef},
    location::PysaLocation, module::ModuleId, scope::ScopeParent};
use pyrefly_python::module_name::ModuleName;
use ruff_source_file::{LineIndex, OneIndexed, PositionEncoding, SourceLocation};
use ruff_text_size::{TextRange, TextSize};
use crate::{natives::Natives, syntax_records::Spans};

fn invalid(message: String) -> ModelError { ModelError::Invalid(message) }

/// Pysa locations (1-based line, 1-based UTF-8 byte column) as byte ranges of the module's text:
/// the exact inverse of `PysaLocation::from_text_range`.
pub struct Locator<'a> { pub line_index: &'a LineIndex, pub text: &'a str }
impl Locator<'_> {
    fn offset(&self, line: u32, column: u32) -> TextSize {
        self.line_index.offset(SourceLocation { line: OneIndexed::new(line as usize).unwrap_or(OneIndexed::MIN),
            character_offset: OneIndexed::new(column as usize).unwrap_or(OneIndexed::MIN) }, self.text, PositionEncoding::Utf8)
    }
    pub fn range(&self, location: &PysaLocation) -> TextRange {
        TextRange::new(self.offset(location.line(), location.col()), self.offset(location.end_line(), location.end_col()))
    }
}

/// What the module's syntax records declare: each declaration's name occurrence, and each `def`'s
/// parameters in order with their kinds.
#[derive(Default)]
pub struct Declared { names: HashMap<Id<Occurrence>, Id<Occurrence>>, parameters: HashMap<Id<Occurrence>, Vec<(i64, Id<Occurrence>, ParameterKind)>> }
impl Declared {
    /// `formals` pairs each parameter occurrence with the `Parameter` node a link names.
    pub fn new(declarations: &[DeclarationObservation], parameters: &[ParameterSyntaxObservation], formals: &[(Id<Occurrence>, Id<Occurrence>)]) -> Result<Self, ModelError> {
        let formals: HashMap<_, _> = formals.iter().copied().collect();
        let mut declared = Self { names: declarations.iter().map(|d| (d.name, d.declaration)).collect(), parameters: HashMap::new() };
        for p in parameters {
            let formal = *formals.get(&p.parameter).ok_or_else(|| invalid("a parameter occurrence has no formal".into()))?;
            declared.parameters.entry(p.function).or_default().push((p.ordinal, formal, p.kind));
        }
        for list in declared.parameters.values_mut() { list.sort_by_key(|(ordinal, _, _)| *ordinal); }
        Ok(declared)
    }
}

/// One undecorated signature's parameters: shapes and displayed annotations.
struct Formal { shape: ParameterShape, display: Option<String> }
fn formals(parameters: &FunctionParameters) -> (SignatureForm, Vec<Formal>) {
    match parameters {
        FunctionParameters::List(list) => (SignatureForm::List, list.iter().map(|p| {
            let (kind, name, required, annotation) = match p {
                FunctionParameter::PosOnly { name, annotation, required } => (ParameterKind::PositionalOnly, name.as_ref().map(ToString::to_string), *required, annotation),
                FunctionParameter::Pos { name, annotation, required } => (ParameterKind::PositionalOrKeyword, Some(name.to_string()), *required, annotation),
                FunctionParameter::VarArg { name, annotation } => (ParameterKind::VarPositional, name.as_ref().map(ToString::to_string), false, annotation),
                FunctionParameter::KwOnly { name, annotation, required } => (ParameterKind::KeywordOnly, Some(name.to_string()), *required, annotation),
                FunctionParameter::Kwargs { name, annotation } => (ParameterKind::VarKeyword, name.as_ref().map(ToString::to_string), false, annotation),
            };
            Formal { shape: ParameterShape { name, kind, required }, display: Some(annotation.string.clone()).filter(|d| !d.is_empty()) }
        }).collect()),
        FunctionParameters::Ellipsis => (SignatureForm::Ellipsis, vec![]),
        FunctionParameters::ParamSpec => (SignatureForm::ParamSpec, vec![]),
    }
}

/// One module's symbol records, all under the module's qualification.
#[derive(Default)]
pub struct SymbolRecords {
    pub symbols: Vec<SymbolObservation>, pub functions: Vec<FunctionTraitObservation>, pub classes: Vec<ClassTraitObservation>,
    pub ancestry: Vec<ClassAncestryObservation>, pub sequences: Vec<(SymbolSequence, Vec<SymbolSequenceMember>)>,
    pub shapes: Vec<ParameterShape>, pub signatures: Vec<(Signature, Vec<SignatureParameter>)>, pub annotations: Vec<ParameterAnnotationObservation>,
    pub declarations: Vec<SymbolDeclaration>, pub parameter_declarations: Vec<ParameterDeclaration>,
    /// Symbols (with a name span) whose declaration no exact span attaches, and why.
    pub unattached: Vec<(Id<ProviderSymbol>, String)>,
}

/// Resolves a module a reference names by Pysa's id and the name it carries.
pub type Resolve<'r> = dyn FnMut(&mut Natives, ModuleId, &ModuleName) -> Result<Id<ProviderModule>, ModelError> + 'r;

/// The records of `definitions`, the module `module` Pysa numbered `module_id`. `complete` states,
/// per class id, whether its resolved MRO is the complete C3 linearization.
#[allow(clippy::too_many_arguments, reason = "the module's definitions and the indices that place them, each distinct")]
pub fn records(definitions: &PysaModuleDefinitions, module: Id<ProviderModule>, qualification: &AssertionQualification, natives: &mut Natives,
    resolve: &mut Resolve<'_>, complete: &HashMap<u32, bool>, locator: &Locator<'_>, spans: &Spans, declared: &Declared) -> Result<SymbolRecords, ModelError> {
    let q = qualification.id();
    let mut out = SymbolRecords::default();
    // Every symbol the module defines, first, so parents can name one another.
    let mut classes: BTreeMap<u32, Id<ProviderSymbol>> = BTreeMap::new();
    let mut functions: BTreeMap<String, Id<ProviderSymbol>> = BTreeMap::new();
    for (id, class) in &definitions.class_definitions {
        classes.insert(id.to_int(), natives.symbol(module, id.to_int().to_string(), class.name.clone(), SymbolKind::Class)?);
    }
    for (id, function) in definitions.function_definitions.as_map() {
        let kind = if function.base.defining_class.is_some() { SymbolKind::Method } else { SymbolKind::Function };
        functions.insert(id.serialize_to_string(), natives.symbol(module, id.serialize_to_string(), function.base.name.to_string(), kind)?);
    }
    let parent = |scope: &ScopeParent| -> Result<Option<Id<ProviderSymbol>>, ModelError> {
        Ok(match scope {
            ScopeParent::TopLevel => None,
            ScopeParent::Class { class_id } => Some(*classes.get(&class_id.to_int()).ok_or_else(|| invalid(format!("Pysa nests a symbol in an undefined class {}", class_id.to_int())))?),
            ScopeParent::Function { func_def_index } => {
                let key = format!("F:{}", func_def_index.0);
                Some(*functions.get(&key).ok_or_else(|| invalid(format!("Pysa nests a symbol in an undefined function {key}")))?)
            }
        })
    };
    let class_ref = |natives: &mut Natives, class: &ClassRef| -> Result<Id<ProviderSymbol>, ModelError> {
        let owner = class.class.module();
        let owner = natives.module(&owner.name().to_string(), owner.path())?;
        natives.symbol(owner, class.class_id.to_int().to_string(), class.class.name().to_string(), SymbolKind::Class)
    };
    let function_ref = |natives: &mut Natives, resolve: &mut Resolve<'_>, function: &FunctionRef, kind: SymbolKind| -> Result<Id<ProviderSymbol>, ModelError> {
        let owner = resolve(natives, function.module_id, &function.module_name)?;
        natives.symbol(owner, function.function_id.serialize_to_string(), function.function_name.to_string(), kind)
    };
    // A declaration link at an exact name span, or the reason there is none.
    let attach = |out: &mut SymbolRecords, symbol: Id<ProviderSymbol>, location: Option<&PysaLocation>| -> Option<Id<Occurrence>> {
        let location = location?;
        let found = spans.get(locator.range(location), SyntaxKind::Identifier).ok().and_then(|name| declared.names.get(&name).copied());
        match found {
            Some(declaration) => { out.declarations.push(SymbolDeclaration { qualification: q, symbol, declaration }); Some(declaration) }
            None => { out.unattached.push((symbol, format!("no declaration names the span {:?}", locator.range(location)))); None }
        }
    };
    for (id, class) in &definitions.class_definitions {
        let symbol = classes[&id.to_int()];
        out.symbols.push(SymbolObservation { qualification: q, symbol, parent: parent(&class.parent)? });
        out.classes.push(ClassTraitObservation { qualification: q, symbol, synthesized: class.is_synthesized, dataclass: class.is_dataclass,
            named_tuple: class.is_named_tuple, typed_dict: class.is_typed_dict });
        if !class.is_synthesized { attach(&mut out, symbol, Some(&class.name_location)); }
        let bases = class.bases.iter().map(|base| class_ref(natives, base)).collect::<Result<Vec<_>, _>>()?;
        let (mro, linearization) = match &class.mro {
            PysaClassMro::Resolved(ancestors) => {
                let linearization = if complete.get(&id.to_int()).copied().unwrap_or(false) { Linearization::Complete } else { Linearization::Prefix };
                (ancestors.iter().map(|ancestor| class_ref(natives, ancestor)).collect::<Result<Vec<_>, _>>()?, linearization)
            }
            PysaClassMro::Cyclic => (vec![], Linearization::Cyclic),
        };
        for (relation, ancestors, linearization) in [(AncestryRelation::Bases, bases, None), (AncestryRelation::Mro, mro, Some(linearization))] {
            let sequence = SymbolSequence::new(&ancestors)?;
            out.ancestry.push(ClassAncestryObservation { qualification: q, class: symbol, relation, ancestors: sequence.0.id(), linearization });
            out.sequences.push(sequence);
        }
    }
    for (id, function) in definitions.function_definitions.as_map() {
        let base = &function.base;
        let symbol = functions[&id.serialize_to_string()];
        out.symbols.push(SymbolObservation { qualification: q, symbol, parent: parent(&base.parent)? });
        let defining_class = base.defining_class.as_ref().map(|class| class_ref(natives, class)).transpose()?;
        let overrides = function.overridden_base_method.as_ref().map(|base| function_ref(natives, resolve, base, SymbolKind::Method)).transpose()?;
        out.functions.push(FunctionTraitObservation { qualification: q, symbol, overload: base.is_overload, staticmethod: base.is_staticmethod,
            classmethod: base.is_classmethod, property_getter: base.is_property_getter, property_setter: base.is_property_setter, stub: base.is_stub,
            def_statement: base.is_def_statement, defining_class, overrides });
        let declaration = if base.is_def_statement { attach(&mut out, symbol, base.name_location.as_ref()) } else { None };
        for (variant, signature) in function.undecorated_signatures.iter().enumerate() {
            let (form, formals) = formals(&signature.parameters);
            let shapes: Vec<ParameterShape> = formals.iter().map(|f| f.shape.clone()).collect();
            let (row, members) = Signature::new(qualification, symbol, variant as i64, form, &shapes)?;
            for (member, formal) in members.iter().zip(&formals) {
                if let Some(display) = &formal.display {
                    out.annotations.push(ParameterAnnotationObservation { qualification: q, scope: qualification.scope, parameter: member.id(), display: display.clone() });
                }
            }
            // A signature's parameters link to the `def`'s own parameters only when the lists
            // agree slot by slot in kind; otherwise the symbol's declaration stands alone.
            if let Some(declaration) = declaration {
                let written = declared.parameters.get(&declaration).map(Vec::as_slice).unwrap_or_default();
                if written.len() == shapes.len() && written.iter().zip(&shapes).all(|((_, _, kind), shape)| *kind == shape.kind) {
                    out.parameter_declarations.extend(members.iter().zip(written).map(|(member, (_, parameter, _))|
                        ParameterDeclaration { qualification: q, parameter: member.id(), declaration: *parameter }));
                } else if function.undecorated_signatures.len() == 1 {
                    out.unattached.push((symbol, format!("the signature's {} parameters differ from the def's {}", shapes.len(), written.len())));
                }
            }
            out.shapes.extend(shapes);
            out.signatures.push((row, members));
        }
    }
    Ok(out)
}
