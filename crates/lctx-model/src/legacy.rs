//! The legacy `lctx-id/v1` identity domain, kept only for the dormant `cpg-schema` until it retires
//! in phases 3–5 (cutover plan §4.1.1). Nothing here is part of the model (DESIGN §15.13); the
//! parity adapters and legacy-ID side relations were removed in P1.2.

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
