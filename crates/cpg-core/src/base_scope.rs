//! Complete primitive Base inventories and selected rich expression/read dependency grains.
use super::execution_scope::{nominal, predicate};
use crate::{
    consumed_rows::{
        ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier, stream_query_at,
    },
    workspace::CompletedInputs,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{base_evaluation as publication, native::*},
    calls::*,
    conditions::entry::{EntryAccessSource, EntryData, EntryValueWitness},
    declarations::*,
    execution::{
        evaluation::{EvaluationData, PreparedEvaluationMetadata},
        read_channels::{PreparedReads, ReadEntries, ReadMetadata, ReadRecords, ReadRoot},
    },
    flow::*,
    lexical::*,
    normalized::{Rows, entities::*},
    source::*,
    symbols::*,
    syntax::*,
    *,
};
use std::{any::TypeId, sync::Arc};
#[derive(Clone, Copy)]
pub(super) enum Kernel {
    Expression,
    Use,
    Attribute,
    DynamicCall,
    DynamicAttribute,
    Formal,
    FieldRoot,
    FieldCall,
    FieldClass,
    FieldStore,
    FieldGlobal,
}
impl Kernel {
    fn type_id(self) -> TypeId {
        match self {
            Self::Expression => TypeId::of::<Occurrence>(),
            Self::Use => TypeId::of::<FlowUseObservation>(),
            Self::Attribute | Self::DynamicAttribute => {
                TypeId::of::<FlowAttributeLoadObservation>()
            }
            Self::DynamicCall | Self::FieldCall => TypeId::of::<CallSyntax>(),
            Self::Formal => TypeId::of::<ParameterEntity>(),
            Self::FieldRoot => TypeId::of::<SourceArtifact>(),
            Self::FieldClass => TypeId::of::<ClassFieldSyntaxObservation>(),
            Self::FieldStore => TypeId::of::<FlowDefinitionObservation>(),
            Self::FieldGlobal => TypeId::of::<BindingObservation>(),
        }
    }
    fn read(self, key: [u8; 16]) -> Result<ReadRoot, ModelError> {
        Ok(match self {
            Self::Use => ReadRoot::Use(nominal(&key)?),
            Self::Attribute => ReadRoot::Attribute(nominal(&key)?),
            Self::DynamicCall => ReadRoot::DynamicCall(nominal(&key)?),
            Self::DynamicAttribute => ReadRoot::DynamicAttribute(nominal(&key)?),
            Self::Formal => ReadRoot::Formal(nominal(&key)?),
            Self::FieldCall => ReadRoot::FieldCall(nominal(&key)?),
            Self::FieldClass => ReadRoot::FieldClass(nominal(&key)?),
            Self::FieldStore => ReadRoot::FieldStore(nominal(&key)?),
            Self::FieldGlobal => ReadRoot::FieldGlobal(nominal(&key)?),
            _ => return Err(ModelError::Schema("Base read root kind")),
        })
    }
}
pub(super) struct BaseData {
    pub(super) data: EvaluationData,
    pub(super) entry: EntryData,
    pub(super) witnesses: Rows<EntryValueWitness>,
    pub(super) sources: Rows<EntryAccessSource>,
}
impl BaseData {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            data: EvaluationData::new(budget),
            entry: EntryData::new(budget),
            witnesses: Rows::new(budget),
            sources: Rows::new(budget),
        }
    }
    fn visit(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.data.visit_input(input, batch)?;
        self.entry.visit_input(input, batch)?;
        if input.type_id() == TypeId::of::<EntryValueWitness>() {
            self.witnesses.decode(batch)?;
        }
        if input.type_id() == TypeId::of::<EntryAccessSource>() {
            self.sources.decode(batch)?;
        }
        Ok(())
    }
    fn roots(
        &self,
        budget: &resources::ResourceBudget,
    ) -> Result<
        (
            std::collections::BTreeSet<Id<SourceArtifact>>,
            Box<dyn resources::Reservation>,
        ),
        ModelError,
    > {
        let bytes = self
            .data
            .artifacts
            .iter()
            .map(|row| row.row_bytes())
            .sum::<usize>()
            + self.data.uses.len() * size_of::<input::ArtifactUse>();
        let copy = budget.reserve(
            "selected-base-root-metadata",
            bytes.saturating_mul(2) + self.data.artifacts.len() * 128,
        )?;
        let roots = admission::analysis_roots(
            &self.data.artifacts.iter().cloned().collect::<Vec<_>>(),
            &self.data.uses.iter().cloned().collect::<Vec<_>>(),
        )?;
        Ok((roots, copy))
    }
}
pub(super) struct BaseScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    expression: usize,
    dynamic_call: usize,
    dynamic_attribute: usize,
    field_call: usize,
    field_global: usize,
    _charge: charged::StateCharge,
}
impl BaseScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut inputs = EvaluationData::validation_inputs();
        inputs.extend(EntryData::facts_inputs());
        inputs.extend([
            ValidationInput::of::<EntryValueWitness>(&["id"]),
            ValidationInput::of::<EntryAccessSource>(&["id"]),
        ]);
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema(input.name()))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let index = |kind: TypeId| {
            inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("Base dependency input"))
        };
        let occurrence = index(TypeId::of::<Occurrence>())?;
        let expression = tables.len();
        tables.push(tables[occurrence].clone());
        let value = tables.len();
        tables.push(tables[occurrence].clone());
        let child = tables.len();
        tables.push(tables[index(TypeId::of::<SyntaxPlacement>())?].clone());
        let inspect = tables.len();
        tables.push(tables[occurrence].clone());
        let dynamic_call = tables.len();
        tables.push(tables[index(TypeId::of::<CallSyntax>())?].clone());
        let dynamic_attribute = tables.len();
        tables.push(tables[index(TypeId::of::<FlowAttributeLoadObservation>())?].clone());
        let field_call = tables.len();
        tables.push(tables[index(TypeId::of::<CallSyntax>())?].clone());
        let field_global = tables.len();
        tables.push(tables[index(TypeId::of::<BindingObservation>())?].clone());
        let binding_event = tables.len();
        tables.push(tables[index(TypeId::of::<BindingEvent>())?].clone());
        let ruff_binding = tables.len();
        tables.push(tables[index(TypeId::of::<ruff::RuffBindingObservation>())?].clone());
        let use_state = tables.len();
        tables.push(tables[index(TypeId::of::<FlowUse>())?].clone());
        let reach_state = tables.len();
        tables.push(tables[index(TypeId::of::<FlowReachingObservation>())?].clone());
        let definition_state = tables.len();
        tables.push(tables[index(TypeId::of::<FlowDefinition>())?].clone());
        let definition_observation = tables.len();
        tables.push(tables[index(TypeId::of::<FlowDefinitionObservation>())?].clone());
        let resolution_state = tables.len();
        tables.push(tables[index(TypeId::of::<LexicalResolution>())?].clone());
        let assignment = tables.len();
        tables.push(tables[occurrence].clone());
        let constructed_call = tables.len();
        tables.push(tables[index(TypeId::of::<CallSyntax>())?].clone());
        let mut plan = NominalClosure::new(tables.clone())?;
        for (source, table) in tables.iter().take(inputs.len()).enumerate() {
            for field in table.relation.fields() {
                // Actual Local owns entry proofs. The selected compiler consumer needs its stored
                // receipt and linkage, not another entire native entry inventory or producer replay.
                if table.relation.type_id() == TypeId::of::<EntryValueWitness>()
                    && !matches!(
                        field.name(),
                        "owner"
                            | "formal"
                            | "access"
                            | "context"
                            | "run"
                            | "access_source"
                            | "link"
                            | "parameter_placement"
                    )
                {
                    continue;
                }
                if (table.relation.type_id() == TypeId::of::<DeclarationObservation>()
                    && field.name() == "docstring")
                    || (table.relation.type_id() == TypeId::of::<ClassFieldSyntaxObservation>()
                        && matches!(field.name(), "annotation" | "value"))
                    || (table.relation.type_id() == TypeId::of::<BindingObservation>()
                        && field.name() == "value")
                    || (table.relation.type_id() == TypeId::of::<FlowDefinitionObservation>()
                        && field.name() == "value")
                {
                    continue;
                }
                let Some((target, _)) = field.target() else {
                    continue;
                };
                let Some(target) = crate::scoped_admission::field_target(&inputs, source, target)?
                else {
                    continue;
                };
                if field.list() {
                    plan.pairs(
                        source,
                        target,
                        format!(
                            "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                            identifier(field.name()),
                            identifier(&table.alias)
                        ),
                    )?;
                } else {
                    plan.follow(source, field.name(), target)?;
                }
                if field.name() == "assertion"
                    || field.name().ends_with("_assertion")
                    || (table.relation.type_id() == TypeId::of::<NativeQualification>()
                        && field.name() == "premise")
                {
                    plan.own(source, field.name(), target)?;
                }
            }
        }
        macro_rules! own {
            ($member:ty,$field:literal,$owner:ty) => {
                plan.own(
                    index(TypeId::of::<$member>())?,
                    $field,
                    index(TypeId::of::<$owner>())?,
                )?
            };
        }
        own!(input::ArtifactUse, "artifact", SourceArtifact);
        own!(OccurrenceOwnership, "occurrence", Occurrence);
        own!(SyntaxPlacement, "occurrence", Occurrence);
        own!(DeclarationObservation, "declaration", Occurrence);
        own!(FlowUse, "occurrence", Occurrence);
        own!(FlowUseObservation, "use_", FlowUse);
        own!(FlowValueObservation, "use_", FlowUse);
        own!(FlowReachingObservation, "use_", FlowUse);
        own!(FlowDefinition, "occurrence", Occurrence);
        own!(FlowDefinitionObservation, "definition", FlowDefinition);
        own!(EntryValueWitness, "access", Occurrence);
        own!(ReferenceObservation, "read", Occurrence);
        own!(LexicalResolution, "read", Occurrence);
        own!(CallSyntax, "site", Occurrence);
        own!(CallTarget, "site", Occurrence);
        own!(BindingEvent, "site", Occurrence);
        own!(BindingObservation, "event", BindingEvent);
        own!(ruff::RuffBindingObservation, "event", BindingEvent);
        own!(ruff::RuffContextObservation, "subject", Occurrence);
        own!(SymbolDeclaration, "declaration", Occurrence);
        own!(SymbolDeclaration, "symbol", ProviderSymbol);
        own!(SymbolObservation, "symbol", ProviderSymbol);
        own!(FunctionTraitObservation, "symbol", ProviderSymbol);
        own!(ClassAncestryObservation, "class", ProviderSymbol);
        own!(SymbolSequenceMember, "sequence", SymbolSequence);
        own!(Signature, "symbol", ProviderSymbol);
        own!(SignatureParameter, "signature", Signature);
        own!(ParameterEntityLink, "parameter", SignatureParameter);
        own!(ParameterEntityLink, "entity", ParameterEntity);
        own!(ParameterDeclaration, "parameter", SignatureParameter);
        own!(
            normalized::callables::EffectiveCallableAssessment,
            "callable",
            CallableEntity
        );
        own!(LexicalScope, "owner", Occurrence);
        let t = |kind: TypeId| Ok::<_, ModelError>(identifier(&tables[index(kind)?].alias));
        macro_rules! table {
            ($ty:ty) => {
                t(TypeId::of::<$ty>())?
            };
        }
        macro_rules! pair {
            ($from:ty,$to:ty,$sql:expr) => {
                plan.pairs(
                    index(TypeId::of::<$from>())?,
                    index(TypeId::of::<$to>())?,
                    $sql,
                )?
            };
        }
        let occurrences = table!(Occurrence);
        let placements = table!(SyntaxPlacement);
        let refs = table!(EntityRef);
        let callables = table!(CallableEntity);
        let classes = table!(ClassEntity);
        let formals = table!(ParameterEntity);
        pair!(
            CallableEntity,
            EntityRef,
            format!(
                "SELECT c.id AS source_id,r.id AS target_id FROM {callables} c JOIN {refs} r ON r.callable_callable=c.id"
            )
        );
        pair!(
            Occurrence,
            CallableEntity,
            format!(
                "SELECT o.id AS source_id,c.id AS target_id FROM {occurrences} o JOIN {callables} c ON c.source_declaration=o.id"
            )
        );
        pair!(
            Occurrence,
            ClassEntity,
            format!(
                "SELECT o.id AS source_id,c.id AS target_id FROM {occurrences} o JOIN {classes} c ON c.source_declaration=o.id"
            )
        );
        pair!(
            Occurrence,
            ParameterEntity,
            format!(
                "SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {formals} p ON p.source_declaration=o.id"
            )
        );
        // The nearest formal owner is a complete structural-prefix competition across source
        // callable anchors. Lexical/default ownership does not replace this signature owner.
        pair!(
            ParameterEntity,
            CallableEntity,
            format!(
                "SELECT p.id AS source_id,c.id AS target_id FROM {formals} p JOIN {occurrences} o ON o.id=p.source_declaration JOIN {occurrences} f ON f.source=o.source AND array_length(f.structural_path)<array_length(o.structural_path) AND array_slice(o.structural_path,1,CAST(array_length(f.structural_path) AS BIGINT))=f.structural_path JOIN {callables} c ON c.source_declaration=f.id"
            )
        );
        let spellings = table!(SyntaxObservation);
        let details = table!(SyntaxDetailObservation);
        pair!(
            Occurrence,
            SyntaxObservation,
            format!(
                "SELECT o.id AS source_id,s.id AS target_id FROM {occurrences} o JOIN {spellings} s ON s.occurrence=o.id WHERE o.syntax_kind IN ({},{})",
                SyntaxKind::ExprName.code(),
                SyntaxKind::Identifier.code()
            )
        );
        plan.pairs(
            expression,
            value,
            format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"),
        )?;
        plan.pairs(
            value,
            occurrence,
            format!("SELECT id AS source_id,id AS target_id FROM {occurrences}"),
        )?;
        plan.pairs(value,index(TypeId::of::<SyntaxDetailObservation>())?,format!("SELECT o.id AS source_id,d.id AS target_id FROM {occurrences} o JOIN {details} d ON d.occurrence=o.id"))?;
        plan.pairs(expression,child,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind NOT IN ({},{},{}) OR p.field!={}",SyntaxKind::StmtFunctionDef.code(),SyntaxKind::StmtClassDef.code(),SyntaxKind::ExprLambda.code(),SyntaxField::Body.code()))?;
        plan.pairs(
            child,
            index(TypeId::of::<SyntaxPlacement>())?,
            format!("SELECT id AS source_id,id AS target_id FROM {placements}"),
        )?;
        plan.pairs(
            child,
            expression,
            format!("SELECT id AS source_id,occurrence AS target_id FROM {placements}"),
        )?;
        // Rich value expansion belongs only to a dynamic inspection or exact-name read.
        // Merely loading a native binding/definition as metadata cannot import its assignment
        // body, and a field declaration does not need the rich value of its initializer.
        let args = table!(CallArgument);
        let calls = table!(CallSyntax);
        let attributes = table!(FlowAttributeLoadObservation);
        let events = table!(BindingEvent);
        let bindings = table!(BindingObservation);
        let ruff_bindings = table!(ruff::RuffBindingObservation);
        let uses_data = table!(FlowUse);
        let reaching = table!(FlowReachingObservation);
        let reaching_targets = table!(ReachingDefinition);
        let definitions = table!(FlowDefinition);
        let observations = table!(FlowDefinitionObservation);
        let resolutions = table!(LexicalResolution);
        let lexical_targets = table!(LexicalTarget);
        let mut identity = |private: usize, actual: usize, table: &str| {
            plan.pairs(
                private,
                actual,
                format!("SELECT id AS source_id,id AS target_id FROM {table}"),
            )
        };
        identity(inspect, occurrence, &occurrences)?;
        identity(dynamic_call, index(TypeId::of::<CallSyntax>())?, &calls)?;
        identity(
            dynamic_attribute,
            index(TypeId::of::<FlowAttributeLoadObservation>())?,
            &attributes,
        )?;
        identity(field_call, index(TypeId::of::<CallSyntax>())?, &calls)?;
        identity(
            field_global,
            index(TypeId::of::<BindingObservation>())?,
            &bindings,
        )?;
        identity(binding_event, index(TypeId::of::<BindingEvent>())?, &events)?;
        identity(
            ruff_binding,
            index(TypeId::of::<ruff::RuffBindingObservation>())?,
            &ruff_bindings,
        )?;
        identity(use_state, index(TypeId::of::<FlowUse>())?, &uses_data)?;
        identity(
            reach_state,
            index(TypeId::of::<FlowReachingObservation>())?,
            &reaching,
        )?;
        identity(
            definition_state,
            index(TypeId::of::<FlowDefinition>())?,
            &definitions,
        )?;
        identity(
            definition_observation,
            index(TypeId::of::<FlowDefinitionObservation>())?,
            &observations,
        )?;
        identity(
            resolution_state,
            index(TypeId::of::<LexicalResolution>())?,
            &resolutions,
        )?;
        identity(assignment, occurrence, &occurrences)?;
        identity(constructed_call, index(TypeId::of::<CallSyntax>())?, &calls)?;
        drop(identity);
        plan.pairs(dynamic_call,index(TypeId::of::<CallArgument>())?,format!("SELECT c.id AS source_id,a.id AS target_id FROM {calls} c JOIN {args} a ON a.call=c.id WHERE a.ordinal IN (0,1) AND a.kind={}",ArgumentKind::Positional.code()))?;
        plan.pairs(dynamic_call,value,format!("SELECT c.id AS source_id,a.value AS target_id FROM {calls} c JOIN {args} a ON a.call=c.id WHERE a.ordinal=1 AND a.kind={}",ArgumentKind::Positional.code()))?;
        plan.pairs(dynamic_call,inspect,format!("SELECT id AS source_id,callee AS target_id FROM {calls} UNION SELECT c.id AS source_id,a.value AS target_id FROM {calls} c JOIN {args} a ON a.call=c.id WHERE a.ordinal=0 AND a.kind={}",ArgumentKind::Positional.code()))?;
        plan.pairs(field_call,index(TypeId::of::<CallArgument>())?,format!("SELECT c.id AS source_id,a.id AS target_id FROM {calls} c JOIN {args} a ON a.call=c.id WHERE a.ordinal=1 AND a.kind={}",ArgumentKind::Positional.code()))?;
        plan.pairs(field_call,value,format!("SELECT c.id AS source_id,a.value AS target_id FROM {calls} c JOIN {args} a ON a.call=c.id WHERE a.ordinal=1 AND a.kind={}",ArgumentKind::Positional.code()))?;
        plan.pairs(dynamic_attribute,inspect,format!("SELECT a.id AS source_id,p.occurrence AS target_id FROM {attributes} a JOIN {placements} p ON p.parent=a.occurrence WHERE p.field={}",SyntaxField::Value.code()))?;
        plan.pairs(dynamic_attribute,index(TypeId::of::<SyntaxPlacement>())?,format!("SELECT a.id AS source_id,p.id AS target_id FROM {attributes} a JOIN {placements} p ON p.parent=a.occurrence WHERE p.field={}",SyntaxField::Value.code()))?;
        plan.pairs(
            field_global,
            binding_event,
            format!("SELECT id AS source_id,event AS target_id FROM {bindings}"),
        )?;
        plan.pairs(inspect,constructed_call,format!("SELECT o.id AS source_id,c.id AS target_id FROM {occurrences} o JOIN {calls} c ON c.site=o.id"))?;
        plan.pairs(
            constructed_call,
            inspect,
            format!("SELECT id AS source_id,callee AS target_id FROM {calls}"),
        )?;
        plan.pairs(inspect,use_state,format!("SELECT o.id AS source_id,u.id AS target_id FROM {occurrences} o JOIN {uses_data} u ON u.occurrence=o.id"))?;
        plan.pairs(inspect,resolution_state,format!("SELECT o.id AS source_id,r.id AS target_id FROM {occurrences} o JOIN {resolutions} r ON r.read=o.id"))?;
        plan.pairs(inspect,binding_event,format!("SELECT o.id AS source_id,e.id AS target_id FROM {occurrences} o JOIN {events} e ON e.site=o.id"))?;
        plan.pairs(use_state,reach_state,format!("SELECT u.id AS source_id,r.id AS target_id FROM {uses_data} u JOIN {reaching} r ON r.use_=u.id"))?;
        plan.pairs(reach_state,definition_state,format!("SELECT r.id AS source_id,t.bound_definition AS target_id FROM {reaching} r JOIN {reaching_targets} t ON t.id=r.target WHERE t.bound_definition IS NOT NULL"))?;
        plan.pairs(definition_state,definition_observation,format!("SELECT d.id AS source_id,o.id AS target_id FROM {definitions} d JOIN {observations} o ON o.definition=d.id"))?;
        plan.pairs(definition_observation,inspect,format!("SELECT id AS source_id,value AS target_id FROM {observations} WHERE value IS NOT NULL"))?;
        plan.pairs(resolution_state,binding_event,format!("SELECT r.id AS source_id,t.binding_event AS target_id FROM {resolutions} r JOIN {lexical_targets} t ON t.id=r.target WHERE t.binding_event IS NOT NULL"))?;
        plan.pairs(binding_event,ruff_binding,format!("SELECT e.id AS source_id,b.id AS target_id FROM {events} e JOIN {ruff_bindings} b ON b.event=e.id"))?;
        plan.pairs(ruff_binding,assignment,format!("SELECT b.id AS source_id,p.parent AS target_id FROM {ruff_bindings} b JOIN {events} e ON e.id=b.event JOIN {placements} p ON p.occurrence=e.site WHERE b.kind={} AND p.parent IS NOT NULL",ruff::RuffBindingKind::Assignment.code()))?;
        plan.pairs(assignment,index(TypeId::of::<SyntaxPlacement>())?,format!("SELECT o.id AS source_id,p.id AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind={} AND p.field={}",SyntaxKind::StmtAssign.code(),SyntaxField::Value.code()))?;
        plan.pairs(assignment,inspect,format!("SELECT o.id AS source_id,p.occurrence AS target_id FROM {occurrences} o JOIN {placements} p ON p.parent=o.id WHERE o.syntax_kind={} AND p.field={}",SyntaxKind::StmtAssign.code(),SyntaxField::Value.code()))?;
        let uses = table!(FlowUseObservation);
        let regions = table!(FlowRegionObservation);
        let qualifications = table!(assertion::AssertionQualification);
        pair!(
            FlowUseObservation,
            FlowRegionObservation,
            format!(
                "SELECT u.id AS source_id,r.id AS target_id FROM {uses} u JOIN {qualifications} uq ON uq.id=u.qualification JOIN {regions} r ON r.scope=u.scope JOIN {qualifications} rq ON rq.id=r.qualification AND rq.context=uq.context"
            )
        );
        let bindings = table!(ruff::RuffBindingObservation);
        pair!(
            ruff::RuffBindingObservation,
            ruff::RuffBindingObservation,
            format!(
                "SELECT a.id AS source_id,b.id AS target_id FROM {bindings} a JOIN {bindings} b ON b.scope=a.scope AND b.native_name=a.native_name"
            )
        );
        let artifacts = table!(SourceArtifact);
        let scopes = table!(CoverageScope);
        let coverage = table!(attribution::ProviderCoverage);
        let modules = table!(Module);
        pair!(
            SourceArtifact,
            attribution::ProviderCoverage,
            format!(
                "SELECT a.id AS source_id,c.id AS target_id FROM {artifacts} a JOIN {scopes} s ON s.input_input=a.input OR s.artifact_artifact=a.id JOIN {coverage} c ON c.scope=s.id UNION SELECT a.id AS source_id,c.id AS target_id FROM {artifacts} a JOIN {modules} m ON m.source=a.id JOIN {scopes} s ON s.module_module=m.id JOIN {coverage} c ON c.scope=s.id"
            )
        );
        // Every competing builtin provider is needed, even when the selected source name has
        // another canonical resolution. This finite native class domain is context-grained.
        let symbols = table!(ProviderSymbol);
        let provider_modules = table!(ProviderModule);
        let names = std::iter::once("BaseException")
            .chain(
                execution::ExactRuntimeException::ALL
                    .iter()
                    .map(|kind| kind.class().1),
            )
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(",");
        pair!(
            assertion::AssertionQualification,
            ProviderSymbol,
            format!(
                "SELECT q.id AS source_id,s.id AS target_id FROM {qualifications} q JOIN {symbols} s ON s.context=q.context JOIN {provider_modules} m ON m.id=s.module WHERE s.kind={} AND s.name IN ({names}) AND m.bundled_name='builtins' AND m.bundled_bundle={}",
                SymbolKind::Class.code(),
                ModuleBundle::Typeshed.code()
            )
        );
        let mut charge = charged::StateCharge::new(budget, "Base-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            expression,
            dynamic_call,
            dynamic_attribute,
            field_call,
            field_global,
            _charge: charge,
        })
    }
    fn index(&self, kind: TypeId) -> Result<usize, ModelError> {
        self.inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("Base scope type"))
    }
    fn table<R: Record>(&self) -> Result<String, ModelError> {
        Ok(identifier(
            &self.tables[self.index(TypeId::of::<R>())?].alias,
        ))
    }
    fn selected(
        &self,
        inv: &publication::AnalysisInvocation,
        source: &str,
    ) -> Result<String, ModelError> {
        let artifacts = self.table::<SourceArtifact>()?;
        let uses = self.table::<input::ArtifactUse>()?;
        let input = predicate(inv.input).replacen("id IN", "a.input IN", 1);
        Ok(format!(
            "EXISTS (SELECT 1 FROM {artifacts} a WHERE a.id={source} AND {input} AND (a.path LIKE '%.py' OR a.path LIKE '%.pyi') AND EXISTS (SELECT 1 FROM {uses} u WHERE u.artifact=a.id AND u.role IN ({},{},{},{})))",
            input::SourceRole::Release.code(),
            input::SourceRole::Example.code(),
            input::SourceRole::Test.code(),
            input::SourceRole::DocBlock.code()
        ))
    }
    pub(super) fn roots(
        &self,
        inv: &publication::AnalysisInvocation,
        kernel: Kernel,
    ) -> Result<String, ModelError> {
        let root = identifier(&self.tables[self.index(kernel.type_id())?].alias);
        let occurrences = self.table::<Occurrence>()?;
        let q = self.table::<assertion::AssertionQualification>()?;
        let context = predicate(inv.context).replacen("id IN", "q.context IN", 1);
        let (joins, source) = match kernel {
            Kernel::Expression => (String::new(), "r.source"),
            Kernel::FieldRoot => (String::new(), "r.id"),
            Kernel::Use => (
                format!(
                    "LEFT JOIN {} u ON u.id=r.use_ LEFT JOIN {occurrences} o ON o.id=u.occurrence",
                    self.table::<FlowUse>()?
                ),
                "o.source",
            ),
            Kernel::Attribute | Kernel::DynamicAttribute => (
                format!("JOIN {occurrences} o ON o.id=r.occurrence"),
                "o.source",
            ),
            Kernel::DynamicCall | Kernel::FieldCall => {
                (format!("JOIN {occurrences} o ON o.id=r.site"), "o.source")
            }
            Kernel::Formal => (
                format!("JOIN {occurrences} o ON o.id=r.source_declaration"),
                "o.source",
            ),
            Kernel::FieldClass => (format!("JOIN {occurrences} o ON o.id=r.target"), "o.source"),
            Kernel::FieldStore => (
                format!(
                    "LEFT JOIN {} d ON d.id=r.definition LEFT JOIN {occurrences} o ON o.id=d.occurrence",
                    self.table::<FlowDefinition>()?
                ),
                "o.source",
            ),
            Kernel::FieldGlobal => (
                format!(
                    "JOIN {} e ON e.id=r.event JOIN {occurrences} o ON o.id=e.site",
                    self.table::<BindingEvent>()?
                ),
                "o.source",
            ),
        };
        let selected = self.selected(inv, source)?;
        let filter = match kernel {
            Kernel::Use => format!("({selected} OR u.id IS NULL)"),
            Kernel::FieldStore => format!("({selected} OR d.id IS NULL)"),
            _ => selected,
        };
        let qualified = if matches!(
            kernel,
            Kernel::Expression | Kernel::Formal | Kernel::FieldRoot
        ) {
            String::new()
        } else {
            format!("JOIN {q} q ON q.id=r.qualification")
        };
        let qualified_filter = if qualified.is_empty() {
            String::new()
        } else {
            format!(" AND {context}")
        };
        let expressions = [
            SyntaxKind::ExprBoolOp,
            SyntaxKind::ExprNamed,
            SyntaxKind::ExprBinOp,
            SyntaxKind::ExprUnaryOp,
            SyntaxKind::ExprLambda,
            SyntaxKind::ExprIf,
            SyntaxKind::ExprDict,
            SyntaxKind::ExprSet,
            SyntaxKind::ExprListComp,
            SyntaxKind::ExprSetComp,
            SyntaxKind::ExprDictComp,
            SyntaxKind::ExprGenerator,
            SyntaxKind::ExprAwait,
            SyntaxKind::ExprYield,
            SyntaxKind::ExprYieldFrom,
            SyntaxKind::ExprCompare,
            SyntaxKind::ExprCall,
            SyntaxKind::ExprFString,
            SyntaxKind::ExprTString,
            SyntaxKind::ExprStringLiteral,
            SyntaxKind::ExprBytesLiteral,
            SyntaxKind::ExprNumberLiteral,
            SyntaxKind::ExprBooleanLiteral,
            SyntaxKind::ExprNoneLiteral,
            SyntaxKind::ExprEllipsisLiteral,
            SyntaxKind::ExprAttribute,
            SyntaxKind::ExprSubscript,
            SyntaxKind::ExprStarred,
            SyntaxKind::ExprName,
            SyntaxKind::ExprList,
            SyntaxKind::ExprTuple,
            SyntaxKind::ExprSlice,
            SyntaxKind::ExprIpyEscapeCommand,
        ];
        let kind = if matches!(kernel, Kernel::Expression) {
            format!(
                " AND r.syntax_kind IN ({})",
                expressions
                    .iter()
                    .map(|kind| kind.code().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        } else {
            String::new()
        };
        Ok(format!(
            "SELECT r.id FROM {root} r {joins} {qualified} WHERE {filter}{qualified_filter}{kind} ORDER BY r.id"
        ))
    }
    pub(super) async fn scope(
        &self,
        kernel: Kernel,
        key: [u8; 16],
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let index = match kernel {
            Kernel::Expression => self.expression,
            Kernel::DynamicCall => self.dynamic_call,
            Kernel::DynamicAttribute => self.dynamic_attribute,
            Kernel::FieldCall => self.field_call,
            Kernel::FieldGlobal => self.field_global,
            _ => self.index(kernel.type_id())?,
        };
        self.edges
            .grain(
                index,
                &crate::scoped_admission::root_predicate(&[key]),
                budget,
            )
            .await
    }
    pub(super) async fn data(
        &self,
        access: &CompletedInputs,
        scope: &PreparedClosure,
        budget: &resources::ResourceBudget,
        opaque_literals: bool,
    ) -> Result<BaseData, ModelError> {
        let mut data = BaseData::new(budget);
        for (table, input) in self.inputs.iter().enumerate() {
            let sql = scope.select(table)?;
            let mut loaded = false;
            if opaque_literals && input.type_id() == TypeId::of::<value::Literal>() {
                let permit = access.read_at::<value::Literal>(input.prefix())?;
                let rich = format!(
                    "SELECT selected.* FROM ({sql}) selected WHERE selected.kind NOT IN (3,4)"
                );
                stream_query_at(&permit, input, scope.session(), &rich, |_, batch| {
                    data.visit(input, batch)
                })
                .await?;
                project_literals(scope.session(), &sql, &mut data, budget).await?;
                continue;
            }
            macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(if !loaded && input.type_id()==TypeId::of::<$ty>() {let permit=access.read_at::<$ty>(input.prefix())?;stream_query_at(&permit,input,scope.session(),&sql,|_,batch|data.visit(input,batch)).await?;loaded=true;})*};}
            lctx_model::execution_evaluation_inputs!(read);
            lctx_model::entry_value_inputs!(read);
            macro_rules! records {($($ty:ty),*)=>{$(if !loaded && input.type_id()==TypeId::of::<$ty>() {let permit=access.read_at::<$ty>(input.prefix())?;stream_query_at(&permit,input,scope.session(),&sql,|_,batch|data.visit(input,batch)).await?;loaded=true;})*};}
            records!(EntryValueWitness, EntryAccessSource);
            if !loaded {
                return Err(ModelError::Schema("Base typed scope loader"));
            }
        }
        Ok(data)
    }
}
async fn project_literals(
    session: &datafusion::prelude::SessionContext,
    sql: &str,
    data: &mut BaseData,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let query = format!(
        "SELECT selected.id,selected.kind FROM ({sql}) selected WHERE selected.kind IN (3,4) ORDER BY selected.id"
    );
    let mut stream = crate::sql::query(session, &query)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let _transfer = budget.reserve(
            "Base-literal-kind-transfer",
            lctx_model::domain::logical_batch_bytes(&batch)?,
        )?;
        let kinds = batch
            .column_by_name("kind")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int16Array>())
            .ok_or(ModelError::Schema("Base literal projection kind"))?;
        for row in 0..batch.num_rows() {
            let key = crate::scoped_admission::column(&batch, "id", row)?
                .ok_or(ModelError::Schema("Base literal projection ID"))?;
            data.data
                .project_opaque_literal(nominal(&key)?, kinds.value(row))?;
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}
fn projected_native_pair(
    kind: i16,
    assertion: [u8; 16],
    support: [u8; 16],
) -> Result<NativeAssertionPremise, ModelError> {
    macro_rules! pair {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{match kind {$($code=>Ok(NativeAssertionPremise::$variant {assertion:nominal::<$assertion>(&assertion)?,support:nominal::<$support>(&support)?}),)*_=>Err(ModelError::Schema("unknown native pair projection kind"))}};}
    lctx_model::native_analysis_pairs!(pair)
}
impl BaseScopes {
    /// Project and prepare complete fixed-value inventories before any rich kernel decode.
    pub(super) async fn metadata(
        &self,
        session: &datafusion::prelude::SessionContext,
        inv: &publication::AnalysisInvocation,
        budget: &resources::ResourceBudget,
    ) -> Result<(PreparedReads, PreparedEvaluationMetadata), ModelError> {
        let mut reads = PreparedReads::new(inv, budget);
        let mut evaluation = PreparedEvaluationMetadata::new(inv.input, inv.context, budget);
        let q = self.table::<assertion::AssertionQualification>()?;
        let context = predicate(inv.context).replacen("id IN", "q.context IN", 1);
        let placements = self.table::<SyntaxPlacement>()?;
        let occurrences = self.table::<Occurrence>()?;
        let owners = self.table::<OccurrenceOwnership>()?;
        let mut duplicates=crate::sql::query(session,&format!("SELECT p.occurrence FROM {placements} p JOIN {q} q ON q.id=p.qualification WHERE {context} GROUP BY p.occurrence HAVING COUNT(*)>1 LIMIT 1")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = duplicates.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid(
                    "execution syntax has ambiguous placement".into(),
                ));
            }
        }
        let native = self.table::<NativeQualification>()?;
        let premises = self.table::<NativeAssertionPremise>()?;
        let mut missing=crate::sql::query(session,&format!("SELECT n.premise FROM {native} n LEFT JOIN {premises} p ON p.id=n.premise WHERE p.id IS NULL LIMIT 1")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = missing.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid("read native pair missing".into()));
            }
        }
        for kind in [
            TypeId::of::<NativeQualification>(),
            TypeId::of::<OccurrenceOwnership>(),
            TypeId::of::<FlowUseObservation>(),
            TypeId::of::<FlowUse>(),
            TypeId::of::<SyntaxPlacement>(),
            TypeId::of::<FlowReachingObservation>(),
            TypeId::of::<CallTarget>(),
            TypeId::of::<Occurrence>(),
        ] {
            let table = identifier(&self.tables[self.index(kind)?].alias);
            let mut counts = crate::sql::query(
                session,
                &format!("SELECT CAST(COUNT(*) AS BIGINT) AS n FROM {table}"),
            )
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(batch) = counts.try_next().await.map_err(ModelError::codec)? {
                let n = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow_array::Int64Array>()
                    .ok_or(ModelError::Schema("Base metadata count"))?;
                for row in 0..batch.num_rows() {
                    reads
                        .scan_metadata(usize::try_from(n.value(row)).map_err(ModelError::codec)?)?;
                }
            }
        }
        let sum = NativeAssertionPremise::sum()
            .ok_or(ModelError::Schema("native pair sum declaration"))?;
        let projection = |suffix: &str| -> Result<String, ModelError> {
            let mut clauses = Vec::new();
            for arm in &sum.arms {
                let field = arm
                    .fields
                    .iter()
                    .find(|field| field.name.ends_with(suffix))
                    .ok_or(ModelError::Schema("native pair projection arm"))?;
                clauses.push(format!(
                    "WHEN {} THEN p.{}",
                    arm.code,
                    identifier(field.name)
                ));
            }
            Ok(format!(
                "CASE p.{} {} END",
                identifier(sum.tag),
                clauses.join(" ")
            ))
        };
        let query = format!(
            "SELECT n.id AS qualification_id,n.premise,p.kind,{} AS assertion_id,{} AS support_id FROM {native} n JOIN {premises} p ON p.id=n.premise ORDER BY n.id",
            projection("_assertion")?,
            projection("_support")?
        );
        let mut native_rows = crate::sql::query(session, &query)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = native_rows.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "Base-native-metadata-transfer",
                lctx_model::domain::logical_batch_bytes(&batch)?,
            )?;
            let kinds = batch
                .column_by_name("kind")
                .and_then(|column| column.as_any().downcast_ref::<arrow_array::Int16Array>())
                .ok_or(ModelError::Schema("Base native pair kind"))?;
            for row in 0..batch.num_rows() {
                let id = |name| {
                    crate::scoped_admission::column(&batch, name, row)?
                        .ok_or(ModelError::Schema("Base native pair ID"))
                };
                let premise = projected_native_pair(
                    kinds.value(row),
                    id("assertion_id")?,
                    id("support_id")?,
                )?;
                let required: Id<NativeAssertionPremise> = nominal(&id("premise")?)?;
                if required != premise.id() {
                    return Err(ModelError::Conflict(
                        "projected native pair identity changed",
                    ));
                }
                reads.prepare(ReadMetadata::Native {
                    pair: premise.assertion_and_support(),
                    premise: required,
                    qualification: nominal(&id("qualification_id")?)?,
                })?;
            }
            tokio::task::yield_now().await;
        }
        let use_observations = self.table::<FlowUseObservation>()?;
        let uses = self.table::<FlowUse>()?;
        let places = self.table::<value::Place>()?;
        let roots = self.table::<value::PlaceRoot>()?;
        let reaching = self.table::<FlowReachingObservation>()?;
        let targets = self.table::<ReachingDefinition>()?;
        let definitions = self.table::<FlowDefinition>()?;
        let call_targets = self.table::<CallTarget>()?;
        let unique_owners = format!(
            "(SELECT m.occurrence,m.entity FROM {owners} m JOIN (SELECT occurrence FROM {owners} GROUP BY occurrence HAVING COUNT(*)=1) unique_owner ON unique_owner.occurrence=m.occurrence)"
        );
        // A repeated child placement retains the last canonical-ID parent, exactly as the
        // finite index. The SQL window resolves that complete group before rich decoding.
        let parameters = format!(
            "(SELECT occurrence,parent FROM (SELECT p.occurrence,p.parent,ROW_NUMBER() OVER (PARTITION BY p.occurrence ORDER BY p.id DESC ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS rank FROM {placements} p JOIN {occurrences} o ON o.id=p.parent WHERE p.field={} AND p.ordinal=0 AND o.syntax_kind={}) ranked WHERE rank=1)",
            SyntaxField::Child.code(),
            SyntaxKind::Parameter.code()
        );
        let queries = [
            (
                0,
                format!("SELECT occurrence AS a,entity AS b FROM {owners} ORDER BY id"),
            ),
            (
                1,
                format!(
                    "SELECT u.id AS a,r.formal_declaration AS b FROM {uses} u JOIN {places} p ON p.id=u.place JOIN {roots} r ON r.id=p.root WHERE r.formal_declaration IS NOT NULL AND EXISTS (SELECT 1 FROM {use_observations} o JOIN {q} q ON q.id=o.qualification WHERE o.use_=u.id AND {context}) ORDER BY u.id"
                ),
            ),
            (
                1,
                format!(
                    "SELECT r.use_ AS a,COALESCE(p.parent,d.occurrence) AS b FROM {reaching} r JOIN {q} q ON q.id=r.qualification JOIN {targets} t ON t.id=r.target JOIN {definitions} d ON d.id=t.bound_definition LEFT JOIN {parameters} p ON p.occurrence=d.occurrence WHERE {context} ORDER BY r.id"
                ),
            ),
            (
                2,
                format!(
                    "SELECT r.id AS a,o.entity AS b FROM {reaching} r JOIN {q} q ON q.id=r.qualification JOIN {targets} t ON t.id=r.target JOIN {uses} u ON u.id=r.use_ JOIN {unique_owners} o ON o.occurrence=u.occurrence WHERE {context} AND t.kind=2 ORDER BY r.id"
                ),
            ),
            (
                3,
                format!(
                    "SELECT t.id AS a,t.site AS b FROM {call_targets} t JOIN {q} q ON q.id=t.qualification WHERE {context} ORDER BY t.id"
                ),
            ),
            (
                4,
                format!(
                    "SELECT o.id AS a,m.entity AS b FROM {occurrences} o JOIN {unique_owners} m ON m.occurrence=o.id WHERE o.syntax_kind={} ORDER BY o.id",
                    SyntaxKind::ExprCall.code()
                ),
            ),
            (
                5,
                format!(
                    "SELECT m.entity AS a,m.entity AS b FROM {owners} m JOIN {occurrences} o ON o.id=m.occurrence WHERE o.syntax_kind IN ({},{}) ORDER BY m.entity",
                    SyntaxKind::ExprYield.code(),
                    SyntaxKind::ExprYieldFrom.code()
                ),
            ),
        ];
        for (kind, query) in queries {
            let mut stream = crate::sql::query(session, &query)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "Base-fixed-metadata-transfer",
                    lctx_model::domain::logical_batch_bytes(&batch)?,
                )?;
                for row in 0..batch.num_rows() {
                    let a = crate::scoped_admission::column(&batch, "a", row)?
                        .ok_or(ModelError::Schema("Base metadata key"))?;
                    let b = crate::scoped_admission::column(&batch, "b", row)?
                        .ok_or(ModelError::Schema("Base metadata value"))?;
                    if kind == 5 {
                        evaluation.lazy_owner(nominal(&a)?)?;
                        continue;
                    }
                    reads.prepare(match kind {
                        0 => ReadMetadata::Owner {
                            occurrence: nominal(&a)?,
                            entity: nominal(&b)?,
                        },
                        1 => ReadMetadata::FormalUse {
                            use_: nominal(&a)?,
                            parameter: nominal(&b)?,
                        },
                        2 => ReadMetadata::Nested {
                            reaching: nominal(&a)?,
                            owner: nominal(&b)?,
                        },
                        3 => ReadMetadata::CallTarget {
                            target: nominal(&a)?,
                            site: nominal(&b)?,
                        },
                        4 => ReadMetadata::Call {
                            occurrence: nominal(&a)?,
                            owner: nominal(&b)?,
                        },
                        _ => return Err(ModelError::Schema("Base metadata kind")),
                    })?;
                }
                tokio::task::yield_now().await;
            }
        }
        Ok((reads, evaluation))
    }
}
pub(super) async fn write_reads(
    output: &crate::workspace::ProducerOutput,
    records: &ReadRecords,
) -> Result<(), ModelError> {
    macro_rules! write {($($field:ident),*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
    write!(
        reads,
        dependencies,
        attributes, formals, dynamic, dynamic_premises
    );
    macro_rules! fields {($($field:ident),*)=>{$(for row in records.fields.$field.iter(){output.push(row.clone()).await?;})*};}
    fields!(locations, assessments, globals, global_assessments);
    Ok(())
}
impl BaseScopes {
    pub(super) async fn reads(
        &self,
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        runtime: &crate::workspace::Workspace,
        output: &crate::workspace::ProducerOutput,
        inv: &publication::AnalysisInvocation,
        actual: &local_semantics::ProducedLocal,
        prepared: &mut PreparedReads,
    ) -> Result<(), ModelError> {
        let budget = runtime.budget();
        for kernel in [
            Kernel::Use,
            Kernel::Attribute,
            Kernel::DynamicCall,
            Kernel::DynamicAttribute,
            Kernel::Formal,
            Kernel::FieldRoot,
            Kernel::FieldCall,
            Kernel::FieldClass,
            Kernel::FieldStore,
            Kernel::FieldGlobal,
        ] {
            if matches!(kernel, Kernel::FieldCall) {
                prepared.begin_field_candidates()?;
            }
            let mut stream = crate::sql::query(session, &self.roots(inv, kernel)?)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "Base-read-root-transfer",
                    lctx_model::domain::logical_batch_bytes(&batch)?,
                )?;
                for row in 0..batch.num_rows() {
                    runtime.cancellation().check()?;
                    let key = crate::scoped_admission::column(&batch, "id", row)?
                        .ok_or(ModelError::Schema("Base read root ID"))?;
                    let scope = self.scope(kernel, key, budget).await?;
                    let data = self.data(access, &scope, budget, false).await?;
                    if matches!(kernel, Kernel::FieldRoot) {
                        let _key = budget.reserve("Base-field-root-key", 128)?;
                        let roots = [nominal(&key)?].into_iter().collect();
                        prepared.field_roots(&data.data, &data.entry, inv, &roots)?;
                    } else {
                        let (roots, _root_charge) = data.roots(budget)?;
                        let values = ReadEntries {
                            witnesses: &data.witnesses,
                            sources: &data.sources,
                            actual,
                        };
                        let records = prepared.produce(
                            &data.data,
                            &data.entry,
                            inv,
                            &roots,
                            kernel.read(key)?,
                            budget,
                            values,
                        )?;
                        write_reads(output, &records).await?;
                    }
                    drop(data);
                    drop(scope);
                    tokio::task::yield_now().await;
                }
            }
        }
        prepared.seal_fields()?;
        let mut classes = charged::ChargedSet::default();
        let mut class_charge = charged::StateCharge::new(budget, "Base-dynamic-class-keys");
        for class in prepared.dynamic_classes() {
            classes.insert(&mut class_charge, *class.bytes())?;
        }
        let mut previous: Option<(Id<ClassEntity>, String)> = None;
        let mut previous_charge: Option<Box<dyn resources::Reservation>> = None;
        loop {
            let Some(next) = prepared.next_field_key(previous.as_ref()) else {
                break;
            };
            let key_charge = budget.reserve(
                "Base-field-key-transfer",
                size_of::<(Id<ClassEntity>, String)>().saturating_mul(2)
                    + next.1.len().saturating_mul(2),
            )?;
            let (class, name) = next.clone();
            let _predicate = budget.reserve(
                "Base-field-class-predicate",
                classes.len().saturating_add(1).saturating_mul(128),
            )?;
            let mut keys = classes.iter().copied().collect::<Vec<_>>();
            keys.push(*class.bytes());
            keys.sort();
            keys.dedup();
            let scope = self
                .edges
                .grain(
                    self.index(TypeId::of::<ClassEntity>())?,
                    &crate::scoped_admission::root_predicate(&keys),
                    budget,
                )
                .await?;
            drop(keys);
            let data = self.data(access, &scope, budget, false).await?;
            let (roots, _root_charge) = data.roots(budget)?;
            let values = ReadEntries {
                witnesses: &data.witnesses,
                sources: &data.sources,
                actual,
            };
            let records = prepared.produce(
                &data.data,
                &data.entry,
                inv,
                &roots,
                ReadRoot::FieldAssessment {
                    class,
                    name: name.clone(),
                },
                budget,
                values,
            )?;
            write_reads(output, &records).await?;
            previous = Some((class, name));
            previous_charge = Some(key_charge);
            drop(data);
            drop(scope);
            tokio::task::yield_now().await;
        }
        drop(previous);
        drop(previous_charge);
        Ok(())
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use assertion::*;
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    fn id<R>(n: u8) -> Id<R> {
        nominal(&[n; 16]).unwrap()
    }
    fn register<R: Record>(session: &SessionContext, rows: &[R]) {
        let batch = <R as Record>::encode(rows).unwrap();
        session.deregister_table(R::NAME).unwrap();
        session
            .register_table(
                R::NAME,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    async fn data(
        session: &SessionContext,
        prepared: &BaseScopes,
        scope: Option<&PreparedClosure>,
        budget: &resources::ResourceBudget,
        opaque_literals: bool,
    ) -> BaseData {
        let mut data = BaseData::new(budget);
        for (table, input) in prepared.inputs.iter().enumerate() {
            let sql = scope
                .map(|scope| scope.select(table).unwrap())
                .unwrap_or_else(|| {
                    format!(
                        "SELECT * FROM {} ORDER BY id",
                        identifier(&prepared.tables[table].alias)
                    )
                });
            if opaque_literals && input.type_id() == TypeId::of::<value::Literal>() {
                project_literals(session, &sql, &mut data, budget)
                    .await
                    .unwrap();
            }
            let sql = if opaque_literals && input.type_id() == TypeId::of::<value::Literal>() {
                format!("SELECT selected.* FROM ({sql}) selected WHERE selected.kind NOT IN (3,4)")
            } else {
                sql
            };
            let mut stream = crate::sql::query(session, &sql)
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                data.visit(input, &batch).unwrap();
            }
        }
        data
    }
    async fn prepare(session: &SessionContext, budget: &resources::ResourceBudget) -> BaseScopes {
        let mut inputs = EvaluationData::validation_inputs();
        inputs.extend(EntryData::facts_inputs());
        inputs.extend([
            ValidationInput::of::<EntryValueWitness>(&["id"]),
            ValidationInput::of::<EntryAccessSource>(&["id"]),
        ]);
        inputs.sort_by_key(|input| (input.name(), input.prefix()));
        inputs.dedup_by_key(|input| (input.name(), input.prefix()));
        let model = model().unwrap();
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: input.name().into(),
            })
            .collect();
        BaseScopes::prepare_bound(inputs, tables, session, budget)
            .await
            .unwrap()
    }
    fn empty(session: &SessionContext) {
        macro_rules! empty {($($field:ident:$ty:ty,)*)=>{$(register::<$ty>(session,&[]);)*};}
        lctx_model::execution_evaluation_inputs!(empty);
        lctx_model::entry_value_inputs!(empty);
        register::<EntryValueWitness>(session, &[]);
        register::<EntryAccessSource>(session, &[]);
    }
    async fn fixture(
        session: &SessionContext,
        budget: &resources::ResourceBudget,
        rich: bool,
        literal_root: bool,
    ) -> (BaseScopes, Occurrence, publication::AnalysisInvocation) {
        empty(session);
        let artifact =
            SourceArtifact::from_bytes(id(1), "scope.py".into(), b"def f():\n None\n").unwrap();
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: id(2),
            scope: scope.id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        let declaration = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 14,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![0],
        };
        let expression = Occurrence {
            start: 10,
            end: 14,
            syntax_kind: if literal_root {
                SyntaxKind::ExprStringLiteral
            } else {
                SyntaxKind::ExprNoneLiteral
            },
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 1],
            ..declaration.clone()
        };
        let name = Occurrence {
            start: 4,
            end: 5,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 0],
            ..declaration.clone()
        };
        let docstring = Occurrence {
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ExprStringLiteral,
            role: OccurrenceRole::Syntax,
            structural_path: if rich { vec![99; 2_000_000] } else { vec![99] },
            ..declaration.clone()
        };
        let unrelated = Occurrence {
            start: 1,
            end: 2,
            syntax_kind: SyntaxKind::StmtPass,
            role: OccurrenceRole::Syntax,
            structural_path: vec![1],
            ..declaration.clone()
        };
        let placements = vec![
            SyntaxPlacement {
                qualification: q.id(),
                occurrence: expression.id(),
                parent: Some(declaration.id()),
                field: SyntaxField::Body,
                ordinal: 0,
            },
            SyntaxPlacement {
                qualification: q.id(),
                occurrence: declaration.id(),
                parent: None,
                field: SyntaxField::Body,
                ordinal: 0,
            },
            SyntaxPlacement {
                qualification: q.id(),
                occurrence: unrelated.id(),
                parent: None,
                field: SyntaxField::Body,
                ordinal: 1,
            },
        ];
        let supports = placements
            .iter()
            .map(|placement| SyntaxPlacementSupport {
                assertion: placement.id(),
                run: id(3),
                surface: id(4),
                evidence: id(5),
                origin: attribution::Origin::AnalyzerAssertion,
                mode: attribution::ExtractionMode::NativeTraversal,
                fidelity: attribution::Fidelity::NativeStructural,
            })
            .collect::<Vec<_>>();
        let mut inventory = NativeInventory::new(budget);
        inventory
            .visit(
                AssertionQualification::NAME,
                &<AssertionQualification as Record>::encode(std::slice::from_ref(&q)).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxPlacement::NAME,
                &<SyntaxPlacement as Record>::encode(&placements).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxPlacementSupport::NAME,
                &<SyntaxPlacementSupport as Record>::encode(&supports).unwrap(),
            )
            .unwrap();
        let native = inventory.collect().unwrap();
        drop(inventory);
        register(
            session,
            &native.premises.iter().cloned().collect::<Vec<_>>(),
        );
        register(
            session,
            &native.qualifications.iter().cloned().collect::<Vec<_>>(),
        );
        drop(native);
        register(session, &[artifact.clone()]);
        register(
            session,
            &[input::ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: input::SourceRole::Release,
            }],
        );
        register(session, &[scope]);
        register(session, std::slice::from_ref(&q));
        register(
            session,
            &[
                expression.clone(),
                declaration.clone(),
                name.clone(),
                docstring.clone(),
                unrelated,
            ],
        );
        register(session, &placements);
        register(session, &supports);
        register(
            session,
            &[OccurrenceOwnership {
                occurrence: expression.id(),
                owner: declaration.id(),
                entity: id(6),
            }],
        );
        register(
            session,
            &[DeclarationObservation {
                qualification: q.id(),
                declaration: declaration.id(),
                name: name.id(),
                kind: DeclarationKind::Function,
                parent: None,
                overload: false,
                docstring: Some(docstring.id()),
            }],
        );
        let value = value::Literal::String {
            value: if rich {
                "x".repeat(8 << 20)
            } else {
                "small".into()
            }
            .into(),
        };
        let detail = SyntaxDetail::Literal {
            literal: value.id(),
        };
        let mut observations = vec![SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: docstring.id(),
            ordinal: 0,
            detail: detail.id(),
        }];
        if literal_root {
            observations.push(SyntaxDetailObservation {
                qualification: q.id(),
                occurrence: expression.id(),
                ordinal: 0,
                detail: detail.id(),
            });
        }
        register(session, &observations);
        register(session, &[detail]);
        register(session, &[value]);
        let detail_supports = observations
            .iter()
            .map(|observation| SyntaxDetailSupport {
                assertion: observation.id(),
                run: id(3),
                surface: id(4),
                evidence: id(5),
                origin: attribution::Origin::AnalyzerAssertion,
                mode: attribution::ExtractionMode::NativeTraversal,
                fidelity: attribution::Fidelity::NativeStructural,
            })
            .collect::<Vec<_>>();
        register(session, &detail_supports);
        let mut inventory = NativeInventory::new(budget);
        inventory
            .visit(
                AssertionQualification::NAME,
                &<AssertionQualification as Record>::encode(std::slice::from_ref(&q)).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxPlacement::NAME,
                &<SyntaxPlacement as Record>::encode(&placements).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxPlacementSupport::NAME,
                &<SyntaxPlacementSupport as Record>::encode(&supports).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxDetailObservation::NAME,
                &<SyntaxDetailObservation as Record>::encode(&observations).unwrap(),
            )
            .unwrap();
        inventory
            .visit(
                SyntaxDetailSupport::NAME,
                &<SyntaxDetailSupport as Record>::encode(&detail_supports).unwrap(),
            )
            .unwrap();
        let native = inventory.collect().unwrap();
        drop(inventory);
        register(
            session,
            &native.premises.iter().cloned().collect::<Vec<_>>(),
        );
        register(
            session,
            &native.qualifications.iter().cloned().collect::<Vec<_>>(),
        );
        drop(native);
        drop(observations);
        drop(detail_supports);
        let modules = [
            ProviderModule::Bundled {
                provider: id(20),
                bundle: ModuleBundle::Typeshed,
                name: "builtins".into(),
            },
            ProviderModule::Bundled {
                provider: id(21),
                bundle: ModuleBundle::Typeshed,
                name: "builtins".into(),
            },
        ];
        let symbols = modules
            .iter()
            .enumerate()
            .map(|(n, module)| ProviderSymbol {
                provider: id(20 + n as u8),
                context: q.context,
                module: module.id(),
                native_key: "ValueError".into(),
                name: "ValueError".into(),
                kind: SymbolKind::Class,
            })
            .collect::<Vec<_>>();
        register(session, &modules);
        register(session, &symbols);
        let definition = execution::configuration::base_evaluation().1;
        let invocation = publication::AnalysisInvocation::new(
            artifact.input,
            q.context,
            definition.id(),
            None,
            [],
        )
        .0;
        drop(docstring);
        drop(placements);
        drop(supports);
        drop(modules);
        drop(symbols);
        (prepare(session, budget).await, expression, invocation)
    }
    #[tokio::test]
    async fn expression_scope_matches_finite_whole_input_and_keeps_competing_builtin_providers() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(32 << 20).unwrap();
        let (prepared, expression, inv) = fixture(&session, &budget, false, true).await;
        let definition = execution::configuration::base_evaluation().1;
        let full = data(&session, &prepared, None, &budget, false).await;
        let (oracle, oracle_owner) = execution::production::evaluate_expression_produced(
            &full.data,
            &full.entry,
            &full.witnesses,
            &full.sources,
            &inv,
            &definition,
            stages::Profile::Behavioral,
            &budget,
            expression.id(),
        )
        .unwrap();
        assert_eq!(oracle.run.evaluated, 1);
        assert_eq!(oracle.run.refused, 0);
        let scope = prepared
            .scope(Kernel::Expression, *expression.id().bytes(), &budget)
            .await
            .unwrap();
        let selected = data(&session, &prepared, Some(&scope), &budget, true).await;
        let (reads, metadata) = prepared.metadata(&session, &inv, &budget).await.unwrap();
        let actual = local_semantics::ProducedLocal::empty(&budget);
        let (output, owner) = execution::production::evaluate_expression_scoped(
            &selected.data,
            &selected.entry,
            &selected.witnesses,
            &selected.sources,
            &inv,
            &definition,
            stages::Profile::Behavioral,
            &budget,
            expression.id(),
            &actual,
            &metadata,
        )
        .unwrap();
        assert!(output.evaluations.same(&oracle.evaluations));
        assert!(output.sources.same(&oracle.sources));
        assert!(output.members.same(&oracle.members));
        assert!(output.operands.same(&oracle.operands));
        assert!(output.boundaries.same(&oracle.boundaries));
        assert_eq!(selected.data.symbols.len(), 2);
        assert_eq!(selected.data.provider_modules.len(), 2);
        drop(output);
        drop(owner);
        drop(actual);
        drop(metadata);
        drop(reads);
        drop(selected);
        drop(scope);
        drop(oracle);
        drop(oracle_owner);
        drop(full);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn expression_metadata_ancestors_exclude_rich_docstring_and_unrelated_same_source_rows() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let (prepared, expression, _) = fixture(&session, &budget, true, false).await;
        let retained = budget.reserved();
        let scope = prepared
            .scope(Kernel::Expression, *expression.id().bytes(), &budget)
            .await
            .unwrap();
        let selected = data(&session, &prepared, Some(&scope), &budget, false).await;
        assert!(selected.data.occurrences.get(expression.id()).is_some());
        assert!(
            selected
                .data
                .occurrences
                .iter()
                .all(|row| row.structural_path.len() < 4)
        );
        assert_eq!(selected.data.literals.len(), 0);
        assert_eq!(selected.data.details.len(), 0);
        drop(selected);
        drop(scope);
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn opaque_string_expression_projects_kind_without_retaining_large_unused_literal_bytes() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let (prepared, expression, inv) = fixture(&session, &budget, true, true).await;
        let retained = budget.reserved();
        let scope = prepared
            .scope(Kernel::Expression, *expression.id().bytes(), &budget)
            .await
            .unwrap();
        let selected = data(&session, &prepared, Some(&scope), &budget, true).await;
        assert_eq!(selected.data.literals.len(), 0);
        assert_eq!(selected.data.details.len(), 1);
        let (reads, metadata) = prepared.metadata(&session, &inv, &budget).await.unwrap();
        let actual = local_semantics::ProducedLocal::empty(&budget);
        let definition = execution::configuration::base_evaluation().1;
        let (output, owner) = execution::production::evaluate_expression_scoped(
            &selected.data,
            &selected.entry,
            &selected.witnesses,
            &selected.sources,
            &inv,
            &definition,
            stages::Profile::Behavioral,
            &budget,
            expression.id(),
            &actual,
            &metadata,
        )
        .unwrap();
        assert_eq!(output.run.evaluated, 1);
        assert_eq!(output.run.refused, 0);
        assert!(
            output
                .evaluations
                .iter()
                .all(|row| row.boolean_value.is_none())
        );
        drop(output);
        drop(owner);
        drop(actual);
        drop(metadata);
        drop(reads);
        drop(selected);
        drop(scope);
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn exact_field_name_kernel_keeps_utf8_bytes_without_decoding_unneeded_receiver_argument()
    {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let (old, expression, inv) = fixture(&session, &budget, true, false).await;
        drop(old);
        let mut rows = crate::sql::query(
            &session,
            &format!("SELECT * FROM {}", identifier(Occurrence::NAME)),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap()
        .iter()
        .flat_map(|batch| <Occurrence as Record>::decode(batch).unwrap())
        .collect::<Vec<_>>();
        let receiver = rows
            .iter()
            .find(|row| row.structural_path.len() > 100)
            .unwrap()
            .id();
        let site = Occurrence {
            syntax_kind: SyntaxKind::ExprCall,
            structural_path: vec![7],
            ..expression.clone()
        };
        rows.push(site.clone());
        register(&session, &rows);
        drop(rows);
        let qualification = crate::sql::query(
            &session,
            &format!("SELECT * FROM {}", identifier(AssertionQualification::NAME)),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap()
        .iter()
        .flat_map(|batch| <AssertionQualification as Record>::decode(batch).unwrap())
        .next()
        .unwrap();
        let call = CallSyntax {
            qualification: qualification.id(),
            site: site.id(),
            callee: expression.id(),
            arguments: ContentHash::of(b"ordered arguments"),
            in_annotation: false,
        };
        let arguments = [
            CallArgument {
                call: call.id(),
                ordinal: 0,
                kind: ArgumentKind::Positional,
                keyword: None,
                value: receiver,
            },
            CallArgument {
                call: call.id(),
                ordinal: 1,
                kind: ArgumentKind::Positional,
                keyword: None,
                value: expression.id(),
            },
        ];
        let literal = value::Literal::String {
            value: "α\0雪".into(),
        };
        let detail = SyntaxDetail::Literal {
            literal: literal.id(),
        };
        register(
            &session,
            &[SyntaxDetailObservation {
                qualification: qualification.id(),
                occurrence: expression.id(),
                ordinal: 0,
                detail: detail.id(),
            }],
        );
        register(&session, &[detail]);
        register(&session, std::slice::from_ref(&literal));
        register(&session, std::slice::from_ref(&call));
        register(&session, &arguments);
        let prepared = prepare(&session, &budget).await;
        let scope = prepared
            .scope(Kernel::FieldCall, *call.id().bytes(), &budget)
            .await
            .unwrap();
        let selected = data(&session, &prepared, Some(&scope), &budget, false).await;
        assert!(selected.data.occurrences.get(receiver).is_none());
        assert_eq!(selected.data.call_arguments.len(), 1);
        assert_eq!(
            selected.data.call_arguments.iter().next().unwrap().ordinal,
            1
        );
        assert_eq!(selected.data.literals.get(literal.id()), Some(&literal));
        let roots = crate::sql::query(&session, &prepared.roots(&inv, Kernel::FieldCall).unwrap())
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(roots.iter().map(|batch| batch.num_rows()).sum::<usize>(), 1);
        drop(selected);
        drop(scope);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn projected_placement_admission_detects_ambiguity_outside_the_expression_grain() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (prepared, _, inv) = fixture(&session, &budget, false, false).await;
        let mut placements = crate::sql::query(
            &session,
            &format!("SELECT * FROM {}", identifier(SyntaxPlacement::NAME)),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap()
        .iter()
        .flat_map(|batch| <SyntaxPlacement as Record>::decode(batch).unwrap())
        .collect::<Vec<_>>();
        let mut duplicate = placements
            .iter()
            .find(|row| row.parent.is_none() && row.ordinal == 1)
            .unwrap()
            .clone();
        duplicate.ordinal = 2;
        placements.push(duplicate);
        register(&session, &placements);
        drop(placements);
        assert!(prepared.metadata(&session, &inv, &budget).await.is_err());
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn empty_frame_has_no_roots_and_releases_prepared_metadata() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        empty(&session);
        let prepared = prepare(&session, &budget).await;
        let definition = execution::configuration::base_evaluation().1;
        let inv = publication::AnalysisInvocation::new(id(1), id(2), definition.id(), None, []).0;
        for kernel in [
            Kernel::Expression,
            Kernel::Use,
            Kernel::Formal,
            Kernel::FieldRoot,
            Kernel::DynamicCall,
        ] {
            let batches = crate::sql::query(&session, &prepared.roots(&inv, kernel).unwrap())
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            assert!(batches.iter().all(|batch| batch.num_rows() == 0));
        }
        let (reads, metadata) = prepared.metadata(&session, &inv, &budget).await.unwrap();
        drop(metadata);
        drop(reads);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
