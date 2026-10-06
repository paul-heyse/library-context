//! Necessary Flow inventory admission over exact owner grains; no producer is rerun.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, identifier},
    scoped_admission::{column, declared, field_target, root_predicate},
    workspace::Cancellation,
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{flow_inventory::InventoryScope, resources::ResourceBudget, *};

fn plan(tables: &[ClosureTable], inputs: &[ValidationInput], scope: &InventoryScope) -> Result<NominalClosure, ModelError> {
    let mut plan = NominalClosure::new(tables.to_vec())?;
    for (source, table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            let Some((target, _)) = field.target() else { continue; };
            let Some(target) = field_target(inputs, source, target)? else { continue; };
            if field.list() {
                plan.pairs(source, target, format!("SELECT id AS source_id, UNNEST({}) AS target_id FROM {}", identifier(field.name()), identifier(&table.alias)))?;
            } else { plan.follow(source, field.name(), target)?; }
        }
    }
    for (member, field, owner) in &scope.memberships {
        plan.own(declared(tables, inputs, member)?, field, declared(tables, inputs, owner)?)?;
    }
    let [observation, use_, occurrence, use_support, view_support, view] = &scope.native_views;
    let observation = declared(tables, inputs, observation)?;
    let view = declared(tables, inputs, view)?;
    let alias = |input: &ValidationInput| -> Result<String, ModelError> {
        Ok(identifier(&tables[declared(tables, inputs, input)?].alias))
    };
    // Native enumeration is an exact source/run/surface correspondence. Selecting every view of
    // the same input would hydrate unrelated provider runs and make missing-inventory ambiguous.
    plan.pairs(observation, view, format!(
        "SELECT o.id AS source_id,v.id AS target_id FROM {} AS o \
         JOIN {} AS u ON u.id=o.use_ JOIN {} AS site ON site.id=u.occurrence \
         JOIN {} AS us ON us.assertion=o.id JOIN {} AS vs ON vs.run=us.run AND vs.surface=us.surface \
         JOIN {} AS v ON v.id=vs.assertion AND v.source=site.source",
        identifier(&tables[observation].alias), alias(use_)?, alias(occurrence)?, alias(use_support)?,
        alias(view_support)?, identifier(&tables[view].alias)))?;
    Ok(plan)
}

