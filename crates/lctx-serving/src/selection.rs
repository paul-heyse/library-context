//! Existing finite requirement and same-context algebra over complete requested semantic domains.
use lctx_model::domain::{
    self, HeapSize, Id, ModelError, Record, ValidationInput, catalog,
    charged::{ChargedSet, ChargedVec, StateCharge},
    resources::ResourceBudget,
    selection,
    serving::*,
};
use lctx_native::{
    NativeReader,
    reader::{NativeRows, target_id},
};
use surrealdb::types::Variables;

/// The complete hydrated domain travels with its heap and vector-capacity reservation.
/// Consumers borrow rows without separating the retained data from its budget owner.
#[derive(Debug)]
pub struct Members {
    rows: ChargedVec<catalog::CatalogMember>,
    charge: StateCharge,
}
impl std::ops::Deref for Members {
    type Target = [catalog::CatalogMember];
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}
impl Members {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: ChargedVec::default(),
            charge: StateCharge::new(budget, "native-retained-members"),
        }
    }
    fn push(&mut self, member: catalog::CatalogMember) -> Result<(), ModelError> {
        self.rows.push(&mut self.charge, member)
    }
    pub fn as_slice(&self) -> &[catalog::CatalogMember] {
        &self.rows
    }
}

struct ClassificationRequest {
    roots: Vec<surrealdb::types::RecordId>,
    requested: ChargedSet<Id<catalog::CatalogMember>>,
    root_charge: StateCharge,
    _requested_charge: StateCharge,
}
impl ClassificationRequest {
    fn new(members: &Members, budget: &ResourceBudget) -> Result<Self, ModelError> {
        if !members
            .charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            return Err(ModelError::Conflict("native member request budget"));
        }
        let mut roots = Vec::new();
        let mut root_charge = StateCharge::new(budget, "native-classification-roots");
        let mut requested = ChargedSet::default();
        let mut requested_charge = StateCharge::new(budget, "native-classification-requested");
        for member in members.iter() {
            let additional = if roots.len() == roots.capacity() {
                roots.capacity().max(4)
            } else {
                0
            };
            // These generated roots have the fixed entity table and full 64-character
            // graph digest string. Admit vector growth and those owned strings first.
            root_charge.grow(
                additional
                    .saturating_mul(size_of::<surrealdb::types::RecordId>())
                    .saturating_add(128),
            )?;
            requested.insert(&mut requested_charge, member.id())?;
            roots.reserve_exact(additional);
            roots.push(target_id(domain::graph::Target::Entity(
                domain::graph::EntityId::of(member.id()),
            )));
        }
        Ok(Self {
            roots,
            requested,
            root_charge,
            _requested_charge: requested_charge,
        })
    }
    fn take_roots(&mut self) -> Vec<surrealdb::types::RecordId> {
        std::mem::take(&mut self.roots)
    }
    fn release_roots(&mut self) {
        self.root_charge.release(self.root_charge.reserved());
    }
}

