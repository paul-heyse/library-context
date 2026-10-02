#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{serving::*, source::SourceArtifact, *};
use support::ServingFixture;

#[tokio::test]
async fn original_unicode_bytes_page_and_corrupt_capture_are_checked() {
    let source = format!(
        "\"\"\"{}\"\"\"\ndef api(value: str) -> str:\n    return value\n",
        "🦀 café\n".repeat(700)
    )
    .into_bytes();
    // Challenge an admitted BaseEvaluation parent; Local intentionally has no earlier
    // analysis parents. Behavioral requests the evaluation rather than NotRequested output.
    let fixture = ServingFixture::start_profile(&source, "behavioral").await;
    let execution = fixture.service.execution().await.unwrap();
    let digest = ContentHash::of(&source);
    let artifact = execution
        .query(move |lease| {
            Box::pin(async move {
                let rows = lease.read::<SourceArtifact>().await?;
                Ok(rows
                    .rows()
                    .iter()
                    .find(|r| r.content == digest)
                    .expect("captured original source")
                    .id())
            })
        })
        .await
        .unwrap();
    let mut request = GetEvidenceRequest {
        source: OriginalReference::Artifact { artifact },
        page: PageRequest::default(),
    };
    let mut bytes = Vec::new();
    loop {
        let response = fixture
            .service
            .evidence(&execution, &request)
            .await
            .unwrap();
        let packet = response.evidence;
        assert_eq!(packet.original.digest, digest);
        assert_eq!(packet.original.encoding.as_str(), "raw_bytes");
        assert_eq!(packet.body.start as usize, bytes.len());
        bytes.extend(packet.body.bytes);
        assert_eq!(packet.body.end as usize, bytes.len());
        assert_eq!(packet.body.omitted as usize, source.len() - bytes.len());
        assert_eq!(packet.body.truncated, bytes.len() < source.len());
        match packet.body.continuation.0 {
            Some(cursor) => request.page.cursor = Optional(Some(cursor)),
            None => break,
        }
    }
    assert_eq!(bytes, source);
    let inputs = execution
        .read::<analysis::base_evaluation::AnalysisInput>()
        .await
        .unwrap();
    let input = inputs
        .rows()
        .first()
        .expect("actual admitted BaseEvaluation parent input");
    let root = ProofReference::from_canonical(derivation::RowRef::of(input.invocation));
    let proof = fixture
        .service
        .explanation(&execution, root.clone())
        .await
        .unwrap();
    assert!(proof.items.iter().any(|s| {
        s.source.row == *input.id().bytes()
            && s.premises
                .iter()
                .any(|p| p.role.as_str() == "parent" && p.premise.row == *input.parent.bytes())
    }));
    // A damaged browsing projection cannot hide canonical support from the explanation owner.
    let schema = fixture.generation.schema();
    let definition: String = sqlx::query_scalar("SELECT pg_get_viewdef(to_regclass($1),false)")
        .bind(format!("\"{schema}\".derivations"))
        .fetch_one(&fixture.db.superuser)
        .await
        .unwrap();
    let hidden = format!(
        "CREATE OR REPLACE VIEW \"{schema}\".derivations AS SELECT * FROM ({}) original WHERE false",
        definition.trim().trim_end_matches(';')
    );
    sqlx::query(sqlx::AssertSqlSafe(hidden))
        .execute(&fixture.db.superuser)
        .await
        .unwrap();
    assert_eq!(
        fixture.service.explanation(&execution, root).await.unwrap(),
        proof
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE OR REPLACE VIEW \"{schema}\".derivations AS {definition}"
    )))
    .execute(&fixture.db.superuser)
    .await
    .unwrap();
    // Changing the nominal original while retaining its cursor is rejected.
    request.page.cursor = Optional(Some(
        Cursor {
            binding: CursorBinding {
                generation: GenerationKey([99; 16]),
                request: Request::GetEvidence(request.clone())
                    .canonical_identity()
                    .unwrap(),
                policy: ranking::RankingPolicy::default().identity().unwrap(),
                wire: wire_identity(),
                channels: ChannelState {
                    lexical: false,
                    vector: VectorChannel::Disabled {},
                }
                .identity(),
                group: Name::new("evidence").unwrap(),
                section: Name::new("bytes").unwrap(),
                member: None,
                ordering: ContentHash::of(b"original-byte-offset/v1"),
            },
            offset: 0,
        }
        .encode()
        .unwrap(),
    ));
    assert!(
        fixture
            .service
            .evidence(&execution, &request)
            .await
            .is_err()
    );
    request.page.cursor = Optional::default();
    // Captured chunks retain canonical IDs but their altered bytes cannot pass the owner digest.
    let schema = fixture.generation.schema();
    let query = format!(
        "UPDATE \"{schema}\".artifact_chunks SET body=decode('616263','hex') WHERE artifact=$1"
    );
    sqlx::query(sqlx::AssertSqlSafe(query))
        .bind(artifact.bytes().to_vec())
        .execute(&fixture.db.superuser)
        .await
        .unwrap();
    assert!(
        fixture
            .service
            .evidence(&execution, &request)
            .await
            .is_err()
    );
    drop(execution);
    fixture.finish().await;
}
