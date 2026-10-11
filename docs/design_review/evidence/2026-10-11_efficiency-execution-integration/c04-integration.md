# C04 compiler packet — source only, 2026-10-11

Off-tree consumer draft against main HEAD 45e816a37b276420647a6538f9b8652a33c3bf2e plus the exact dirty compiler.rs captured under baseline/. packet.json records SHA256 per baseline/target. No repository source was changed; no Cargo, build, readiness, environment, service or fixture command ran.

## Packet and ownership

- c04.patch: combined consumer delta, SHA256 recorded in packet.json.
- provider-demand.patch: compiler_provider.rs + private compiler_provider/demand.rs. Independent of the new native API, but keep paired with direct-member implementation for C04 acceptance.
- held-members-dependent.patch: compiler.rs consumer delta. DEPENDENT on the real native capability packet below, not independently apply/compile ready.
- tree/: complete proposed files; baseline/: exact original files for merge review. Existing native selection.rs/installed_view.rs were not copied or edited.

## Demand behavior

The provider examines actual coerced Expr (DataFusion 55.1.0). It recognizes plain id of FixedSizeBinary(16), equality with a fixed-width 16-byte literal on either side, positive IN with only such literals or NULLs, and mandatory AND. Two finite demands intersect; one survives an unrecognized conjunct. OR, NOT IN, casts (including identity casts), aliases wrapping expressions, runtime/nonliteral operands and nonnominal representations supply no demand. SQL NULL equality yields explicit empty; positive IN discards NULLs and sorts/deduplicates. Qualified ordinary columns resolved by DataFusion remain ordinary columns.

Recognition borrows expressions without key allocation; extraction reserves peak key/vector state before allocation, then releases temporary intersection storage and retains the resulting capacity charge through the plan/stream. Existing Exact classification is preserved only where SupportedFilter already fully accepts the typed filter. Otherwise new recognition is Inexact; unsupported residual stays under DataFusion with needed columns. The advisory limit remains unused. Exact filters consisting solely of accepted nominal predicates are enforced by the charged membership intersection without repeating them against payload bodies. Every other accepted native predicate remains intact; newly Inexact filters remain intact above the source.

## Direct lookup consumer dependency — not yet earned by this packet

Native owner proposed the following real route (must be integrated/frozen before this compiler delta):

1. Native install_live_view under the actual open attempt, epoch/validity and immutable exact physical/incarnation map. It must audit full selected dependency closure, aliases/backing, no-extra membership and checked EOF, seal v2 namespace via native effect/CAS, and retain durable holds. Whole-attempt frozen completion alone is insufficient for predecessor reads.
2. Compiler NativeCompilerStore::installed_view(&CompletedView,&ResourceBudget) -> Result<Arc<HeldInstalledView>,ModelError>, async, caching/retaining actual owner with correct charge/pool and operation/attempt-drain ownership. It must support both registered views and the existing internal exact singleton reconstructed from a validated completed descriptor, without making arbitrary public DTOs authority.
3. HeldInstalledView::members(&NativeReader<()>,&[[u8;16]],&ResourceBudget) -> Result<installed_view::SelectedMembers,ModelError>, async. SelectedMembers::rows() -> &[InstalledMember], with public key:[u8;16], node:RecordId, content:ContentHash fields. Keys sorted/distinct, bounded by NATIVE_WINDOW_ROWS; returned present members sorted/distinct. Missing means absence only after actual held completion is validated. Member addresses use H(native-view-member/v2,held.realization,relation,key), with exact envelope/namespace/current validity/incarnation/root→member→node holds. Canonical aliases must already be admitted by the factory. Native member query cancellation/drop retains and drains owned transport independently; compact metadata/charges survive until SelectedMembers drops.

No constructor for HeldInstalledView, caller-claimed completion, global contributor lookup, native fallback, fake current row or audit copy is introduced here.

Once that API exists, row_tokens validates/holds before empty, uses bounded compact multi-get for sparse demand, and retains the existing cached sorted run for genuinely dense demand. Known-key SelectedRows does not clone an owner inventory or query compiler_membership. id-only and zero-column projection produce only compact semantic keys if no native value predicate/preparation remains. Otherwise the exact member nodes feed existing projected payload SQL. A cloned installed owner follows the payload CompilerRows into its actual transport-drop drain task. Existing broad/atomic sorted/pointer validation remains; this packet does not replace its selection machinery.

## Controls and limits

Eight new Rust controls are source only / NOT_RUN:

- compiler_provider::demand::tests::{equality_both_orders_and_null_keep_explicit_empty,positive_in_is_sorted_distinct_and_discards_only_nulls,conjunction_intersects_but_disjunction_is_never_mandatory,casts_aliases_and_non_nominal_representations_do_not_mint_keys,demand_storage_refuses_before_allocation_and_releases_with_owner}.
- compiler_provider::tests::{nominal_demand_retains_residual_projection_limit_and_plan_charge,actual_binary_coercion_does_not_strip_a_column_cast}. These use actual pinned analyzer/optimizer/provider planning; no connected server, no fabricated held capability.
- compiler::c04_projection_tests::compact_projection_never_bypasses_a_value_predicate_or_required_payload.

Compile/testing remains blocked by assigned source-only/capacity boundary and incomplete native factory integration. No persistence, native refusal, query-shape, latency or C04 closure is claimed.

After integration, independently enumerate a small admitted native view and compare sparse/dense, duplicate/missing, virtual/absent, aliased/conflicting, foreign backing/incarnation/namespace and incomplete-view outcomes. Include key-only and value predicates, explicit empty, zero columns, cancellation/drop drainage and tight-budget owner release. Direct-path query shape must show bounded point multi-get and no global membership scan.

Additional end-to-end boundary: DataFusion can replace a constant-false/NULL predicate by EmptyRelation before calling provider.scan. The raw empty scan now requires the installed getter, but that cannot validate a provider the optimizer eliminates entirely. If invalid-view refusal is required there, input/provider creation must already carry earned selected authority (or reject construction until it does); this consumer packet cannot obtain async native authority inside the current synchronous table_provider constructor. Do not call raw-scan controls closure of that optimizer-elided case.

## Locked API source checks

Read locally (no network/docs fetch): datafusion-expr-55.1.0 expr.rs Literal/BinaryExpr/InList/Cast declarations; datafusion-common-55.1.0 scalar/mod.rs is_null; datafusion-expr-common-55.1.0 type_coercion/binary.rs binary_coercion (FixedSizeBinary/Binary coerces Binary); datafusion-55.1.0 dataframe/mod.rs filter/select/limit; datafusion-common-55.1.0 dfschema.rs has_column_with_unqualified_name; datafusion-physical-plan-55.1.0 streaming.rs filter/limit/projection execution contract. Skill df.pushdown contract read earlier; exact workspace pins DataFusion55.1.0/Arrow59.3.0 verified.
