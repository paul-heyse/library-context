//! Pinned native Pyrefly types lowered directly into model terms and attributed observations.
#![deny(clippy::wildcard_enum_match_arm)]
use crate::{
    natives::Natives,
    syntax_records::{Records as Syntax, Spans},
};
use lctx_model::domain::{
    assertion::AssertionQualification,
    attribution::{Fidelity, Provider},
    class_metadata::{ClassMetadataObservation, ClassMemberObservation, MemberKind, MemberOrigin, MetadataBasis, RecordOptions, RecordTransformDefaults, RecordTransformFieldSpecifier, FieldSpecifierKind, field_specifier_shape},
    calls::{ParameterKind, ProviderModule, ProviderSymbol, SymbolKind, Signature, SignatureParameter, SignatureRole, SignatureForm, ParameterShape},
    charged::StateCharge,
    lexical::SyntaxField,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    types::*,
    value::Literal,
    *,
};
use pyrefly::report::pysa::context::ModuleContext;
use pyrefly_python::{module_name::ModuleName, qname::QName};
use pyrefly_types::{
    callable::{Callable, Param, ParamList, Params, PrefixParam, Required},
    class::Class,
    literal::Lit,
    quantified::{Quantified, QuantifiedKind, QuantifiedOrigin},
    tuple::Tuple,
    type_alias::TypeAliasData,
    type_var::{Restriction, PreInferenceVariance},
    typed_dict::TypedDict,
    types::{AnyStyle, BoundMethodType, Forallable, NeverStyle, Type},
};
use ruff_text_size::Ranged;
use std::collections::BTreeSet;

#[allow(clippy::wildcard_enum_match_arm, reason = "only native function metadata grants descriptor adjustment; other callable values remain unbound or unknown")]
fn native_field_receiver(ty: &Type) -> NativeReceiver {
    let flags = match ty {
        Type::Function(f) => Some(&f.metadata.flags),
        Type::Forall(f) => match &f.body { Forallable::Function(f) => Some(&f.metadata.flags), Forallable::Callable(_) | Forallable::TypeAlias(_) => None },
        Type::Overload(f) => Some(&f.metadata.flags),
        Type::Callable(_) => return NativeReceiver::Unbound,
        _ => None,
    };
    match flags {
        Some(flags) if flags.property_metadata.is_some() || flags.is_cached_property => NativeReceiver::Property,
        Some(flags) if flags.is_staticmethod => NativeReceiver::Unbound,
        Some(flags) if flags.is_classmethod => NativeReceiver::Class,
        Some(_) => NativeReceiver::Instance,
        None => NativeReceiver::Unknown,
    }
}

fn native_variance(value: PreInferenceVariance) -> Option<TypeVariance> {
    match value { PreInferenceVariance::Covariant => Some(TypeVariance::Covariant), PreInferenceVariance::Contravariant => Some(TypeVariance::Contravariant), PreInferenceVariance::Invariant => Some(TypeVariance::Invariant), PreInferenceVariance::Undefined => None }
}

pub type ResolveModule<'a> =
    dyn FnMut(&mut Natives, ModuleName) -> Result<Id<ProviderModule>, ModelError> + 'a;
