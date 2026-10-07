//! Necessary stream admission checks canonical companions and exact nominal window membership.
#[path="fixtures/embedding_consumption.rs"] mod fixture;
use fixture::*;
use lctx_model::domain::{embedding::{analytic::*,text::*,*},resources::ResourceBudget,*};
#[test]
fn streamed_consumption_admits_actual_full_projection_before_references() {
 let b=ResourceBudget::fixed(8<<20).unwrap();let f=fixture(&b,true);let(full,p,published)=winner(&f,&b);let uses=consume(&f,&published,&b);let(_,outcomes)=outcomes(&f,&uses,&b);let mut check=admission(&f,&b);
 visit(&mut *check,&[full]).unwrap();visit(&mut *check,&[p]).unwrap();for row in uses.iter(){visit(&mut *check,std::slice::from_ref(row)).unwrap();}for row in outcomes.iter(){visit(&mut *check,std::slice::from_ref(row)).unwrap();}check.finish().unwrap();
}
#[test]
fn stream_refuses_missing_companions_rewritten_uses_and_duplicate_membership() {
 let b=ResourceBudget::fixed(8<<20).unwrap();let f=fixture(&b,true);let(full,p,published)=winner(&f,&b);let uses=consume(&f,&published,&b);let row=uses.iter().next().unwrap();let mut check=admission(&f,&b);assert!(visit(&mut *check,std::slice::from_ref(row)).is_err());
 let mut check=admission(&f,&b);visit(&mut *check,&[full]).unwrap();visit(&mut *check,&[p]).unwrap();let mut changed=row.clone();changed.input=ContentHash::of(b"rewritten");assert!(visit(&mut *check,&[changed]).is_err());
}
#[test]
fn canonical_streaming_window_boundaries_preserve_unicode() {
 let b=ResourceBudget::fixed(8<<20).unwrap();let f=fixture(&b,true);let text="αβ\n🦀🦀\nlast";let definition=TextDefinition {window_bytes:6,..TextDefinition::builtin()};
 let rows=embedding::text::windows(text,definition.window_bytes as usize,&b).unwrap();let assembled=rows.iter().map(|(_,text)|text).collect::<String>();assert_eq!(assembled,text);
}