pub struct MemberSelection {
    pub prepared: PreparedSelection,
    pub selected: selection::evaluate::Selected,
}
/// The shared interpretation retains its viewer/pin lease through every request borrow.
pub struct PreparedSelection(PreparedSelectionOwner);
enum PreparedSelectionOwner {
    Fresh(selection::evaluate::Prepared),
    Cached(crate::preparation::PreparedClassification),
}
impl std::ops::Deref for PreparedSelection {
    type Target = selection::evaluate::Prepared;
    fn deref(&self) -> &Self::Target {
        match &self.0 {
            PreparedSelectionOwner::Fresh(value) => value,
            PreparedSelectionOwner::Cached(value) => value,
        }
    }
}
pub(crate) fn interpret(
    batches: &lctx_native::batches::CanonicalBatches,
    budget: &ResourceBudget,
) -> Result<selection::evaluate::Prepared, ModelError> {
    let mut data = selection::classification::ClassificationData::new(budget);
    let mut output = selection::build::Output::new(budget);
    for (name, batch) in &batches.batches {
        let input = data.visit(name, batch)?;
        let result = output.visit(name, batch)?;
        if !input && !result {
            return Err(ModelError::Schema("undeclared native classification input"));
        }
    }
    selection::evaluate::Prepared::from_local_rows(data, output, budget)
}
pub async fn classify(
    reader: &NativeReader,
    members: &Members,
    selection: &selection::Selection,
    budget: &ResourceBudget,
) -> Result<MemberSelection, ModelError> {
    classify_inner(reader, members, selection, budget, None).await
}
pub(crate) async fn classify_prepared(
    reader: &NativeReader,
    members: &Members,
    selection: &selection::Selection,
    budget: &ResourceBudget,
    preparation: &crate::preparation::Preparation<'_>,
) -> Result<MemberSelection, ModelError> {
    classify_inner(reader, members, selection, budget, Some(preparation)).await
}
async fn classify_inner(
    reader: &NativeReader,
    members: &Members,
    selection: &selection::Selection,
    budget: &ResourceBudget,
    preparation: Option<&crate::preparation::Preparation<'_>>,
) -> Result<MemberSelection, ModelError> {
    let mut inputs = selection::classification::ClassificationData::inputs();
    inputs.extend(selection::build::Output::inputs());
    inputs.sort_by_key(ValidationInput::name);
    inputs.dedup_by_key(|i| i.name());
    let mut request = ClassificationRequest::new(members, budget)?;
    let prepared = if let Some(preparation) = preparation {
        let cached = preparation
            .hydrate(
                reader,
                request.take_roots(),
                &inputs,
                &inputs,
                crate::scope::OWNED_FIELDS,
            )
            .await?;
        request.release_roots();
        PreparedSelection(PreparedSelectionOwner::Cached(
            preparation.classification(&cached).await?,
        ))
    } else {
        let fresh = crate::scope::hydrate(reader, request.take_roots(), &inputs, budget).await?;
        request.release_roots();
        PreparedSelection(PreparedSelectionOwner::Fresh(interpret(&fresh, budget)?))
    };
    let mut selected = prepared.select(selection, budget)?;
    selected
        .candidates
        .retain(|c| request.requested.contains(&c.member));
    Ok(MemberSelection { prepared, selected })
}

/// Exact native root selection uses normalized capture identity and canonical member paths.
/// A path constraint is resolved against the module spelling and the member's own path.
pub async fn members<Context>(
    reader: &NativeReader<Context>,
    library: Option<&Name>,
    operation: Option<&OperationSelector>,
    budget: &ResourceBudget,
) -> Result<Members, ModelError> {
    let mut vars = Variables::new();
    vars.insert("library", library.map(|s| s.as_str().to_owned()));
    vars.insert(
        "member",
        operation.and_then(|s| match s {
            OperationSelector::Member { member } => Some(member.hex()),
            _ => None,
        }),
    );
    vars.insert(
        "path",
        operation.and_then(|s| match s {
            OperationSelector::PublicPath { path } => Some(
                path.iter()
                    .map(|p| p.as_str().to_owned())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        }),
    );
    let mut sorted = lctx_native::acknowledged_candidates::AsyncCandidateSort::new(budget).await?;
    let discovery = async {
        let packages = discovery_rows(reader, library_packages_sql(), vars.clone(), "entity")?;
        collect_library_candidates(reader, &mut vars, packages, &mut sorted).await
    }
    .await;
    if let Err(error) = discovery {
        let mut completion = domain::completion::Completion::default();
        completion.step(
            "native member candidate sorting drain",
            sorted.drain().await,
        );
        return domain::completion::complete(Err(error), completion);
    }
    let ordered_result = sorted.finish().await;
    let mut ordered = match ordered_result {
        Ok(ordered) => ordered,
        Err(error) => {
            let mut completion = domain::completion::Completion::default();
            completion.step(
                "native member candidate sorting drain",
                sorted.drain().await,
            );
            return domain::completion::complete(Err(error), completion);
        }
    };
    let result = async {
        let mut members = Members::new(budget);
        loop {
            let candidates = ordered.next_batch(128).await?;
            if candidates.is_empty() {
                break;
            }
            let window = reader
                .records_from_candidates::<catalog::CatalogMember>(&candidates)
                .await?;
            let mut transfer = StateCharge::new(budget, "native-member-hydration-window");
            transfer.admit(&window)?;
            for member in window {
                // Moved strings become the retained owner's responsibility.
                transfer.release(member.heap_bytes());
                members.push(member)?;
            }
        }
        Ok(members)
    }
    .await;
    let mut completion = domain::completion::Completion::default();
    completion.step("native ordered members drain", ordered.drain().await);
    domain::completion::complete(result, completion)
}

/// These branches stream compact discovery rows without union, distinct or ordering state.
/// Package spelling is residual policy; physical endpoints drive both reverse branches.
fn library_packages_sql() -> &'static str {
    "SELECT id FROM entity WITH INDEX semantic_key WHERE semantic_type='packages' AND ($library=NONE OR $library=NULL OR body.name=$library);"
}
fn library_releases_sql() -> &'static str {
    "SELECT in AS id FROM reference WITH INDEX incoming WHERE derivation=$lctx_derivation AND out=$package_anchor AND field='package' AND in.semantic_type='releases';"
}
fn library_inputs_sql() -> &'static str {
    "SELECT in AS id FROM participant WITH INDEX incoming WHERE derivation=$lctx_derivation AND out=$release_anchor AND field='release' AND in.semantic_type='input_distributions' AND in.body.role=0;"
}

