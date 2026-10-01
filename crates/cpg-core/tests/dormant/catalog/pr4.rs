// DORMANT P5: transport projection and bundle identity controls only.
// Pure classifier/render/spec realization expectations moved to the typed C2/E0 owners.
use arrow_array::RecordBatch;
use cpg_schema::{Id, Table, catalog::*, serving_projection::projected_rows, wire::*};
use std::{collections::BTreeMap, path::Path};

pub fn check(generation_dir: &Path) {
    let tables: BTreeMap<String, Vec<RecordBatch>> = cpg_schema::bundle::files(0)
        .iter()
        .map(|f| {
            let reader = arrow_ipc::reader::FileReader::try_new(
                std::fs::File::open(generation_dir.join(format!("{}.arrow", f.name))).unwrap(),
                None,
            )
            .unwrap();
            (
                f.name.into(),
                reader.collect::<Result<Vec<_>, _>>().unwrap(),
            )
        })
        .collect();
    let members = projected_rows::<CatalogMembers>(&tables).unwrap();
    for forge_subject in [true, false] {
        let mut forged = tables.clone();
        let batches = forged.get_mut("retrieval_units").unwrap();
        let batch = batches.iter_mut().find(|b| b.num_rows() > 0).unwrap();
        let details = batch
            .column_by_name("detail")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();
        let mut values = details
            .iter()
            .map(|s| s.unwrap().to_owned())
            .collect::<Vec<_>>();
        let mut unit: cpg_schema::retrieval::Unit = serde_json::from_str(&values[0]).unwrap();
        if forge_subject {
            let other = members
                .iter()
                .find(|m| {
                    !unit
                        .subjects
                        .contains(&cpg_schema::retrieval::Subject::Member(
                            PublicMemberId::from_storage(m.member_id),
                        ))
                })
                .unwrap();
            unit.subjects = vec![cpg_schema::retrieval::Subject::Member(
                PublicMemberId::from_storage(other.member_id),
            )];
        } else {
            unit.text.push_str(" forged");
        }
        values[0] = serde_json::to_string(&unit).unwrap();
        let mut columns = batch.columns().to_vec();
        columns[2] = std::sync::Arc::new(arrow_array::StringArray::from(values));
        *batch = RecordBatch::try_new(batch.schema(), columns).unwrap();
        assert!(
            cpg_schema::retrieval::validate_projection(&forged)
                .unwrap_err()
                .to_string()
                .contains("canonical catalog rendering")
        );
    }
    let mut forged = tables.clone();
    let mut rows = projected_rows::<CatalogSelectionDomains>(&tables).unwrap();
    rows[0].domain_id = Id([199; 16]);
    replace::<CatalogSelectionDomains>(&mut forged, &rows);
    assert!(cpg_schema::selection::catalog::validate_projection(&forged).is_err());
}

fn replace<T: Table>(tables: &mut BTreeMap<String, Vec<RecordBatch>>, rows: &[T::Row]) {
    let batch = T::to_batch(rows).unwrap();
    let indices = (1..batch.num_columns()).collect::<Vec<_>>();
    let projected = batch.project(&indices).unwrap();
    let schema = tables[T::NAME][0].schema();
    tables.insert(
        T::NAME.into(),
        vec![RecordBatch::try_new(schema, projected.columns().to_vec()).unwrap()],
    );
}
/// Independently prepared P5 bundle fixtures: different specs, cold replay and unavailable realization.
/// Fixture production is separate from these exact transport identity assertions.
pub fn independent_retrieval_artifacts_reuse_exactly_the_same_canonical_generation(
    ga: &cpg_core::bundle::Generation, gb: &cpg_core::bundle::Generation,
    cold: &cpg_core::bundle::Generation, unavailable_generation: &cpg_core::bundle::Generation,
) {
    assert_ne!(ga.key, gb.key);
    assert_eq!(ga.manifest["snapshot_id"], gb.manifest["snapshot_id"]);
    for name in [
        "catalog_members",
        "catalog_selection_domains",
        "catalog_parameters",
    ] {
        assert_eq!(ga.manifest["files"][name], gb.manifest["files"][name]);
    }
    cpg_core::bundle::verify(&ga.dir).unwrap();
    cpg_core::bundle::verify(&gb.dir).unwrap();
    assert_eq!(ga.key, cold.key);
    assert_ne!(unavailable_generation.key, ga.key);
}
