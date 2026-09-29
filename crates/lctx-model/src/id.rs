//! Identity: 16-byte ids and 32-byte digests (DESIGN §15.3, §3.4.1).
//!
//! Every id is a length-prefixed BLAKE3 hash: `domain ‖ u64le len‖kind ‖ (u64le len‖field)*`, of
//! which an id keeps the first 16 bytes and a digest all 32. The domain `lctx-id/v2` names its kind
//! by a declared `IdKind`; the legacy domain `lctx-id/v1` takes an ad hoc tag and lives in
//! [`crate::legacy`] until phase 5.

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident, $len:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub [u8; $len]);

        impl $name {
            /// All zeros: a placeholder while a row's own id is being derived.
            pub const ZERO: Self = Self([0; $len]);
            pub fn bytes(&self) -> &[u8; $len] {
                &self.0
            }
            pub fn hex(&self) -> String {
                self.0.iter().map(|b| format!("{b:02x}")).collect()
            }
            /// Exactly twice as many hex digits as bytes, either case. Anything else, a sign or a
            /// non-ASCII character included, is refused rather than half-parsed (H1 C4).
            pub fn from_hex(s: &str) -> Option<Self> {
                if s.len() != 2 * $len || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                let mut out = [0u8; $len];
                for (i, byte) in out.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
                }
                Some(Self(out))
            }
        }

        impl Default for $name { fn default()->Self { Self::ZERO } }
        impl serde::Serialize for $name {
            fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{s.serialize_str(&self.hex())}
        }
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error>{
                let s=<String as serde::Deserialize>::deserialize(d)?;
                Self::from_hex(&s).ok_or_else(||serde::de::Error::custom(concat!("invalid ",stringify!($name))))
            }
        }
        impl schemars::JsonSchema for $name {
            fn schema_name()->std::borrow::Cow<'static,str>{stringify!($name).into()}
            fn json_schema(_: &mut schemars::SchemaGenerator)->schemars::Schema {
                schemars::json_schema!({"type":"string","minLength":2*$len,"maxLength":2*$len,"pattern":"^[0-9a-fA-F]+$"})
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.hex())
            }
        }
    };
}

id_type!(
    /// A 16-byte identity (node, fact, run, snapshot, release, context, producer).
    Id,
    16
);
id_type!(
    /// A 32-byte content, schema or spec digest.
    Digest,
    32
);

/// Length-prefixed BLAKE3 hasher for ids and digests.
#[derive(Clone)]
pub struct IdHasher(pub(crate) blake3::Hasher);

impl IdHasher {
    pub fn bytes(&mut self, field: &[u8]) -> &mut Self {
        self.0.update(&(field.len() as u64).to_le_bytes());
        self.0.update(field);
        self
    }

    pub fn str(&mut self, field: &str) -> &mut Self {
        self.bytes(field.as_bytes())
    }

    pub fn i64(&mut self, field: i64) -> &mut Self {
        self.bytes(&field.to_le_bytes())
    }

    /// A float by its IEEE-754 bits, so `-0.0` and `0.0` differ and every NaN payload is its own
    /// value; the analytics refuse non-finite results before hashing.
    pub fn f64(&mut self, field: f64) -> &mut Self {
        self.bytes(&field.to_bits().to_le_bytes())
    }

    pub fn opt_f64(&mut self, field: Option<f64>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).f64(v),
        }
    }

    pub fn bool(&mut self, field: bool) -> &mut Self {
        self.bytes(&[u8::from(field)])
    }

    pub fn id(&mut self, field: Id) -> &mut Self {
        self.bytes(&field.0)
    }

    pub fn digest_field(&mut self, field: Digest) -> &mut Self {
        self.bytes(&field.0)
    }

    /// Absent and present-but-empty are distinct: a presence byte precedes the value.
    pub fn opt_str(&mut self, field: Option<&str>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).str(v),
        }
    }

    pub fn opt_i64(&mut self, field: Option<i64>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).i64(v),
        }
    }

    pub fn opt_bool(&mut self, field: Option<bool>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).bool(v),
        }
    }

    pub fn opt_bytes(&mut self, field: Option<&[u8]>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).bytes(v),
        }
    }

    pub fn opt_id(&mut self, field: Option<Id>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).id(v),
        }
    }

    pub fn opt_digest(&mut self, field: Option<Digest>) -> &mut Self {
        match field {
            None => self.bytes(&[0]),
            Some(v) => self.bytes(&[1]).digest_field(v),
        }
    }

    /// A list: its length, then each element length-prefixed.
    pub fn strs<'a>(&mut self, fields: impl IntoIterator<Item = &'a str>) -> &mut Self {
        let items: Vec<&str> = fields.into_iter().collect();
        self.i64(items.len() as i64);
        for s in items {
            self.str(s);
        }
        self
    }

    pub fn finish_id(&self) -> Id {
        let mut out = [0u8; 16];
        out.copy_from_slice(&self.0.finalize().as_bytes()[..16]);
        Id(out)
    }

    pub fn finish_digest(&self) -> Digest {
        Digest(*self.0.finalize().as_bytes())
    }
}