/// Discovery SQL only nominates physical identities. The native owner intersects
/// every nomination with the protected exact inventory before payload hydration.
fn discovery_rows<Context>(
    reader: &NativeReader<Context>,
    sql: &str,
    vars: Variables,
    table: &str,
) -> Result<NativeRows, ModelError> {
    reader.derivation()?;
    reader.candidate_payload_rows(
        lctx_native::prepared::PreparedQuery::new(vars, vec![], vec![sql.into()])?
            .with_bindings(reader.view_bindings()),
        table,
        "semantic_key",
    )
}

async fn settle_discovery<T>(
    rows: &mut NativeRows,
    result: Result<T, ModelError>,
) -> Result<T, ModelError> {
    let mut completion = domain::completion::Completion::default();
    completion.step(
        "native member discovery stream drain",
        rows.drain_transport().await,
    );
    domain::completion::complete(result, completion)
}

async fn collect_library_candidates<Context>(
    reader: &NativeReader<Context>,
    vars: &mut Variables,
    mut packages: NativeRows,
    sorted: &mut lctx_native::acknowledged_candidates::AsyncCandidateSort,
) -> Result<(), ModelError> {
    let result = async {
        while let Some(package) = packages.next().await? {
            let surrealdb::types::Value::Object(package) = package else {
                return Err(ModelError::Schema("native library package pointer"));
            };
            vars.insert(
                "package_anchor",
                package
                    .get("anchor")
                    .cloned()
                    .ok_or(ModelError::Schema("native package anchor"))?,
            );
            let mut releases =
                discovery_rows(reader, library_releases_sql(), vars.clone(), "entity")?;
            let result = async {
                while let Some(release) = releases.next().await? {
                    let surrealdb::types::Value::Object(release) = release else {
                        return Err(ModelError::Schema("native library release pointer"));
                    };
                    vars.insert(
                        "release_anchor",
                        release
                            .get("anchor")
                            .cloned()
                            .ok_or(ModelError::Schema("native release anchor"))?,
                    );
                    let mut inputs =
                        discovery_rows(reader, library_inputs_sql(), vars.clone(), "assertion")?;
                    let result = async {
                        while let Some(input) = inputs.next().await? {
                            let input = input
                                .as_object()
                                .and_then(|row| row.get("body"))
                                .and_then(surrealdb::types::Value::as_object)
                                .and_then(|body| body.get("input"))
                                .ok_or(ModelError::Schema("native input distribution input"))?;
                            vars.insert(
                                "scope",
                                format!(
                                    "catalog_members|input|{}",
                                    lctx_native::prepared::scope_string(&input)
                                ),
                            );
                            let mut rows = discovery_rows(
                                reader,
                                member_candidates_sql(),
                                vars.clone(),
                                "entity",
                            )?;
                            let result = async {
                                while let Some(value) = rows.next().await? {
                                    let surrealdb::types::Value::Object(row) = value else {
                                        return Err(ModelError::Schema("native member candidate"));
                                    };
                                    let Some(surrealdb::types::Value::String(key)) =
                                        row.get("semantic_key")
                                    else {
                                        return Err(ModelError::Schema(
                                            "native member nominal identity",
                                        ));
                                    };
                                    let Some(surrealdb::types::Value::RecordId(node)) =
                                        row.get("id")
                                    else {
                                        return Err(ModelError::Schema(
                                            "native member backing pointer",
                                        ));
                                    };
                                    let key: [u8; 16] = hex::decode(key)
                                        .map_err(ModelError::codec)?
                                        .try_into()
                                        .map_err(|_| {
                                            ModelError::Schema("native member key width")
                                        })?;
                                    if let Some(surrealdb::types::Value::Array(path)) =
                                        vars.get("path")
                                    {
                                        let Some(surrealdb::types::Value::Bytes(canonical)) =
                                            row.get("canonical")
                                        else {
                                            return Err(ModelError::Schema(
                                                "native member canonical",
                                            ));
                                        };
                                        let _decode = reader.resource_budget()?.reserve(
                                            "native member path decode",
                                            canonical.len().saturating_mul(8).saturating_add(4096),
                                        )?;
                                        let domain::graph::Entity::CatalogMember(member) =
                                            serde_json::from_slice(canonical)
                                                .map_err(ModelError::codec)?
                                        else {
                                            return Err(ModelError::Schema(
                                                "native member canonical kind",
                                            ));
                                        };
                                        let modules = reader
                                            .records::<domain::source::Module>(
                                                lctx_native::reader::RecordSelection::Keys(vec![
                                                    *member.access.bytes(),
                                                ]),
                                            )
                                            .await?;
                                        let matches = modules.iter().any(|module| {
                                            module
                                                .qualified_name
                                                .split('.')
                                                .map(str::to_owned)
                                                .chain(member.path.iter().cloned())
                                                .map(surrealdb::types::Value::String)
                                                .collect::<Vec<_>>()
                                                == path.as_slice()
                                        });
                                        if !matches {
                                            continue;
                                        }
                                    }
                                    sorted
                                        .push(lctx_native::ordered_rows::Candidate {
                                            relation: catalog::CatalogMember::NAME.into(),
                                            key,
                                            node: node.clone(),
                                            content: None,
                                        })
                                        .await?;
                                }
                                Ok(())
                            }
                            .await;
                            settle_discovery(&mut rows, result).await?;
                        }
                        Ok(())
                    }
                    .await;
                    settle_discovery(&mut inputs, result).await?;
                }
                Ok(())
            }
            .await;
            settle_discovery(&mut releases, result).await?;
        }
        Ok(())
    }
    .await;
    settle_discovery(&mut packages, result).await
}

