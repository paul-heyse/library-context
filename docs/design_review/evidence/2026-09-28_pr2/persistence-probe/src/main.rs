use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
#[salsa::input(persist,singleton)]
struct Input { value:String }
#[salsa::tracked(persist)]
fn persistent(db:&dyn salsa::Database)->String {Input::get(db).value(db).to_uppercase()}
#[salsa::tracked]
fn transient(db:&dyn salsa::Database)->String {Input::get(db).value(db).to_lowercase()}
#[salsa::db]
#[derive(Clone)]
struct Db {storage:salsa::Storage<Self>, executions:Arc<AtomicUsize>}
#[salsa::db]
impl salsa::Database for Db {}
impl Default for Db {
 fn default()->Self {
  let executions=Arc::new(AtomicUsize::new(0));let sink=executions.clone();
  Self{storage:salsa::Storage::new(Some(Box::new(move|event|{if matches!(event.kind,salsa::EventKind::WillExecute{..}){sink.fetch_add(1,Ordering::Relaxed);}}))),executions}
 }
}
fn restore(raw:&str,expected:&str)->Result<Db,String>{
 let envelope:serde_json::Value=serde_json::from_str(raw).map_err(|e|e.to_string())?;
 if envelope["compatibility"]!=expected{return Err("incompatible ownership/pin/source envelope".into())}
 let mut db=Db::default();
 <dyn salsa::Database>::deserialize(&mut db,&mut serde_json::Deserializer::from_str(envelope["database"].as_str().ok_or("missing original serialization")?)).map_err(|e|e.to_string())?;
 Ok(db)
}
fn main(){
 let mut db=Db::default();Input::new(&db,"Pinned".into());
 assert_eq!(persistent(&db),"PINNED");assert_eq!(transient(&db),"pinned");
 let database=serde_json::to_string(&<dyn salsa::Database>::as_serialize(&mut db)).unwrap();
 let raw=serde_json::json!({"compatibility":"salsa-0.28.2/probe-format-1/source-A","database":database}).to_string();
 let restored=restore(&raw,"salsa-0.28.2/probe-format-1/source-A").unwrap();
 assert_eq!(persistent(&restored),"PINNED");assert_eq!(restored.executions.load(Ordering::Relaxed),0);
 assert_eq!(transient(&restored),"pinned");assert_eq!(restored.executions.load(Ordering::Relaxed),1);
 assert!(restore(&raw,"salsa-0.28.4/probe-format-1/source-A").is_err());
 assert!(restore(&raw,"salsa-0.28.2/probe-format-1/source-B").is_err());
 println!("{}",serde_json::json!({"salsa":"0.28.2","roundtrip":"passed","persisted_executions_after_restore":0,"transient_executions_after_restore":1,"changed_pin_or_source":"refused_by_owner_envelope","bytes":raw.len(),"scope":"isolated DTO persistence; no production database serialization"}));
}
