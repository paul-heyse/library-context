//! Actual native candidate queries in a private schema fixture, independent of graph admission.
use lctx_model::domain::{*,graph::{Entity,EntityId,Target},catalog::CatalogMember,serving::{*,ranking::RankingPolicy},retrieval::{Family,Unit,Fragment},attribution::AnalysisContext};
use lctx_surrealdb::{Credentials,NativeReader,Loader,reader,loader::json_value};
use surrealdb::types::{Variables,Value,RecordId,Bytes};
fn id<R:Record>(byte:u8)->Id<R>{serde_json::from_value(serde_json::to_value([byte;16]).unwrap()).unwrap()}
async fn insert(reader:&NativeReader,table:&str,relation:bool,rows:Vec<Value>){let mut vars=Variables::new();vars.insert("rows",rows);reader.query::<Value>(format!("INSERT {}INTO {table} $rows RETURN NONE;",if relation{"RELATION "}else{""}),vars).await.unwrap();}
fn object(body:serde_json::Value,id:RecordId)->surrealdb::types::Object{let Value::Object(mut obj)=json_value(body).unwrap()else{panic!("object")};obj.insert("id",id);obj}
fn occurrence(table:&str,key:&str,source:RecordId,out:RecordId,input:[u8;16],member:Option<Id<CatalogMember>>,context:Id<AnalysisContext>,unit:Id<Unit>)->Value{
 let mut obj=object(serde_json::json!({"family":Family::ApiOptions as i16,"unit":unit,"fragment":id::<Fragment>(7),"context":context,"member":member,"anchor":null,"input":input,"eligible":true,"occurrence_key":key}),RecordId::new(table,key.to_owned()));obj.insert("in",source);obj.insert("out",out);Value::Object(obj)
}
#[tokio::test]
async fn native_channels_admit_exact_context_pairs_and_members_before_candidate_caps(){
 let cfg:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable SurrealDB fixture required")).unwrap()).unwrap();
 let credentials=Credentials::Root{username:cfg["admin_user"].as_str().unwrap().into(),password:cfg["admin_password"].as_str().unwrap().into()};let ns="gn_search_controls";let db=format!("channels_{}",std::process::id());
 let client=reader::connect(cfg["grpc_endpoint"].as_str().unwrap(),&credentials,ns,&db).await.unwrap();client.query(format!("DEFINE NAMESPACE IF NOT EXISTS {ns}; DEFINE DATABASE OVERWRITE {db} STRICT;")).await.unwrap().check().unwrap();
 let loader=Loader::new(client.clone());loader.install(&lctx_serving::native_definitions()).await.unwrap();
 let input=id::<input::InputRevision>(1);let member=CatalogMember{input,access:id::<source::Module>(2),path:vec!["connect".into()],name:"connect".into()};let out=reader::target_id(Target::Entity(EntityId::of(member.id())));loader.entities(&[Entity::from(member.clone())]).await.unwrap();
 let handle=SnapshotHandle{semantic:ContentHash::of(b"private search fixture"),realization:lctx_surrealdb::schema::realization_identity(&lctx_serving::native_definitions()),database:DatabaseIdentity{namespace:Name::new(ns).unwrap(),database:Name::new(&db).unwrap()}};let native=NativeReader::new(client.clone(),handle);let good=id::<AnalysisContext>(3);let other=id::<AnalysisContext>(4);let unit=id::<Unit>(5);let inputs=[*input.bytes()];let pairs=[(*member.id().bytes(),*good.bytes())];
 let mut docs=vec![];let mut occurrences=vec![];
 // More than the native document cap of nonmember occurrences cannot crowd the member out.
 for n in 0..102{let key=format!("doc{n:03}");let text=if n==101{"connect eligible member with deliberately longer descriptive text".to_owned()}else{format!("connect connect connect {n}")};let doc=RecordId::new("search_api_options",key.clone());docs.push(Value::Object(object(serde_json::json!({"text":text,"digest":ContentHash::of(text.as_bytes())}),doc.clone())));occurrences.push(occurrence("lex_occurs",&key,doc,out.clone(),*input.bytes(),if n==101{Some(member.id())}else{None},good,unit));}
 let bad=RecordId::new("search_api_options","other_context");docs.push(Value::Object(object(serde_json::json!({"text":"connect connect connect","digest":ContentHash::of(b"connect connect connect")}),bad.clone())));occurrences.push(occurrence("lex_occurs","other_context",bad,out.clone(),*input.bytes(),Some(member.id()),other,unit));
 insert(&native,"search_api_options",false,docs).await;insert(&native,"lex_occurs",true,occurrences).await;
 let policy=RankingPolicy::default();let lexical=lctx_serving::search::lexical(&native,"connect absentterm",Family::ApiOptions,&inputs,Some(&pairs),true,None,100,&policy).await.unwrap();assert_eq!(lexical.len(),1);assert_eq!(lexical[0].occurrence.context,good);assert!(lexical[0].score.unwrap()>0.0);
 let spec=ContentHash::of(b"native-vector-fixture");let mut vectors=vec![];let mut vec_occurrences=vec![];let mut query=vec![0f32;1024];query[0]=1.;
 for (key,context,vector) in [("excluded",other,query.clone()),("eligible",good,{let mut v=vec![0f32;1024];v[0]=0.8;v[1]=0.6;v})]{let source=RecordId::new("vector",key);let digest=embedding::value::value_digest(&vector);let mut obj=object(serde_json::json!({"specification":spec,"input":ContentHash::of(key.as_bytes()),"digest":digest,"embedding":vector}),source.clone());obj.insert("bytes",Bytes::from(embedding::value::encode_vector(&vector)));vectors.push(Value::Object(obj));vec_occurrences.push(occurrence("vec_occurs",key,source,out.clone(),*input.bytes(),Some(member.id()),context,unit));}
 insert(&native,"vector",false,vectors).await;insert(&native,"vec_occurs",true,vec_occurrences).await;
 let vector=lctx_serving::search::vector(&native,&query,spec,embedding::value::value_digest(&query),Family::ApiOptions,&inputs,Some(&pairs),true,None,100,&policy).await.unwrap();assert_eq!(vector.len(),1);assert_eq!(vector[0].occurrence.context,good);assert!(vector[0].score.unwrap()<1.0);
 client.query("DEFINE FUNCTION OVERWRITE fn::lctx_operation_definition() { RETURN 'incompatible'; };").await.unwrap().check().unwrap();
 let service=lctx_serving::NativeService::new(native,ResourceLimits::default()).unwrap();let refusal=service.execute("find_operations",r#"{"library":"unqueried"}"#).await.unwrap_err();assert!(refusal.to_string().contains("published operation executable"));
 client.query(format!("REMOVE DATABASE {db}")).await.unwrap().check().unwrap();
}