/// Candidate extraction has one indexed equality branch and no native union/order state.
/// The path predicate is owned by the finite member operation, never request SQL.
pub fn member_candidates_sql() -> &'static str {
    "SELECT id FROM entity WITH INDEX by_scope WHERE semantic_type='catalog_members' AND scope_keys CONTAINS $scope AND ($member=NONE OR $member=NULL OR semantic_key=$member);"
}

#[cfg(test)]
mod retained_member_tests {
    use super::*;
    fn id<R: Record>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    fn member(index: usize) -> catalog::CatalogMember {
        let path = vec![
            format!("module_{index:04}_{}", "p".repeat(80)),
            "operation".into(),
        ];
        let member = catalog::CatalogMember {
            input: id(1),
            access: id(2),
            name: path.join("."),
            path,
        };
        member.validate().unwrap();
        member
    }
    #[test]
    fn retained_members_charge_complete_heap_and_keep_order_through_consumer_lifetime() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut rows = Members::new(&budget);
        let expected = (0..300).rev().map(member).collect::<Vec<_>>();
        for row in &expected {
            rows.push(row.clone()).unwrap();
        }
        assert_eq!(rows.as_slice(), expected.as_slice());
        assert_eq!(rows.len(), 300);
        let retained = budget.reserved();
        assert!(
            retained
                >= expected
                    .iter()
                    .map(|row| size_of::<catalog::CatalogMember>() + row.heap_bytes())
                    .sum::<usize>()
        );
        fn consumer(rows: Members) -> Members {
            assert_eq!(rows.len(), 300);
            rows
        }
        let rows = consumer(rows);
        assert_eq!(budget.reserved(), retained);
        assert_eq!(rows.as_slice(), expected.as_slice());
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn classification_roots_and_requested_set_retain_their_own_budget_until_consumed() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut rows = Members::new(&budget);
        for index in 0..180 {
            rows.push(member(index)).unwrap();
        }
        let member_bytes = budget.reserved();
        let mut request = ClassificationRequest::new(&rows, &budget).unwrap();
        assert_eq!(request.roots.len(), 180);
        assert_eq!(request.requested.len(), 180);
        assert!(rows.iter().all(|row| request.requested.contains(&row.id())));
        assert!(request.root_charge.reserved() > 0);
        assert!(request._requested_charge.reserved() > 0);
        let roots = request.take_roots();
        assert_eq!(roots.len(), 180);
        assert!(request.root_charge.reserved() > 0);
        drop(roots);
        request.release_roots();
        assert_eq!(request.root_charge.reserved(), 0);
        assert!(budget.reserved() > member_bytes);
        drop(request);
        assert_eq!(budget.reserved(), member_bytes);
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn retained_member_and_classification_admission_refuse_before_uncharged_growth() {
        let tiny = ResourceBudget::fixed(256).unwrap();
        let mut rows = Members::new(&tiny);
        assert!(matches!(
            rows.push(member(0)),
            Err(ModelError::Resource {
                owner: "native-retained-members",
                ..
            })
        ));
        assert!(rows.is_empty());
        assert_eq!(tiny.reserved(), 0);
        let budget = ResourceBudget::fixed(8192).unwrap();
        let mut rows = Members::new(&budget);
        rows.push(member(0)).unwrap();
        let member_bytes = budget.reserved();
        let occupied = budget
            .reserve("competing-request-work", budget.limit() - member_bytes - 1)
            .unwrap();
        assert!(matches!(
            ClassificationRequest::new(&rows, &budget),
            Err(ModelError::Resource {
                owner: "native-classification-roots",
                ..
            })
        ));
        assert_eq!(budget.reserved(), member_bytes + occupied.size());
        drop(occupied);
        assert!(matches!(
            ClassificationRequest::new(&rows, &ResourceBudget::fixed(8192).unwrap()),
            Err(ModelError::Conflict("native member request budget"))
        ));
        assert_eq!(budget.reserved(), member_bytes);
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
}

#[cfg(test)]
mod streamed_library_tests {
    use super::*;
    use domain::{
        ContentHash,
        graph::{Assertion, Entity},
        input::{DistributionRole, InputDistribution, InputRevision, Package, Release},
        source::{Module, SourceArtifact},
    };
    use lctx_native::reader;