pub struct Records {
    pub protocols: Option<crate::protocol_records::Records>,
    charge: StateCharge,
    pub class_metadata: Vec<ClassMetadataObservation>,
    pub class_members: Vec<(ClassMemberObservation, Fidelity)>,
    pub record_options: Vec<RecordOptions>,
    pub transforms: Vec<RecordTransformDefaults>,
    pub transform_specifiers: Vec<RecordTransformFieldSpecifier>,
    pub signatures: Vec<(Signature, Vec<SignatureParameter>, Vec<ParameterShape>)>,
    pub specializations: Vec<(GenericSpecializationObservation, Fidelity)>,
    pub native_signatures: Vec<(NativeSignatureObservation, Fidelity)>,
    pub port_subjects: Vec<SignatureTypeSubject>,
    pub port_types: Vec<(SignatureTypeObservation, Fidelity)>,
    pub terms: Vec<TypeTerm>,
    pub sequences: Vec<TypeSequence>,
    pub members: Vec<TypeSequenceMember>,
    pub lists: Vec<CallableParameterList>,
    pub slots: Vec<CallableParameter>,
    pub dict_lists: Vec<TypedDictFieldList>,
    pub dict_fields: Vec<TypedDictField>,
    pub variables: Vec<TypeVariable>,
    pub literals: Vec<Literal>,
    pub observations: Vec<(TypeObservation, Fidelity)>,
    pub presentations: Vec<(TypePresentation, Fidelity)>,
    pub restrictions: Vec<(TypeVariableRestriction, Fidelity)>,
    pub bodies: Vec<FunctionBodyObservation>,
    pub fields: Vec<(RecordFieldObservation, Fidelity)>,
    pub boundaries: Vec<(Option<Id<Occurrence>>, ObligationKind, String)>,
}
impl Records {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            protocols:None,
            charge: StateCharge::new(budget, "native_type_records"),
            class_metadata: vec![],
            class_members: vec![],
            record_options: vec![],
            transforms: vec![],
            transform_specifiers: vec![],
            specializations: vec![],
            signatures: vec![], native_signatures: vec![], port_subjects: vec![], port_types: vec![],
            terms: vec![],
            sequences: vec![],
            members: vec![],
            lists: vec![],
            slots: vec![],
            dict_lists: vec![],
            dict_fields: vec![],
            variables: vec![],
            literals: vec![],
            observations: vec![],
            presentations: vec![],
            restrictions: vec![],
            bodies: vec![],
            fields: vec![],
            boundaries: vec![],
        }
    }
    fn hold<T: HeapSize>(&mut self, value: &T) -> Result<(), ModelError> {
        self.charge.grow(
            size_of::<T>()
                .saturating_mul(4)
                .saturating_add(value.heap_bytes()),
        )
    }
}
#[derive(Clone, Copy)]
struct Built {
    id: Id<TypeTerm>,
    opaque: bool,
}
impl Built {
    fn fidelity(self) -> Fidelity {
        if self.opaque {
            Fidelity::DisplayOnly
        } else {
            Fidelity::NativeStructural
        }
    }
}
struct Builder<'a, 'r> {
    context: &'a ModuleContext<'a>,
    qualification: &'a AssertionQualification,
    provider: &'a Provider,
    natives: &'a mut Natives,
    resolve: &'a mut ResolveModule<'r>,
    out: Records,
    variables: BTreeSet<Id<TypeVariable>>,
    work: usize,
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn required(r: &Required) -> bool {
    match r {
        Required::Required => true,
        Required::Optional(_) => false,
    }
}
fn variable_kind(k: QuantifiedKind) -> TypeVariableKind {
    match k {
        QuantifiedKind::TypeVar => TypeVariableKind::TypeVar,
        QuantifiedKind::ParamSpec => TypeVariableKind::ParamSpec,
        QuantifiedKind::TypeVarTuple => TypeVariableKind::TypeVarTuple,
        QuantifiedKind::IntVar => TypeVariableKind::IntVar,
    }
}
impl Builder<'_, '_> {
    fn class(&mut self, class: &Class) -> Result<Id<ProviderSymbol>, ModelError> {
        let reference = pyrefly::report::pysa::class::ClassRef::from_class(class, self.context);
        let m = class.module();
        let module = self.natives.module(&m.name().to_string(), m.path())?;
        self.natives.symbol(
            module,
            reference.class_id.to_int().to_string(),
            class.name().to_string(),
            SymbolKind::Class,
        )
    }
    fn function(
        &mut self,
        kind: &pyrefly_types::function::FunctionKind,
    ) -> Result<Option<Id<ProviderSymbol>>, ModelError> {
        let Some(id) = kind.as_func_def_id() else {
            return Ok(None);
        };
        let m = id.qname.module();
        let module = self.natives.module(&m.name().to_string(), m.path())?;
        self.natives
            .symbol(
                module,
                format!("F:{}", id.def_index.0),
                id.qname.id().to_string(),
                if id.cls.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                },
            )
            .map(Some)
    }
    fn sequence(
        &mut self,
        role: TypeChildRole,
        mut children: Vec<Built>,
        unordered: bool,
    ) -> Result<(Id<TypeSequence>, bool), ModelError> {
        let opaque = children.iter().any(|c| c.opaque);
        if unordered {
            children.sort_by_key(|c| c.id);
            children.dedup_by_key(|c| c.id);
        }
        let values: Vec<_> = children.into_iter().map(|c| (role, c.id)).collect();
        let (row, members) = TypeSequence::new(&values)?;
        self.out.hold(&row)?;
        self.out.sequences.push(row.clone());
        for member in members {
            self.out.hold(&member)?;
            self.out.members.push(member);
        }
        Ok((row.id(), opaque))
    }
    fn types(
        &mut self,
        types: &[Type],
        role: TypeChildRole,
        depth: usize,
        unordered: bool,
    ) -> Result<(Id<TypeSequence>, bool), ModelError> {
        let children = types
            .iter()
            .map(|t| self.term(t, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        self.sequence(role, children, unordered)
    }
    fn slots(&mut self, slots: &[Slot]) -> Result<Id<CallableParameterList>, ModelError> {
        let (row, members) = CallableParameterList::new(slots).map_err(|error| match error {
            ModelError::Invalid(message) => {
                ModelError::Invalid(format!("{message}; native callable slots {slots:?}"))
            }
            error @ (ModelError::Resource { .. }
            | ModelError::Limit { .. }
            | ModelError::Frontier(_)
            | ModelError::Schema(_)
            | ModelError::Identity(_)
            | ModelError::Conflict(_)
            | ModelError::Codec(_)
            | ModelError::Infrastructure { .. }) => error,
        })?;
        self.out.hold(&row)?;
        self.out.lists.push(row.clone());
        for member in members {
            self.out.hold(&member)?;
            self.out.slots.push(member);
        }
        Ok(row.id())
    }
    fn dict_fields(
        &mut self,
        slots: &[TypedDictSlot],
    ) -> Result<Id<TypedDictFieldList>, ModelError> {
        let (row, members) = TypedDictFieldList::new(slots)?;
        self.out.hold(&row)?;
        let id = row.id();
        self.out.dict_lists.push(row);
        for member in members {
            self.out.hold(&member)?;
            self.out.dict_fields.push(member);
        }
        Ok(id)
    }
    fn params(&mut self, list: &ParamList, depth: usize) -> Result<(Vec<Slot>, bool), ModelError> {
        let mut slots = Vec::new();
        let mut opaque = false;
        for p in list.items() {
            let (name, kind, req) = match p {
                Param::PosOnly(n, _, r) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::PositionalOnly,
                    Some(required(r)),
                ),
                Param::Pos(n, _, r) => (
                    Some(n.to_string()),
                    ParameterKind::PositionalOrKeyword,
                    Some(required(r)),
                ),
                Param::Varargs(n, _) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::VarPositional,
                    None,
                ),
                Param::KwOnly(n, _, r) => (
                    Some(n.to_string()),
                    ParameterKind::KeywordOnly,
                    Some(required(r)),
                ),
                Param::Kwargs(n, _) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::VarKeyword,
                    None,
                ),
            };
            let term = self.term(p.as_type(), depth + 1)?;
            opaque |= term.opaque;
            slots.push(Slot {
                name: name.map(Into::into),
                kind,
                required: req,
                term: term.id,
            });
        }
        Ok((slots, opaque))
    }
    fn prefix(
        &mut self,
        list: &[PrefixParam],
        depth: usize,
    ) -> Result<(Vec<Slot>, bool), ModelError> {
        let mut slots = Vec::new();
        let mut opaque = false;
        for p in list {
            let (name, kind, req, ty) = match p {
                PrefixParam::PosOnly(n, ty, r) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::PositionalOnly,
                    required(r),
                    ty,
                ),
                PrefixParam::Pos(n, ty, r) => (
                    Some(n.to_string()),
                    ParameterKind::PositionalOrKeyword,
                    required(r),
                    ty,
                ),
            };
            let term = self.term(ty, depth + 1)?;
            opaque |= term.opaque;
            slots.push(Slot {
                name: name.map(Into::into),
                kind,
                required: Some(req),
                term: term.id,
            });
        }
        Ok((slots, opaque))
    }
    fn callable(
        &mut self,
        callable: &Callable,
        function: Option<Id<ProviderSymbol>>,
        depth: usize,
    ) -> Result<(TypeTerm, bool), ModelError> {
        let (mut form, slots, mut opaque, param_spec) = match &callable.params {
            Params::List(list) => {
                let (s, o) = self.params(list, depth)?;
                (CallableForm::List, s, o, None)
            }
            Params::Partial(list) => {
                let (s, o) = self.params(list, depth)?;
                (CallableForm::Partial, s, o, None)
            }
            Params::Ellipsis => (CallableForm::Ellipsis, vec![], false, None),
            Params::Materialization => (CallableForm::Materialization, vec![], false, None),
            Params::ParamSpec(prefix, p) => {
                let (s, o) = self.prefix(prefix, depth)?;
                let p = self.term(p, depth + 1)?;
                (CallableForm::ParamSpec, s, o || p.opaque, Some(p.id))
            }
        };
        if slots
            .iter()
            .any(|s| s.name.as_ref().is_some_and(|name| name.is_empty()))
        {
            if param_spec.is_some() {
                return Err(invalid("native parameter prefix has an empty name"));
            }
            form = CallableForm::NativeUnavailable;
            opaque = true;
            self.boundary(
                None,
                ObligationKind::OutsideProviderModel,
                "native expanded callable slots are unavailable for invocation".into(),
            )?;
        }
        let parameters = self.slots(&slots)?;
        let returns = self.term(&callable.ret, depth + 1)?;
        opaque |= returns.opaque;
        Ok((
            TypeTerm::Callable {
                function,
                form,
                parameters,
                param_spec,
                returns: returns.id,
            },
            opaque,
        ))
    }
    /// Consume owned structural native terms, retaining generic origins and overload family.
    #[allow(clippy::wildcard_enum_match_arm, reason = "non-callable native terms retain an opaque signature rather than invented slots")]
    fn signature_variants(&mut self, owner: Id<ProviderSymbol>, role: SignatureRole, ty: &Type, receiver: NativeReceiver) -> Result<(), ModelError> {
        let root = self.term(ty, 0)?;
        let mut pending = vec![(root.id, root.id, receiver)];
        let mut leaves = Vec::new();
        while let Some((id, origin, receiver)) = pending.pop() {
            if pending.len() + leaves.len() > 4096 { return Err(invalid("native signature variant work bound")); }
            let term = self.out.terms.iter().find(|t| t.id() == id).cloned().ok_or_else(|| invalid("native callable term absent"))?;
            match term {
                TypeTerm::Generic { body, .. } => pending.push((body, origin, receiver)),
                TypeTerm::BoundMethod { function, .. } => pending.push((function, origin, NativeReceiver::Instance)),
                TypeTerm::Overload { signatures, .. } | TypeTerm::Overloaded { alternatives: signatures } => {
                    let mut children: Vec<_> = self.out.members.iter().filter(|m| m.sequence == signatures).cloned().collect();
                    children.sort_by_key(|m| (m.ordinal, m.id()));
                    children.dedup_by_key(|m| m.id());
                    for child in children.into_iter().rev() { pending.push((child.child, child.child, receiver)); }
                }
                other => leaves.push((other, origin, receiver)),
            }
        }
        for (ordinal, (term, origin, receiver)) in leaves.into_iter().enumerate() {
            let (form, list, returns, implementation) = match &term {
                TypeTerm::Callable { form, parameters, returns, function, .. } => (*form, Some(*parameters), Some(*returns), *function),
                _ => (CallableForm::NativeUnavailable, None, None, None),
            };
            let mut native_slots: Vec<_> = list.into_iter().flat_map(|list| self.out.slots.iter().filter(move |s| s.list == list).cloned()).collect();
            native_slots.sort_by_key(|s| (s.ordinal, s.id()));
            native_slots.dedup_by_key(|s| s.id());
            let shapes: Vec<_> = native_slots.iter().map(|p| ParameterShape { name: p.name.clone(), kind: p.kind, required: p.required.unwrap_or(false) }).collect();
            let form = match form {
                CallableForm::List => SignatureForm::List,
                CallableForm::Ellipsis => SignatureForm::Ellipsis,
                CallableForm::ParamSpec if shapes.is_empty() => SignatureForm::ParamSpec,
                _ => SignatureForm::NativeUnavailable,
            };
            let (signature, parameters) = match Signature::new(self.qualification, role, Some(origin), owner, ordinal as i64, form, &shapes) {
                Ok(value) => value,
                Err(ModelError::Invalid(_)) => Signature::new(self.qualification, role, Some(origin), owner, ordinal as i64, SignatureForm::NativeUnavailable, &shapes)?,
                Err(error) => return Err(error),
            };
            let implementation = self.out.terms.iter().find(|t| t.id() == root.id).and_then(|t| match t {
                TypeTerm::Overload { function, .. } => Some(*function),
                _ => None,
            }).or(implementation);
            let observation = NativeSignatureObservation { qualification: self.qualification.id(), signature: signature.id(), scope: self.qualification.scope, term: origin,
                family: (origin != root.id).then_some(root.id), implementation, receiver, complete: signature.form == SignatureForm::List && !root.opaque };
            self.out.hold(&observation)?;
            self.out.native_signatures.push((observation, root.fidelity()));
            for (parameter, native) in parameters.iter().zip(&native_slots) {
                self.port(SignatureTypeSubject::Parameter { parameter: parameter.id() }, native.term, root.fidelity())?;
            }
            if let Some(term) = returns { self.port(SignatureTypeSubject::Return { signature: signature.id() }, term, root.fidelity())?; }
            self.out.hold(&signature)?;
            for p in &parameters { self.out.hold(p)?; }
            for p in &shapes { self.out.hold(p)?; }
            self.out.signatures.push((signature, parameters, shapes));
        }
        Ok(())
    }
    fn port(&mut self, subject: SignatureTypeSubject, term: Id<TypeTerm>, fidelity: Fidelity) -> Result<(), ModelError> {
        let row = SignatureTypeObservation { qualification: self.qualification.id(), subject: subject.id(), term, scope: self.qualification.scope };
        self.out.hold(&subject)?; self.out.hold(&row)?;
        self.out.port_subjects.push(subject); self.out.port_types.push((row, fidelity));
        Ok(())
    }
    fn variable(&mut self, q: &Quantified, depth: usize) -> Result<Id<TypeVariable>, ModelError> {
        let id = q.identity();
        let module = (self.resolve)(self.natives, id.module)?;
        let origin = match id.origin {
            QuantifiedOrigin::ScopedLegacy => TypeVariableOrigin::ScopedLegacy,
            QuantifiedOrigin::Pep695 => TypeVariableOrigin::Pep695,
            QuantifiedOrigin::Synthetic { is_self: false } => TypeVariableOrigin::Synthetic,
            QuantifiedOrigin::Synthetic { is_self: true } => TypeVariableOrigin::SyntheticSelf,
            QuantifiedOrigin::MapIntTuplesParameter => TypeVariableOrigin::MapIntTuples,
            QuantifiedOrigin::NormalizedMapIntTuplesParameter => {
                TypeVariableOrigin::NormalizedMapIntTuples
            }
        };
        let row = TypeVariable {
            provider: self.provider.id(),
            context: self.qualification.context,
            module,
            anchor_start: i64::from(id.anchor.range.start().to_u32()),
            anchor_end: i64::from(id.anchor.range.end().to_u32()),
            slot: i64::from(id.anchor.index),
            origin,
            kind: variable_kind(q.kind),
            name: q.name.to_string(),
            declared_variance: native_variance(q.variance()),
            inferred_variance: None,
        };
        let key = row.id();
        if !self.variables.contains(&key) {
            self.out.hold(&row)?;
            self.out.charge.grow(96)?;
            self.variables.insert(key);
            self.out.variables.push(row);
            self.restrictions(key, &q.restriction, q.default.as_ref(), depth)?;
        }
        Ok(key)
    }
    fn declaration_variable(
        &mut self,
        qname: &QName,
        kind: TypeVariableKind,
        variance: Option<TypeVariance>,
        restriction: &Restriction,
        default: Option<&Type>,
        depth: usize,
    ) -> Result<Id<TypeVariable>, ModelError> {
        let m = qname.module();
        let module = self.natives.module(&m.name().to_string(), m.path())?;
        let row = TypeVariable {
            provider: self.provider.id(),
            context: self.qualification.context,
            module,
            anchor_start: i64::from(qname.range().start().to_u32()),
            anchor_end: i64::from(qname.range().end().to_u32()),
            slot: 0,
            origin: TypeVariableOrigin::ScopedLegacy,
            kind,
            name: qname.id().to_string(),
            declared_variance: variance,
            inferred_variance: None,
        };
        let key = row.id();
        if !self.variables.contains(&key) {
            self.out.hold(&row)?;
            self.out.charge.grow(96)?;
            self.variables.insert(key);
            self.out.variables.push(row);
            self.restrictions(key, restriction, default, depth)?;
        }
        Ok(key)
    }
    fn restrictions(
        &mut self,
        variable: Id<TypeVariable>,
        restriction: &Restriction,
        default: Option<&Type>,
        depth: usize,
    ) -> Result<(), ModelError> {
        let mut values = Vec::new();
        match restriction {
            Restriction::Bound(ty) => values.push((TypeRestrictionKind::Bound, 0, ty)),
            Restriction::Constraints(types) => {
                for (i, ty) in types.iter().enumerate() {
                    values.push((TypeRestrictionKind::Constraint, i as i64, ty));
                }
            }
            Restriction::ShapeExtension(_) => self.boundary(
                None,
                ObligationKind::OutsideProviderModel,
                "experimental type-variable shape restriction".into(),
            )?,
            Restriction::Unrestricted => {}
        }
        if let Some(ty) = default {
            values.push((TypeRestrictionKind::Default, 0, ty));
        }
        for (kind, ordinal, ty) in values {
            let term = self.term(ty, depth + 1)?;
            let row = TypeVariableRestriction {
                qualification: self.qualification.id(),
                scope: self.qualification.scope,
                variable,
                kind,
                ordinal,
                term: term.id,
            };
            self.out.hold(&row)?;
            self.out.restrictions.push((row, term.fidelity()));
        }
        Ok(())
    }
    fn opaque(&self, ty: &Type, variant: &str) -> TypeTerm {
        TypeTerm::Other {
            provider: self.provider.id(),
            context: self.qualification.context,
            variant: variant.into(),
            display: ty.to_string(),
        }
    }
    fn boundary(
        &mut self,
        subject: Option<Id<Occurrence>>,
        reason: ObligationKind,
        detail: String,
    ) -> Result<(), ModelError> {
        self.out.hold(&detail)?;
        self.out.charge.grow(128)?;
        self.out.boundaries.push((subject, reason, detail));
        Ok(())
    }
    fn term(&mut self, ty: &Type, depth: usize) -> Result<Built, ModelError> {
        self.work += 1;
        let (row, mut opaque) = if depth > 32 || self.work > 100_000 {
            (
                TypeTerm::Truncated {
                    provider: self.provider.id(),
                    context: self.qualification.context,
                    reason: ObligationKind::BudgetReached,
                    display: ty.to_string(),
                },
                true,
            )
        } else {
            self.build(ty, depth)?
        };
        opaque |= matches!(row, TypeTerm::Other { .. } | TypeTerm::Truncated { .. });
        let built = Built {
            id: row.id(),
            opaque,
        };
        self.out.hold(&row)?;
        self.out.terms.push(row);
        let presentation = TypePresentation {
            qualification: self.qualification.id(),
            scope: self.qualification.scope,
            term: built.id,
            display: ty.to_string(),
            detail: None,
        };
        self.out.hold(&presentation)?;
        self.out
            .presentations
            .push((presentation, built.fidelity()));
        Ok(built)
    }
    fn build(&mut self, ty: &Type, depth: usize) -> Result<(TypeTerm, bool), ModelError> {
        Ok(match ty {
            Type::Literal(lit) => match &lit.value {
                Lit::Enum(e) => (
                    TypeTerm::EnumLiteral {
                        class: self.class(e.class.class_object())?,
                        member: e.member.to_string(),
                    },
                    false,
                ),
                value @ (Lit::Str(_) | Lit::Int(_) | Lit::Bool(_) | Lit::Bytes(_)) => {
                    let value = match value {
                        Lit::Str(s) => Literal::String {
                            value: s.to_string().into(),
                        },
                        Lit::Int(i) => Literal::Integer {
                            decimal: i.to_string(),
                        },
                        Lit::Bool(b) => Literal::Bool { value: *b },
                        Lit::Bytes(b) => Literal::Bytes {
                            value: EvidenceBytes(b.to_vec()),
                        },
                        Lit::Enum(_) => unreachable!("enum handled above"),
                    };
                    self.out.hold(&value)?;
                    let id = value.id();
                    self.out.literals.push(value);
                    (TypeTerm::Literal { value: id }, false)
                }
            },
            Type::LiteralString(_) => (TypeTerm::LiteralString, false),
            Type::Callable(c) => self.callable(c, None, depth)?,
            Type::Function(f) => {
                let function = self.function(&f.metadata.kind)?;
                self.callable(&f.signature, function, depth)?
            }
            Type::BoundMethod(b) => {
                let receiver = self.term(&b.obj, depth + 1)?;
                let function = match &b.func {
                    BoundMethodType::Function(f) => Type::Function(Box::new(f.clone())),
                    BoundMethodType::Forall(f) => BoundMethodType::Forall(f.clone()).as_type(),
                    BoundMethodType::Overload(o) => Type::Overload(o.clone()),
                };
                let function = self.term(&function, depth + 1)?;
                (
                    TypeTerm::BoundMethod {
                        receiver: receiver.id,
                        function: function.id,
                    },
                    receiver.opaque || function.opaque,
                )
            }
            Type::Overload(o) => {
                if let Some(function) = self.function(&o.metadata.kind)? {
                    let signatures = o
                        .signatures
                        .iter()
                        .map(|s| self.term(&s.as_type(), depth + 1))
                        .collect::<Result<Vec<_>, _>>()?;
                    let (signatures, opaque) =
                        self.sequence(TypeChildRole::Signature, signatures, false)?;
                    (
                        TypeTerm::Overload {
                            function,
                            signatures,
                        },
                        opaque,
                    )
                } else {
                    (self.opaque(ty, "unnamed_native_overload"), true)
                }
            }
            Type::Union(u) => {
                let (members, o) = self.types(&u.members, TypeChildRole::Member, depth, true)?;
                (TypeTerm::Union { members }, o)
            }
            Type::Intersect(i) => {
                let (members, o) = self.types(&i.0, TypeChildRole::Member, depth, true)?;
                (TypeTerm::Intersection { members }, o)
            }
            Type::ClassDef(c) => (
                TypeTerm::ClassObject {
                    class: self.class(c)?,
                },
                false,
            ),
            Type::ClassType(c) | Type::SelfType(c) => {
                let class = self.class(c.class_object())?;
                let (arguments, o) =
                    self.types(c.targs().as_slice(), TypeChildRole::Argument, depth, false)?;
                (
                    if matches!(ty, Type::SelfType(_)) {
                        TypeTerm::SelfType { class, arguments }
                    } else {
                        TypeTerm::ClassInstance { class, arguments }
                    },
                    o,
                )
            }
            Type::TypedDict(d) | Type::PartialTypedDict(d) => {
                let partial = matches!(ty, Type::PartialTypedDict(_));
                match d {
                    TypedDict::TypedDict(c) => {
                        let class = self.class(c.class_object())?;
                        let (arguments, o) = self.types(
                            c.targs().as_slice(),
                            TypeChildRole::Argument,
                            depth,
                            false,
                        )?;
                        (
                            TypeTerm::TypedDict {
                                class,
                                arguments,
                                partial,
                            },
                            o,
                        )
                    }
                    TypedDict::Anonymous(c) => {
                        let mut slots = vec![];
                        let mut opaque = false;
                        for (name, field) in &c.fields {
                            let term = self.term(&field.ty, depth + 1)?;
                            opaque |= term.opaque;
                            slots.push((name.to_string().into(), field.required, term.id));
                        }
                        (
                            TypeTerm::AnonymousTypedDict {
                                fields: self.dict_fields(&slots)?,
                                partial,
                            },
                            opaque,
                        )
                    }
                }
            }
            Type::Tuple(tuple) => {
                let mut values = vec![];
                match tuple {
                    Tuple::Concrete(types) => {
                        for t in types {
                            values.push((TypeChildRole::Element, self.term(t, depth + 1)?));
                        }
                    }
                    Tuple::Unbounded(t) => {
                        values.push((TypeChildRole::Variadic, self.term(t, depth + 1)?))
                    }
                    Tuple::Unpacked(parts) => {
                        for t in parts.prefix() {
                            values.push((TypeChildRole::Element, self.term(t, depth + 1)?));
                        }
                        values.push((
                            TypeChildRole::Variadic,
                            self.term(parts.middle(), depth + 1)?,
                        ));
                        for t in parts.suffix() {
                            values.push((TypeChildRole::Element, self.term(t, depth + 1)?));
                        }
                    }
                };
                let opaque = values.iter().any(|(_, t)| t.opaque);
                let values: Vec<_> = values.into_iter().map(|(r, t)| (r, t.id)).collect();
                let (row, members) = TypeSequence::new(&values)?;
                self.out.hold(&row)?;
                let elements = row.id();
                self.out.sequences.push(row);
                for m in members {
                    self.out.hold(&m)?;
                    self.out.members.push(m);
                }
                (TypeTerm::Tuple { elements }, opaque)
            }
            Type::Module(m) => {
                let name = m
                    .parts()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(".");
                (
                    TypeTerm::Module {
                        module: (self.resolve)(self.natives, ModuleName::from_str(&name))?,
                    },
                    false,
                )
            }
            Type::Forall(f) => {
                let mut parameters = vec![];
                for q in f.tparams.as_vec() {
                    parameters.push(self.term(&Type::Quantified(Box::new(q.clone())), depth + 1)?);
                }
                let (parameters, o) =
                    self.sequence(TypeChildRole::TypeParameter, parameters, false)?;
                let body = match &f.body {
                    Forallable::TypeAlias(a) => Type::TypeAlias(Box::new(a.clone())),
                    Forallable::Function(f) => Type::Function(Box::new(f.clone())),
                    Forallable::Callable(c) => Type::Callable(Box::new(c.clone())),
                };
                let body = self.term(&body, depth + 1)?;
                (
                    TypeTerm::Generic {
                        parameters,
                        body: body.id,
                    },
                    o || body.opaque,
                )
            }
            Type::Quantified(q) => {
                let variable = self.variable(q, depth)?;
                (
                    match q.kind {
                        QuantifiedKind::TypeVar | QuantifiedKind::IntVar => {
                            TypeTerm::TypeVar { variable }
                        }
                        QuantifiedKind::ParamSpec => TypeTerm::ParamSpec { variable },
                        QuantifiedKind::TypeVarTuple => TypeTerm::TypeVarTuple { variable },
                    },
                    false,
                )
            }
            Type::QuantifiedValue(q)
            | Type::ElementOfTypeVarTuple(q)
            | Type::Args(q)
            | Type::Kwargs(q)
            | Type::ArgsValue(q)
            | Type::KwargsValue(q) => {
                let variable = self.variable(q, depth)?;
                #[allow(
                    clippy::wildcard_enum_match_arm,
                    reason = "Outer exhaustive match restricts this branch to variable forms"
                )]
                let form = match ty {
                    Type::QuantifiedValue(_) => VariableFormKind::Value,
                    Type::ElementOfTypeVarTuple(_) => VariableFormKind::Element,
                    Type::Args(_) => VariableFormKind::Args,
                    Type::Kwargs(_) => VariableFormKind::Kwargs,
                    Type::ArgsValue(_) => VariableFormKind::ArgsValue,
                    Type::KwargsValue(_) => VariableFormKind::KwargsValue,
                    _ => unreachable!("variable forms"),
                };
                (TypeTerm::VariableForm { variable, form }, false)
            }
            Type::TypeVar(v) => {
                let variable = self.declaration_variable(
                    v.qname(),
                    variable_kind(v.kind()),
                    native_variance(v.variance()),
                    v.restriction(),
                    v.default(),
                    depth,
                )?;
                (
                    TypeTerm::VariableForm {
                        variable,
                        form: VariableFormKind::Value,
                    },
                    false,
                )
            }
            Type::ParamSpec(v) => {
                let variable = self.declaration_variable(
                    v.qname(),
                    TypeVariableKind::ParamSpec,
                    None,
                    &Restriction::Unrestricted,
                    v.default(),
                    depth,
                )?;
                (
                    TypeTerm::VariableForm {
                        variable,
                        form: VariableFormKind::Value,
                    },
                    false,
                )
            }
            Type::TypeVarTuple(v) => {
                let variable = self.declaration_variable(
                    v.qname(),
                    TypeVariableKind::TypeVarTuple,
                    None,
                    &Restriction::Unrestricted,
                    v.default(),
                    depth,
                )?;
                (
                    TypeTerm::VariableForm {
                        variable,
                        form: VariableFormKind::Value,
                    },
                    false,
                )
            }
            Type::TypeGuard(t)
            | Type::TypeIs(t)
            | Type::Annotated(t, _)
            | Type::Unpack(t)
            | Type::Type(t)
            | Type::TypeForm(t) => {
                let t = self.term(t, depth + 1)?;
                (
                    #[allow(
                        clippy::wildcard_enum_match_arm,
                        reason = "Outer exhaustive match restricts this branch to unary forms"
                    )]
                    match ty {
                        Type::TypeGuard(_) => TypeTerm::TypeGuard {
                            form: GuardForm::TypeGuard,
                            target: t.id,
                        },
                        Type::TypeIs(_) => TypeTerm::TypeGuard {
                            form: GuardForm::TypeIs,
                            target: t.id,
                        },
                        Type::Annotated(_, _) => TypeTerm::Annotated { target: t.id },
                        Type::Unpack(_) => TypeTerm::Unpack { target: t.id },
                        Type::Type(_) => TypeTerm::TypeOf { target: t.id },
                        Type::TypeForm(_) => TypeTerm::TypeForm { target: t.id },
                        _ => unreachable!("unary type forms"),
                    },
                    t.opaque,
                )
            }
            Type::Concatenate(prefix, p) => {
                let (slots, o) = self.prefix(prefix, depth)?;
                let p = self.term(p, depth + 1)?;
                (
                    TypeTerm::ParamList {
                        parameters: self.slots(&slots)?,
                        param_spec: Some(p.id),
                    },
                    o || p.opaque,
                )
            }
            Type::ParamSpecValue(list) => {
                let (slots, o) = self.params(list, depth)?;
                (
                    TypeTerm::ParamList {
                        parameters: self.slots(&slots)?,
                        param_spec: None,
                    },
                    o,
                )
            }
            Type::SpecialForm(form) => {
                use pyrefly_types::special_form::SpecialForm as F;
                let form = match form {
                    F::Annotated => TypingForm::Annotated,
                    F::Callable => TypingForm::Callable,
                    F::ClassVar => TypingForm::ClassVar,
                    F::Concatenate => TypingForm::Concatenate,
                    F::Final => TypingForm::Final,
                    F::Generic => TypingForm::Generic,
                    F::Literal => TypingForm::Literal,
                    F::LiteralString => TypingForm::LiteralString,
                    F::Never => TypingForm::Never,
                    F::NoReturn => TypingForm::NoReturn,
                    F::NotRequired => TypingForm::NotRequired,
                    F::Optional => TypingForm::Optional,
                    F::Protocol => TypingForm::Protocol,
                    F::ReadOnly => TypingForm::ReadOnly,
                    F::Required => TypingForm::Required,
                    F::SelfType => TypingForm::SelfType,
                    F::Tuple => TypingForm::Tuple,
                    F::Type => TypingForm::Type,
                    F::TypeAlias => TypingForm::TypeAlias,
                    F::TypeForm => TypingForm::TypeForm,
                    F::TypeGuard => TypingForm::TypeGuard,
                    F::TypeIs => TypingForm::TypeIs,
                    F::TypedDict => TypingForm::TypedDict,
                    F::Union => TypingForm::Union,
                    F::Unpack => TypingForm::Unpack,
                };
                (TypeTerm::SpecialForm { form }, false)
            }
            Type::Ellipsis => (
                TypeTerm::SpecialForm {
                    form: TypingForm::Ellipsis,
                },
                false,
            ),
            Type::Any(style) => (
                TypeTerm::Any {
                    flavor: match style {
                        AnyStyle::Explicit => AnyFlavor::Explicit,
                        AnyStyle::Implicit => AnyFlavor::Implicit,
                        AnyStyle::Error => AnyFlavor::Error,
                    },
                },
                false,
            ),
            Type::Never(style) => (
                TypeTerm::Never {
                    flavor: match style {
                        NeverStyle::NoReturn => NeverFlavor::NoReturn,
                        NeverStyle::Never => NeverFlavor::Never,
                    },
                },
                false,
            ),
            Type::None => (TypeTerm::None, false),
            Type::TypeAlias(alias) | Type::UntypedAlias(alias) => {
                let untyped = matches!(ty, Type::UntypedAlias(_));
                match alias.as_ref() {
                    TypeAliasData::Value(v) => {
                        let t = self.term(&v.as_type(), depth + 1)?;
                        (
                            TypeTerm::TypeAlias {
                                name: v.name.to_string(),
                                untyped,
                                target: t.id,
                            },
                            t.opaque,
                        )
                    }
                    TypeAliasData::Ref(r) => {
                        let module = (self.resolve)(self.natives, r.module_name)?;
                        let (arguments, o) = self.types(
                            r.args.as_ref().map_or(&[], |a| a.as_slice()),
                            TypeChildRole::Argument,
                            depth,
                            false,
                        )?;
                        (
                            TypeTerm::TypeAliasReference {
                                module,
                                name: r.name.to_string(),
                                untyped,
                                arguments,
                            },
                            o,
                        )
                    }
                }
            }
            Type::Overloaded(alternatives) => {
                let (alternatives, opaque) = self.types(alternatives.as_slice(), TypeChildRole::Member, depth, false)?;
                (TypeTerm::Overloaded { alternatives }, opaque)
            },
            Type::NamedInts(_) => (self.opaque(ty, "named_ints"), true),
            Type::TypeLevelDslCall(_) => (self.opaque(ty, "type_level_dsl_call"), true),
            Type::ShapedArray(_) => (self.opaque(ty, "shaped_array"), true),
            Type::IntTuple(_) => (self.opaque(ty, "int_tuple"), true),
            Type::NNModule(_) => (self.opaque(ty, "nn_module"), true),
            Type::DataFrame(_) => (self.opaque(ty, "data_frame"), true),
            Type::Series(_) => (self.opaque(ty, "series"), true),
            Type::Int(_) => (self.opaque(ty, "int"), true),
            Type::Var(_) => (self.opaque(ty, "var"), true),
            Type::Sentinel(_) => (self.opaque(ty, "sentinel"), true),
            Type::SuperInstance(_) => (self.opaque(ty, "super_instance"), true),
            Type::KwCall(_) => (self.opaque(ty, "kw_call"), true),
            Type::Materialization => (self.opaque(ty, "materialization"), true),
        })
    }
    fn observe(
        &mut self,
        subject: Id<Occurrence>,
        role: TypeRole,
        declared: bool,
        ty: &Type,
    ) -> Result<(), ModelError> {
        let term = self.term(ty, 0)?;
        let row = TypeObservation {
            qualification: self.qualification.id(),
            subject,
            role,
            declared,
            term: term.id,
        };
        self.out.hold(&row)?;
        self.out.observations.push((row, term.fidelity()));
        if term.opaque {
            self.boundary(
                Some(subject),
                ObligationKind::OutsideProviderModel,
                "type closure includes display-only or bounded structure".into(),
            )?;
        }
        Ok(())
    }
}

