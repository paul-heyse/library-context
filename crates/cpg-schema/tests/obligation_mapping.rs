//! The model's obligation codebook absorbs the legacy boundary reasons code for code and text for
//! text, and every legacy kernel boundary maps (cutover plan §3.1, WP0.4).

use cpg_schema::codebook::BoundaryReason;
use cpg_schema::condition_kernel::KernelBoundary;
use lctx_model::decl::codebook::Codebook;
use lctx_model::obligation::legacy;

#[test]
fn every_legacy_boundary_reason_is_the_obligation_of_its_code() {
    for reason in BoundaryReason::all() {
        let obligation = legacy::from_boundary_code(reason.code()).expect("mapped");
        assert_eq!(obligation.code(), reason.code());
        assert_eq!(obligation.text(), reason.text(), "{reason:?}");
    }
}

#[test]
fn every_legacy_kernel_boundary_maps() {
    for boundary in [
        KernelBoundary::SourceOverBudget,
        KernelBoundary::AtomLimit,
        KernelBoundary::WorkPreflight,
        KernelBoundary::NodeLimit,
        KernelBoundary::TransferUnsupported,
        KernelBoundary::AtomNameCollision,
    ] {
        assert!(legacy::from_kernel_code(boundary.code()).is_some(), "{boundary:?}");
    }
}
