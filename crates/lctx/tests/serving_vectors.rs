//! Actual Catalog compilation and disposable PG18 qualify the vector artifact. The HTTP scorer
//! is a controlled numerical seam, not a live embedding model or retrieval-quality receipt.
use lctx_model::domain::{*,embedding::{self,EmbeddingSpec,value},retrieval::consumption::RetrievalEmbeddingUse,resources::ResourceBudget};
use lctx_postgres::{generations::{GenerationId,GenerationStore,GenerationService,GenerationCatalog,ListFilter},roles::RoleConfig,testing::DisposableDatabase};
use std::{io::{BufRead,Read,Write},net::TcpListener,path::Path,process::Command,sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}},time::Duration};

fn write(path:&Path,bytes:impl AsRef<[u8]>) {std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(path,bytes).unwrap();}
fn input(root:&Path) {
    write(&root.join("libraries/demo/pyproject.toml"),"[project]\nname='lctx-library-demo'\nversion='0'\ndependencies=['demo==1.0']\n[tool.lctx]\nrelease=['demo']\n");
    write(&root.join("libraries/demo/.python-version"),"3.14.7\n");
    write(&root.join("libraries/demo/uv.lock"),format!("version=1\nrevision=3\nrequires-python='==3.14.*'\n[[package]]\nname='demo'\nversion='1.0'\nsource={{registry='https://pypi.org/simple'}}\nwheels=[{{url='https://x/demo.whl',hash='sha256:{}',size=1}}]\n[[package]]\nname='lctx-library-demo'\nversion='0'\nsource={{virtual='.'}}\ndependencies=[{{name='demo'}}]\n","a".repeat(64)));
    write(&root.join("envs/demo/pyvenv.cfg"),"home=/x\nuv=0.12.18\nversion_info=3.14.7\n");
    let source=b"__all__=['api','consume']\ndef api(x: int) -> int:\n    \"\"\"Return an API value.\"\"\"\n    return x\n\ndef consume(x: int) -> int:\n    \"\"\"Consume the API value.\"\"\"\n    return api(x)\n";
    let metadata=b"Metadata-Version: 2.4\nName: demo\nVersion: 1.0\n";
    let site=root.join("envs/demo/lib/python3.14/site-packages");
    write(&site.join("demo/__init__.py"),source);write(&site.join("demo-1.0.dist-info/METADATA"),metadata);
    use base64::{Engine as _,engine::general_purpose::URL_SAFE_NO_PAD};use sha2::{Digest as _,Sha256};
    write(&site.join("demo-1.0.dist-info/RECORD"),format!("demo/__init__.py,sha256={},{}\ndemo-1.0.dist-info/METADATA,sha256={},{}\ndemo-1.0.dist-info/RECORD,,\n",URL_SAFE_NO_PAD.encode(Sha256::digest(source)),source.len(),URL_SAFE_NO_PAD.encode(Sha256::digest(metadata)),metadata.len()));
    write(&root.join("libraries/demo/analytics.toml"),"version=1\n[subsystem]\nmodule_prefixes=['demo']\npublic_roots=['demo']\n[seeds]\nprimary=['demo.api']\ndistractors=[]\n[pass_a]\nmax_depth=2\nmax_vertices=256\nmax_edges=1024\nmax_witnesses=4\n[briefs]\nbudget=1\n");
    write(&root.join("bin/uv"),"#!/bin/sh\nexit 0\n");
    use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(root.join("bin/uv"),std::fs::Permissions::from_mode(0o700)).unwrap();
}
fn numerical_vector(dimensions:usize)->Vec<f32> {let mut v=vec![0.0;dimensions];v[0]=1.0;v[1]=-0.0;v}
struct Scorer {address:String,stop:Arc<AtomicBool>,calls:Arc<AtomicUsize>,thread:Option<std::thread::JoinHandle<()>>}
impl Scorer {
    fn start()->Self {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=format!("http://{}",listener.local_addr().unwrap());listener.set_nonblocking(true).unwrap();
        let stop=Arc::new(AtomicBool::new(false));let calls=Arc::new(AtomicUsize::new(0));let done=stop.clone();let observed=calls.clone();
        let thread=std::thread::spawn(move||while !done.load(Ordering::Acquire) {
            let (mut stream,_)=match listener.accept(){Ok(v)=>v,Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{std::thread::sleep(Duration::from_millis(5));continue;},Err(e)=>panic!("{e}")};
            stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let mut reader=std::io::BufReader::new(stream.try_clone().unwrap());let mut line=String::new();reader.read_line(&mut line).unwrap();let path=line.split_whitespace().nth(1).unwrap().to_owned();
            let mut length=None;loop {line.clear();assert!(reader.read_line(&mut line).unwrap()>0);if line=="\r\n" {break;}if let Some(v)=line.to_ascii_lowercase().strip_prefix("content-length:"){length=Some(v.trim().parse::<usize>().unwrap());}}
            let length=length.unwrap();assert!(length<1<<20);let mut body=vec![0;length];reader.read_exact(&mut body).unwrap();let request:serde_json::Value=serde_json::from_slice(&body).unwrap();
            observed.fetch_add(1,Ordering::AcqRel);
            let response=match path.as_str(){
                "/tokenize"=>serde_json::json!({"count":request["prompt"].as_str().unwrap().len().div_ceil(4)}),
                "/v1/embeddings"=>{let dimensions=request["dimensions"].as_u64().unwrap() as usize;let values=numerical_vector(dimensions);serde_json::json!({"model":request["model"],"data":request["input"].as_array().unwrap().iter().enumerate().map(|(index,_)|serde_json::json!({"index":index,"embedding":values})).collect::<Vec<_>>()})},
                other=>panic!("unexpected numerical route {other}"),};
            let bytes=serde_json::to_vec(&response).unwrap();write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();stream.write_all(&bytes).unwrap();
        });
        Self{address,stop,calls,thread:Some(thread)}
    }
}
impl Drop for Scorer {fn drop(&mut self){self.stop.store(true,Ordering::Release);self.thread.take().unwrap().join().unwrap();}}
fn compile(root:&Path,scorer:&Scorer,dimensions:u32)->GenerationId {
    let spec=cpg_core::embedding_service::FakeEmbedder::new();
    use cpg_core::embedding_service::Embedder;
    let mut selected=spec.spec().clone();selected.model="controlled-v0-numerical-seam".into();selected.server="controlled-http".into();selected.dimensions=dimensions;selected.source_dimensions=dimensions;
    // The spec is explicit: these known vectors do not claim the deterministic fake model's output.
    write(&root.join("vector-spec.json"),selected.canonical_json());
    let output=Command::new(env!("CARGO_BIN_EXE_lctx")).args(["compile","demo","--through","catalog","--profile","catalog","--embedder","vllm","--techniques","+knn"])
        .arg("--embedding-endpoint").arg(&scorer.address).arg("--embedding-spec").arg(root.join("vector-spec.json"))
        .arg("--database").arg(root.join("postgres.json")).arg("--libraries").arg(root.join("libraries")).arg("--envs").arg(root.join("envs")).arg("--sources").arg(root.join("sources"))
        .env("PATH",format!("{}:{}",root.join("bin").display(),std::env::var("PATH").unwrap_or_default())).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let report:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(report["selected"],false);GenerationId::from_hex(report["generation"].as_str().unwrap()).unwrap()
}
fn prepare_cli(root:&Path,generation:GenerationId)->std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lctx")).arg("--database").arg(root.join("postgres.json")).args(["serving","prepare","--generation",&generation.hex()]).output().unwrap()
}
async fn vector_count(db:&DisposableDatabase)->i64 {sqlx::query_scalar("SELECT count(*) FROM lctx_cache.serving_vectors").fetch_one(&db.superuser).await.unwrap()}

