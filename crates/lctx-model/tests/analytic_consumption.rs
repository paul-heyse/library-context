//! Actual canonical full winner/projection consumption, independent of service availability.
#[path = "fixtures/embedding_consumption.rs"]
mod fixture;
use fixture::*;
use lctx_model::domain::{
    embedding::{analytic::*, consumption::*, projection::*, value::*},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
#[test]
fn full_winner_and_projection_are_shared_reference_consumption() {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let mut f = fixture(&b, true);
    let (full, p, published) = winner(&f, &b);
    assert_eq!(full.bytes.0.len(), 4096 * 4);
    assert_eq!(p.bytes.0.len(), 1024 * 4);
    assert_eq!(
        decode_vector(&full.bytes.0, 4096).unwrap()[2].to_bits(),
        (-0.0f32).to_bits()
    );
    f.data
        .values
        .admit_full(&full, &f.encoder, &f.policy)
        .unwrap();
    f.data.values.admit_projection(&p).unwrap();
    f.data.projected_values.insert(p).unwrap();
    let uses = consume(&f, &published, &b);
    let (invocations, outcomes) = outcomes(&f, &uses, &b);
    f.data.validate(&invocations, &outcomes, &uses, &b).unwrap();
    assert_eq!(uses.iter().next().unwrap().value, Some(full.id()));
    assert!(
        AnalysisEmbeddingUse::schema()
            .field_with_name("bytes")
            .is_err()
    );
    drop(f);
    drop(uses);
    drop(invocations);
    drop(outcomes);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn immutable_winners_conflict_and_missing_or_corrupt_projections_refuse() {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let mut f = fixture(&b, true);
    let (full, p, published) = winner(&f, &b);
    let uses = consume(&f, &published, &b);
    let (invocations, outcomes) = outcomes(&f, &uses, &b);
    assert!(f.data.validate(&invocations, &outcomes, &uses, &b).is_err());
    f.data
        .values
        .admit_full(&full, &f.encoder, &f.policy)
        .unwrap();
    assert!(f.data.validate(&invocations, &outcomes, &uses, &b).is_err());
    let mut corrupt = p.clone();
    corrupt.bytes.0[0] ^= 1;
    assert!(f.data.values.admit_projection(&corrupt).is_err());
    f.data.values.admit_projection(&p).unwrap();
    f.data.validate(&invocations, &outcomes, &uses, &b).unwrap();
    let mut changed = full.clone();
    let mut v = decode_vector(&changed.bytes.0, 4096).unwrap();
    v[2] = 0.0;
    changed.bytes = EvidenceBytes(encode_vector(&v));
    changed.digest = value_digest(&v);
    assert!(
        f.data
            .values
            .admit_full(&changed, &f.encoder, &f.policy)
            .is_err()
    );
}
#[test]
fn exact_document_recipe_and_original_input_are_required() {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let mut f = fixture(&b, true);
    let (full, p, published) = winner(&f, &b);
    f.data
        .values
        .admit_full(&full, &f.encoder, &f.policy)
        .unwrap();
    f.data.values.admit_projection(&p).unwrap();
    let uses = consume(&f, &published, &b);
    let (invocations, outcomes) = outcomes(&f, &uses, &b);
    for mutation in 0..3 {
        let mut row = uses.iter().next().unwrap().clone();
        match mutation {
            0 => row.document = id(90),
            1 => row.input = ContentHash::of(b"rewritten"),
            _ => row.admitted_tokens = Some(8),
        };
        let mut altered = Rows::new(&b);
        altered.insert(row).unwrap();
        assert!(
            f.data
                .validate(&invocations, &outcomes, &altered, &b)
                .is_err()
        );
    }
    let mut query = f.spec.clone();
    query.query_task.push_str(" changed");
    assert_eq!(query.hash(), f.spec.hash());
    assert_eq!(query.document_hash(), f.spec.document_hash());
    assert_ne!(query.query_hash(), f.spec.query_hash());
    query.max_document_tokens += 1;
    assert_eq!(query.query_hash(), query.query_recipe().identity());
    assert_eq!(query.hash(), f.spec.hash());
}
#[test]
fn token_and_service_refusals_remain_partial_and_unrequested_has_no_uses() {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let f = fixture(&b, true);
    for (availability, tokens) in [
        (VectorAvailability::ServiceUnavailable, None),
        (
            VectorAvailability::TokenLimit,
            Some(f.document.max_tokens + 1),
        ),
    ] {
        let mut uses = Rows::new(&b);
        uses.insert(AnalysisEmbeddingUse {
            invocation: f.invocation.id(),
            window: f.window.id(),
            specification: f.encoder.id(),
            document: f.document.id(),
            input: input_hash(&f.spec.document_text(f.window.text.as_str())),
            availability,
            admitted_tokens: tokens,
            value: None,
            projection: None,
        })
        .unwrap();
        let (invocations, outcomes) = outcomes(&f, &uses, &b);
        f.data.validate(&invocations, &outcomes, &uses, &b).unwrap();
        assert_eq!(
            outcomes.iter().next().unwrap().status,
            analysis::AnalysisStatus::Partial
        );
    }
    let f = fixture(&b, false);
    let uses = Rows::new(&b);
    let (invocations, outcomes) = outcomes(&f, &uses, &b);
    f.data.validate(&invocations, &outcomes, &uses, &b).unwrap();
    assert_eq!(
        outcomes.iter().next().unwrap().status,
        analysis::AnalysisStatus::NotRequested
    );
}
#[test]
fn independent_prefix_math_and_actual_projection_policy_are_checked() {
    let p = project_prefix(&[0.0, 0.6, 0.8], 2).unwrap();
    assert_eq!(p, [0.0, 1.0]);
    assert!(project_prefix(&[0.0, 0.0, 1.0], 2).is_err());
    assert!(project_prefix(&[1.0, 0.0], 3).is_err());
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let f = fixture(&b, true);
    let (full, _, _) = winner(&f, &b);
    let other = ProjectionDefinition {
        dimensions: 4096,
        algorithm: Algorithm::PrefixL2F64F32,
    };
    let projected = ProjectedValue::new(&full, &other).unwrap();
    let mut index = ValueIndex::new(&b);
    index.admit_full(&full, &f.encoder, &f.policy).unwrap();
    assert!(index.admit_projection(&projected).is_err());
}
