//! Existing finite requirement and same-context algebra over complete requested semantic domains.
use lctx_model::domain::{
    self, HeapSize, Id, ModelError, Record, ValidationInput, catalog,
    charged::{ChargedSet, ChargedVec, StateCharge},
    resources::ResourceBudget,
    selection,
    serving::*,
};
use lctx_surrealdb::{NativeReader, reader::target_id};
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
    let mut inputs = selection::classification::ClassificationData::inputs();
    inputs.extend(selection::build::Output::inputs());
    inputs.sort_by_key(ValidationInput::name);
    inputs.dedup_by_key(|i| i.name());
    let mut request = ClassificationRequest::new(members, budget)?;
    let batches = crate::scope::hydrate(reader, request.take_roots(), &inputs, budget).await?;
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
pub async fn members(
    reader: &NativeReader,
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
    let inputs: Vec<surrealdb::types::Value> = reader
        .query_native("RETURN fn::lctx_library_inputs($library);", vars.clone())
        .await?;
    let mut sorted =
        lctx_surrealdb::acknowledged_candidates::AsyncCandidateSort::new(budget).await?;
    for input in inputs {
        vars.insert(
            "scope",
            format!(
                "catalog_members|input|{}",
                lctx_surrealdb::prepared::scope_string(&input)
            ),
        );
        let mut rows = reader.query_stream(member_candidates_sql(), vars.clone(), 1)?;
        while let Some(value) = rows.next().await? {
            let surrealdb::types::Value::Object(row) = value else {
                return Err(ModelError::Schema("native member candidate"));
            };
            let Some(surrealdb::types::Value::String(key)) = row.get("semantic_key") else {
                return Err(ModelError::Schema("native member nominal identity"));
            };
            let Some(surrealdb::types::Value::RecordId(node)) = row.get("node") else {
                return Err(ModelError::Schema("native member backing pointer"));
            };
            let key: [u8; 16] = hex::decode(key)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("native member key width"))?;
            sorted
                .push(lctx_surrealdb::ordered_rows::Candidate {
                    relation: catalog::CatalogMember::NAME.into(),
                    key,
                    node: node.clone(),
                })
                .await?;
        }
    }
    let mut ordered = sorted.finish().await?;
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
            // The bounded transfer keeps its vector capacity; moved strings become
            // the retained member owner's responsibility without double charging.
            transfer.release(member.heap_bytes());
            members.push(member)?;
        }
    }
    Ok(members)
}

/// Candidate extraction has one indexed equality branch and no native union/order state.
/// The path predicate is owned by the finite member operation, never request SQL.
pub fn member_candidates_sql() -> &'static str {
    "SELECT semantic_key,id AS node FROM entity WITH INDEX by_scope WHERE semantic_type='catalog_members' AND scope_keys CONTAINS $scope AND ($member=NONE OR $member=NULL OR semantic_key=$member) AND ($path=NONE OR $path=NULL OR array::concat(string::split((SELECT VALUE out.body.qualified_name FROM reference WITH INDEX outgoing WHERE in=$parent.id AND field='access')[0],'.'),body.path)=$path);"
}

pub fn native_definitions() -> &'static str {
    r#"
DEFINE FUNCTION fn::lctx_library_inputs($name: option<string|null>) {
 LET $packages = SELECT VALUE id FROM entity WHERE semantic_type='packages' AND ($name=NONE OR $name=NULL OR body.name=$name);
 LET $releases = SELECT VALUE in FROM reference WHERE field='package' AND out IN $packages;
 LET $distributions = SELECT VALUE in FROM participant WHERE field='release' AND out IN $releases;
 RETURN SELECT VALUE body.input FROM assertion WHERE id IN $distributions AND semantic_type='input_distributions' AND body.role=0;
};

"#
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
