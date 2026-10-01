//! Model callbacks run before ordinary or grouped publication grants and acknowledgements.
use lctx_model::{Domain, domain::{*, normalized::Rows, resources::ResourceBudget, stages::*, value::Literal}};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="publication_audits", publication_checks=checks, semantic_source=include_bytes!("publication_checks.rs"))]
struct Audit {
    #[model(key)]
    content: ContentHash,
    rows: i64,
    behavioral: bool,
}
fn checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        name: "publication_audit_sources",
        inputs: vec![ValidationInput::of::<Audit>(&["id"]), ValidationInput::of::<Literal>(&["id"])],
        create: Arc::new(|budget| Box::new(Check { audits: Rows::new(budget), literals: Rows::new(budget) })),
    }]
}
struct Check { audits: Rows<Audit>, literals: Rows<Literal> }
impl PublicationCheck for Check {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        match relation {
            Audit::NAME => self.audits.decode(batch),
            Literal::NAME => self.literals.decode(batch),
            _ => Err(ModelError::Invalid("unexpected publication input".into())),
        }
    }
    fn finish(self: Box<Self>, sources: &[CompletedRelation], profile: Profile) -> Result<(), ModelError> {
        let source = sources.iter().find(|s|s.relation()==Literal::NAME)
            .ok_or_else(||ModelError::Invalid("missing acknowledged source".into()))?;
        let receipt = source.receipt();
        if sources.len()!=1 || self.audits.len()!=1 || self.literals.len() as u64!=receipt.rows
            || self.audits.iter().any(|row|row.content!=receipt.content || u64::try_from(row.rows).ok()!=Some(receipt.rows) || row.behavioral!=(profile==Profile::Behavioral)) {
            return Err(ModelError::Invalid("publication source snapshot differs".into()));
        }
        Ok(())
    }
}
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {name,inputs,outputs,contributes:vec![],coverage:vec![],provider:None,profiles:Profile::ALL.to_vec(),effect:Effect::Pure,code:ContentHash::of(b"publication callback control"),configuration:ContentHash::of(b"test")}
}

#[tokio::test]
async fn publication_callbacks_bind_old_sources_and_refuse_raw_forgery_before_acknowledgement() {
    for profile in Profile::ALL {
      for grouped in [false,true] {
        for case in 0..4 {
            let forged = case==1;
            let undeclared = case==2;
            let wrong_profile = case==3;
            let db=DisposableDatabase::start().await;
            let model=Arc::new(ValidatedModel::validate(vec![Relation::of::<Literal>(),Relation::of::<Audit>()]).unwrap());
            let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
            let schedule=Schedule::build_with_publications(&model,vec![
                stage("facts",vec![],vec![RelationUse::of::<Literal>()]),
                stage("later",vec![],vec![RelationUse::of::<Literal>()]),
                stage("audit",if undeclared {vec![]}else{vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Facts)]},vec![RelationUse::of::<Audit>()]),
            ],&[],profile,vec![
                PublicationGroup::new(PublicationBoundary::Facts,vec!["facts"]),
                PublicationGroup::new(PublicationBoundary::Dispatch,if grouped {vec!["later","audit"]}else{vec!["later"]}),
            ]).unwrap();
            let budget=ResourceBudget::fixed(64<<20).unwrap();
            let mut execution=schedule.execute();
            let attempt=store.begin_conformance(db.writer.clone(),&mut execution,budget.clone()).await.unwrap();
            let generation=attempt.generation();
            for (name,value) in [("facts",Literal::None),("later",Literal::Bool{value:true})] {
                let mut access=execution.begin(name).unwrap();
                let batch=Batch::new(&model,vec![value],&budget).unwrap();
                access.write::<Literal,_>(async|permit|attempt.copy(permit,&batch).await).await.unwrap();
                access.complete(&attempt,ProviderOutcome::Complete).await.unwrap();
            }
            let mut access=execution.begin("audit").unwrap();
            let content=if undeclared {ContentHash::of(b"not an input grant")}else{access.read::<Literal>().unwrap().source().unwrap().receipt().content};
            let batch=Batch::new(&model,vec![Audit{content,rows:1,behavioral:profile==Profile::Behavioral}],&budget).unwrap();
            access.write::<Audit,_>(async|permit|attempt.copy(permit,&batch).await).await.unwrap();
            if forged || wrong_profile {
                // Locate the private table by its declared columns, independently of name hashing.
                let physical: String = if grouped {
                    sqlx::query_scalar("SELECT table_name FROM information_schema.columns WHERE table_schema=$1 AND table_name LIKE '__delta_%' AND column_name='rows'")
                        .bind(generation.schema()).fetch_one(&db.superuser).await.unwrap()
                } else { Audit::NAME.to_owned() };
                // A raw mutation bypasses typed construction; lifecycle validation must still refuse it.
                let change=if wrong_profile {"behavioral=NOT behavioral"}else{"\"rows\"=2"};
                sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {}.\"{physical}\" SET {change}",generation.schema())))
                    .execute(&db.superuser).await.unwrap();
            }
            let result=access.complete(&attempt,ProviderOutcome::Complete).await;
            if forged || undeclared || wrong_profile {
                assert!(result.is_err(),"invalid publication was acknowledged (grouped={grouped}, case={case})");
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex') AND stage_name='audit'")
                    .bind(generation.hex()).fetch_one(&db.superuser).await.unwrap();
                assert_eq!(count,0);
                if grouped {
                    let count:i64=sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex') AND stage_name='later'")
                        .bind(generation.hex()).fetch_one(&db.superuser).await.unwrap();
                    assert_eq!(count,0,"sibling result was not rolled back");
                    let count:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.{}",generation.schema(),Literal::NAME)))
                        .fetch_one(&db.superuser).await.unwrap();
                    assert_eq!(count,1,"candidate vocabulary was not rolled back");
                }
                assert!(sqlx::query(sqlx::AssertSqlSafe(format!("SELECT * FROM {}.{}",generation.schema(),Audit::NAME))).fetch_all(&db.writer).await.is_err());
                assert!(execution.finish().is_err());
                attempt.abort().await.unwrap();
            } else {
                result.unwrap();
                attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap().publish().await.unwrap();
                store.retire(generation).await.unwrap();
            }
        }
    }
}
}