#[tokio::test]
async fn explicit_preparation_admits_exact_canonical_vectors_refuses_corruption_and_cleans_up() {
    let db=DisposableDatabase::start().await;db.migrate().await;let model=Arc::new(lctx_model::domain::model().unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let dir=tempfile::tempdir().unwrap();input(dir.path());db.write_configs(dir.path()).unwrap();let scorer=Scorer::start();let g=compile(dir.path(),&scorer,1024);
    let role=RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
    let service=GenerationService::admit(model.clone(),&role,Some(g)).await.unwrap();
    let guard=service.guard();assert!(guard.vector_artifact().await.is_err());assert_eq!(vector_count(&db).await,0,"read-only admission must not prepare artifacts");
    let mut lease=store.pin(&db.reader,g,ResourceBudget::fixed(128<<20).unwrap()).await.unwrap();
    let specifications=lease.read::<EmbeddingSpec>().await.unwrap();let specification=specifications.rows()[0].configuration().unwrap();
    let retrieval=lease.read::<RetrievalEmbeddingUse>().await.unwrap();let analytic=lease.read::<embedding::analytic::AnalysisEmbeddingUse>().await.unwrap();
    let uses:Vec<_>=retrieval.rows().iter().filter(|r|r.availability==embedding::analytic::VectorAvailability::Available).cloned().collect();
    assert!(uses.len()>1);assert!(analytic.rows().iter().any(|r|r.availability==embedding::analytic::VectorAvailability::Available));
    for bytes in uses.iter().map(|r|r.bytes.as_ref().unwrap()).chain(analytic.rows().iter().filter_map(|r|r.bytes.as_ref())) {
        let decoded=value::decode_vector(&bytes.0,1024).unwrap();assert_eq!(decoded[0].to_bits(),1.0f32.to_bits());assert_eq!(decoded[1].to_bits(),(-0.0f32).to_bits());
    }
    drop(specifications);drop(retrieval);drop(analytic);lease.release().await.unwrap();
    let calls=scorer.calls.load(Ordering::Acquire);let prepared=prepare_cli(dir.path(),g);assert!(prepared.status.success(),"{}",String::from_utf8_lossy(&prepared.stderr));
    assert_eq!(scorer.calls.load(Ordering::Acquire),calls,"preparation must not call an embedding service");
    let artifact=guard.vector_artifact().await.unwrap();let report:serde_json::Value=serde_json::from_slice(&prepared.stdout).unwrap();assert_eq!(report["artifact_key"],artifact.key.hex());assert_eq!(report["selected"],false);
    assert_eq!(artifact.specification,specification);assert_eq!(vector_count(&db).await,uses.len() as i64);
    let readback:Vec<f32>=sqlx::query_scalar("SELECT value::real[] FROM lctx_cache.serving_vectors WHERE artifact_key=$1 AND retrieval_use=$2").bind(artifact.key.0.to_vec()).bind(uses[0].id().bytes().to_vec()).fetch_one(&db.reader).await.unwrap();
    assert_eq!(value::encode_vector(&readback),uses[0].bytes.as_ref().unwrap().0);
    assert!(GenerationCatalog::new(db.reader.clone()).list(&ListFilter::default()).await.unwrap().iter().all(|g|!g.selected));
    assert!(store.prepare_vector_artifact(&db.reader,g,ResourceBudget::fixed(1024).unwrap()).await.is_err());
    assert_eq!(vector_count(&db).await,uses.len() as i64,"resource refusal must not replace an existing artifact");
    let execution=service.execution().await.unwrap();let mut eligible:Vec<_>=uses.iter().map(Record::id).collect();eligible.reverse();
    let scores=artifact.score(&execution,eligible.clone(),numerical_vector(1024)).await.unwrap();assert_eq!(scores.values().len(),eligible.len());assert!(scores.values().iter().all(|(_,score)|(*score-1.0).abs()<1e-12));assert!(scores.values().windows(2).all(|w|w[0].0<w[1].0));drop(scores);
    let mut opposite=numerical_vector(1024);opposite[0]=-1.0;assert!(artifact.score(&execution,eligible.clone(),opposite).await.unwrap().values().iter().all(|(_,score)|(*score+1.0).abs()<1e-12));
    let mut orthogonal=numerical_vector(1024);orthogonal[0]=0.0;orthogonal[1]=1.0;assert!(artifact.score(&execution,eligible.clone(),orthogonal).await.unwrap().values().iter().all(|(_,score)|score.abs()<1e-12));
    assert!(artifact.score(&execution,vec![eligible[0],eligible[0]],numerical_vector(1024)).await.is_err());
    let foreign:Id<RetrievalEmbeddingUse>=serde_json::from_value(serde_json::to_value([255u8;16]).unwrap()).unwrap();assert!(artifact.score(&execution,vec![foreign],numerical_vector(1024)).await.is_err());
    for query in [vec![0.0;1024],vec![f32::NAN;1024],vec![f32::INFINITY;1024],vec![1.0;512]] {assert!(artifact.score(&execution,eligible.clone(),query).await.is_err());}
    let denied=sqlx::query("DELETE FROM lctx_cache.serving_vectors WHERE artifact_key=$1").bind(artifact.key.0.to_vec()).execute(&db.reader).await;assert!(denied.is_err());
    // A separately admitted service for the same generation does not revive the original guard.
    let other=GenerationService::admit(model.clone(),&role,Some(g)).await.unwrap();let other_execution=other.execution().await.unwrap();assert!(artifact.score(&other_execution,eligible.clone(),numerical_vector(1024)).await.is_err());drop(other_execution);other.shutdown().await.unwrap();
    // Wrong-but-valid numerical content is not trusted even by an already admitted artifact.
    let mut changed=numerical_vector(1024);changed[1]=0.0;
    sqlx::query("UPDATE lctx_cache.serving_vectors SET value=$3::real[]::lctx_ext.vector WHERE artifact_key=$1 AND retrieval_use=$2").bind(artifact.key.0.to_vec()).bind(uses[0].id().bytes().to_vec()).bind(changed).execute(&db.superuser).await.unwrap();
    assert!(guard.vector_artifact().await.is_err());assert!(artifact.score(&execution,eligible.clone(),numerical_vector(1024)).await.is_err());
    assert!(prepare_cli(dir.path(),g).status.success());assert_eq!(guard.vector_artifact().await.unwrap().key,artifact.key);
    sqlx::query("DELETE FROM lctx_cache.serving_vectors WHERE artifact_key=$1 AND retrieval_use=$2").bind(artifact.key.0.to_vec()).bind(uses[0].id().bytes().to_vec()).execute(&db.superuser).await.unwrap();
    assert!(guard.vector_artifact().await.is_err());assert!(artifact.score(&execution,eligible.clone(),numerical_vector(1024)).await.is_err());assert!(prepare_cli(dir.path(),g).status.success());
    sqlx::query("INSERT INTO lctx_cache.serving_vectors(artifact_key,retrieval_use,value) VALUES($1,$2,$3::real[]::lctx_ext.vector)").bind(artifact.key.0.to_vec()).bind(foreign.bytes().to_vec()).bind(numerical_vector(1024)).execute(&db.superuser).await.unwrap();
    assert!(guard.vector_artifact().await.is_err());assert!(prepare_cli(dir.path(),g).status.success());
    for field in ["generation_id","model_digest","source_content","spec_hash","physical_digest"] {
        let original:Vec<u8>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT {field} FROM lctx_cache.serving_vector_artifacts WHERE artifact_key=$1"))).bind(artifact.key.0.to_vec()).fetch_one(&db.superuser).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE lctx_cache.serving_vector_artifacts SET {field}=$2 WHERE artifact_key=$1"))).bind(artifact.key.0.to_vec()).bind(vec![99u8;original.len()]).execute(&db.superuser).await.unwrap();assert!(guard.vector_artifact().await.is_err());
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE lctx_cache.serving_vector_artifacts SET {field}=$2 WHERE artifact_key=$1"))).bind(artifact.key.0.to_vec()).bind(original).execute(&db.superuser).await.unwrap();
    }
    sqlx::query("ALTER TABLE lctx_cache.serving_vectors ADD COLUMN forged integer").execute(&db.superuser).await.unwrap();assert!(guard.vector_artifact().await.is_err());assert!(!prepare_cli(dir.path(),g).status.success());sqlx::query("ALTER TABLE lctx_cache.serving_vectors DROP COLUMN forged").execute(&db.superuser).await.unwrap();
    sqlx::query("GRANT INSERT ON lctx_cache.serving_vectors TO lctx_serving").execute(&db.superuser).await.unwrap();assert!(guard.vector_artifact().await.is_err());sqlx::query("REVOKE INSERT ON lctx_cache.serving_vectors FROM lctx_serving").execute(&db.superuser).await.unwrap();
    sqlx::query("DROP INDEX lctx_cache.serving_vector_generation").execute(&db.superuser).await.unwrap();assert!(guard.vector_artifact().await.is_err());sqlx::query("CREATE INDEX serving_vector_generation ON lctx_cache.serving_vector_artifacts(generation_id)").execute(&db.superuser).await.unwrap();
    sqlx::query("UPDATE pg_extension SET extversion='0.8.7' WHERE extname='vector'").execute(&db.superuser).await.unwrap();assert!(guard.vector_artifact().await.is_err());assert!(!prepare_cli(dir.path(),g).status.success());sqlx::query("UPDATE pg_extension SET extversion='0.8.6' WHERE extname='vector'").execute(&db.superuser).await.unwrap();
    // Corrupt canonical input refuses; explicit preparation does not repair it from its old cache.
    let table=format!("{}.{}",g.schema(),RetrievalEmbeddingUse::NAME);
    for bytes in [value::encode_vector(&vec![0.0;1024]),value::encode_vector(&vec![f32::INFINITY;1024])] {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET bytes=$2 WHERE id=$1"))).bind(uses[0].id().bytes().to_vec()).bind(bytes).execute(&db.superuser).await.unwrap();assert!(!prepare_cli(dir.path(),g).status.success());assert!(guard.vector_artifact().await.is_err());assert_eq!(vector_count(&db).await,uses.len() as i64);
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET bytes=$2 WHERE id=$1"))).bind(uses[0].id().bytes().to_vec()).bind(uses[0].bytes.as_ref().unwrap().0.clone()).execute(&db.superuser).await.unwrap();
    }
    drop(execution);
    // Actual blocked scoring retains its query grant and reservations after caller cancellation.
    let baseline=service.memory_reserved();let mut lock=db.owner.pool().begin().await.unwrap();sqlx::query("LOCK TABLE lctx_cache.serving_vectors IN ACCESS EXCLUSIVE MODE").execute(&mut *lock).await.unwrap();
    let executing=service.execution().await.unwrap();let retained=artifact.clone();let ids=eligible.clone();let task=tokio::spawn(async move{retained.score(&executing,ids,numerical_vector(1024)).await});
    tokio::time::timeout(Duration::from_secs(10),async{loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation='lctx_cache.serving_vectors'::regclass AND NOT granted)").fetch_one(&db.superuser).await.unwrap();if waiting {break;}tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();
    task.abort();assert!(task.await.is_err_and(|e|e.is_cancelled()));assert!(service.memory_reserved()>baseline);lock.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(10),async{while service.memory_reserved()!=baseline {tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();
    let next=service.execution().await.unwrap();assert_eq!(artifact.score(&next,eligible,numerical_vector(1024)).await.unwrap().values().len(),uses.len());drop(next);
    let unsupported=compile(dir.path(),&scorer,512);assert!(!prepare_cli(dir.path(),unsupported).status.success());
    let lower=GenerationService::admit(model,&role,Some(unsupported)).await.unwrap();let foreign_execution=lower.execution().await.unwrap();assert!(artifact.score(&foreign_execution,vec![uses[0].id()],numerical_vector(1024)).await.is_err());drop(foreign_execution);lower.shutdown().await.unwrap();
    assert!(matches!(store.retire(g).await,Err(lctx_postgres::generations::Error::Busy)));service.shutdown().await.unwrap();drop(artifact);drop(guard);
    store.retire(g).await.unwrap();assert_eq!(vector_count(&db).await,0);let manifests:i64=sqlx::query_scalar("SELECT count(*) FROM lctx_cache.serving_vector_artifacts WHERE generation_id=$1").bind(g.bytes().to_vec()).fetch_one(&db.superuser).await.unwrap();assert_eq!(manifests,0);store.retire(unsupported).await.unwrap();
    assert!(GenerationCatalog::new(db.reader.clone()).list(&ListFilter::default()).await.unwrap().is_empty());
}
