//! Append-only `Int16` codebooks (DESIGN §3.5, §15.2).
//!
//! A codebook is a closed category stored as `Int16`. Codes are dense from 0 and **never
//! renumbered, reordered or removed**; a new value is appended with the next code. Each registry
//! that declares codebooks snapshots them, so a change to an existing code is a reviewed diff.

/// A closed category stored as `Int16` and validated against its codebook.
pub trait Codebook: Copy + Eq + std::fmt::Debug + 'static {
    /// The codebook's name, carried in Arrow field metadata (`lctx.codebook`).
    const NAME: &'static str;
    /// Every value, in code order.
    fn all() -> &'static [Self];
    fn code(self) -> i16;
    fn text(self) -> &'static str;
    fn from_code(code: i16) -> Option<Self>;
}

/// One codebook as data: its name and `(code, text)` pairs in code order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodebookEntry {
    pub name: &'static str,
    pub values: Vec<(i16, &'static str)>,
}

impl CodebookEntry {
    pub fn of<C: Codebook>() -> Self {
        Self {
            name: C::NAME,
            values: C::all().iter().map(|c| (c.code(), c.text())).collect(),
        }
    }
}

/// Declares a codebook enum and its [`Codebook`] impl.
#[macro_export]
macro_rules! codebook {
    ($(#[$meta:meta])* $ty:ident = $name:literal { $($(#[$vmeta:meta])* $variant:ident = $code:literal => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(i16)]
        pub enum $ty { $($(#[$vmeta])* $variant = $code),+ }

        impl $crate::decl::codebook::Codebook for $ty {
            const NAME: &'static str = $name;
            fn all() -> &'static [Self] { &[$($ty::$variant),+] }
            fn code(self) -> i16 { self as i16 }
            fn text(self) -> &'static str { match self { $($ty::$variant => $text),+ } }
            fn from_code(code: i16) -> Option<Self> {
                match code { $($code => Some($ty::$variant),)+ _ => None }
            }
        }
    };
}
