//! Independent float64 cosine and addressable-winner controls against real PostgreSQL.
use arrow_array::{Array, FixedSizeBinaryArray, FixedSizeListArray, Float32Array, StringArray};
use cpg_schema::{Id, wire::Selection};
use lctx_postgres::{repository::PinnedGeneration, serving::ServingStore};
use std::{collections::BTreeMap, path::Path};
fn rows(root: &Path, name: &str) -> Vec<arrow_array::RecordBatch> {
    arrow_ipc::reader::FileReader::try_new(
        std::fs::File::open(root.join(format!("{name}.arrow"))).unwrap(),
        None,
    )
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}
pub async fn unit_ranks(reader: &ServingStore, pinned: &PinnedGeneration, root: &Path) {
    cpu_cancellation(reader).await;
    let prepared = reader
        .prepare_selection(pinned, &Selection::default(), "")
        .await
        .unwrap();
    let first = prepared.page(pinned, 1, None).unwrap();
    let path = first["supported"]["items"][0]["access_path"]
        .as_str()
        .unwrap();
    let strict:Selection=serde_json::from_value(serde_json::json!({"mode":"strict","requirements":[{"predicate":"release_version","distribution":"not-this-distribution","version":"0"}]})).unwrap();
    let strict = reader
        .prepare_selection(pinned, &strict, path)
        .await
        .unwrap();
    assert_eq!(strict.scope()["eligible"], serde_json::json!([]));
    assert_eq!(strict.scope()["promoted"], serde_json::json!([]));
    assert_eq!(
        reader
            .finish_selection_search(pinned, &strict, vec![], "lexical-only", 20, None)
            .await
            .unwrap()["supported"]["total"],
        0
    );
    let mut query = vec![0f32; 1024];
    query[0] = 1.;
    let raw = reader
        .selection_vectors(
            pinned,
            &prepared,
            &query,
            pinned.manifest().spec_hash.as_deref().unwrap(),
        )
        .await
        .unwrap();
    let mut subjects = BTreeMap::<Id, Vec<Id>>::new();
    for b in rows(root, "retrieval_subjects") {
        let units = b
            .column_by_name("unit_id")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let members = b
            .column_by_name("member_id")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        for n in 0..b.num_rows() {
            if !members.is_null(n) {
                subjects
                    .entry(Id(units.value(n).try_into().unwrap()))
                    .or_default()
                    .push(Id(members.value(n).try_into().unwrap()));
            }
        }
    }
    let mut fragments = BTreeMap::new();
    for b in rows(root, "retrieval_fragments") {
        let ids = b
            .column_by_name("fragment_id")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let units = b
            .column_by_name("unit_id")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let families = b
            .column_by_name("family")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        for n in 0..b.num_rows() {
            fragments.insert(
                Id(ids.value(n).try_into().unwrap()),
                (
                    Id(units.value(n).try_into().unwrap()),
                    families.value(n).to_owned(),
                ),
            );
        }
    }
    let mut expected = BTreeMap::<(String, Id), (f64, Id, Id)>::new();
    for b in rows(root, "retrieval_vectors") {
        let ids = b
            .column_by_name("fragment_id")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        let vectors = b
            .column_by_name("vector")
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .unwrap();
        for n in 0..b.num_rows() {
            let fragment = Id(ids.value(n).try_into().unwrap());
            let (unit, family) = &fragments[&fragment];
            let values = vectors.value(n);
            let vector = values.as_any().downcast_ref::<Float32Array>().unwrap();
            let norm = vector
                .values()
                .iter()
                .map(|x| f64::from(*x).powi(2))
                .sum::<f64>()
                .sqrt();
            let score = f64::from(vector.value(0)) / norm;
            for member in subjects.get(unit).into_iter().flatten() {
                let candidate = (score, *unit, fragment);
                let old = expected
                    .entry((family.clone(), *member))
                    .or_insert(candidate);
                if score > old.0 || (score == old.0 && (*unit, fragment) < (old.1, old.2)) {
                    *old = candidate;
                }
            }
        }
    }
    let mut count = 0;
    let mut winners = Vec::new();
    for batch in arrow_ipc::reader::StreamReader::try_new(std::io::Cursor::new(raw), None).unwrap()
    {
        let batch = batch.unwrap();
        let ids = |col: usize| {
            batch
                .column(col)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
        };
        let families = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let ranks = batch
            .column(5)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
            .unwrap();
        let scores = batch
            .column(6)
            .as_any()
            .downcast_ref::<arrow_array::Float64Array>()
            .unwrap();
        for n in 0..batch.num_rows() {
            let member = Id(ids(0).value(n).try_into().unwrap());
            let family = families.value(n);
            let unit = Id(ids(3).value(n).try_into().unwrap());
            let fragment = Id(ids(4).value(n).try_into().unwrap());
            let wanted = expected[&(family.to_owned(), member)];
            assert!((scores.value(n) - wanted.0).abs() < 1e-5);
            assert_eq!((unit, fragment), (wanted.1, wanted.2));
            let mut ordered = expected
                .iter()
                .filter(|((f, _), _)| f == family)
                .collect::<Vec<_>>();
            ordered.sort_by(|a, b| b.1.0.total_cmp(&a.1.0).then(a.0.1.cmp(&b.0.1)));
            assert_eq!(ordered[ranks.value(n) as usize - 1].0.1, member);
            winners.push(cpg_schema::retrieval::Winner {
                member_id: cpg_schema::wire::PublicMemberId::from_storage(member),
                family: serde_json::from_value(family.into()).unwrap(),
                channel: cpg_schema::retrieval::Channel::Vector,
                unit_id: cpg_schema::wire::RetrievalUnitId::from_storage(unit),
                fragment_id: fragment,
                score: scores.value(n),
                rank: ranks.value(n),
            });
            count += 1;
        }
    }
    assert!(count > 0);
    assert_eq!(count, expected.len());
    let mut foreign = winners.clone();
    foreign[0].fragment_id = Id([255; 16]);
    let foreign = cpg_schema::retrieval::fuse(&foreign, &Default::default());
    assert!(
        reader
            .finish_selection_search(pinned, &prepared, foreign, "vector:test-control", 1, None)
            .await
            .is_err(),
        "foreign fragment refused by metadata membership join"
    );
    let mut impossible = winners.clone();
    impossible[0].rank += 1;
    let impossible = cpg_schema::retrieval::fuse(&impossible, &Default::default());
    assert!(
        reader
            .finish_selection_search(
                pinned,
                &prepared,
                impossible,
                "vector:test-control",
                2,
                None
            )
            .await
            .is_err(),
        "sparse or duplicate channel ranks refused"
    );
    let ranked = cpg_schema::retrieval::fuse(&winners, &Default::default());
    let page = reader
        .finish_selection_search(pinned, &prepared, ranked, "vector:test-control", 2, None)
        .await
        .unwrap();
    let cursor = page["supported"]["next_cursor"].as_str().unwrap();
    assert!(
        prepared.page(pinned, 2, Some(cursor)).is_err(),
        "find cannot replay a search cursor"
    );
    let winner = &page["supported"]["items"][0]["ranking"]["winners"][0];
    let unit =
        cpg_schema::wire::RetrievalUnitId::parse(winner["unit_id"].as_str().unwrap()).unwrap();
    let evidence = reader
        .get_retrieval_unit(pinned, &pinned.manifest().snapshot_id, unit, None, false)
        .await
        .unwrap();
    assert_eq!(evidence["unit"]["unit_id"], winner["unit_id"]);
    let member = page["supported"]["items"][0]["member_id"].as_str().unwrap();
    reader
        .get_operation(pinned, &pinned.manifest().snapshot_id, member)
        .await
        .unwrap();
    for winner in winners
        .iter()
        .filter(|w| w.family == cpg_schema::retrieval::Family::ApiOptions)
        .take(1)
    {
        let mut original = false;
        let mut cursor = None;
        for _ in 0..100 {
            let page = reader
                .get_retrieval_unit(
                    pinned,
                    &pinned.manifest().snapshot_id,
                    winner.unit_id,
                    cursor.as_deref(),
                    false,
                )
                .await
                .unwrap();
            original |= !page["original"].is_null();
            cursor = page["next_cursor"].as_str().map(str::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        assert!(original, "API/options winner reaches original bytes");
    }
    let selection: Selection = serde_json::from_value(
        serde_json::json!({"requirements":[{"predicate":"public_path","path":path}]}),
    )
    .unwrap();
    let bounded = reader
        .prepare_selection(pinned, &selection, "")
        .await
        .unwrap();
    let expected = bounded.page(pinned, 2, None).unwrap()["contradicted_count"].clone();
    assert!(expected.as_u64().unwrap() > 0);
    let eligible = bounded.eligible();
    let winners = winners
        .into_iter()
        .filter(|w| eligible.contains(&w.member_id))
        .collect();
    let ranked = cpg_schema::retrieval::fuse(
        &cpg_schema::retrieval::channel_winners(winners).unwrap(),
        &Default::default(),
    );
    assert_eq!(
        reader
            .finish_selection_search(pinned, &bounded, ranked, "vector:test-control", 2, None)
            .await
            .unwrap()["contradicted_count"],
        expected
    );
}

async fn cpu_cancellation(reader: &ServingStore) {
    let mut jobs = Vec::new();
    let mut releases = Vec::new();
    let mut completions = Vec::new();
    for _ in 0..2 {
        let reader = reader.clone();
        let (release, blocked) = std::sync::mpsc::channel();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (finished, done) = tokio::sync::oneshot::channel();
        let job = tokio::spawn(async move {
            reader
                .run_cpu(move || {
                    started.send(()).unwrap();
                    blocked.recv().unwrap();
                    let _ = finished.send(());
                    Ok(())
                })
                .await
        });
        ready.await.unwrap();
        jobs.push(job);
        releases.push(release);
        completions.push(done);
    }
    for job in &jobs {
        job.abort();
    }
    // Cancellation of callers cannot admit a third job while either CPU closure still runs.
    assert!(reader.run_cpu(|| Ok(())).await.is_err());
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), reader.check())
            .await
            .unwrap()
            .is_ok(),
        "I/O remains responsive while both CPU slots are occupied"
    );
    for release in releases {
        release.send(()).unwrap();
    }
    for done in completions {
        done.await.unwrap();
    }
    for _ in 0..100 {
        if reader.run_cpu(|| Ok(())).await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("CPU capacity was not released");
}
