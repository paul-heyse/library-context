//! Allocation admission for this canonical JSON boundary. Slice decoding borrows strings;
//! serde's sequence hint is suppressed so typed Vec growth can be admitted before each seed.
use lctx_model::domain::{ModelError, resources::{Reservation,ResourceBudget}};
use serde::de::{self, DeserializeOwned, DeserializeSeed, Deserializer, Visitor, SeqAccess, MapAccess, EnumAccess, VariantAccess};
use std::fmt;

struct State { charge: Box<dyn Reservation>, failure: Option<ModelError> }
impl State {
    fn admit<E: de::Error>(&mut self, bytes: usize) -> Result<(),E> {
        let result = self.charge.size().checked_add(bytes)
            .ok_or_else(|| ModelError::Invalid("canonical JSON allocation overflow".into()))
            .and_then(|size| self.charge.try_resize(size));
        result.map_err(|error| { self.failure=Some(error); E::custom("canonical JSON allocation admission") })
    }
}
pub(super) fn decode<'de,T,D>(input:D,budget:&ResourceBudget)->Result<(T,Box<dyn Reservation>),ModelError>
where T:serde::Deserialize<'de>,D:Deserializer<'de> {
    decode_with(input,budget.reserve("native-canonical-decoded",0)?)
}
pub(super) fn decode_with<'de,T,D>(input:D,charge:Box<dyn Reservation>)->Result<(T,Box<dyn Reservation>),ModelError>
where T:serde::Deserialize<'de>,D:Deserializer<'de> {
    decode_seed_with(input,Payload::<T> {depth:0,marker:std::marker::PhantomData},charge)
}
fn decode_seed_with<'de,S,D>(input:D,seed:S,charge:Box<dyn Reservation>)->Result<(S::Value,Box<dyn Reservation>),ModelError>
where S:DeserializeSeed<'de>,D:Deserializer<'de> {
    let mut state=State {charge,failure:None};
    state.charge.try_resize(state.charge.size().checked_add(size_of::<S::Value>()).ok_or(ModelError::Schema("canonical decode inline overflow"))?)?;
    let value=seed.deserialize(Charged {inner:input,state:&mut state});
    match value {Ok(value)=>Ok((value,state.charge)),Err(error)=>Err(state.failure.take().unwrap_or_else(||ModelError::codec(error.to_string())))}
}
pub(super) fn from_wrapped_slice<T:DeserializeOwned>(input:&[u8],depth:usize,budget:&ResourceBudget,charge:Box<dyn Reservation>)->Result<(T,Box<dyn Reservation>),ModelError>{
    let _scratch=json_scratch(input.len(),budget)?;
    let mut json=serde_json::Deserializer::from_slice(input);
    let result=decode_seed_with(&mut json,Payload::<T> {depth,marker:std::marker::PhantomData},charge)?;
    json.end().map_err(ModelError::codec)?;Ok(result)
}
struct Payload<T>{depth:usize,marker:std::marker::PhantomData<T>}
impl<'de,T:serde::Deserialize<'de>> DeserializeSeed<'de> for Payload<T>{
    type Value=T;
    fn deserialize<D:Deserializer<'de>>(self,input:D)->Result<T,D::Error>{
        if self.depth==0 {T::deserialize(input)}else{input.deserialize_map(self)}
    }
}
impl<'de,T:serde::Deserialize<'de>> Visitor<'de> for Payload<T>{
    type Value=T;
    fn expecting(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{f.write_str("one canonical payload wrapper")}
    fn visit_map<A:MapAccess<'de>>(self,mut input:A)->Result<T,A::Error>{
        if input.next_key::<de::IgnoredAny>()?.is_none(){return Err(de::Error::custom("empty canonical payload wrapper"));}
        let value=input.next_value_seed(Payload {depth:self.depth-1,marker:std::marker::PhantomData})?;
        if input.next_key::<de::IgnoredAny>()?.is_some(){return Err(de::Error::custom("multiple canonical payload wrappers"));}
        Ok(value)
    }
}
pub(super) fn from_slice<T:DeserializeOwned>(input:&[u8],budget:&ResourceBudget)->Result<(T,Box<dyn Reservation>),ModelError> {
    // serde_json's escaped-string Vec may round its encoded-input bound up while growing.
    // Typed String and container allocations have their own admission below.
    let _scratch=json_scratch(input.len(),budget)?;
    let mut json=serde_json::Deserializer::from_slice(input);
    let result=decode(&mut json,budget)?;
    json.end().map_err(ModelError::codec)?;
    Ok(result)
}
fn json_scratch(bytes:usize,budget:&ResourceBudget)->Result<Box<dyn Reservation>,ModelError>{
    budget.reserve("native-canonical-json-scratch",bytes.checked_next_power_of_two().ok_or(ModelError::Schema("canonical JSON scratch overflow"))?.max(8))
}
struct Charged<'a,D> {inner:D,state:&'a mut State}
struct CheckedVisitor<'a,V> {inner:V,state:&'a mut State}
struct Seed<'a,S> {inner:S,state:&'a mut State,inline:usize}
impl<'de,S:DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<'_,S> {
    type Value=S::Value;
    fn deserialize<D:Deserializer<'de>>(self,deserializer:D)->Result<Self::Value,D::Error> {
        self.state.admit::<D::Error>(self.inline)?;
        self.inner.deserialize(Charged {inner:deserializer,state:self.state})
    }
}
impl<'de,D:Deserializer<'de>> Deserializer<'de> for Charged<'_,D> {
    type Error=D::Error;
    fn deserialize_any<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_any(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_bool<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_bool(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_i8<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_i8(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_i16<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_i16(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_i32<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_i32(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_i64<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_i64(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_i128<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_i128(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_u8<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_u8(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_u16<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_u16(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_u32<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_u32(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_u64<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_u64(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_u128<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_u128(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_f32<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_f32(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_f64<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_f64(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_char<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_char(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_str<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_str(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_string<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_string(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_bytes<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_bytes(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_byte_buf<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_byte_buf(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_option<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_option(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_unit<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_unit(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_seq<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_seq(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_map<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_map(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_identifier<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_identifier(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_ignored_any<V:Visitor<'de>>(self,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_ignored_any(CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_unit_struct<V:Visitor<'de>>(self,name:&'static str,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_unit_struct(name,CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_newtype_struct<V:Visitor<'de>>(self,name:&'static str,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_newtype_struct(name,CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_tuple<V:Visitor<'de>>(self,len:usize,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_tuple(len,CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_tuple_struct<V:Visitor<'de>>(self,name:&'static str,len:usize,visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_tuple_struct(name,len,CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_struct<V:Visitor<'de>>(self,name:&'static str,fields:&'static [&'static str],visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_struct(name,fields,CheckedVisitor {inner:visitor,state:self.state}) }
    fn deserialize_enum<V:Visitor<'de>>(self,name:&'static str,variants:&'static [&'static str],visitor:V)->Result<V::Value,Self::Error> { self.inner.deserialize_enum(name,variants,CheckedVisitor {inner:visitor,state:self.state}) }
    fn is_human_readable(&self)->bool {self.inner.is_human_readable()}
}
impl<'de,V:Visitor<'de>> Visitor<'de> for CheckedVisitor<'_,V> {
    type Value=V::Value;
    fn expecting(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {self.inner.expecting(f)}
    fn visit_bool<E:de::Error>(self,value:bool)->Result<Self::Value,E> {self.inner.visit_bool(value)}
    fn visit_i8<E:de::Error>(self,value:i8)->Result<Self::Value,E> {self.inner.visit_i8(value)}
    fn visit_i16<E:de::Error>(self,value:i16)->Result<Self::Value,E> {self.inner.visit_i16(value)}
    fn visit_i32<E:de::Error>(self,value:i32)->Result<Self::Value,E> {self.inner.visit_i32(value)}
    fn visit_i64<E:de::Error>(self,value:i64)->Result<Self::Value,E> {self.inner.visit_i64(value)}
    fn visit_i128<E:de::Error>(self,value:i128)->Result<Self::Value,E> {self.inner.visit_i128(value)}
    fn visit_u8<E:de::Error>(self,value:u8)->Result<Self::Value,E> {self.inner.visit_u8(value)}
    fn visit_u16<E:de::Error>(self,value:u16)->Result<Self::Value,E> {self.inner.visit_u16(value)}
    fn visit_u32<E:de::Error>(self,value:u32)->Result<Self::Value,E> {self.inner.visit_u32(value)}
    fn visit_u64<E:de::Error>(self,value:u64)->Result<Self::Value,E> {self.inner.visit_u64(value)}
    fn visit_u128<E:de::Error>(self,value:u128)->Result<Self::Value,E> {self.inner.visit_u128(value)}
    fn visit_f32<E:de::Error>(self,value:f32)->Result<Self::Value,E> {self.inner.visit_f32(value)}
    fn visit_f64<E:de::Error>(self,value:f64)->Result<Self::Value,E> {self.inner.visit_f64(value)}
    fn visit_char<E:de::Error>(self,value:char)->Result<Self::Value,E> {self.inner.visit_char(value)}
    fn visit_str<E:de::Error>(self,value:&str)->Result<Self::Value,E> {self.state.admit::<E>(value.len())?;self.inner.visit_str(value)}
    fn visit_borrowed_str<E:de::Error>(self,value:&'de str)->Result<Self::Value,E> {self.state.admit::<E>(value.len())?;self.inner.visit_borrowed_str(value)}
    fn visit_string<E:de::Error>(self,value:String)->Result<Self::Value,E> {self.state.admit::<E>(value.capacity())?;self.inner.visit_string(value)}
    fn visit_bytes<E:de::Error>(self,value:&[u8])->Result<Self::Value,E> {self.state.admit::<E>(value.len())?;self.inner.visit_bytes(value)}
    fn visit_borrowed_bytes<E:de::Error>(self,value:&'de [u8])->Result<Self::Value,E> {self.state.admit::<E>(value.len())?;self.inner.visit_borrowed_bytes(value)}
    fn visit_byte_buf<E:de::Error>(self,value:Vec<u8>)->Result<Self::Value,E> {self.state.admit::<E>(value.capacity())?;self.inner.visit_byte_buf(value)}
    fn visit_none<E:de::Error>(self)->Result<Self::Value,E>{self.inner.visit_none()}
    fn visit_unit<E:de::Error>(self)->Result<Self::Value,E>{self.inner.visit_unit()}
    fn visit_some<D:Deserializer<'de>>(self,input:D)->Result<Self::Value,D::Error>{self.inner.visit_some(Charged {inner:input,state:self.state})}
    fn visit_newtype_struct<D:Deserializer<'de>>(self,input:D)->Result<Self::Value,D::Error>{self.inner.visit_newtype_struct(Charged {inner:input,state:self.state})}
    fn visit_seq<A:SeqAccess<'de>>(self,input:A)->Result<Self::Value,A::Error>{self.inner.visit_seq(Sequence {inner:input,state:self.state,len:0,capacity:0})}
    fn visit_map<A:MapAccess<'de>>(self,input:A)->Result<Self::Value,A::Error>{self.inner.visit_map(Mapping {inner:input,state:self.state})}
    fn visit_enum<A:EnumAccess<'de>>(self,input:A)->Result<Self::Value,A::Error>{self.inner.visit_enum(Enumeration {inner:input,state:self.state})}
}
struct Sequence<'a,A>{inner:A,state:&'a mut State,len:usize,capacity:usize}
struct Element<'a,S>{inner:S,state:&'a mut State,len:&'a mut usize,capacity:&'a mut usize}
impl<'de,S:DeserializeSeed<'de>> DeserializeSeed<'de> for Element<'_,S> {
    type Value=S::Value;
    fn deserialize<D:Deserializer<'de>>(self,input:D)->Result<Self::Value,D::Error>{
        // serde Vec receives no preallocation hint. Its first/amortized growth follows
        // RawVec's element-size minimum and doubling; arrays merely receive an allowance.
        if *self.len==*self.capacity {
            let width=size_of::<S::Value>();
            let minimum=if width==1 {8}else if width<=1024 {4}else {1};
            let next=self.capacity.checked_mul(2).unwrap_or(usize::MAX).max(minimum);
            let bytes=next.checked_sub(*self.capacity).and_then(|n|n.checked_mul(width)).ok_or_else(||<D::Error as de::Error>::custom("canonical sequence allocation overflow"))?;
            self.state.admit::<D::Error>(bytes)?;*self.capacity=next;
        }
        *self.len+=1;
        self.inner.deserialize(Charged {inner:input,state:self.state})
    }
}
impl<'de,A:SeqAccess<'de>> SeqAccess<'de> for Sequence<'_,A>{
    type Error=A::Error;
    fn next_element_seed<S:DeserializeSeed<'de>>(&mut self,seed:S)->Result<Option<S::Value>,Self::Error>{
        self.inner.next_element_seed(Element {inner:seed,state:self.state,len:&mut self.len,capacity:&mut self.capacity})
    }
    fn size_hint(&self)->Option<usize>{None}
}
struct Mapping<'a,A>{inner:A,state:&'a mut State}
impl<'de,A:MapAccess<'de>> MapAccess<'de> for Mapping<'_,A>{
    type Error=A::Error;
    fn next_key_seed<S:DeserializeSeed<'de>>(&mut self,seed:S)->Result<Option<S::Value>,Self::Error>{
        // A map node's per-entry overhead follows the existing model charged-map contract.
        self.inner.next_key_seed(Seed {inner:seed,state:self.state,inline:size_of::<S::Value>()+32})
    }
    fn next_value_seed<S:DeserializeSeed<'de>>(&mut self,seed:S)->Result<S::Value,Self::Error>{
        self.inner.next_value_seed(Seed {inner:seed,state:self.state,inline:size_of::<S::Value>()})
    }
    fn size_hint(&self)->Option<usize>{None}
}
struct Enumeration<'a,A>{inner:A,state:&'a mut State}
struct Variant<'a,A>{inner:A,state:&'a mut State}
impl<'de,'a,A:EnumAccess<'de>> EnumAccess<'de> for Enumeration<'a,A>{
    type Error=A::Error; type Variant=Variant<'a,A::Variant>;
    fn variant_seed<S:DeserializeSeed<'de>>(self,seed:S)->Result<(S::Value,Self::Variant),Self::Error>{
        let (value,variant)=self.inner.variant_seed(Seed {inner:seed,state:&mut *self.state,inline:0})?;
        Ok((value,Variant {inner:variant,state:self.state}))
    }
}
impl<'de,A:VariantAccess<'de>> VariantAccess<'de> for Variant<'_,A>{
    type Error=A::Error;
    fn unit_variant(self)->Result<(),Self::Error>{self.inner.unit_variant()}
    fn newtype_variant_seed<S:DeserializeSeed<'de>>(self,seed:S)->Result<S::Value,Self::Error>{self.inner.newtype_variant_seed(Seed {inner:seed,state:self.state,inline:0})}
    fn tuple_variant<V:Visitor<'de>>(self,len:usize,visitor:V)->Result<V::Value,Self::Error>{self.inner.tuple_variant(len,CheckedVisitor {inner:visitor,state:self.state})}
    fn struct_variant<V:Visitor<'de>>(self,fields:&'static [&'static str],visitor:V)->Result<V::Value,Self::Error>{self.inner.struct_variant(fields,CheckedVisitor {inner:visitor,state:self.state})}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    thread_local! {static CONSTRUCTED:Cell<usize>=const {Cell::new(0)};}
    #[derive(Debug)]
    struct Wide {_storage:[u8;4096]}
    impl<'de> serde::Deserialize<'de> for Wide {
        fn deserialize<D:Deserializer<'de>>(input:D)->Result<Self,D::Error> {
            struct WideVisitor;
            impl<'de> Visitor<'de> for WideVisitor {
                type Value=Wide;
                fn expecting(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {f.write_str("null")}
                fn visit_unit<E:de::Error>(self)->Result<Wide,E> {CONSTRUCTED.with(|count|count.set(count.get()+1));Ok(Wide {_storage:[0;4096]})}
            }
            input.deserialize_unit(WideVisitor)
        }
    }
    #[test]
    fn tiny_json_refuses_large_typed_sequence_before_constructing_next_row() {
        CONSTRUCTED.with(|count|count.set(0));
        let budget=ResourceBudget::fixed(5000).unwrap();
        let error=from_slice::<Vec<Wide>>(b"[null,null]",&budget).unwrap_err();
        assert!(matches!(error,ModelError::Resource {..}));
        CONSTRUCTED.with(|count|assert_eq!(count.get(),1));
        assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn decoded_nested_allocations_remain_charged_until_owner_drop() {
        let budget=ResourceBudget::fixed(64*1024).unwrap();
        let (rows,charge)=from_slice::<Vec<Vec<String>>>(br#"[["alpha","beta"],["escaped\ntext"]]"#,&budget).unwrap();
        let held=size_of::<Vec<Vec<String>>>()+rows.capacity()*size_of::<Vec<String>>()+rows.iter().map(|row|row.capacity()*size_of::<String>()+row.iter().map(|value|value.capacity()).sum::<usize>()).sum::<usize>();
        assert!(charge.size()>=held);
        assert_eq!(budget.reserved(),charge.size());
        drop(rows);drop(charge);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn wrapped_json_payload_obeys_the_same_typed_admission() {
        let budget=ResourceBudget::fixed(5000).unwrap();
        let charge=budget.reserve("test",0).unwrap();
        assert!(matches!(from_wrapped_slice::<Vec<Wide>>(br#"{"Native":{"Rows":[null,null]}}"#,2,&budget,charge),Err(ModelError::Resource {..})));
        assert_eq!(budget.reserved(),0);
    }
}
