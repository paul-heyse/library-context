//! Borrowed, preflighted format-2 envelope. Section bodies retain their exact opaque bytes.
use lctx_model::domain::{ModelError,compilation_product::{PortableProduct,ProductRequest,ProductOutcome,ProductSection,PRODUCT_FORMAT},resources::{ResourceBudget,Reservation}};
use serde::{Serialize,Deserialize};
fn size<T:Serialize+?Sized>(value:&T)->Result<usize,ModelError>{postcard::experimental::serialized_size(value).map_err(ModelError::codec)}
fn add(left:usize,right:usize)->Result<usize,ModelError>{left.checked_add(right).ok_or(ModelError::Schema("portable product size overflow"))}
fn outcome(value:ProductOutcome)->u8{match value{ProductOutcome::Complete=>0,ProductOutcome::NotRequested=>1}}
/// Size preparation uses trusted declarations only and allocates no envelope or section data.
pub(super) fn encoded_size(product:&PortableProduct)->Result<usize,ModelError>{
 product.validate()?;
 let request=size(&product.request)?;
 let mut total=size(&(PRODUCT_FORMAT,outcome(product.outcome),request as u64))?;
 total=add(total,request)?;total=add(total,size(&(product.sections.len() as u64))?)?;
 for section in &product.sections{total=add(total,size(&(section.name.as_str(),section.rows,section.bytes.as_slice()))?)?;}
 Ok(total)
}
pub(super) fn encode(product:&PortableProduct)->Result<Vec<u8>,ModelError>{
 let total=encoded_size(product)?;
 let request=size(&product.request)?;
 let bytes=Vec::with_capacity(total);
 let bytes=postcard::to_extend(&(PRODUCT_FORMAT,outcome(product.outcome),request as u64),bytes).map_err(ModelError::codec)?;
 let bytes=postcard::to_extend(&product.request,bytes).map_err(ModelError::codec)?;
 let mut bytes=postcard::to_extend(&(product.sections.len() as u64),bytes).map_err(ModelError::codec)?;
 for section in &product.sections{bytes=postcard::to_extend(&(section.name.as_str(),section.rows,section.bytes.as_slice()),bytes).map_err(ModelError::codec)?;}
 if bytes.len()!=total{return Err(ModelError::Conflict("portable product encoded size"));}Ok(bytes)
}
/// Postcard permits some redundant integer encodings; require the canonical consumed length.
fn take<'a,T:Deserialize<'a>+Serialize>(input:&mut &'a[u8])->Result<T,ModelError>{
 let (value,rest)=postcard::take_from_bytes::<T>(input).map_err(ModelError::codec)?;
 if input.len()-rest.len()!=size(&value)?{return Err(ModelError::Conflict("portable product noncanonical metadata"));}
 *input=rest;Ok(value)
}
/// Compare canonical serialization directly with borrowed metadata: no request-byte clone.
struct Matches<'a>(&'a[u8]);
impl postcard::ser_flavors::Flavor for Matches<'_>{
 type Output=();
 fn try_push(&mut self,byte:u8)->postcard::Result<()>{self.try_extend(&[byte])}
 fn try_extend(&mut self,bytes:&[u8])->postcard::Result<()>{
  if self.0.get(..bytes.len())!=Some(bytes){return Err(postcard::Error::SerializeBufferFull);}
  self.0=&self.0[bytes.len()..];Ok(())
 }
 fn finalize(self)->postcard::Result<()>{if self.0.is_empty(){Ok(())}else{Err(postcard::Error::SerializeBufferFull)}}
}
pub(super) fn request_matches(request:&ProductRequest,bytes:&[u8])->bool{postcard::serialize_with_flavor(request,Matches(bytes)).is_ok()}
/// Reject every untrusted length/count/name before creating owned variable containers.
/// The supplied buffer's charge does not cover the independently reserved decoded product.
pub(super) fn decode(bytes:&[u8],capacity:u64,request:&ProductRequest,budget:&ResourceBudget)->Result<(PortableProduct,Box<dyn Reservation>),ModelError>{
 request.validate()?;
 if bytes.is_empty() || u64::try_from(bytes.len()).map_err(ModelError::codec)?>capacity{return Err(ModelError::Conflict("portable product retained length"));}
 let mut input=bytes;
 let (format,outcome,encoded_request,count):(u32,u8,&[u8],u64)=take(&mut input)?;
 if format!=PRODUCT_FORMAT || !matches!(outcome,0|1) || !request_matches(request,encoded_request) || count!=request.outputs.len() as u64{return Err(ModelError::Conflict("portable product request or section inventory"));}
 let sections_start=input;
 let mut owned=add(size_of::<PortableProduct>(),request.retained_bytes()?)?;
 let mut body_bytes=0usize;
 for name in &request.outputs{
  let (actual,rows,body):(&str,u64,&[u8])=take(&mut input)?;
  if actual!=name || usize::try_from(rows).is_err() || rows>body.len() as u64 || (rows>0 && body.is_empty()){return Err(ModelError::Conflict("portable product section bounds or order"));}
  body_bytes=add(body_bytes,body.len())?;
  owned=add(owned,add(size_of::<ProductSection>(),add(name.len(),body.len())?)?)?;
 }
 if !input.is_empty() || body_bytes>bytes.len(){return Err(ModelError::Conflict("portable product trailing bytes or section total"));}
 // Capacity can exceed initialized length for cloned vectors and strings. This independent
 // reservation is admitted before cloning even the current trusted request.
 let charge=budget.reserve("portable-decoded-product",owned.checked_mul(2).ok_or(ModelError::Schema("portable product allocation overflow"))?)?;
 let mut input=sections_start;
 let mut sections=Vec::with_capacity(request.outputs.len());
 for _ in &request.outputs{let(name,rows,body):(&str,u64,&[u8])=take(&mut input)?;sections.push(ProductSection{name:name.to_owned(),rows,bytes:body.to_vec()});}
 let product=PortableProduct{request:request.clone(),outcome:if outcome==0{ProductOutcome::Complete}else{ProductOutcome::NotRequested},sections};
 product.validate()?;
 if product.retained_bytes()?>charge.size(){return Err(ModelError::Conflict("portable decoded allocation accounting"));}
 Ok((product,charge))
}
#[cfg(test)]
mod controls{
 use super::*;
 use lctx_model::domain::{ContentHash,compilation_product::ProductKind};
 use std::collections::BTreeSet;
 fn product()->PortableProduct{PortableProduct{request:ProductRequest{kind:ProductKind::PureRows,operation:"codec-control".into(),model:ContentHash::of(b"model"),implementation:ContentHash::of(b"implementation"),policy:ContentHash::of(b"policy"),result_contract:ContentHash::of(b"rows"),configuration:None,profile:"catalog".into(),parameters:vec![1,2,3],dependencies:vec![],outputs:BTreeSet::from(["empty".into(),"rows".into()])},outcome:ProductOutcome::Complete,sections:vec![ProductSection{name:"empty".into(),rows:0,bytes:b"[]".to_vec()},ProductSection{name:"rows".into(),rows:1,bytes:(0..=255).collect()}]}}
 fn frame(product:&PortableProduct,count:u64)->Vec<u8>{let request=postcard::to_allocvec(&product.request).unwrap();postcard::to_allocvec(&(PRODUCT_FORMAT,0u8,request.as_slice(),count)).unwrap()}
 #[test]
 fn binary_envelope_preserves_exact_opaque_sections_and_precharges_owned_decode(){
  let product=product();let bytes=encode(&product).unwrap();assert_eq!(bytes.len(),encoded_size(&product).unwrap());assert!(bytes.len()<serde_json::to_vec(&product).unwrap().len());
  let budget=ResourceBudget::fixed(1<<20).unwrap();let input=budget.reserve("codec-input-control",bytes.len()).unwrap();let(decoded,charge)=decode(&bytes,bytes.len() as u64,&product.request,&budget).unwrap();assert_eq!(decoded,product);assert!(charge.size()>=decoded.retained_bytes().unwrap());assert_eq!(budget.reserved(),input.size()+charge.size());
  drop(decoded);drop(charge);assert_eq!(budget.reserved(),input.size());
  let tiny=ResourceBudget::fixed(bytes.len()+1).unwrap();let _input=tiny.reserve("codec-input-control",bytes.len()).unwrap();assert!(matches!(decode(&bytes,u64::MAX,&product.request,&tiny),Err(ModelError::Resource{..})));
 }
 #[test]
 fn borrowed_preflight_rejects_counts_inventory_bounds_and_noncanonical_metadata_before_allocation(){
  let product=product();let bytes=encode(&product).unwrap();let tiny=ResourceBudget::fixed(1).unwrap();
  let huge=frame(&product,u64::MAX);assert!(matches!(decode(&huge,u64::MAX,&product.request,&tiny),Err(ModelError::Conflict(_))));
  let mut wrong=frame(&product,2);wrong=postcard::to_extend(&("unexpected",0u64,b"[]".as_slice()),wrong).unwrap();assert!(matches!(decode(&wrong,u64::MAX,&product.request,&tiny),Err(ModelError::Conflict(_))));
  let mut rows=frame(&product,2);rows=postcard::to_extend(&("empty",u64::MAX,b"[]".as_slice()),rows).unwrap();assert!(matches!(decode(&rows,u64::MAX,&product.request,&tiny),Err(ModelError::Conflict(_))));
  let mut request=product.request.clone();request.parameters.push(4);assert!(matches!(decode(&bytes,u64::MAX,&request,&tiny),Err(ModelError::Conflict(_))));
  let mut noncanonical=vec![0x82,0];noncanonical.extend_from_slice(&bytes[1..]);assert!(matches!(decode(&noncanonical,u64::MAX,&product.request,&tiny),Err(ModelError::Conflict(_))));
  let mut trailing=bytes.clone();trailing.push(0);assert!(matches!(decode(&trailing,u64::MAX,&product.request,&tiny),Err(ModelError::Conflict(_))));
  assert!(matches!(decode(&bytes,(bytes.len()-1) as u64,&product.request,&tiny),Err(ModelError::Conflict(_))));
 }
 #[test]
 fn truncation_and_malicious_borrowed_lengths_never_create_owned_product(){
  let product=product();let bytes=encode(&product).unwrap();let budget=ResourceBudget::fixed(1<<20).unwrap();
  for end in 0..bytes.len(){assert!(decode(&bytes[..end],u64::MAX,&product.request,&budget).is_err(),"truncation at {end}");assert_eq!(budget.reserved(),0);}
  let mut body=frame(&product,2);body=postcard::to_extend(&("empty",0u64,u64::MAX),body).unwrap();assert!(decode(&body,u64::MAX,&product.request,&budget).is_err());assert_eq!(budget.reserved(),0);
  let mut name=frame(&product,2);name=postcard::to_extend(&u64::MAX,name).unwrap();assert!(decode(&name,u64::MAX,&product.request,&budget).is_err());assert_eq!(budget.reserved(),0);
 }
}
