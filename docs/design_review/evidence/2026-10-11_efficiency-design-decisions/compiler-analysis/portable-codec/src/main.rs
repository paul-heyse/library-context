use serde::{Deserialize,Serialize};
#[derive(Debug,PartialEq,Serialize,Deserialize)] struct Section {name:String,rows:u64,bytes:Vec<u8>}
#[derive(Debug,PartialEq,Serialize,Deserialize)] struct Envelope {format:u32,outcome:u8,request:Vec<u8>,sections:Vec<Section>}
const REQUEST:&[u8]=b"exact-current-request";
// No owned Serde DTO is decoded from untrusted data. Borrowed scalar/string/bytes frame preflight
// checks exact expected count, declared names, lengths, request, canonicality, then clone admission.
fn checked(bytes:&[u8],budget:usize)->Result<Envelope,String>{
 if bytes.len()>budget{return Err("input admission".into());}
 let ((format,outcome,request,count),mut rest)=postcard::take_from_bytes::<(u32,u8,&[u8],u64)>(bytes).map_err(|e|e.to_string())?;
 if format!=2||outcome>1||request!=REQUEST||count!=2{return Err("format/request/inventory".into());}
 let mut slots=[None,None];let mut needed=REQUEST.len()+2*std::mem::size_of::<Section>();
 for (i,expected) in ["empty","rows"].iter().enumerate(){let (section,tail)=postcard::take_from_bytes::<(&str,u64,&[u8])>(rest).map_err(|e|e.to_string())?;if section.0!=*expected||section.2.len()>1<<20||section.1>1000{return Err("section bound/inventory".into());}needed=needed.checked_add(section.0.len()+section.2.len()).ok_or("size overflow")?;slots[i]=Some(section);rest=tail;}
 if !rest.is_empty(){return Err("trailing bytes".into());}
 if bytes.len().checked_add(needed).is_none_or(|n|n>budget){return Err("decoded admission".into());}
 // Allocation is allowed only after complete borrowed framing + separate input/decoded charge.
 let v=Envelope{format,outcome,request:request.to_vec(),sections:slots.into_iter().map(|s|{let (name,rows,body)=s.unwrap();Section{name:name.into(),rows,bytes:body.to_vec()}}).collect()};
 if postcard::to_allocvec(&v).unwrap()!=bytes{return Err("noncanonical encoding".into());}Ok(v)
}
fn main(){let body=br#"[{"id":"0011","source":"exact-source-id","name":"ready"}]"#.repeat(100);let v=Envelope{format:2,outcome:0,request:REQUEST.to_vec(),sections:vec![Section{name:"empty".into(),rows:0,bytes:b"[]".to_vec()},Section{name:"rows".into(),rows:100,bytes:body.clone()}]};let bytes=postcard::to_allocvec(&v).unwrap();assert_eq!(checked(&bytes,1<<20).unwrap(),v);let json=serde_json::to_vec(&v).unwrap();assert!(bytes.len()<body.len()+96);assert!(checked(&bytes,bytes.len()).unwrap_err().contains("decoded admission"));let malicious=postcard::to_allocvec(&(2u32,0u8,REQUEST,u64::MAX)).unwrap();assert!(checked(&malicious,1<<20).unwrap_err().contains("inventory"));let mut extra=bytes.clone();extra.push(0);assert!(checked(&extra,1<<20).is_err());assert!(checked(&bytes[..bytes.len()-1],1<<20).is_err());let mut changed=v;changed.sections[1].bytes[30]^=1;assert_ne!(postcard::to_allocvec(&changed).unwrap(),bytes);println!("opaque_roundtrip=true section_bytes={} postcard_envelope={} json_envelope={} borrowed_preflight=true huge_count_refused_before_allocation=true separate_decode_admission=true trailing_and_truncated_refused=true",body.len()+2,bytes.len(),json.len());}
