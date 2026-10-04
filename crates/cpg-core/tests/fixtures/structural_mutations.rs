//! Adversarial rows enter the actual PostgreSQL sink before it creates production receipts.
use cpg_core::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::AttemptRuntime,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, sources::CapturedSources, structural as owner},
    resources::ResourceBudget,
    stages::*,
    structural::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Truthful,
    Strengthen,
    Missing,
    Extra,
    Paired,
    EraseCondition,
}
impl Case {
    pub const ALL: [Self; 6] = [
        Self::Truthful,
        Self::Strengthen,
        Self::Missing,
        Self::Extra,
        Self::Paired,
        Self::EraseCondition,
    ];
}
#[derive(Default)]
pub struct State {
    invocations: BTreeMap<Id<owner::Invocation>, owner::Invocation>,
    forged: BTreeSet<Id<owner::Invocation>>,
    erased: BTreeMap<Id<assertion::AssertionQualification>, Id<assertion::AssertionQualification>>,
    first: Option<Id<owner::Invocation>>,
    extra: Option<Id<owner::Invocation>>,
    subject: Option<Id<normalized::entities::EntityRef>>,
    coverage: BTreeMap<Id<owner::AnalysisCoverage>, Id<owner::AnalysisCoverage>>,
    requirements: BTreeMap<Id<owner::CoverageRequirement>, Id<owner::CoverageRequirement>>,
    pub changed: usize,
    pub changed_coverage: usize,
    pub stops: BTreeMap<&'static str, BTreeSet<Vec<u8>>>,
    pub forwarded_stops: BTreeMap<&'static str, BTreeSet<Vec<u8>>>,
}
pub async fn load_admission(
    access: &StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    admission: &mut CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let reader = AttemptSession::open(
        config,
        attempt,
        access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(access);
    async fn load<R: Record>(
        access: &StageAccess<'_, '_>,
        reader: &AttemptSession,
        session: &cpg_core::model_runtime::StageSession,
        admission: &mut CoverageAdmission<'_>,
    ) -> Result<(), ModelError> {
        let permit = access.read::<R>()?;
        session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        let query = session
            .query(&format!("SELECT * FROM \"{}\"", R::NAME))
            .await
            .map_err(ModelError::codec)?;
        let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            admission.visit_if_expected(&permit, &batch)?;
        }
        Ok(())
    }
    macro_rules! read {($($f:ident:$ty:ty,)*)=>{$(if access.stage().reads::<$ty>() {load::<$ty>(access,&reader,&session,admission).await?;})*};}
    lctx_model::expected_domain_inputs!(read);
    drop(session);
    reader.close().await.map_err(ModelError::codec)
}
pub struct MutatingSink<'a, 's> {
    pub sink: &'a GenerationAttempt,
    pub model: &'a ValidatedModel,
    pub budget: &'a ResourceBudget,
    pub admission: &'a CoverageAdmission<'s>,
    pub definitions: &'a [analysis::AnalysisDefinition],
    pub case: Case,
    pub state: Arc<Mutex<State>>,
}
impl StageSink for MutatingSink<'_, '_> {
    async fn compute(&self, c: StageCompletion) -> Result<ComputedStage, ModelError> {
        self.sink.compute(c).await
    }
    async fn close_group(&self, c: GroupCompletion) -> Result<ClosedGroup, ModelError> {
        self.sink.close_group(c).await
    }
    async fn complete(&self, c: StageCompletion) -> Result<CompletedStage, ModelError> {
        self.sink.complete(c).await
    }
    async fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> Result<(), ModelError> {
        let changed = (|| -> Result<Option<Batch<R>>, ModelError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ModelError::Invalid("mutation observer poisoned".into()))?;
            macro_rules! remember_stop {
                ($ty:ty) => {
                    if R::NAME == <$ty>::NAME {
                        for row in <$ty>::decode(batch.arrow())? {
                            if row.stop.is_some() {
                                state
                                    .stops
                                    .entry(R::NAME)
                                    .or_default()
                                    .insert(row.id().bytes().to_vec());
                            }
                        }
                    }
                };
            }
            remember_stop!(Traversal);
            remember_stop!(controls::ControlTraversal);
            if self.case == Case::Paired && R::NAME.starts_with("structural_") {
                state.changed += batch.rows().len();
                return Ok(Some(Batch::new(self.model, Vec::<R>::new(), self.budget)?));
            }
            macro_rules! change {($ty:ty,$rows:ident,$body:block)=>{if R::NAME==<$ty>::NAME {let mut $rows=<$ty>::decode(batch.arrow())?;$body let encoded=<$ty>::encode(&$rows)?;return Ok(Some(Batch::new(self.model,R::decode(&encoded)?,self.budget)?));}};}
            change!(assertion::AssertionQualification, rows, {
                if self.case == Case::EraseCondition {
                    let added = rows
                        .iter()
                        .filter(|q| q.condition != conditions::Diagram::always().id())
                        .cloned()
                        .map(|mut q| {
                            let original = q.id();
                            q.condition = conditions::Diagram::always().id();
                            state.erased.insert(original, q.id());
                            q
                        })
                        .collect::<Vec<_>>();
                    rows.extend(added);
                    rows.sort_by_key(Record::id);
                    rows.dedup();
                }
            });
            change!(controls::ArgumentFlow, rows, {
                if self.case == Case::EraseCondition {
                    for row in &mut rows {
                        if row.conditional {
                            row.qualification = *state.erased.get(&row.qualification).expect(
                                "derived qualification vocabulary precedes its structural rows",
                            );
                            row.conditional = false;
                            state.changed += 1;
                        }
                    }
                }
            });
            if R::NAME == PublicCandidate::NAME && state.subject.is_none() {
                state.subject = PublicCandidate::decode(batch.arrow())?
                    .first()
                    .map(|r| r.entity);
            }
            change!(owner::Invocation, rows, {
                for row in &rows {
                    state.invocations.insert(row.id(), row.clone());
                }
                if self.case == Case::Extra && state.extra.is_none() {
                    let mut extra = rows.first().expect("native frame has invocations").clone();
                    state.first = Some(extra.id());
                    extra.subject = Some(state.subject.expect("native public candidate exists"));
                    state.extra = Some(extra.id());
                    rows.push(extra);
                }
            });
            change!(owner::AnalysisOutcome, rows, {
                match self.case {
                    Case::Strengthen => {
                        for row in &mut rows {
                            if row.status == analysis::AnalysisStatus::Partial
                                && row.reason == Some(obligation::ObligationKind::BudgetReached)
                            {
                                state.forged.insert(row.invocation);
                                row.status = analysis::AnalysisStatus::Completed;
                                row.reason = None;
                                state.changed += 1;
                            }
                        }
                    }
                    Case::Missing => {
                        if state.changed == 0 && !rows.is_empty() {
                            rows.remove(0);
                            state.changed += 1;
                        }
                    }
                    Case::Extra => {
                        if let Some(extra) = state.extra {
                            let mut row = rows
                                .iter()
                                .find(|r| Some(r.invocation) == state.first)
                                .expect("original outcome")
                                .clone();
                            row.invocation = extra;
                            rows.push(row);
                            state.changed += 1;
                        }
                    }
                    _ => {}
                }
            });
            change!(owner::AnalysisCoverage, rows, {
                if self.case == Case::Strengthen {
                    for row in &mut rows {
                        if state.forged.contains(&row.invocation) {
                            let invocation = state
                                .invocations
                                .get(&row.invocation)
                                .expect("declared native invocation precedes coverage");
                            let definition = self
                                .definitions
                                .iter()
                                .find(|d| d.id() == invocation.definition)
                                .expect("canonical definition");
                            let capability = outcomes::capability(definition.method)?;
                            let domain = owner::coverage::admit(
                                invocation,
                                definition,
                                capability,
                                self.admission,
                                self.budget,
                            )?;
                            let scope = domain
                                .scopes()
                                .iter()
                                .find(|s| s.expectation().scope == row.scope)
                                .expect("admitted native scope");
                            let (forged, _) = owner::coverage::assess(
                                scope.expectation(),
                                scope.observations(),
                                analysis::AnalysisStatus::Completed,
                                None,
                                self.budget,
                            )?;
                            assert_eq!(forged.id(), row.id());
                            *row = forged;
                            state.changed_coverage += 1;
                        }
                    }
                }
                if self.case == Case::Extra {
                    let first = state.first;
                    let extra = state.extra.unwrap();
                    let added = rows
                        .iter()
                        .filter(|r| Some(r.invocation) == first)
                        .cloned()
                        .map(|mut r| {
                            let old = r.id();
                            r.invocation = extra;
                            state.coverage.insert(old, r.id());
                            r
                        })
                        .collect::<Vec<_>>();
                    rows.extend(added);
                }
            });
            macro_rules! extra_links {
                ($ty:ty,$field:ident) => {
                    change!($ty, rows, {
                        if self.case == Case::Extra {
                            let added = rows
                                .iter()
                                .filter(|r| Some(r.$field) == state.first)
                                .cloned()
                                .map(|mut r| {
                                    r.$field = state.extra.unwrap();
                                    r
                                })
                                .collect::<Vec<_>>();
                            rows.extend(added);
                        }
                    });
                };
            }
            extra_links!(owner::AnalysisInput, invocation);
            extra_links!(owner::SourceReceipt, invocation);
            extra_links!(owner::ProjectionInput, invocation);
            change!(owner::CoverageRequirement, rows, {
                if self.case == Case::Extra {
                    let first = state.first;
                    let extra = state.extra.unwrap();
                    let added = rows
                        .iter()
                        .filter(|r| Some(r.invocation) == first)
                        .cloned()
                        .map(|mut r| {
                            let old = r.id();
                            r.invocation = extra;
                            state.requirements.insert(old, r.id());
                            r
                        })
                        .collect::<Vec<_>>();
                    rows.extend(added);
                }
            });
            change!(owner::CoverageRequiredSource, rows, {
                if self.case == Case::Extra {
                    let added = rows
                        .iter()
                        .filter_map(|r| {
                            state.requirements.get(&r.requirement).map(|id| {
                                let mut r = r.clone();
                                r.requirement = *id;
                                r
                            })
                        })
                        .collect::<Vec<_>>();
                    rows.extend(added);
                }
            });
            change!(owner::AnalysisCoveragePremise, rows, {
                if self.case == Case::Extra {
                    let added = rows
                        .iter()
                        .filter_map(|r| {
                            state.coverage.get(&r.coverage).map(|id| {
                                let mut r = r.clone();
                                r.coverage = *id;
                                r
                            })
                        })
                        .collect::<Vec<_>>();
                    rows.extend(added);
                }
            });
            Ok(None)
        })()?;
        let forwarded = changed.as_ref().unwrap_or(batch);
        {
            let mut state = self.state.lock()
                .map_err(|_| ModelError::Invalid("mutation observer poisoned".into()))?;
            macro_rules! record_forwarded {
                ($ty:ty) => {
                    if R::NAME == <$ty>::NAME {
                        for row in <$ty>::decode(forwarded.arrow())? {
                            if row.stop.is_some() {
                                state.forwarded_stops.entry(R::NAME).or_default()
                                    .insert(row.id().bytes().to_vec());
                            }
                        }
                    }
                };
            }
            record_forwarded!(Traversal);
            record_forwarded!(controls::ControlTraversal);
        }
        self.sink.copy(permit, forwarded).await
    }
}
pub fn capture(
    access: &StageAccess<'_, '_>,
    budget: &ResourceBudget,
) -> Result<CapturedSources, ModelError> {
    CapturedSources::capture(access, budget)
}
