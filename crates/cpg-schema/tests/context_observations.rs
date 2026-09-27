use cpg_schema::codebook::{ParameterKind, SignatureForm};
use cpg_schema::context_observations::{parameters, parameters_sql};
use cpg_schema::id::Id;
use cpg_schema::tables::ContextParametersRow;
#[tokio::test]
async fn repeated_reports_agree_without_hiding_signature_drift_or_duplicate_facts() {
    let first = ContextParametersRow {
        snapshot_id: Id([1; 16]),
        fact_id: Id([2; 16]),
        symbol_node_id: Id([3; 16]),
        module_node_id: Id([4; 16]),
        signature_index: 0,
        form: SignatureForm::List,
        ordinal: Some(0),
        kind: Some(ParameterKind::PositionalOrKeyword),
        name: Some("obj".into()),
        required: Some(true),
    };
    let second = ContextParametersRow {
        fact_id: Id([5; 16]),
        ..first.clone()
    };
    assert_eq!(parameters([&first, &second]).unwrap(), [&first]);
    assert_eq!(parameters([&second, &first]).unwrap(), [&first]);
    let conflict = ContextParametersRow {
        required: Some(false),
        ..second.clone()
    };
    assert!(parameters([&first, &conflict]).is_err());
    assert!(parameters([&first, &first]).is_err());
    use cpg_schema::{Table, tables::ContextParameters};
    for (rows, expected) in [
        (vec![second.clone(), first.clone()], 1),
        (vec![first.clone(), conflict], 0),
        (vec![first.clone(), first.clone()], 0),
    ] {
        let ctx = datafusion::prelude::SessionContext::new();
        ctx.register_batch(
            "context_parameters",
            ContextParameters::to_batch(&rows).unwrap(),
        )
        .unwrap();
        let output = ctx
            .sql(&parameters_sql())
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(output.iter().map(|b| b.num_rows()).sum::<usize>(), expected);
        if expected == 1 {
            use cpg_schema::query::QueryRow;
            let output: Vec<_> = output
                .iter()
                .flat_map(|b| ContextParametersRow::read_batch(b).unwrap())
                .collect();
            assert_eq!(output.as_slice(), std::slice::from_ref(&first));
        }
    }
}
