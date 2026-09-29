use std::sync::Arc;
use lctx_model::{Domain,domain::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::Harness;
use lctx_postgres::testing::DisposableDatabase;

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "proof_nodes",rule = "follows",semantic_source = b"proof fixture v1")]
struct Node { #[model(key)] name: String,#[model(premise)] next: Option<Id<Node>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "proof_steps",rule = "step",conclusion = result,semantic_source = b"proof fixture v1")]
struct Step { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Id<Node> }


#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "self_steps",rule = "self_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct SelfStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<SelfStep>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "left_steps",rule = "left_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct LeftStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<RightStep>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "right_steps",rule = "right_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct RightStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<LeftStep>> }

#[tokio::test]
async fn sealed_derivations_refuse_cross_source_cycles_and_expose_only_nominal_targets() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone(); let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Node>(),Relation::of::<Step>(),Relation::of::<SelfStep>(),Relation::of::<LeftStep>(),Relation::of::<RightStep>()]).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let a = Node { name: "a".into(),next: None }; let b = Node { name: "b".into(),next: None }; let c = Node { name: "c".into(),next: None };
    for case in 0..4 {
        let cyclic = case != 0;
        let a = Node { next: Some(b.id()),..a.clone() };
        let step = Step { name: "step".into(),result: b.id(),previous: if case == 1 { a.id() } else { c.id() } };
        let mut g_h = Harness::begin(&store, writer.clone(), lctx_model::domain::stages::Profile::Behavioral, budget()).await.unwrap(); let g = g_h.generation();
        g_h.copy(&Batch::new(&model,vec![a.clone(),b.clone(),c.clone()], &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,vec![step.clone()], &budget()).unwrap(), &budget()).await.unwrap();
        let own = SelfStep { name: "s".into(),result: a.id(),previous: None };
        let left = LeftStep { name: "l".into(),result: a.id(),previous: None };
        let right = RightStep { name: "r".into(),result: c.id(),previous: None };
        let own = SelfStep { previous: (case == 2).then_some(own.id()),..own };
        let left = LeftStep { previous: Some(right.id()),..left };
        let right = RightStep { previous: (case == 3).then_some(left.id()),..right };
        g_h.copy(&Batch::new(&model,vec![own], &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,vec![left], &budget()).unwrap(), &budget()).await.unwrap();
        g_h.copy(&Batch::new(&model,vec![right], &budget()).unwrap(), &budget()).await.unwrap();
        let select = format!("SELECT conclusion_relation,conclusion_id FROM {}.derivations WHERE source_relation='proof_steps'",g.schema());
        assert!(sqlx::query(sqlx::AssertSqlSafe(select.clone())).fetch_all(&reader).await.is_err());
        g_h.seal().await.unwrap();
        if cyclic {
            let error = g_h.validate(&budget()).await.unwrap_err(); assert!(matches!(error,Error::Model(_)) && error.to_string().contains("cyclic"),"{error}");
            assert!(g_h.publish().await.is_err()); g_h.abort().await.unwrap();
        } else {
            g_h.validate(&budget()).await.unwrap(); g_h.publish().await.unwrap();
            let _lease = store.pin(&reader,g, budget()).await.unwrap();
            let row: (String,Vec<u8>) = sqlx::query_as(sqlx::AssertSqlSafe(select)).fetch_one(&reader).await.unwrap();
            assert_eq!(row,(Node::NAME.into(),b.id().bytes().to_vec()));
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.derivation_premises",g.schema()))).fetch_one(&reader).await.unwrap();
            assert_eq!(count,3,"absent optional premises do not fabricate edges");
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
