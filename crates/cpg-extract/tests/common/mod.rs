//! Shared helpers: run the extractor on a fixture copied into a temp dir, and read columns.
#![allow(dead_code, reason = "each test binary uses a subset")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use arrow_array::{Array, RecordBatch};
use arrow_cast::display::array_value_to_string;
use cpg_extract::{ExtractInput, ExtractOutput, Release, extract};
use cpg_schema::id::Id;

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture(name: &str) -> PathBuf {
    repo().join("fixtures/python").join(name)
}

pub fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let p = entry.unwrap().path();
        let target = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_tree(&p, &target);
        } else {
            std::fs::copy(&p, &target).unwrap();
        }
    }
}

/// A fixture laid out as an analysis tree plus an empty analysis venv, both in `root`.
pub struct Layout {
    pub release_root: PathBuf,
    pub venv_root: PathBuf,
    pub site_packages: PathBuf,
}

pub fn layout(fixture_name: &str, root: &Path) -> Layout {
    let release_root = root.join("release");
    copy_tree(&fixture(fixture_name), &release_root);
    let venv_root = root.join("venv");
    let site_packages = venv_root.join("lib/python3.14/site-packages");
    std::fs::create_dir_all(&site_packages).unwrap();
    Layout {
        release_root: std::fs::canonicalize(release_root).unwrap(),
        venv_root: std::fs::canonicalize(venv_root).unwrap(),
        site_packages: std::fs::canonicalize(site_packages).unwrap(),
    }
}

pub fn input(l: &Layout, label: &str) -> ExtractInput {
    ExtractInput {
        release: Release::from_tree(l.release_root.clone(), label).unwrap(),
        venv_root: l.venv_root.clone(),
        site_packages: vec![l.site_packages.clone()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: Id([7; 16]),
        keep_pysa_json: false,
        test_hooks: Default::default(),
    }
}

pub fn run(fixture_name: &str) -> (tempfile::TempDir, ExtractOutput) {
    let dir = tempfile::tempdir().unwrap();
    let l = layout(fixture_name, dir.path());
    let out = extract(&input(&l, fixture_name)).expect("extraction succeeds");
    (dir, out)
}

/// Cell `row` of `column` rendered as text ("" for null).
pub fn cell(batch: &RecordBatch, column: &str, row: usize) -> String {
    let col = batch.column(batch.schema().index_of(column).unwrap());
    if col.is_null(row) {
        String::new()
    } else {
        array_value_to_string(col, row).unwrap()
    }
}

pub fn column(batch: &RecordBatch, column: &str) -> Vec<String> {
    (0..batch.num_rows())
        .map(|r| cell(batch, column, r))
        .collect()
}

/// `fact_id` (rendered) → the fact's `(origin, modality)` codes from the `facts` table.
pub fn provenance(out: &ExtractOutput) -> HashMap<String, (String, String)> {
    let facts = out.table("facts").unwrap();
    (0..facts.num_rows())
        .map(|r| {
            (
                cell(facts, "fact_id", r),
                (cell(facts, "origin", r), cell(facts, "modality", r)),
            )
        })
        .collect()
}
