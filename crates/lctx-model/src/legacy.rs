//! Temporary migration machinery (cutover plan §3.4–§3.6), deleted in phase 5.
//!
//! Nothing here is part of the model (DESIGN §15.13). It keeps legacy producers and their
//! identities working while each layer cuts over.

use crate::id::IdHasher;

/// The legacy identity domain. Part of every legacy id and digest.
pub const ID_TAG_V1: &[u8] = b"lctx-id/v1";

impl IdHasher {
    /// A legacy `lctx-id/v1` hasher over an ad hoc kind tag. New code uses a declared kind.
    pub fn new(kind_tag: &str) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(ID_TAG_V1);
        let mut this = Self(h);
        this.bytes(kind_tag.as_bytes());
        this
    }

}
