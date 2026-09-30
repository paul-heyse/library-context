//! Explicit Q measurement against an already published generation; never mutates the store.
use cpg_core::generation_read::{GenerationSession, InspectionSession, ProviderOptions};
use lctx_model::domain::{normalized::Rows, projection::*, resources::ResourceBudget, *};
use lctx_postgres::{generations::GenerationId, roles::RoleConfig};
use std::{path::PathBuf, sync::Arc, time::Instant};

#[tokio::test]
#[ignore = "measurement: requires LCTX_PROJECTION_GENERATION and LCTX_PROJECTION_RECEIPT"]
async fn persisted_graph_hydration_measurement() {
    let generation =
        GenerationId::from_hex(&std::env::var("LCTX_PROJECTION_GENERATION").unwrap()).unwrap();
    let config = RoleConfig::load(
        &PathBuf::from(std::env::var("HOME").unwrap())
            .join(".config/library-context/postgres-serving.json"),
    )
    .unwrap();
    let budget = ResourceBudget::fixed(64 << 30).unwrap();
    let session = GenerationSession::open(
        &config,
        Arc::new(model().unwrap()),
        generation,
        ProviderOptions::default(),
    )
    .await
    .unwrap();
    let inspection = InspectionSession::new(session).unwrap();
    let started = Instant::now();
    let mut assessments = Rows::<ProjectionSourceAssessment>::new(&budget);
    let mut snapshots = Rows::<ProjectionSnapshot>::new(&budget);
    let mut chunks = Rows::<ProjectionSnapshotChunk>::new(&budget);
    macro_rules! read {
        ($ty:ty,$rows:ident) => {{
            let query = inspection
                .query(&format!("SELECT * FROM {}", <$ty>::NAME))
                .await
                .unwrap();
            assert_eq!(query.scan_demand(), 1);
            let batches = query.collect().await.unwrap();
            for batch in batches.iter() {
                $rows.decode(batch).unwrap();
            }
        }};
    }
    read!(ProjectionSourceAssessment, assessments);
    read!(ProjectionSnapshot, snapshots);
    read!(ProjectionSnapshotChunk, chunks);
    inspection.close().await.unwrap();
    let read_seconds = started.elapsed().as_secs_f64();
    let mut measured = Vec::new();
    for header in snapshots.iter() {
        let source = assessments.get(header.assessment).unwrap();
        let started = Instant::now();
        let graph = snapshot::hydrate(header, source, &chunks, &budget).unwrap();
        let hydrate_seconds = started.elapsed().as_secs_f64();
        let started = Instant::now();
        let encoded = graph.encode(&budget).unwrap();
        let encode_seconds = started.elapsed().as_secs_f64();
        let mut ordered: Vec<_> = chunks
            .iter()
            .filter(|c| c.snapshot == header.id())
            .collect();
        ordered.sort_by_key(|c| c.ordinal);
        let mut offset = 0;
        for row in ordered {
            assert_eq!(
                &encoded.bytes()[offset..offset + row.payload.0.len()],
                row.payload.0.as_slice()
            );
            offset += row.payload.0.len();
        }
        assert_eq!(offset, encoded.bytes().len());
        measured.push(serde_json::json!({"input":source.input.hex(),"context":source.context.hex(),"projection":format!("{:?}",source.projection),"vertices":graph.vertex_count(),"arcs":graph.arc_count(),"bytes":header.bytes,"chunks":header.chunks,"hydrate_seconds":hydrate_seconds,"encode_seconds":encode_seconds,"canonical_reencoding":"passed"}));
    }
    let peak = budget.peak();
    drop(assessments);
    drop(snapshots);
    drop(chunks);
    assert_eq!(budget.reserved(), 0);
    let receipt = serde_json::json!({"generation":generation.hex(),"leased_read_seconds":read_seconds,"graph_measurements":measured,"peak_graph_reservation_bytes":peak,"released_reservations":"passed","scope":"hydration and reencoding only; SQL input/chunk reconstruction measured separately; no graph construction or performance comparison"});
    std::fs::write(
        std::env::var("LCTX_PROJECTION_RECEIPT").unwrap(),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
