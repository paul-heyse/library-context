//! Canonical payload bytes for fact ids (DESIGN §3.4.1): every column type feeds the
//! length-prefixed [`IdHasher`] the same way, so `fact_id` needs no per-table code.

use crate::codebook::Codebook;
use crate::id::{Digest, Id, IdHasher};

pub trait HashField {
    fn hash_into(&self, h: &mut IdHasher);
}

impl HashField for Id {
    fn hash_into(&self, h: &mut IdHasher) {
        h.id(*self);
    }
}
impl HashField for Option<Id> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.opt_id(*self);
    }
}
impl HashField for Digest {
    fn hash_into(&self, h: &mut IdHasher) {
        h.digest_field(*self);
    }
}
impl HashField for String {
    fn hash_into(&self, h: &mut IdHasher) {
        h.str(self);
    }
}
impl HashField for Option<String> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.opt_str(self.as_deref());
    }
}
impl HashField for i64 {
    fn hash_into(&self, h: &mut IdHasher) {
        h.i64(*self);
    }
}
impl HashField for Option<i64> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.opt_i64(*self);
    }
}
impl HashField for bool {
    fn hash_into(&self, h: &mut IdHasher) {
        h.bool(*self);
    }
}
impl HashField for Option<bool> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.opt_bool(*self);
    }
}
impl<C: Codebook> HashField for C {
    fn hash_into(&self, h: &mut IdHasher) {
        h.i64(i64::from(self.code()));
    }
}
impl<C: Codebook> HashField for Option<C> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.opt_i64(self.map(|c| i64::from(c.code())));
    }
}
impl HashField for Vec<String> {
    fn hash_into(&self, h: &mut IdHasher) {
        h.strs(self.iter().map(String::as_str));
    }
}
