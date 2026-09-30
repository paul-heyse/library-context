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

/// Plan A2 (T6): derived artifacts are written only into the frozen copy's reserved `_lctx/`
/// namespace and join its manifest; no original may live there.
#[test]
fn derivations_live_only_in_the_frozen_reserved_namespace() {
    use cpg_extract::capture::Derived;
    let input = tempfile::tempdir().unwrap();
    fs::write(input.path().join("doc.md"),b"# Doc\n").unwrap(); fs::write(input.path().join("x.py"),b"x = 1").unwrap();
    let budget = ResourceBudget::fixed(4 << 20).unwrap();
    let paths = vec!["doc.md".to_owned(),"x.py".to_owned()];
    let block = |path: &str| Derived { path: path.into(),document: "doc.md".into(),ordinal: 0,fence: (0,1),bytes: b"y = 2".to_vec() };
    let captured = CapturedInput::capture_derived(input.path(),&paths,&budget,&["doc.md".into()],|document,bytes| {
        assert_eq!((document,bytes),("doc.md",b"# Doc\n".as_slice()),"derivations read the frozen document"); Ok(vec![block("_lctx/blocks/b.py")])
    }).unwrap();
    assert_eq!(captured.artifacts().iter().map(|a| a.path.as_str()).collect::<Vec<_>>(),["_lctx/blocks/b.py","doc.md","x.py"]);
    assert_eq!(captured.derivations().len(),1);
    assert!(captured.root().join("_lctx/blocks/b.py").exists() && !input.path().join("_lctx").exists(),"only the frozen copy holds the derivation");
    captured.verify().unwrap(); drop(captured);
    for (paths,derived,documents) in [(vec!["_lctx/x.py".to_owned()],"_lctx/y.py","doc.md"),(paths.clone(),"blocks/b.py","doc.md"),(paths.clone(),"_lctx/b.py","x.md")] {
        let _ = fs::create_dir_all(input.path().join("_lctx")); let _ = fs::write(input.path().join("_lctx/x.py"),b"z");
        let result = CapturedInput::capture_derived(input.path(),&paths,&budget,&[documents.into()],|_,_| Ok(vec![block(derived)]));
        assert!(result.is_err(),"{paths:?} deriving {derived} from {documents}");
        assert_eq!(budget.reserved(),0);
    }
    let twice = CapturedInput::capture_derived(input.path(),&["doc.md".to_owned()],&budget,&["doc.md".into()],|_,_| Ok(vec![block("_lctx/b.py"),block("_lctx/b.py")]));
    assert!(matches!(twice,Err(CaptureError::Reserved(_))),"a derived path is written once");
}
