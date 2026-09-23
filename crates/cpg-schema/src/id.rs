//! Id and digest derivation (DESIGN §3.4.1, ADR-0007).
//!
//! `BLAKE3("lctx-id/v1" ‖ len‖kind_tag ‖ len‖field …)` with u64 little-endian lengths. Ids are the
//! first 16 bytes of the hash; digests are all 32. The tag is a contract: changing the encoding is
//! `lctx-id/v2` and a migration (DM-51).

/// The encoding tag. Part of every id and digest.
pub const ID_TAG: &[u8] = b"lctx-id/v1";

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

/// Kind tags. A new kind is appended; an existing tag never changes meaning.
pub mod kind {
    pub const RELEASE: &str = "release";
    pub const CONTEXT: &str = "context";
    pub const PRODUCER: &str = "producer";
    pub const RUN: &str = "run";
    pub const MODULE: &str = "module";
    pub const DECLARATION: &str = "declaration";
    pub const SYNTAX: &str = "syntax";
    pub const FACT: &str = "fact";
    pub const CONTENT: &str = "content";
    pub const ENVIRONMENT: &str = "environment";
    pub const COMPILER: &str = "compiler";
    pub const SNAPSHOT_CONTENT: &str = "snapshot-content";
    // The graph catalog (DESIGN §3.4.1, §3.8; ADR-0014).
    pub const EDGE: &str = "edge";
    pub const ARGUMENT: &str = "argument";
    pub const EXPORT: &str = "export";
    pub const SYNTHETIC_CALLABLE: &str = "synthetic_callable";
    pub const EXTERNAL_MODULE: &str = "external_module";
    pub const EXTERNAL_SYMBOL: &str = "external_symbol";
    /// A Pysa call row's run-independent payload digest (`pysa_calls.payload_id`).
    pub const PYSA_CALL: &str = "pysa-call";
    // The lexical family (C3).
    pub const SCOPE: &str = "scope";
    pub const BINDING: &str = "binding";
    pub const REFERENCE: &str = "reference";
    pub const TYPE: &str = "type";
    pub const FIELD: &str = "field";
    pub const DOCUMENT: &str = "document";
    pub const PASSAGE: &str = "passage";
    pub const CODE_BLOCK: &str = "code_block";
}

/// The id recipes computed in Rust whose inputs are also columns, so the `lctx_id` UDF recomputes
/// them in SQL and a generated rule checks they agree (DESIGN §3.4.1, ADR-0014). Every field uses
/// the `opt_*` encoding, as the UDF does.
pub mod recipe {
    use super::{Id, IdHasher, kind};

    /// A call's argument at an ordinal: a role, never the expression's own id.
    pub fn argument(call: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::ARGUMENT)
            .opt_id(Some(call))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    /// A dependency module: its owner (the distribution whose `RECORD` lists the file, else
    /// `pyrefly-bundled`, else `unowned`), the owner's version (else the fork revision, else the
    /// file's content digest), and its name.
    pub fn external_module(owner: &str, owner_version: &str, module: &str) -> Id {
        IdHasher::new(kind::EXTERNAL_MODULE)
            .opt_str(Some(owner))
            .opt_str(Some(owner_version))
            .opt_str(Some(module))
            .finish_id()
    }

    /// A lexical scope: the node that opens it (the module, a declaration, a lambda or a
    /// comprehension).
    pub fn scope(owner: Id) -> Id {
        IdHasher::new(kind::SCOPE).opt_id(Some(owner)).finish_id()
    }

    /// A binding event: its site (a declaration, a parameter, or the syntax id of the name,
    /// alias, handler or statement that binds) and the name it binds there.
    pub fn binding(site: Id, name: &str) -> Id {
        IdHasher::new(kind::BINDING)
            .opt_id(Some(site))
            .opt_str(Some(name))
            .finish_id()
    }

    /// A name reference: a role of the name's syntax node.
    pub fn reference(name_node: Id) -> Id {
        IdHasher::new(kind::REFERENCE)
            .opt_id(Some(name_node))
            .finish_id()
    }

    /// A record field: a role of its class (C4).
    pub fn field(class: Id, name: &str) -> Id {
        IdHasher::new(kind::FIELD)
            .opt_id(Some(class))
            .opt_str(Some(name))
            .finish_id()
    }

    /// A document of a corpus release (C5).
    pub fn document(release: Id, path: &str) -> Id {
        IdHasher::new(kind::DOCUMENT)
            .opt_id(Some(release))
            .opt_str(Some(path))
            .finish_id()
    }

    /// A passage or code block: a role of its document at an ordinal (C5).
    pub fn passage(document: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::PASSAGE)
            .opt_id(Some(document))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    pub fn code_block(document: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::CODE_BLOCK)
            .opt_id(Some(document))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    /// A definition in a dependency module: its module, definition kind code and Pysa key.
    pub fn external_symbol(module: Id, definition_kind: i16, key: &str) -> Id {
        IdHasher::new(kind::EXTERNAL_SYMBOL)
            .opt_id(Some(module))
            .opt_i64(Some(i64::from(definition_kind)))
            .opt_str(Some(key))
            .finish_id()
    }
}

/// Length-prefixed BLAKE3 hasher for ids and digests.
#[derive(Clone)]
pub struct IdHasher(blake3::Hasher);

impl IdHasher {
    pub fn new(kind_tag: &str) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(ID_TAG);
        let mut this = Self(h);
        this.bytes(kind_tag.as_bytes());
        this
    }

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

/// The digest of arbitrary content bytes (source files, configurations, manifests).
pub fn content_digest(bytes: &[u8]) -> Digest {
    IdHasher::new(kind::CONTENT).bytes(bytes).finish_digest()
}
