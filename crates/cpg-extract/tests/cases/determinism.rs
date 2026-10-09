//! Typed production facts retain identity across physical layouts and transfer boundaries.
use crate::typed_driver;
use crate::inspector;
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::{CapturedInputs, refuse_ambient},
    capture::CapturedInput,
    pyrefly_stage::Pyrefly,
    typed_syntax::SyntaxLimits,
};
use lctx_model::domain::{batching::TransferLimits, stages::Profile};
use typed_driver::Tables;
inspector!(All, source::Occurrence);
fn captured(reverse: bool, profile: Profile) -> std::sync::Arc<CapturedInputs> {
    let mut inputs = vec![];
    for label in ["one", "two"] {
        let root = tempfile::tempdir().unwrap();
        let mut paths = vec![];
        for (name, source) in [
            ("a.py", "from b import f\nx = f(3).bit_length()\n"),
            ("b.py", "from a import x\ndef f(n):\n    return n\n"),
        ] {
            std::fs::write(root.path().join(name), source).unwrap();
            paths.push(name.to_string());
        }
        if reverse {
            paths.reverse();
        }
        inputs.push(AcquiredInput::tree(
            CapturedInput::capture(root.path(), &paths, &typed_driver::budget()).unwrap(),
            label,
        ));
    }
    if reverse {
        inputs.reverse();
    }
    std::sync::Arc::new(CapturedInputs::new(
        inputs,
        cpg_extract::native_context::NativeContextConfig::committed(
            profile,
            &typed_driver::budget(),
        )
        .unwrap(),
    ))
}
#[tokio::test]
async fn shuffle_relocation_repeat_and_transfer_sizes_preserve_content() {
    for profile in Profile::ALL {
        let mut expected = None;
        for (rows, reverse) in [(4096, false), (1, true), (97, false), (4096, true)] {
            let digest = typed_driver::run_profile_with_limits(
                captured(reverse, profile),
                Pyrefly::new(SyntaxLimits::default()),
                All(Tables::default()),
                profile,
                TransferLimits {
                    rows,
                    ..TransferLimits::default()
                },
                reverse,
            )
            .await
            .unwrap();
            if let Some(expected) = expected {
                assert_eq!(
                    digest,
                    expected,
                    "{} rows={rows} reverse={reverse}",
                    profile.name()
                );
            } else {
                expected = Some(digest);
            }
        }
    }
}
#[test]
fn ambient_configuration_is_refused_by_the_shared_preflight() {
    for name in [
        "PYREFLY_STACK_SIZE",
        "PYREFLY_FIXPOINT_DETAILS",
        "PYSA_DUMP",
        "PYSA_DUMP_CALL_GRAPH",
    ] {
        assert!(
            refuse_ambient([(
                std::ffi::OsString::from(name),
                std::ffi::OsString::from("1")
            )])
            .is_err()
        );
    }
    assert!(refuse_ambient([]).is_ok());
}
