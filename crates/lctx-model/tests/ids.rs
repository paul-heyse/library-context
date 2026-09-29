//! Identity v2 (DESIGN §15.3): the encoding, the declared kinds and the recipes.

use lctx_model::decl::codebook::{Codebook, CodebookEntry};
use lctx_model::id::{Id, IdHasher, IdKind};

/// Sample recipes for the retained `recipe!` machinery; the production recipe catalog is retired
/// (the typed `domain` owns identity).
mod recipes {
    use lctx_model::id::Id;
    lctx_model::recipe!(occurrence, OCCURRENCE = Occurrence { module: Id, start: i64, end: i64, syntax_kind: &str });
    lctx_model::recipe!(symbol_key, SYMBOL_KEY = SymbolKey { distribution: &str, qualified_path: &str, descriptor: &str });
    pub const ALL: &[lctx_model::id::Recipe] = &[OCCURRENCE, SYMBOL_KEY];
}
use proptest::prelude::*;

/// The encoding written out by hand, independently of `IdHasher` and `recipe!`: the v2 domain,
/// the length-prefixed kind text, then each field as a length-prefixed presence byte followed by
/// the length-prefixed value.
fn by_hand(kind: &str, fields: &[&[u8]]) -> Id {
    let mut h = blake3::Hasher::new();
    h.update(b"lctx-id/v2");
    let mut part = |bytes: &[u8]| {
        h.update(&(bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    };
    part(kind.as_bytes());
    for field in fields {
        part(&[1]);
        part(field);
    }
    let mut out = [0u8; 16];
    out.copy_from_slice(&h.finalize().as_bytes()[..16]);
    Id(out)
}

#[test]
fn the_occurrence_recipe_is_the_documented_encoding() {
    let module = Id([7; 16]);
    let id = recipes::occurrence(module, 10, 24, "call");
    let expected = by_hand(
        "occurrence",
        &[&module.0, &10i64.to_le_bytes(), &24i64.to_le_bytes(), b"call"],
    );
    assert_eq!(id, expected);
}

#[test]
fn v1_and_v2_never_collide_on_the_same_kind_text() {
    let v1 = IdHasher::new("occurrence").str("x").finish_id();
    let v2 = IdHasher::v2(IdKind::Occurrence).str("x").finish_id();
    assert_ne!(v1, v2);
}

#[test]
fn absent_and_empty_are_distinct() {
    let absent = {
        let mut h = IdHasher::v2(IdKind::Place);
        lctx_model::id::RecipeField::feed(&None::<&str>, &mut h);
        h.finish_id()
    };
    let empty = {
        let mut h = IdHasher::v2(IdKind::Place);
        lctx_model::id::RecipeField::feed(&Some(""), &mut h);
        h.finish_id()
    };
    assert_ne!(absent, empty);
}

#[test]
fn known_answer_vectors_pin_the_encoding() {
    let vectors = [
        (
            "occurrence",
            recipes::occurrence(Id([1; 16]), 0, 5, "name").hex(),
        ),
        (
            "symbol_key",
            recipes::symbol_key("fastmcp", "fastmcp.server.FastMCP", "method:run").hex(),
        ),
    ];
    insta::assert_debug_snapshot!(vectors);
}

#[test]
fn the_kinds_are_an_append_only_codebook() {
    insta::assert_debug_snapshot!(CodebookEntry::of::<IdKind>());
    for (i, kind) in IdKind::all().iter().enumerate() {
        assert_eq!(kind.code(), i as i16, "dense from 0");
    }
}

#[test]
fn every_recipe_names_its_fields_in_hash_order() {
    assert_eq!(
        recipes::OCCURRENCE.fields,
        &["module", "start", "end", "syntax_kind"]
    );
    let names: Vec<_> = recipes::ALL.iter().map(|r| r.name).collect();
    let mut sorted = names.clone();
    sorted.dedup();
    assert_eq!(names.len(), sorted.len(), "recipe names are unique");
}

proptest! {
    /// Changing any one identity field changes the id.
    #[test]
    fn each_identity_field_moves_the_id(
        module in any::<[u8; 16]>(), other in any::<[u8; 16]>(),
        start in any::<i64>(), end in any::<i64>(), delta in 1i64..1000,
        kind in "[a-z_]{1,12}", kind2 in "[a-z_]{1,12}",
    ) {
        let base = recipes::occurrence(Id(module), start, end, &kind);
        if other != module {
            prop_assert_ne!(base, recipes::occurrence(Id(other), start, end, &kind));
        }
        prop_assert_ne!(base, recipes::occurrence(Id(module), start.wrapping_add(delta), end, &kind));
        prop_assert_ne!(base, recipes::occurrence(Id(module), start, end.wrapping_add(delta), &kind));
        if kind2 != kind {
            prop_assert_ne!(base, recipes::occurrence(Id(module), start, end, &kind2));
        }
    }
}
