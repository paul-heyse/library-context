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


/// The current identity domain (DESIGN §15.3). Part of every model id and digest.
pub const ID_TAG_V2: &[u8] = b"lctx-id/v2";

crate::codebook!(
    /// The declared identity kinds (DESIGN §15.3). A recipe names exactly one; an ad hoc tag does
    /// not compile. Append-only: a new kind takes the next code, and a kind's text (which is what
    /// the hash covers) never changes.
    IdKind = "id_kind" {
        Occurrence = 0 => "occurrence",
        Entity = 1 => "entity",
        SymbolKey = 2 => "symbol_key",
        Place = 3 => "place",
        Atom = 4 => "atom",
        Condition = 5 => "condition",
        ConditionNode = 6 => "condition_node",
        CallTarget = 7 => "call_target",
        CallBinding = 8 => "call_binding",
        Transfer = 9 => "transfer",
        Obligation = 10 => "obligation",
        Derivation = 11 => "derivation",
        Finding = 12 => "finding",
        Invocation = 13 => "invocation",
    }
);

impl IdHasher {
    /// An `lctx-id/v2` hasher for a declared kind: the domain, then the kind's text.
    pub fn v2(kind: IdKind) -> Self {
        use crate::decl::codebook::Codebook;
        let mut h = blake3::Hasher::new();
        h.update(ID_TAG_V2);
        let mut this = Self(h);
        this.bytes(kind.text().as_bytes());
        this
    }
}

/// One identity field of a v2 recipe. Every field is encoded with a presence byte — `0` for
/// absent, else `1` and the length-prefixed value — so a recipe computed in Rust equals the
/// `lctx_id_v2` UDF over nullable columns, which cannot tell a declared non-null column from a
/// nullable one holding a value.
pub trait RecipeField {
    fn feed(&self, h: &mut IdHasher);
}

macro_rules! present_field {
    ($($ty:ty => |$v:ident, $h:ident| $body:expr;)+) => {
        $(
            impl RecipeField for $ty {
                fn feed(&self, $h: &mut IdHasher) {
                    let $v = self;
                    $h.bytes(&[1]);
                    $body;
                }
            }
        )+
    };
}

present_field! {
    Id => |v, h| h.id(*v);
    Digest => |v, h| h.digest_field(*v);
    str => |v, h| h.str(v);
    String => |v, h| h.str(v);
    i64 => |v, h| h.i64(*v);
    bool => |v, h| h.bool(*v);
}

impl<T: RecipeField + ?Sized> RecipeField for &T {
    fn feed(&self, h: &mut IdHasher) {
        (**self).feed(h);
    }
}

impl<T: RecipeField> RecipeField for Option<T> {
    fn feed(&self, h: &mut IdHasher) {
        match self {
            None => {
                h.bytes(&[0]);
            }
            Some(v) => v.feed(h),
        }
    }
}

/// A codebook value is its code, which is append-only and never renumbered (DESIGN §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Code<C>(pub C);

impl<C: crate::decl::codebook::Codebook> RecipeField for Code<C> {
    fn feed(&self, h: &mut IdHasher) {
        h.bytes(&[1]);
        h.i64(i64::from(self.0.code()));
    }
}

/// A recipe as data: its kind and its identity fields in hash order. Relation declarations name
/// a recipe, and identity validators and the `lctx_id_v2` UDF recompute it from exactly these
/// columns; no other column (provenance) can reach the id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipe {
    pub kind: IdKind,
    pub name: &'static str,
    pub fields: &'static [&'static str],
}

/// Declares a v2 identity recipe: a typed function of exactly its identity fields, and its
/// [`Recipe`] metadata.
#[macro_export]
macro_rules! recipe {
    ($(#[$meta:meta])* $name:ident, $meta_name:ident = $kind:ident { $($field:ident : $ty:ty),+ $(,)? }) => {
        $(#[$meta])*
        pub fn $name($($field: $ty),+) -> $crate::id::Id {
            let mut h = $crate::id::IdHasher::v2($crate::id::IdKind::$kind);
            $($crate::id::RecipeField::feed(&$field, &mut h);)+
            h.finish_id()
        }

        #[doc = concat!("The metadata of [`", stringify!($name), "`].")]
        pub const $meta_name: $crate::id::Recipe = $crate::id::Recipe {
            kind: $crate::id::IdKind::$kind,
            name: stringify!($name),
            fields: &[$(stringify!($field)),+],
        };
    };
}

/// The model's identity recipes (DESIGN §15.3). Each vocabulary module adds the recipes of the
/// relations it owns as those contracts are declared.
pub mod recipes {
    use super::Id;

    crate::recipe!(
        /// A source occurrence: its module entity, byte span and syntax kind. The syntax kind is
        /// the parser's stable node-kind name, so two occurrences sharing a span stay distinct.
        occurrence, OCCURRENCE = Occurrence { module: Id, start: i64, end: i64, syntax_kind: &str }
    );

    crate::recipe!(
        /// A release-independent symbol key: distribution, qualified path and member descriptor,
        /// for cross-release and dependency joins.
        symbol_key, SYMBOL_KEY = SymbolKey { distribution: &str, qualified_path: &str, descriptor: &str }
    );

    /// Every recipe declared here, for validators and the UDF's known answers.
    pub const ALL: &[super::Recipe] = &[OCCURRENCE, SYMBOL_KEY];
}