pub(crate) async fn validate_inventory(
    invariant: &Invariant, scope: &InventoryScope, tables: Vec<ClosureTable>,
    session: &SessionContext, budget: &ResourceBudget, cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let prepared = plan(&tables, &invariant.inputs, scope)?.prepare(session, budget).await?;
    // Each owner may have up to the model's finite candidate bound. One root keeps rich state
    // bounded to that owner's actual closure, and the fresh checker is dropped before the next.
    let _root_charge = budget.reserve("flow-inventory-admission-root", 128)?;
    for root in &scope.roots {
        let root = declared(&tables, &invariant.inputs, root)?;
        let sql = format!("SELECT id FROM {} ORDER BY id", identifier(&tables[root].alias));
        let mut roots = crate::sql::query(session, &sql).await.map_err(ModelError::codec)?
            .execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            for row in 0..batch.num_rows() {
                let id = column(&batch, "id", row)?.ok_or(ModelError::Schema("flow inventory root ID"))?;
                let closure = prepared.grain(root, &root_predicate(&[id]), budget).await?;
                let mut check = (invariant.create)(budget);
                for (index, input) in invariant.inputs.iter().enumerate() {
                    let order = input.order().iter().map(|field| identifier(field)).collect::<Vec<_>>().join(",");
                    let sql = format!("SELECT * FROM ({}) AS owner_rows{}", closure.select(index)?,
                        if order.is_empty() { String::new() } else { format!(" ORDER BY {order}") });
                    let mut rows = crate::sql::query(closure.session(), &sql).await.map_err(ModelError::codec)?
                        .execute_stream().await.map_err(ModelError::codec)?;
                    while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                        cancellation.check()?;
                        check.visit_input(input, &batch)?;
                    }
                }
                check.finish()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionConfig};
    use lctx_model::domain::{assertion::{Approximation, AssertionQualification}, attribution::{ExtractionMode, Fidelity, Modality, Origin, ProviderRun}, conditions::{Condition, ConditionNode}, flow::*, flow_inventory::*, source::{Occurrence, OccurrenceRole, SyntaxKind}};
    use std::{any::TypeId, sync::Arc};
    fn nominal<R>(byte: u8) -> Id<R> { serde_json::from_value(serde_json::to_value([byte;16]).unwrap()).unwrap() }
    struct Fixture {
        invariant: Invariant, scope: InventoryScope, tables: Vec<ClosureTable>, session: SessionContext,
        budget: ResourceBudget, inventory: FlowUseInventoryObservation, candidates: Vec<FlowUseCandidate>,
    }
    impl Fixture {
        fn put<R: Record>(&self, rows: &[R]) {
            let table = self.tables.iter().find(|table| table.relation.type_id()==TypeId::of::<R>()).unwrap();
            self.session.deregister_table(table.alias.as_str()).unwrap();
            let batch = R::encode(rows).unwrap();
            self.session.register_table(table.alias.as_str(), Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();
        }
        fn new() -> Self {
            let model = lctx_model::domain::model().unwrap();
            let invariant = model.invariant("flow_use_inventory_admission").unwrap().clone();
            assert_eq!(invariant.purpose, InvariantPurpose::Admission);
            let budget = ResourceBudget::fixed(64<<20).unwrap();
            let scope = (invariant.create)(&budget).inventory_scope().unwrap();
            let session = SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            let tables: Vec<_> = invariant.inputs.iter().enumerate().map(|(index,input)| ClosureTable {
                relation:model.relation(input.name()).unwrap().clone(), alias:format!("inventory_input_{index}"),
            }).collect();
            for table in &tables {
                let batch = arrow_array::RecordBatch::new_empty(table.relation.schema().clone());
                session.register_table(table.alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();
            }
            let occurrence = Occurrence { source:nominal(1),start:0,end:1,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Syntax,structural_path:vec![] };
            let use_ = FlowUse { occurrence:occurrence.id(),place:nominal(2) };
            let condition = Condition { root:ConditionNode::True.id() };
            let qualification = AssertionQualification { context:nominal(3),scope:nominal(4),condition:condition.id(),modality:Modality::Definite,approximation:Approximation::Exact,assumptions:nominal(5) };
            let observation = FlowUseObservation { qualification:qualification.id(),use_:use_.id(),scope:nominal(6),annotation:false };
            let run = ProviderRun { provider:nominal(7),context:qualification.context,input:nominal(8),configuration:ContentHash::of(b"config"),requested_families:ContentHash::of(b"families") };
            let support = FlowUseSupport { assertion:observation.id(),run:run.id(),surface:nominal(9),evidence:nominal(10),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural };
            let view = FlowSourceViewObservation { qualification:qualification.id(),source:occurrence.source,original_content:ContentHash::of(b"x"),view_content:ContentHash::of(b"x"),byte_len:1,renamed_type_checking:0 };
            let view_support = FlowSourceViewSupport { assertion:view.id(),run:run.id(),surface:support.surface,evidence:support.evidence,origin:support.origin,mode:support.mode,fidelity:support.fidelity };
            let state = CandidateState { kind:FlowCandidateKind::Undefined,pruned:true,loop_expanded:false,unattached:false,reachability:None,narrowing:None,narrowing_unavailable:true,narrowing_precision_lost:false,condition_unavailable:true,reachability_lost:false,mapped_count:0 };
            let (inventory,candidates,_) = FlowUseInventoryObservation::new(qualification.id(),use_.id(),observation.scope,view.id(),&[state.clone(),state],&[]).unwrap();
            let inventory_support = FlowUseInventorySupport { assertion:inventory.id(),run:run.id(),surface:support.surface,evidence:support.evidence,origin:support.origin,mode:support.mode,fidelity:support.fidelity };
            let fixture = Self { invariant,scope,tables,session,budget,inventory,candidates };
            fixture.put(&[occurrence]);fixture.put(&[use_]);fixture.put(&[qualification]);fixture.put(&[observation]);
            fixture.put(&[run]);fixture.put(&[support]);fixture.put(&[view]);fixture.put(&[view_support]);
            fixture.put(&[ConditionNode::True]);fixture.put(&[condition]);
            fixture.put(&[fixture.inventory.clone()]);fixture.put(&fixture.candidates);fixture.put(&[inventory_support]);
            fixture
        }
        async fn validate(&self) -> Result<(),ModelError> {
            validate_inventory(&self.invariant,&self.scope,self.tables.clone(),&self.session,&self.budget,&Cancellation::default()).await
        }
    }
    #[tokio::test]
    async fn necessary_inventory_accepts_order_and_releases_each_owner_grain() {
        let fixture = Fixture::new();
        let mut candidates = fixture.candidates.clone(); candidates.reverse();fixture.put(&candidates);
        fixture.validate().await.unwrap();assert_eq!(fixture.budget.reserved(),0);
        fixture.validate().await.unwrap();assert_eq!(fixture.budget.reserved(),0);
    }
    #[tokio::test]
    async fn absent_inventory_and_absent_candidate_cannot_escape_roots() {
        let fixture = Fixture::new();
        fixture.put::<FlowUseCandidate>(&[]);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("count/digest"));
        fixture.put::<FlowUseInventoryObservation>(&[]);fixture.put::<FlowUseInventorySupport>(&[]);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("missing its enumeration inventory"));
        assert_eq!(fixture.budget.reserved(),0);
    }
    #[tokio::test]
    async fn duplicate_ordinal_and_wrong_digest_are_rejected() {
        let fixture = Fixture::new();
        let mut candidates = fixture.candidates.clone(); candidates.push(candidates[0].clone());fixture.put(&candidates);
        assert!(matches!(fixture.validate().await,Err(ModelError::Conflict(_))));
        candidates = fixture.candidates.clone();candidates[0].pruned=false;fixture.put(&candidates);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("count/digest"));
        candidates = fixture.candidates.clone();candidates[1].ordinal=2;fixture.put(&candidates);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("missing ordinal"));
        assert_eq!(fixture.budget.reserved(),0);
    }
    #[tokio::test]
    async fn foreign_member_run_and_absent_owner_support_are_rejected() {
        let fixture = Fixture::new();
        let reaching = FlowReachingObservation { qualification:fixture.inventory.qualification,use_:fixture.inventory.use_,target:ReachingDefinition::Unbound.id(),loop_carried:false };
        let support = FlowReachingSupport { assertion:reaching.id(),run:nominal(90),surface:nominal(9),evidence:nominal(10),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural };
        fixture.put(&[ReachingDefinition::Unbound]);fixture.put(&[reaching.clone()]);fixture.put(&[support.clone()]);
        fixture.put(&[FlowUseInventoryMember { inventory:fixture.inventory.id(),ordinal:0,reaching:reaching.id(),support:support.id() }]);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("foreign inventory member"));
        fixture.put::<FlowUseInventoryMember>(&[]);fixture.put::<FlowReachingSupport>(&[]);fixture.put::<FlowReachingObservation>(&[]);
        fixture.put::<FlowUseInventorySupport>(&[]);
        assert!(fixture.validate().await.unwrap_err().to_string().contains("exact provider support"));
        assert_eq!(fixture.budget.reserved(),0);
    }
}
