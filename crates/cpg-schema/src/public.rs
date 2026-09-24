//! The one public-path authority (the holistic assessment's A1): every name a consumer shows, a
//! seed resolves by, a bundle promotes on or a gold operation matches is a row of the
//! `public_paths` relation, persisted as the analysis table of the same name.
//!
//! - **Exports.** Each declaration exported under a public root (bound as `$roots`) by a path with
//!   no private segment: its functions, and its **classes**. Where one path names several
//!   declarations, the one the seed resolution picks: an implementation before a stub, then the
//!   least node.
//! - **Members.** Each public member (or `__init__`, `__call__`: [`DUNDER_MEMBERS`]) an exported
//!   class declares or inherits along its MRO, by the rule `member()` resolves seeds with
//!   (DESIGN §9.1 step 1; slice 1.4 review F2):
//!   - the nearest class along the MRO that defines the name wins, and among its definitions the
//!     seed rank picks one: an implementation before an `@overload` stub, then the one Pysa
//!     describes, then the last in source order;
//!   - nothing is inherited past an **unresolved base** or an ancestor **outside the release**
//!     that precedes the definition (either may define the name), and nothing when a class up to
//!     and including the defining one **binds the name otherwise** (an assignment or import, such
//!     as `tool = helper`).
//! - **`own`** is true exactly when the path's export declares the node: a direct export, or a
//!   member its exported class defines itself.
//! - **The preferred path** (the increment-2 review's F4), the one name every consumer shows a
//!   node by: an own path first (so a classmethod is never named through a subclass it would bind
//!   differently), then the fewest segments, then the least. Exactly one per node.

use crate::codebook::{AncestryRelation, BindingKind, Codebook, DeclarationKind, LexicalScopeKind};
use crate::flows::codes;

/// The dunder members that are public API: construction and calling.
pub const DUNDER_MEMBERS: &[&str] = &["__init__", "__call__"];

/// The declaration kinds a public path names: functions and classes.
pub const PATH_KINDS: &[DeclarationKind] = &[
    DeclarationKind::Function,
    DeclarationKind::AsyncFunction,
    DeclarationKind::Class,
];

fn public_paths_sql() -> String {
    let dunders = DUNDER_MEMBERS
        .iter()
        .map(|d| format!("'{d}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "WITH exported AS ( \
           SELECT access_path, node_id, export_node_id, snapshot_id FROM ( \
             SELECT e.access_path, e.declaration_node_id AS node_id, e.export_node_id, \
                    e.snapshot_id, row_number() OVER (PARTITION BY e.access_path \
                      ORDER BY s.is_stub, e.declaration_node_id, e.export_node_id) AS pick \
             FROM exports e JOIN declarations d ON d.node_id = e.declaration_node_id \
             JOIN source_files s ON s.module_node_id = d.module_node_id \
             WHERE array_has($roots, split_part(e.access_path, '.', 1)) \
               AND strpos(e.access_path, '._') = 0 AND NOT starts_with(e.access_path, '_')) \
           WHERE pick = 1), \
         direct AS ( \
           SELECT x.snapshot_id, x.node_id, x.access_path, x.export_node_id, d.kind, true AS own \
           FROM exported x JOIN declarations d ON d.node_id = x.node_id \
           WHERE d.kind IN ({kinds})), \
         classes AS ( \
           SELECT x.snapshot_id, x.node_id AS class_node_id, x.access_path, x.export_node_id \
           FROM exported x JOIN declarations c ON c.node_id = x.node_id AND c.kind = {class}), \
         chain AS ( \
           SELECT class_node_id, class_node_id AS entry, -1 AS ordinal FROM classes \
           UNION ALL \
           SELECT c.class_node_id, t.ancestor_node_id, a.ordinal FROM classes c \
           JOIN ancestry_targets t ON t.class_node_id = c.class_node_id \
           JOIN class_ancestry a ON a.fact_id = t.ancestry_fact_id AND a.relation = {mro}), \
         blocked AS ( \
           SELECT ch.class_node_id, min(ch.ordinal) AS ordinal FROM chain ch \
           LEFT JOIN declarations c ON c.node_id = ch.entry AND c.kind = {class} \
           WHERE ch.entry IS NULL OR c.node_id IS NULL \
           GROUP BY ch.class_node_id), \
         rebound AS ( \
           SELECT DISTINCT ch.class_node_id, ch.ordinal, b.name FROM chain ch \
           JOIN scopes s ON s.owner_node_id = ch.entry AND s.kind = {class_scope} \
           JOIN bindings b ON b.scope_id = s.node_id \
           WHERE b.kind NOT IN ({definitions})), \
         defs AS ( \
           SELECT ch.class_node_id, ch.ordinal, d.name, d.node_id, d.kind, \
                  row_number() OVER (PARTITION BY ch.class_node_id, ch.ordinal, d.name \
                    ORDER BY CASE WHEN d.is_overload THEN 0 ELSE 2 END \
                             + CASE WHEN m.node_id IS NULL THEN 0 ELSE 1 END DESC, \
                             d.start_byte DESC, d.node_id) AS pick \
           FROM chain ch JOIN declarations d ON d.parent_node_id = ch.entry \
           LEFT JOIN provider_node_map m ON m.node_id = d.node_id), \
         nearest AS ( \
           SELECT class_node_id, name, min(ordinal) AS ordinal FROM defs \
           GROUP BY class_node_id, name), \
         members AS ( \
           SELECT d.class_node_id, d.node_id, d.name, d.kind, d.ordinal FROM defs d \
           JOIN nearest n ON n.class_node_id = d.class_node_id AND n.name = d.name \
             AND n.ordinal = d.ordinal \
           LEFT JOIN blocked k ON k.class_node_id = d.class_node_id \
           LEFT ANTI JOIN rebound r ON r.class_node_id = d.class_node_id \
             AND r.name = d.name AND r.ordinal <= d.ordinal \
           WHERE d.pick = 1 AND (k.ordinal IS NULL OR d.ordinal < k.ordinal)), \
         methods AS ( \
           SELECT c.snapshot_id, m.node_id, c.access_path || '.' || m.name AS access_path, \
                  c.export_node_id, m.kind, m.ordinal = -1 AS own \
           FROM members m JOIN classes c ON c.class_node_id = m.class_node_id \
           WHERE m.kind IN ({kinds}) \
             AND (NOT starts_with(m.name, '_') OR m.name IN ({dunders}))), \
         paths AS (SELECT * FROM direct UNION ALL SELECT * FROM methods), \
         ranked AS ( \
           SELECT *, row_number() OVER ( \
             PARTITION BY node_id ORDER BY own DESC, \
               length(access_path) - length(replace(access_path, '.', '')), access_path) AS rank \
           FROM paths) \
         SELECT snapshot_id, node_id, access_path, export_node_id, kind, own, \
                rank = 1 AS preferred \
         FROM ranked ORDER BY node_id, access_path",
        kinds = codes(PATH_KINDS),
        class = DeclarationKind::Class.code(),
        mro = AncestryRelation::Mro.code(),
        class_scope = LexicalScopeKind::Class.code(),
        definitions = codes(&[
            BindingKind::FunctionDef,
            BindingKind::ClassDef,
            BindingKind::AnnotationOnly
        ]),
    )
}

crate::relations! {
    inventory all;
    /// Every public path of the release under `$roots` (a list of root package names), as
    /// `PublicPathsRow`s, ordered by node and path.
    public_paths = "public_paths",
        deps = [
            "exports", "declarations", "source_files", "ancestry_targets", "class_ancestry",
            "scopes", "bindings", "provider_node_map",
        ],
        sql = public_paths_sql();
}
