use std::sync::Arc;
use lctx_model::{Domain,domain::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

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
    let (image,tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PostgreSQL18 required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap(); let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Node>(),Relation::of::<Step>(),Relation::of::<SelfStep>(),Relation::of::<LeftStep>(),Relation::of::<RightStep>()]).unwrap());
    let store = GenerationStore::install(owner,model.clone()).await.unwrap();
    let a = Node { name: "a".into(),next: None }; let b = Node { name: "b".into(),next: None }; let c = Node { name: "c".into(),next: None };
    for case in 0..4 {
        let cyclic = case != 0;
        let a = Node { next: Some(b.id()),..a.clone() };
        let step = Step { name: "step".into(),result: b.id(),previous: if case == 1 { a.id() } else { c.id() } };
        let g = store.create_conformance(ContentHash::of(b"proof-fixture"),"behavioral").await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,vec![a.clone(),b.clone(),c.clone()]).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,vec![step.clone()]).unwrap()).await.unwrap();
        let own = SelfStep { name: "s".into(),result: a.id(),previous: None };
        let left = LeftStep { name: "l".into(),result: a.id(),previous: None };
        let right = RightStep { name: "r".into(),result: c.id(),previous: None };
        let own = SelfStep { previous: (case == 2).then_some(own.id()),..own };
        let left = LeftStep { previous: Some(right.id()),..left };
        let right = RightStep { previous: (case == 3).then_some(left.id()),..right };
        store.copy(&writer,g,&Batch::new(&model,vec![own]).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,vec![left]).unwrap()).await.unwrap();
        store.copy(&writer,g,&Batch::new(&model,vec![right]).unwrap()).await.unwrap();
        let select = format!("SELECT conclusion_relation,conclusion_id FROM {}.derivations WHERE source_relation='proof_steps'",g.schema());
        assert!(sqlx::query(sqlx::AssertSqlSafe(select.clone())).fetch_all(&reader).await.is_err());
        store.seal(g).await.unwrap();
        if cyclic {
            let error = store.validate(g).await.unwrap_err(); assert!(matches!(error,Error::Model(_)) && error.to_string().contains("cyclic"),"{error}");
            assert!(store.publish(g).await.is_err()); store.abort(g).await.unwrap();
        } else {
            store.validate(g).await.unwrap(); store.publish(g).await.unwrap();
            let _lease = store.pin(&reader,g).await.unwrap();
            let row: (String,Vec<u8>) = sqlx::query_as(sqlx::AssertSqlSafe(select)).fetch_one(&reader).await.unwrap();
            assert_eq!(row,(Node::NAME.into(),b.id().bytes().to_vec()));
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.derivation_premises",g.schema()))).fetch_one(&reader).await.unwrap();
            assert_eq!(count,3,"absent optional premises do not fabricate edges");
        }
    }
}
