//! Existing finite requirement and same-context algebra over complete requested semantic domains.
use lctx_model::domain::{
    self, HeapSize, Id, ModelError, Record, ValidationInput, catalog,
    charged::{ChargedSet, ChargedVec, StateCharge},
    resources::ResourceBudget,
    selection,
    serving::*,
};
use lctx_surrealdb::{
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
    pub prepared: selection::evaluate::Prepared,
    pub selected: selection::evaluate::Selected,
}
pub async fn classify(
    reader: &NativeReader,
    members: &Members,
    selection: &selection::Selection,
    budget: &ResourceBudget,
) -> Result<MemberSelection, ModelError> {
    classify_inner(reader, members, selection, budget, None).await
}
pub(crate) async fn classify_prepared(reader: &NativeReader, members: &Members, selection: &selection::Selection, budget: &ResourceBudget, preparation: &crate::preparation::Preparation<'_>) -> Result<MemberSelection, ModelError> {
    classify_inner(reader, members, selection, budget, Some(preparation)).await
}
async fn classify_inner(reader: &NativeReader, members: &Members, selection: &selection::Selection, budget: &ResourceBudget, preparation: Option<&crate::preparation::Preparation<'_>>) -> Result<MemberSelection, ModelError> {
    let mut inputs = selection::classification::ClassificationData::inputs();
    inputs.extend(selection::build::Output::inputs());
    inputs.sort_by_key(ValidationInput::name);
    inputs.dedup_by_key(|i| i.name());
    let mut request = ClassificationRequest::new(members, budget)?;
    let cached;
    let fresh;
    let batches = if let Some(preparation) = preparation {
        cached = preparation.hydrate(reader, request.take_roots(), &inputs, &inputs, crate::scope::OWNED_FIELDS).await?;
        &*cached
    } else {
        fresh = crate::scope::hydrate(reader, request.take_roots(), &inputs, budget).await?;
        &fresh
    };
    request.release_roots();
    let mut data = selection::classification::ClassificationData::new(budget);
    let mut output = selection::build::Output::new(budget);
    for (name, batch) in &batches.batches {
        let input = data.visit(name, batch)?;
        let result = output.visit(name, batch)?;
        if !input && !result {
            return Err(ModelError::Schema("undeclared native classification input"));
        }
    }
    let prepared = selection::evaluate::Prepared::from_local_rows(data, output, budget)?;
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
    let mut vars = reader.view_bindings();
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
    let mut sorted =
        lctx_surrealdb::acknowledged_candidates::AsyncCandidateSort::new(budget).await?;
    let discovery = async {
        let packages = reader.query_stream(library_packages_sql(), vars.clone(), 1)?;
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
    "SELECT VALUE id FROM entity WITH INDEX semantic_key WHERE id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type='packages' AND ($library=NONE OR $library=NULL OR body.name=$library);"
}
fn library_releases_sql() -> &'static str {
    "SELECT VALUE in FROM reference WITH INDEX incoming WHERE out=$package.anchor AND in IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND field='package';"
}
fn library_inputs_sql() -> &'static str {
    "SELECT VALUE in.body.input FROM participant WITH INDEX incoming WHERE out=$release.anchor AND in IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND field='release' AND in.semantic_type='input_distributions' AND in.body.role=0;"
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
    sorted: &mut lctx_surrealdb::acknowledged_candidates::AsyncCandidateSort,
) -> Result<(), ModelError> {
    let result = async {
        while let Some(package) = packages.next().await? {
            let surrealdb::types::Value::RecordId(package) = package else {
                return Err(ModelError::Schema("native library package pointer"));
            };
            vars.insert("package", package);
            let mut releases = reader.query_stream(library_releases_sql(), vars.clone(), 1)?;
            let result = async {
                while let Some(release) = releases.next().await? {
                    let surrealdb::types::Value::RecordId(release) = release else {
                        return Err(ModelError::Schema("native library release pointer"));
                    };
                    vars.insert("release", release);
                    let mut inputs = reader.query_stream(library_inputs_sql(), vars.clone(), 1)?;
                    let result = async {
                        while let Some(input) = inputs.next().await? {
                            vars.insert(
                                "scope",
                                format!(
                                    "catalog_members|input|{}",
                                    lctx_surrealdb::prepared::scope_string(&input)
                                ),
                            );
                            let mut rows =
                                reader.query_stream(member_candidates_sql(), vars.clone(), 1)?;
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
                                        row.get("node")
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
                                    sorted
                                        .push(lctx_surrealdb::ordered_rows::Candidate {
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
    "SELECT semantic_key,id AS node FROM entity WITH INDEX by_scope WHERE id IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views) AND semantic_type='catalog_members' AND scope_keys CONTAINS $scope AND ($member=NONE OR $member=NULL OR semantic_key=$member) AND ($path=NONE OR $path=NULL OR { LET $member_node=id; LET $member_path=body.path; LET $access=SELECT VALUE out FROM reference WITH INDEX outgoing WHERE in=$member_node AND field='access'; LET $module=SELECT VALUE node.body.qualified_name FROM compiler_view_member WHERE view IN $lctx_views AND node.anchor IN $access; RETURN array::concat(string::split($module[0],'.'),$member_path)=$path; });"
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
    use lctx_surrealdb::reader;

    #[tokio::test]
    async fn native_library_discovery_preserves_complete_eligibility_and_drains_late_upstream_failure()
     {
        let config=crate::scoped_fixture::config();
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
            let source =
                SourceArtifact::from_bytes(input.id(), format!("{name}.py"), name.as_bytes())
                    .unwrap();
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
        let native=crate::scoped_fixture::reader(&config,&entities,&assertions).await.unwrap();
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
        for library in [surrealdb::types::Value::None, surrealdb::types::Value::Null] {
            let mut vars = native.view_bindings();
            vars.insert("library", library);
            let mut packages = native
                .query_stream(library_packages_sql(), vars, 1)
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
                "package",
                surrealdb::types::Value::RecordId(reader::target_id(
                    domain::graph::Target::Entity(domain::graph::EntityId::of(package.id())),
                )),
            ),
            (
                library_inputs_sql(),
                "incoming",
                "release",
                surrealdb::types::Value::RecordId(reader::target_id(
                    domain::graph::Target::Entity(domain::graph::EntityId::of(second.id())),
                )),
            ),
        ] {
            let mut vars = native.view_bindings();
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
        let mut vars = native.view_bindings();
        vars.insert("library", library.as_str().to_owned());
        vars.insert("member", surrealdb::types::Value::Null);
        vars.insert("path", surrealdb::types::Value::Null);
        let packages = native
            .query_stream(
                format!(
                    "{} THROW 'late upstream library failure';",
                    library_packages_sql()
                ),
                vars.clone(),
                2,
            )
            .unwrap();
        let mut sorted = lctx_surrealdb::acknowledged_candidates::AsyncCandidateSort::new(&budget)
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
    }
}
