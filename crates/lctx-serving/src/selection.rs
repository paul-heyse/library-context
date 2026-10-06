//! Existing finite requirement and same-context algebra over complete requested semantic domains.
use lctx_model::domain::{
    self, ModelError, Record, ValidationInput, catalog, resources::ResourceBudget, selection,
    serving::*,
};
use lctx_surrealdb::{NativeReader, reader::target_id};
use surrealdb::types::Variables;

pub struct MemberSelection {
    pub prepared: selection::evaluate::Prepared,
    pub selected: selection::evaluate::Selected,
}
pub async fn classify(
    reader: &NativeReader,
    members: &[catalog::CatalogMember],
    selection: &selection::Selection,
    budget: &ResourceBudget,
) -> Result<MemberSelection, ModelError> {
    let mut inputs = selection::classification::ClassificationData::inputs();
    inputs.extend(selection::build::Output::inputs());
    inputs.sort_by_key(ValidationInput::name);
    inputs.dedup_by_key(|i| i.name());
    let roots = members
        .iter()
        .map(|m| {
            target_id(domain::graph::Target::Entity(domain::graph::EntityId::of(
                m.id(),
            )))
        })
        .collect();
    let batches = crate::scope::hydrate(reader, roots, &inputs, budget).await?;
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
    let requested: std::collections::BTreeSet<_> = members.iter().map(Record::id).collect();
    selected
        .candidates
        .retain(|c| requested.contains(&c.member));
    Ok(MemberSelection { prepared, selected })
}

/// Exact native root selection uses normalized capture identity and canonical member paths.
/// A path constraint is resolved against the module spelling and the member's own path.
pub async fn members(
    reader: &NativeReader,
    library: Option<&Name>,
    operation: Option<&OperationSelector>,
) -> Result<Vec<catalog::CatalogMember>, ModelError> {
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
    let keys: Vec<String> = reader
        .query("RETURN fn::lctx_member_keys($library,$member,$path);", vars)
        .await?;
    let roots = keys
        .iter()
        .map(|key| {
            hex::decode(key)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("native member key width"))
        })
        .collect::<Result<Vec<[u8; 16]>, _>>()?;
    reader
        .records(lctx_surrealdb::RecordSelection::Keys(roots))
        .await
}

pub fn native_definitions() -> &'static str {
    r#"
DEFINE FUNCTION fn::lctx_library_inputs($name: option<string|null>) {
 LET $packages = SELECT VALUE id FROM entity WHERE semantic_type='packages' AND ($name=NONE OR $name=NULL OR body.name=$name);
 LET $releases = SELECT VALUE in FROM reference WHERE field='package' AND out IN $packages;
 LET $distributions = SELECT VALUE in FROM participant WHERE field='release' AND out IN $releases;
 RETURN SELECT VALUE body.input FROM assertion WHERE id IN $distributions AND semantic_type='input_distributions' AND body.role=0;
};
DEFINE FUNCTION fn::lctx_member_keys($name: option<string|null>, $member: option<string|null>, $path: option<array<string>|null>) {
 LET $inputs=fn::lctx_library_inputs($name);
 RETURN SELECT VALUE semantic_key FROM entity WHERE semantic_type='catalog_members' AND scope_keys CONTAINSANY $inputs.map(|$v|'catalog_members|input|'+<string>$v) AND ($member=NONE OR $member=NULL OR semantic_key=$member) AND ($path=NONE OR $path=NULL OR array::concat(string::split((SELECT VALUE out.body.qualified_name FROM reference WHERE in=$parent.id AND field='access')[0],'.'),body.path)=$path) ORDER BY semantic_key;
};
"#
}
