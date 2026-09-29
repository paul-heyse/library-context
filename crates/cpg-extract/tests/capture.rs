use std::{fs,io::Write};
use cpg_extract::capture::{CapturedInput,CaptureError};
use lctx_model::domain::{ContentHasher,ContentHash,Record,artifact::{ArtifactVerifier,ARTIFACT_CHUNK_BYTES},resources::ResourceBudget};

#[test]
fn complete_bytes_stream_under_small_memory_and_frozen_input_outlives_original_changes() {
    let input = tempfile::tempdir().unwrap();
    let mut file = fs::File::create(input.path().join("large.bin")).unwrap();
    let block = vec![0x80u8;64 * 1024]; let mut digest = ContentHasher::default();
    for _ in 0..(65 * 16) { file.write_all(&block).unwrap(); digest.update(&block); }
    drop(file);
    fs::write(input.path().join("empty.py"),b"").unwrap();
    fs::write(input.path().join("binary.py"),[0xff,0x00,0x80]).unwrap();
    let budget = ResourceBudget::fixed(3 << 20).unwrap();
    let captured = CapturedInput::capture(input.path(),&["large.bin".into(),"empty.py".into(),"binary.py".into()],&budget).unwrap();
    assert_eq!(captured.artifacts().iter().map(|a| a.path.as_str()).collect::<Vec<_>>(),vec!["binary.py","empty.py","large.bin"]);
    assert_eq!(captured.artifacts()[2].content,digest.finish());
    let metadata = budget.reserved(); assert!(metadata > 0 && metadata < 64 * 1024);
    let original_revision = captured.revision().id();
    fs::write(input.path().join("binary.py"),b"changed after capture").unwrap();
    fs::remove_file(input.path().join("large.bin")).unwrap();
    for (index,artifact) in captured.artifacts().iter().enumerate() {
        assert_eq!(artifact.input,original_revision);
        let mut verifier = ArtifactVerifier::new(artifact).unwrap(); let mut bytes = 0;
        let count = captured.emit_chunks(index,|chunk| {
            assert!(budget.reserved() > metadata && budget.reserved() <= 3 << 20);
            assert!(chunk.body.0.len() <= ARTIFACT_CHUNK_BYTES);
            verifier.push(chunk)?; bytes += chunk.body.0.len(); Ok(())
        }).unwrap();
        verifier.finish().unwrap(); assert_eq!(bytes as i64,artifact.byte_len);
        assert_eq!(count,match artifact.path.as_str() { "large.bin" => 65,"empty.py" => 0,_ => 1 });
        assert_eq!(budget.reserved(),metadata);
    }
    let root = captured.root().to_owned(); captured.verify().unwrap(); drop(captured);
    assert_eq!(budget.reserved(),0); assert!(!root.exists());
}

#[test]
fn capture_rejects_ambiguous_paths_budget_refusal_and_modified_frozen_content() {
    let input = tempfile::tempdir().unwrap(); fs::write(input.path().join("x.py"),b"x = 1").unwrap();
    let budget = ResourceBudget::fixed(4 << 20).unwrap();
    for paths in [vec!["../x.py".into()],vec!["x.py".into(),"x.py".into()]] {
        assert!(CapturedInput::capture(input.path(),&paths,&budget).is_err()); assert_eq!(budget.reserved(),0);
    }
    let small = ResourceBudget::fixed(1024).unwrap();
    assert!(matches!(CapturedInput::capture(input.path(),&["x.py".into()],&small),Err(CaptureError::Model(_))));
    assert_eq!(small.reserved(),0);
    let frozen = CapturedInput::capture(input.path(),&["x.py".into()],&budget).unwrap();
    assert_eq!(frozen.artifacts()[0].content,ContentHash::of(b"x = 1"));
    let baseline = budget.reserved();
    let result = frozen.emit_chunks(0, |_| Err(lctx_model::domain::ModelError::Invalid("downstream refused".into())));
    assert!(result.is_err()); assert_eq!(budget.reserved(),baseline);
    fs::remove_file(frozen.root().join("x.py")).unwrap(); fs::write(frozen.root().join("x.py"),b"y = 2").unwrap();
    assert!(frozen.verify().is_err()); assert_eq!(budget.reserved(),baseline);
}

#[cfg(unix)]
#[test]
fn input_symlinks_are_not_implicit_acquisition_members() {
    let input = tempfile::tempdir().unwrap(); let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.py"),b"outside").unwrap();
    std::os::unix::fs::symlink(outside.path(),input.path().join("linked")).unwrap();
    let budget = ResourceBudget::fixed(4 << 20).unwrap();
    assert!(matches!(CapturedInput::capture(input.path(),&["linked/outside.py".into()],&budget),Err(CaptureError::Unsupported(_))));
    assert_eq!(budget.reserved(),0);
}
