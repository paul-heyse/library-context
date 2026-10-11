//! Physical recovery declaration adapter, shared by admission, selection and restore.
use lctx_model::domain as d;

/// Native table and data-only codec classification. Publication is the enclosing manifest
/// root, not completed content; it never enters the semantic completed-state format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryTableKind {
    Content(d::recovery_closure::RecoveryFamily),
    PublicationRoot,
}
#[derive(Debug, Clone, Copy)]
pub struct RecoveryTable {
    pub name: &'static str,
    pub kind: RecoveryTableKind,
    pub relation: bool,
}
/// Physical names are an exhaustive adapter over the model-owned recovery declaration.
pub const fn recovery_tables(
    family: d::recovery_closure::RecoveryFamily,
) -> &'static [&'static str] {
    use d::recovery_closure::RecoveryFamily::*;
    match family {
        Contributions => &["compiler_contribution"],
        Memberships => &["compiler_membership"],
        Views => &["compiler_view"],
        TypedBacking => &["compiler_record"],
        Bindings => &["compiler_binding"],
        Aliases => &["compiler_alias"],
        GraphPayloads => &["entity", "assertion"],
        GraphRoles => &["participant", "reference"],
        ExternalEndpoints => &["external"],
        OriginalHeaders => &["original"],
        OriginalChunks => &["original_chunk"],
        SearchOccurrences => &["lex_occurs", "vec_occurs"],
        SearchDocuments => &[
            "search_api_options",
            "search_documentation_deployment",
            "search_scenario",
            "search_source",
        ],
        SearchVectors => &["vector"],
    }
}
/// Codec behavior is exhaustive too: a new semantic family cannot inherit a default kind.
pub const fn recovery_relation(family: d::recovery_closure::RecoveryFamily) -> bool {
    use d::recovery_closure::RecoveryFamily::*;
    match family {
        GraphRoles | SearchOccurrences => true,
        Contributions | Memberships | Views | TypedBacking | Bindings | Aliases | GraphPayloads
        | ExternalEndpoints | OriginalHeaders | OriginalChunks | SearchDocuments
        | SearchVectors => false,
    }
}
pub fn recovery_table_inventory() -> impl Iterator<Item = RecoveryTable> {
    d::recovery_closure::RECOVERY_FAMILIES
        .iter()
        .copied()
        .flat_map(|family| {
            recovery_tables(family)
                .iter()
                .map(move |name| RecoveryTable {
                    name,
                    kind: RecoveryTableKind::Content(family),
                    relation: recovery_relation(family),
                })
        })
        .chain(std::iter::once(RecoveryTable {
            name: "publication",
            kind: RecoveryTableKind::PublicationRoot,
            relation: false,
        }))
}
pub fn classify_recovery_table(name: &str) -> Option<RecoveryTable> {
    recovery_table_inventory().find(|table| table.name == name)
}
pub const fn completed_table(family: d::recovery_closure::RecoveryFamily) -> &'static str {
    recovery_tables(family)[0]
}
pub const STATE_TABLES: [&str; 6] = {
    let families = d::recovery_closure::COMPLETED_FAMILIES;
    let mut names = [""; 6];
    let mut index = 0;
    while index < families.len() {
        names[index] = completed_table(families[index]);
        index += 1;
    }
    names
};