    #[tokio::test]
    async fn native_library_discovery_preserves_complete_eligibility_and_drains_late_upstream_failure()
     {
        let config = crate::scoped_fixture::config();
        let package = Package {
            name: "discovery-selected".into(),
        };
        let other = Package {
            name: "discovery-other".into(),
        };
        let first = Release {
            package: package.id(),
            version: "1".into(),
        };
        let second = Release {
            package: package.id(),
            version: "2".into(),
        };
        let foreign = Release {
            package: other.id(),
            version: "1".into(),
        };
        let mut entities = vec![
            Entity::from(package.clone()),
            Entity::from(other),
            Entity::from(first.clone()),
            Entity::from(second.clone()),
            Entity::from(foreign.clone()),
        ];
        let mut inputs = Vec::new();
        let mut members = Vec::new();
        for name in ["first", "second", "dependency", "foreign", "orphan"] {
            let input = InputRevision {
                manifest: ContentHash::of(name.as_bytes()),
            };
            let source = SourceArtifact::from_bytes(input.id(), format!("{name}.py"), b"").unwrap();
            let module = Module {
                source: source.id(),
                qualified_name: "discovery".into(),
            };
            let member = catalog::CatalogMember {
                input: input.id(),
                access: module.id(),
                path: vec![name.into()],
                name: name.into(),
            };
            entities.extend([
                Entity::from(input.clone()),
                Entity::from(source),
                Entity::from(module),
                Entity::from(member.clone()),
            ]);
            inputs.push(input);
            members.push(member);
        }
        let assertions = [
            InputDistribution {
                input: inputs[0].id(),
                release: first.id(),
                role: DistributionRole::FirstParty,
            },
            InputDistribution {
                input: inputs[0].id(),
                release: second.id(),
                role: DistributionRole::FirstParty,
            },
            InputDistribution {
                input: inputs[1].id(),
                release: second.id(),
                role: DistributionRole::FirstParty,
            },
            InputDistribution {
                input: inputs[2].id(),
                release: second.id(),
                role: DistributionRole::Dependency,
            },
            InputDistribution {
                input: inputs[3].id(),
                release: foreign.id(),
                role: DistributionRole::FirstParty,
            },
        ]
        .into_iter()
        .map(|row| Assertion::from_record(row).unwrap())
        .collect::<Vec<_>>();
        let native = crate::scoped_fixture::reader(&config, &entities, &assertions)
            .await
            .unwrap();
        let mut foreign_member = members[1].clone();
        foreign_member.path = vec!["unselected".into()];
        foreign_member.name = "unselected".into();
        assert_ne!(
            foreign_member.id(),
            members[1].id(),
            "a valid foreign candidate shares the selected input scope but has no selected membership"
        );
        let mut foreign_module = entities
            .iter()
            .find_map(|entity| match entity {
                Entity::Module(module) => Some(module.clone()),
                _ => None,
            })
            .unwrap();
        foreign_module.qualified_name = "unselected.module".into();
        let foreign_copy = crate::scoped_fixture::reader(
            &config,
            &[
                Entity::CatalogMember(foreign_member),
                Entity::Module(foreign_module),
            ],
            &[],
        )
        .await
        .unwrap();
        native.store.abandon().await.unwrap();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let library = Name::new("discovery-selected").unwrap();
        let mut expected = members[..2].to_vec();
        expected.sort_by_key(Record::id);
        let selected = super::members(&native, Some(&library), None, &budget)
            .await
            .unwrap();
        assert_eq!(
            selected.as_slice(),
            expected,
            "multiple releases/captures and duplicate input preserve complete nominal dedup"
        );
        drop(selected);
        let unfiltered = super::members(&native, None, None, &budget).await.unwrap();
        let mut expected_all = vec![members[0].clone(), members[1].clone(), members[3].clone()];
        expected_all.sort_by_key(Record::id);
        assert_eq!(
            unfiltered.as_slice(),
            expected_all,
            "dependency and orphan input never widen unfiltered library eligibility"
        );
        drop(unfiltered);
        assert!(
            super::members(&native, Some(&Name::new("absent").unwrap()), None, &budget)
                .await
                .unwrap()
                .is_empty()
        );
        for selector in [
            OperationSelector::Member {
                member: members[1].id(),
            },
            OperationSelector::PublicPath {
                path: vec![
                    Name::new("discovery").unwrap(),
                    Name::new("second").unwrap(),
                ],
            },
        ] {
            let selected = super::members(&native, Some(&library), Some(&selector), &budget)
                .await
                .unwrap();
            assert_eq!(selected.as_slice(), &members[1..2]);
        }
        assert_eq!(budget.reserved(), 0);
        let cancelled = native
            .reader
            .clone()
            .with_read_cancellation(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                true,
            )));
        assert!(
            super::members(&cancelled, Some(&library), None, &budget)
                .await
                .is_err(),
            "cancellation is not successful empty discovery"
        );
        drop(cancelled);
        for library in [surrealdb::types::Value::None, surrealdb::types::Value::Null] {
            let mut vars = Variables::new();
            vars.insert("library", library);
            let mut packages = native
                .candidate_payload_rows(
                    lctx_native::prepared::PreparedQuery::new(
                        vars,
                        vec![],
                        vec![library_packages_sql().into()],
                    )
                    .unwrap(),
                    "entity",
                    "semantic_key",
                )
                .unwrap();
            let mut count = 0;
            while packages.next().await.unwrap().is_some() {
                count += 1;
            }
            assert_eq!(
                count, 2,
                "missing/null library keeps the full package universe"
            );
        }
        for (sql, index, binding, value) in [
            (
                library_packages_sql(),
                "semantic_key",
                "library",
                surrealdb::types::Value::from_t(library.as_str().to_owned()),
            ),
            (
                library_releases_sql(),
                "incoming",
                "package_anchor",
                surrealdb::types::Value::RecordId(reader::target_id(
                    domain::graph::Target::Entity(domain::graph::EntityId::of(package.id())),
                )),
            ),
            (
                library_inputs_sql(),
                "incoming",
                "release_anchor",
                surrealdb::types::Value::RecordId(reader::target_id(
                    domain::graph::Target::Entity(domain::graph::EntityId::of(second.id())),
                )),
            ),
        ] {
            let mut vars = Variables::new();
            vars.insert(binding, value);
            let plan: serde_json::Value = native
                .query(format!("{} EXPLAIN", sql.trim_end_matches(';')), vars)
                .await
                .unwrap();
            let plan = plan.to_string();
            assert!(plan.contains(index), "{plan}");
            for material in [
                "UnionIndexScan",
                "Sort",
                "Aggregate",
                "Distinct",
                "TableScan",
            ] {
                assert!(
                    !plan.contains(material),
                    "discovery retains whole-match state: {plan}"
                );
            }
        }
        // The real parent stream emits a package, allowing all descendants to submit
        // candidates, before its later terminal fails. It cannot seal sorted output.
        let mut vars = Variables::new();
        vars.insert("library", library.as_str().to_owned());
        vars.insert("member", surrealdb::types::Value::Null);
        vars.insert("path", surrealdb::types::Value::Null);
        let packages = native
            .query_stream(
                format!(
                    "{} THROW 'late upstream library failure';",
                    library_packages_sql().replacen("SELECT id", "SELECT *", 1)
                ),
                vars.clone(),
                2,
            )
            .unwrap();
        let mut sorted = lctx_native::acknowledged_candidates::AsyncCandidateSort::new(&budget)
            .await
            .unwrap();
        let before_candidates = budget.reserved();
        let error = collect_library_candidates(&native, &mut vars, packages, &mut sorted)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("late upstream library failure"),
            "{error}"
        );
        assert!(
            budget.reserved() > before_candidates,
            "provisional upstream rows submitted candidates before the late failure"
        );
        sorted.drain().await.unwrap();
        assert_eq!(budget.reserved(), 0);
        native.close().await.unwrap();
        foreign_copy.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scoped_nomination_and_get_preserve_same_nominal_copy_selected_alias_and_abandoned_source()
     {
        use domain::{
            Id,
            normalized::entities::EntityRef,
            retrieval::{Family, Unit},
            value::Place,
        };
        use surrealdb::types::Value;
        fn id<R: Record>(byte: u8) -> Id<R> {
            serde_json::from_value(serde_json::json!(vec![byte; 16])).unwrap()
        }
        let config = crate::scoped_fixture::config();
        let selected = Unit {
            input: id(71),
            context: id(72),
            family: Family::ApiOptions,
            origin: id(73),
            corpus: id(74),
            title: "selected body".into(),
        };
        let mut foreign = selected.clone();
        foreign.corpus = id(75);
        foreign.title = "foreign body".into();
        assert_eq!(selected.id(), foreign.id());
        assert_ne!(selected, foreign);
        let place = Place {
            root: id(76),
            path: id(77),
        };
        let alias = EntityRef::Place { place: place.id() };
        let selected_node =
            lctx_native::loader::entity_payload_id(&Entity::from(selected.clone())).unwrap();
        let foreign_node =
            lctx_native::loader::entity_payload_id(&Entity::from(foreign.clone())).unwrap();
        let place_node =
            lctx_native::loader::entity_payload_id(&Entity::from(place.clone())).unwrap();
        let alias_node =
            lctx_native::loader::entity_payload_id(&Entity::from(alias.clone())).unwrap();
        assert_ne!(selected_node, foreign_node);
        // Canonical Place endpoints are introduced at Normalized, not Facts. Use the
        // real producer path so the reverse-alias witness is owned and audited.
        let native = {
            use domain::{
                Relation,
                admission::Frontier,
                completed::ContributionSpec,
                stages::{Profile, ProviderOutcome},
            };
            use lctx_compiler_store::compiler::NativeCompilerStore;
            let store = NativeCompilerStore::begin(&config, Frontier::Normalized)
                .await
                .unwrap();
            let budget = ResourceBudget::fixed(64 << 20).unwrap();
            let relations = [Relation::of::<Unit>(), Relation::of::<Place>()];
            let contribution = store
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: "scoped-normalized-alias-fixture".into(),
                    profile: domain::completed::OutputProfile::Specific(Profile::Catalog),
                    model: domain::model().unwrap().digest(),
                    implementation: ContentHash::of(b"scoped-normalized-alias-fixture/v1"),
                    configuration: None,
                    inputs: vec![],
                    outputs: relations
                        .iter()
                        .map(|relation| relation.name().to_owned())
                        .collect(),
                })
                .await
                .unwrap();
            store
                .write_batch(
                    &contribution,
                    &relations[0],
                    &Unit::encode(std::slice::from_ref(&selected)).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            store
                .write_batch(
                    &contribution,
                    &relations[1],
                    &Place::encode(&[place]).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            let views = store
                .complete_contribution(
                    contribution,
                    ProviderOutcome::Complete,
                    &relations,
                    &std::collections::BTreeMap::new(),
                    &budget,
                )
                .await
                .unwrap();
            let descriptors = views.values().cloned().collect::<Vec<_>>();
            let reader = store.reader_for_views(&descriptors, &budget).await.unwrap();
            let mut installed = Vec::new();
            for view in &descriptors {
                installed.push(store.installed_view(view, &budget).await.unwrap());
            }
            crate::scoped_fixture::ScopedFixture {
                reader,
                store,
                installed,
            }
        };
        let other = crate::scoped_fixture::reader(&config, &[Entity::from(foreign)], &[])
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        native.store.abandon().await.unwrap();
        let result = async {
            let answers = native
                .selected_membership(
                    &[
                        selected_node.clone(),
                        foreign_node.clone(),
                        alias_node.clone(),
                    ],
                    &budget,
                )
                .await?;
            assert!(answers.contains(&selected_node));
            assert!(!answers.contains(&foreign_node));
            assert!(answers.contains(&alias_node));
            let alias_answer = answers
                .answers
                .iter()
                .find(|answer| answer.requested == alias_node)
                .unwrap();
            assert!(!alias_answer.witnesses.is_empty());
            assert!(
                alias_answer
                    .witnesses
                    .iter()
                    .all(|witness| witness.member == place_node
                        && witness.ancestry == vec![alias_node.clone(), place_node.clone()]),
                "alias access comes only from its selected Place owner"
            );
            drop(answers);
            let mut vars = Variables::new();
            vars.insert(
                "nodes",
                vec![selected_node.clone(), foreign_node, alias_node.clone()],
            );
            let query = lctx_native::prepared::PreparedQuery::new(
                vars,
                vec![],
                vec!["SELECT id FROM $nodes".into()],
            )?;
            let mut rows = native.candidate_payload_rows(query, "entity", "semantic_key")?;
            let observed = async {
                let mut actual = std::collections::BTreeSet::new();
                while let Some(row) = rows.next().await? {
                    let Some(Value::RecordId(node)) = row.as_object().and_then(|row| row.get("id"))
                    else {
                        return Err(ModelError::Schema("scoped nominated physical identity"));
                    };
                    actual.insert(node.clone());
                }
                assert_eq!(
                    actual,
                    std::collections::BTreeSet::from([selected_node, alias_node])
                );
                Ok::<_, ModelError>(())
            }
            .await;
            let mut terminal = domain::completion::Completion::default();
            terminal.step(
                "scoped nomination complete drain",
                rows.drain_transport().await,
            );
            domain::completion::complete(observed, terminal)?;
            assert_eq!(
                native
                    .records::<Unit>(lctx_native::RecordSelection::Keys(vec![
                        *selected.id().bytes()
                    ]))
                    .await?,
                vec![selected]
            );
            assert_eq!(
                native
                    .records::<EntityRef>(lctx_native::RecordSelection::Keys(vec![
                        *alias.id().bytes()
                    ]))
                    .await?,
                vec![alias]
            );
            assert!(
                native
                    .records::<Unit>(lctx_native::RecordSelection::Keys(vec![]))
                    .await?
                    .is_empty()
            );
            Ok::<_, ModelError>(())
        }
        .await;
        let mut terminal = domain::completion::Completion::default();
        terminal.step("scoped selected fixture close", native.close().await);
        terminal.step("scoped foreign fixture close", other.close().await);
        domain::completion::complete(result, terminal).unwrap();
        assert!(
            native
                .records::<Unit>(lctx_native::RecordSelection::Keys(vec![]))
                .await
                .is_err(),
            "empty demand cannot bypass released scope authority"
        );
        assert_eq!(budget.reserved(), 0);
    }
}