/// Types from the same retained session and occurrence inventory as syntax and calls.
#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
#[allow(
    clippy::type_complexity,
    reason = "Native provider callbacks borrow the current module session"
)]
pub fn records<'a>(
    protocol: (&pyrefly::state::state::Transaction<'_>, &pyrefly_build::handle::Handle, &ruff_python_ast_latest::ModModule),
    context: &'a ModuleContext<'a>,
    qualification: &'a AssertionQualification,
    provider: &'a Provider,
    natives: &'a mut Natives,
    resolve: &'a mut ResolveModule<'_>,
    syntax: &Syntax,
    spans: &Spans,
    solutions: Option<&pyrefly::alt::answers::Solutions>,
    budget: &ResourceBudget,
    module_context: &mut dyn FnMut(
        &Class,
    )
        -> Option<pyrefly::report::pysa::context::ModuleAnswersContext>,
    field_occurrence: &mut dyn FnMut(
        &Class,
        ruff_text_size::TextRange,
    ) -> Result<Option<Id<Occurrence>>, ModelError>,
) -> Result<Records, ModelError> {
    use pyrefly::{
        binding::binding::{Key, KeyAnnotation, KeyClassMetadata, KeyAbstractClassCheck, KeyClassSynthesizedFields},
        report::pysa::{
            class::{
                get_all_classes, get_class_field_declaration,
                get_class_field_from_current_class_only, get_class_mro,
            },
            function::get_all_decorated_functions,
        },
    };
    use pyrefly_types::function::BodyKind;
    let ctx = &context.answers_context;
    let mut b = Builder {
        context,
        qualification,
        provider,
        natives,
        resolve,
        out: Records::new(budget),
        variables: BTreeSet::new(),
        work: 0,
    };
    for node in pyrefly::report::pysa::function::get_all_functions(ctx).filter(|node| node.should_export(ctx)) {
        use pyrefly::report::pysa::function::FunctionNode;
        let reference = node.as_function_ref(ctx);
        let module = b.natives.module(&ctx.module_info.name().to_string(), ctx.module_info.path())?;
        let owner = b.natives.symbol(module, reference.function_id.serialize_to_string(), reference.function_name.to_string(), if matches!(&node, FunctionNode::ClassField { .. }) || matches!(&node, FunctionNode::DecoratedFunction(f) if f.undecorated.defining_cls.is_some()) { SymbolKind::Method } else { SymbolKind::Function })?;
        let (ty, role, receiver) = match &node {
            FunctionNode::DecoratedFunction(f) => {
                let key = ctx.bindings().key_to_idx(&Key::Definition(f.undecorated.identifier));
                let ty = ctx.answers.get_type_at(key);
                let receiver = if f.undecorated.defining_cls.is_some() {
                    ty.as_ref().map(native_field_receiver).unwrap_or(NativeReceiver::Unknown)
                } else { NativeReceiver::Unbound };
                (ty, SignatureRole::EffectiveTyped, receiver)
            }
            FunctionNode::ClassField { field, .. } => {
                let ty = field.ty();
                let receiver = if field.is_property() { NativeReceiver::Property }
                    else if field.is_simple_instance_attribute() { NativeReceiver::Unbound }
                    else { native_field_receiver(&ty) };
                (Some(ty), SignatureRole::Synthesized, receiver)
            },
        };
        if let Some(ty) = ty { b.signature_variants(owner, role, &ty, receiver)?; }
        else { b.boundary(None, ObligationKind::MissingEvidence, "native effective callable type unavailable".into())?; }
    }
    for f in get_all_decorated_functions(ctx) {
        let name = spans
            .get(f.undecorated.identifier.range(), SyntaxKind::Identifier)
            .ok();
        let declaration = name.and_then(|name| {
            syntax
                .declarations
                .iter()
                .find(|d| d.name == name)
                .map(|d| d.declaration)
        });
        let Some(declaration) = declaration else {
            b.boundary(
                None,
                ObligationKind::AttachmentUnmatched,
                "typed def has no exact declaration name".into(),
            )?;
            continue;
        };
        let flags = &f.undecorated.metadata.flags;
        let body = match flags.body_kind {
            BodyKind::RaiseNotImplementedError => FunctionBodyKind::RaiseNotImplementedError,
            BodyKind::ReturnNotImplemented => FunctionBodyKind::ReturnNotImplemented,
            BodyKind::Ellipsis => FunctionBodyKind::Ellipsis,
            BodyKind::Trivial => FunctionBodyKind::Trivial,
            BodyKind::Other => FunctionBodyKind::Other,
        };
        let row = FunctionBodyObservation {
            qualification: qualification.id(),
            declaration,
            body,
            abstract_method: flags.is_abstract_method,
            in_protocol_class: flags.is_in_protocol_class,
            in_type_checking_block: flags.is_in_type_checking_block,
            overload: flags.is_overload,
        };
        b.out.hold(&row)?;
        b.out.bodies.push(row);
        for (ordinal, p) in f.undecorated.params.iter().enumerate() {
            let parameter = syntax
                .parameters
                .iter()
                .find(|p| p.function == declaration && p.ordinal == ordinal as i64);
            if let Some(parameter) = parameter {
                let subject = syntax
                    .formals
                    .iter()
                    .find(|(p, _)| *p == parameter.parameter)
                    .map(|(_, formal)| *formal)
                    .ok_or_else(|| invalid("typed parameter has no formal occurrence"))?;
                b.observe(
                    subject,
                    TypeRole::Parameter,
                    parameter.annotation.is_some(),
                    p.as_type(),
                )?;
            } else {
                b.boundary(
                    Some(declaration),
                    ObligationKind::MissingEvidence,
                    format!("native parameter {ordinal} has no syntax parameter"),
                )?;
            }
        }
        let annotation = KeyAnnotation::ReturnAnnotation(f.undecorated.identifier);
        let annotated = ctx.bindings().keys::<KeyAnnotation>()
            .find(|idx| ctx.bindings().idx_to_key(*idx) == &annotation)
            .and_then(|idx| ctx.answers.get_annotation_type_at(idx));
        if let Some(ty) = annotated {
            b.observe(declaration, TypeRole::Return, true, &ty)?;
        } else {
            let idx = ctx
                .bindings()
                .key_to_idx(&Key::ReturnType(f.undecorated.identifier));
            if let Some(ty) = ctx.answers.get_type_at(idx) {
                b.observe(declaration, TypeRole::Return, false, &ty)?;
            } else {
                b.boundary(
                    Some(declaration),
                    ObligationKind::MissingEvidence,
                    "native return type unavailable".into(),
                )?;
            }
        }
    }
    for (call, arguments) in &syntax.calls {
        if call.in_annotation {
            continue;
        }
        b.trace(call.site, TypeRole::CallResult, spans.range_of(call.site))?;
        if let Some(Type::BoundMethod(method)) = spans.range_of(call.callee).and_then(|range| ctx.answers.get_type_trace(range)) {
            let receiver = match &method.obj { Type::ClassType(c) | Type::SelfType(c) => Some(c), _ => None };
            let kind = match &method.func { BoundMethodType::Function(f) => &f.metadata.kind, BoundMethodType::Forall(f) => &f.body.metadata.kind, BoundMethodType::Overload(f) => &f.metadata.kind };
            let owner = b.function(kind)?;
            if let (Some(receiver), Some(owner)) = (receiver, owner) {
                let declarations: Vec<_> = b.out.native_signatures.iter().filter_map(|(native, _)| {
                    b.out.signatures.iter().find(|(signature, _, _)| signature.id() == native.signature && signature.symbol == owner).map(|_| native.id())
                }).collect();
                if !declarations.is_empty() {
                    let receiver_term = b.term(&method.obj, 0)?;
                    for (variable, argument) in receiver.tparams().iter().zip(receiver.targs().as_slice().iter()) {
                        let variable = b.variable(variable, 0)?;
                        let argument = b.term(argument, 0)?;
                        let fidelity = if receiver_term.opaque || argument.opaque { Fidelity::DisplayOnly } else { Fidelity::NativeStructural };
                        for declaration in &declarations {
                            let row = GenericSpecializationObservation { qualification: b.qualification.id(), scope: b.qualification.scope, site: call.callee, declaration: *declaration, receiver: receiver_term.id, variable, argument: argument.id };
                            b.out.hold(&row)?; b.out.specializations.push((row, fidelity));
                        }
                    }
                } else if !receiver.tparams().is_empty() {
                    b.boundary(Some(call.callee), ObligationKind::MissingEvidence, "native generic receiver lacks emitted signature origin".into())?;
                }
            }
        }
        // Pyrefly overload traces are keyed by the native Arguments range. Attach
        // only through its unique canonical child, retaining the call as subject.
        let mut argument_nodes = spans.nodes().filter_map(|(node, kind)| {
            (kind == SyntaxKind::Arguments && spans.parent(node).is_some_and(|(parent, _)| parent == call.site)).then_some(node)
        });
        let argument_range = argument_nodes.next().filter(|_| argument_nodes.next().is_none()).and_then(|node| spans.range_of(node));
        if let Some(range) = argument_range {
        if let Some(ty) = ctx.answers.get_chosen_overload_trace(range) { b.observe(call.site, TypeRole::ChosenOverload, false, &ty)?; }
        if let Some((alternatives, _closest_index)) = ctx.answers.get_all_overload_trace(range) {
            // This index is merely closest in unresolved cases. Never use it as choice evidence.
            for ty in alternatives { b.observe(call.site, TypeRole::OverloadCandidates, false, &Type::Callable(Box::new(ty)))?; }
        }
        }
        for argument in arguments {
            b.trace(
                argument.value,
                TypeRole::Argument,
                spans.range_of(argument.value),
            )?;
        }
    }
    let mut nodes: Vec<_> = spans.nodes().collect();
    nodes.sort_by_key(|(id, _)| *id);
    for (node, kind) in nodes {
        if let Some((parent, SyntaxField::Value)) = spans.parent(node) {
            let parent_kind = spans.kind(parent);
            let role = match parent_kind {
                Some(SyntaxKind::ExprAttribute) => Some(TypeRole::AttributeBase),
                Some(SyntaxKind::StmtAssign | SyntaxKind::StmtAnnAssign) => Some(TypeRole::AssignmentValue),
                Some(SyntaxKind::StmtReturn) => Some(TypeRole::ReturnExpression),
                _ => None,
            };
            if let Some(role) = role {
                let range = spans.range_of(node);
                b.trace(node, role, range)?;
                if let Some(ty) = range.and_then(|r| ctx.answers.get_expected_type_trace(r)) {
                    b.observe(node, TypeRole::Expected, false, &ty)?;
                }
            }
        }
        if let Some((parent, SyntaxField::Exc)) = spans.parent(node) {
            b.trace(parent, TypeRole::Raised, spans.range_of(node))?;
        }
        if matches!(
            kind,
            SyntaxKind::ExprName | SyntaxKind::ExprAttribute | SyntaxKind::ExprSubscript
        ) {
            let mut parent = node;
            let mut test = false;
            for _ in 0..256 {
                let Some((owner, field)) = spans.parent(parent) else {
                    break;
                };
                if field == SyntaxField::Test {
                    test = true;
                    break;
                }
                parent = owner;
            }
            if test {
                b.trace(node, TypeRole::TestOperand, spans.range_of(node))?;
            }
        }
    }
    if let Some(solutions) = solutions {
        for class in get_all_classes(ctx) {
            let metadata = solutions.get(&KeyClassMetadata(class.index()));
            let native_class = b.class(&class)?;
            let metaclass = b.class(metadata.metaclass(&ctx.stdlib).class_object())?;
            let custom_metaclass = metadata.custom_metaclass().map(|c| b.class(c.class_object())).transpose()?;
            let mut abstract_members: Vec<_> = solutions.get(&KeyAbstractClassCheck(class.index()))
                .unimplemented_abstract_methods().iter().map(ToString::to_string).collect();
            abstract_members.sort();
            let mut protocol_members: Vec<_> = metadata.protocol_metadata().map(|p| p.members.iter().map(ToString::to_string).collect()).unwrap_or_default();
            protocol_members.sort();
            let slots = metadata.slots_info().map(|info| { let mut names: Vec<_> = info.names.iter().map(ToString::to_string).collect(); names.sort(); names });
            let record = if metadata.is_typed_dict() { Some(RecordKind::TypedDict) }
                else if metadata.named_tuple_metadata().is_some() { Some(RecordKind::NamedTuple) }
                else if metadata.is_pydantic_model() { Some(RecordKind::Pydantic) }
                else if metadata.is_attrs_class() { Some(RecordKind::Attrs) }
                else if metadata.dataclass_metadata().is_some() { Some(RecordKind::Dataclass) }
                else { None };
            let record_options = if let Some(dm) = metadata.dataclass_metadata() {
                let k = &dm.kws;
                let options = RecordOptions { init: k.init, eq: k.eq, order: k.order, frozen: k.frozen,
                    match_args: k.match_args, kw_only: k.kw_only, unsafe_hash: k.unsafe_hash, slots: k.slots,
                    extra: k.extra, strict: k.strict, auto_attribs: k.auto_attribs, attrs_setattr_frozen: k.attrs_setattr_frozen };
                b.out.hold(&options)?;
                let id = options.id(); b.out.record_options.push(options); Some(id)
            } else { None };
            let transform = if let Some(t) = metadata.dataclass_transform_metadata() {
                use pyrefly_types::{types::CalleeKind, class::ClassKind};
                b.out.charge.grow(t.field_specifiers.len().saturating_mul(128))?;
                let classifications: Vec<_> = t.field_specifiers.iter().map(|specifier| match specifier {
                    CalleeKind::Callable => (FieldSpecifierKind::Callable, None),
                    CalleeKind::Function(_) => (FieldSpecifierKind::Function, None),
                    CalleeKind::Class(kind) => match kind {
                        ClassKind::StaticMethod(n) => (FieldSpecifierKind::StaticMethod, Some(n.to_string())),
                        ClassKind::ClassMethod(n) => (FieldSpecifierKind::ClassMethod, Some(n.to_string())),
                        ClassKind::Property(n) => (FieldSpecifierKind::Property, Some(n.to_string())),
                        ClassKind::CachedProperty(n) => (FieldSpecifierKind::CachedProperty, Some(n.to_string())),
                        ClassKind::Class => (FieldSpecifierKind::Class, None),
                        ClassKind::EnumMember => (FieldSpecifierKind::EnumMember, None),
                        ClassKind::EnumNonmember => (FieldSpecifierKind::EnumNonmember, None),
                        ClassKind::DataclassField => (FieldSpecifierKind::DataclassField, None),
                    },
                }).collect();

                let defaults = RecordTransformDefaults { eq: t.eq_default, order: t.order_default,
                    kw_only: t.kw_only_default, frozen: t.frozen_default, field_specifier_count: t.field_specifiers.len() as i64, field_specifier_shape: field_specifier_shape(&classifications),
                    field_specifier_identity_reason: (!t.field_specifiers.is_empty()).then_some(ObligationKind::NativeUnavailable) };
                if !t.field_specifiers.is_empty() {
                    b.boundary(None, ObligationKind::NativeUnavailable, "dataclass transform field-specifier payload retains callee classification, not complete structural type identity".into())?;
                }
                b.out.hold(&defaults)?;
                let id = defaults.id();
                for (ordinal, (kind, name)) in classifications.into_iter().enumerate() {
                    let specifier = RecordTransformFieldSpecifier { transform: id, ordinal: ordinal as i64, kind, name };
                    b.out.hold(&specifier)?; b.out.transform_specifiers.push(specifier);
                }
                b.out.transforms.push(defaults); Some(id)
            } else { None };
            let row = ClassMetadataObservation { qualification: b.qualification.id(), class: native_class,
                basis: MetadataBasis::NativeEffective, metaclass, custom_metaclass,
                final_declaration: metadata.is_final(), protocol: metadata.is_protocol(),
                runtime_checkable: metadata.is_runtime_checkable_protocol(), new_type: metadata.is_new_type(),
                enumeration: metadata.is_enum(), explicitly_abstract: metadata.is_explicitly_abstract(),
                abstract_members, abstract_absence_known: false, protocol_members,
                explicit_slots: metadata.has_explicit_slots(), slots, record, record_options, transform,
                deprecated: metadata.deprecation().is_some(),
                deprecation_message: metadata.deprecation().and_then(|d| d.message.clone()) };
            row.validate()?; b.out.hold(&row)?; b.out.class_metadata.push(row);
            let mut owners = vec![(class.clone(), ctx.clone())];
            for ancestor in get_class_mro(&class, ctx).ancestors_no_object() {
                let declaring = ancestor.class_object();
                let owner = if declaring.module() == &ctx.module_info { Some(ctx.clone()) } else { module_context(declaring) };
                if let Some(owner) = owner { owners.push((declaring.clone(), owner)); }
            }
            let synthesized = solutions.get(&KeyClassSynthesizedFields(class.index()));
            let mut seen = BTreeSet::new();
            for (defining, owner) in &owners {
                let fields = &owner.bindings().metadata().get_class(defining.index()).fields;
                let mut names: Vec<_> = fields.names().cloned().collect();
                if defining == &class { names.extend(synthesized.fields().map(|(n, _)| n.clone())); }
                names.sort(); names.dedup();
                for name in names {
                    if !seen.insert(name.clone()) { continue; }
                    let Some(field) = get_class_field_from_current_class_only(defining, &name, owner) else { continue; };
                    let ty = field.ty();
                    let term = b.term(&ty, 0)?;
                    let enum_value = if let Type::Literal(value) = &ty {
                        if let Lit::Enum(value) = &value.value { Some(b.term(&value.ty, 0)?) } else { None }
                    } else { None };
                    let declaration = fields.field_decl_range(&name).map(|r| field_occurrence(defining, r)).transpose()?.flatten();
                    let origin = if defining != &class { MemberOrigin::Inherited }
                        else if !fields.contains(&name) || get_class_field_declaration(defining, &name, owner).is_some_and(|d|
                            matches!(d.definition, pyrefly::binding::binding::ClassFieldDefinition::DeclaredWithoutAnnotation)) { MemberOrigin::Synthesized }
                        else { MemberOrigin::Source };
                    let row = ClassMemberObservation { qualification: qualification.id(), class: native_class,
                        defining_class: b.class(defining)?, name: name.to_string(), basis: MetadataBasis::NativeEffective, origin,
                        kind: if field.is_property() { MemberKind::Property } else if field.is_simple_instance_attribute() { MemberKind::InstanceAttribute } else { MemberKind::Other },
                        term: term.id, enum_value: enum_value.map(|t| t.id), declaration, abstract_declaration: field.is_abstract(), final_declaration: field.is_final() };
                    b.out.hold(&row)?; b.out.class_members.push((row, if enum_value.is_some_and(|t| t.opaque) { Fidelity::DisplayOnly } else { term.fidelity() }));
                }
            }


            let (record, names): (RecordKind, Vec<(String, Option<bool>)>) =
                if let Some(td) = metadata.typed_dict_metadata() {
                    (
                        RecordKind::TypedDict,
                        td.fields
                            .iter()
                            .map(|(n, total)| (n.to_string(), Some(*total)))
                            .collect(),
                    )
                } else if let Some(nt) = metadata.named_tuple_metadata() {
                    (
                        RecordKind::NamedTuple,
                        nt.elements.iter().map(|n| (n.to_string(), None)).collect(),
                    )
                } else if let Some(dm) = metadata.dataclass_metadata() {
                    let kind = if metadata.is_pydantic_model() {
                        RecordKind::Pydantic
                    } else {
                        match &dm.kind {
                            pyrefly::alt::types::class_metadata::DataclassKind::Attrs {
                                ..
                            } => RecordKind::Attrs,
                            pyrefly::alt::types::class_metadata::DataclassKind::Dataclass {
                                ..
                            } => RecordKind::Dataclass,
                        }
                    };
                    (
                        kind,
                        dm.instance_fields()
                            .map(|n| (n.to_string(), None))
                            .collect(),
                    )
                } else {
                    continue;
                };
            let native_class = b.class(&class)?;
            let mut ordinal = 0;
            for (name, total) in names {
                let pname = ruff_python_ast::name::Name::new(&name);
                // A method assignment does not redeclare an inherited record field. Walk the
                // native MRO in order, keeping the declaring class's source as a referent.
                let classes = std::iter::once(&class).chain(
                    get_class_mro(&class, ctx)
                        .ancestors_no_object()
                        .iter()
                        .map(|c| c.class_object()),
                );
                let mut found = None;
                for declaring in classes {
                    let foreign = if declaring.module() == &ctx.module_info {
                        None
                    } else {
                        module_context(declaring)
                    };
                    let declaring_ctx = if declaring.module() == &ctx.module_info {
                        ctx
                    } else {
                        let Some(foreign) = foreign.as_ref() else {
                            continue;
                        };
                        foreign
                    };
                    if get_class_field_declaration(declaring,&pname,declaring_ctx).is_some_and(|d| matches!(d.definition,pyrefly::binding::binding::ClassFieldDefinition::DefinedInMethod { .. })) { continue; }
                    if let Some(field) =
                        get_class_field_from_current_class_only(declaring, &pname, declaring_ctx)
                    {
                        let range = declaring_ctx
                            .bindings()
                            .metadata()
                            .get_class(declaring.index())
                            .fields
                            .field_decl_range(&pname);
                        let declaration = match range {
                            Some(range) if declaring.module() == &ctx.module_info => {
                                spans.event(range, Some(SyntaxKind::ExprName))
                            }
                            Some(range) => field_occurrence(declaring, range)?,
                            None => None,
                        };
                        // The field and heap travel together; its type remains a native reference,
                        // while inherited declarations refer to the captured declaring artifact.
                        found = Some((field.clone(), declaring_ctx.answers.clone(), declaration));
                        break;
                    }
                }
                let Some((field, answers, declaration)) = found else {
                    b.boundary(
                        None,
                        ObligationKind::NativeUnavailable,
                        format!("record field {name} has no retained native field"),
                    )?;
                    continue;
                };
                let term = b.term(&field.ty(), 0)?;
                let (has_default, init, alias, kw_only, required, read_only) = match record {
                    RecordKind::Dataclass | RecordKind::Attrs | RecordKind::Pydantic => {
                        let flags = field.dataclass_flags_of(answers.heap());
                        (
                            Some(flags.default.is_some()),
                            Some(flags.init),
                            flags.init_by_alias.as_ref().map(ToString::to_string),
                            flags.kw_only,
                            None,
                            None,
                        )
                    }
                    RecordKind::NamedTuple => (
                        Some(!required(&field.as_named_tuple_requiredness())),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    RecordKind::TypedDict => {
                        let info = field.as_typed_dict_field_info(total.unwrap_or(true));
                        (
                            None,
                            None,
                            None,
                            None,
                            info.as_ref().map(|i| i.required),
                            info.as_ref().map(|i| i.read_only_reason.is_some()),
                        )
                    }
                };
                let default_term = if matches!(record, RecordKind::Dataclass | RecordKind::Attrs | RecordKind::Pydantic) {
                    field.dataclass_flags_of(answers.heap()).default.as_ref().map(|t| b.term(t, 0)).transpose()?
                } else { None };
                let row = RecordFieldObservation {
                    qualification: qualification.id(),
                    class: native_class,
                    name: name.into(),
                    record,
                    ordinal,
                    term: term.id,
                    declared: field.has_explicit_annotation(),
                    declaration,
                    has_default,
                    default_term: default_term.map(|t| t.id),
                    init,
                    alias,
                    kw_only,
                    required,
                    read_only,
                };
                b.out.hold(&row)?;
                b.out.fields.push((row, if default_term.is_some_and(|t| t.opaque) { Fidelity::DisplayOnly } else { term.fidelity() }));
                ordinal += 1;
            }
        }
    } else {
        b.boundary(
            None,
            ObligationKind::NativeUnavailable,
            "native class/member metadata and record solutions unavailable".into(),
        )?;
    }
    let protocols=crate::protocol_records::records(protocol.0,protocol.1,protocol.2,spans,qualification,budget,|ty| {
        let built=b.term(ty,0)?;Ok((built.id,built.fidelity()))
    })?;
    for (subject,reason,detail) in &protocols.boundaries {b.boundary(*subject,*reason,detail.clone())?;}
    b.out.protocols=Some(protocols);
    Ok(b.out)
}
impl Builder<'_, '_> {
    fn trace(
        &mut self,
        subject: Id<Occurrence>,
        role: TypeRole,
        range: Option<ruff_text_size::TextRange>,
    ) -> Result<(), ModelError> {
        match range.and_then(|range| self.context.answers_context.answers.get_type_trace(range)) {
            Some(ty) => self.observe(subject, role, false, &ty),
            None => self.boundary(
                Some(subject),
                ObligationKind::MissingEvidence,
                format!("native type trace unavailable for {role:?}"),
            ),
        }
    }
}
