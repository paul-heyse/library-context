//! Bounded evidence only: no production cache, input scheduler or persistent query authority.
use cpg_core::catalog::{Contracts, PreparedCatalog};
use cpg_schema::{Id, findings::PublicPathsRow};
use salsa::Setter;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, salsa::SalsaValue)]
struct Pinned {
    // These contain only owned canonical values and no Salsa-managed IDs or references.
    #[salsa_value(unsafe(prove_safe_to_retain_manually))]
    catalog: Arc<PreparedCatalog>,
    #[salsa_value(unsafe(prove_safe_to_retain_manually))]
    public: Arc<Vec<PublicPathsRow>>,
}
#[salsa::input]
struct Inputs {
    #[no_eq]
    pinned: Pinned,
    roots: Vec<String>,
    snapshot: String,
}
#[salsa::db]
#[derive(Clone)]
struct Database {
    storage: salsa::Storage<Self>,
    events: Arc<Mutex<Vec<String>>>,
}
#[salsa::db]
impl salsa::Database for Database {}
impl Default for Database {
    fn default() -> Self {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        Self {
            storage: salsa::Storage::new(Some(Box::new(move |event| {
                if matches!(
                    event.kind,
                    salsa::EventKind::WillExecute { .. }
                        | salsa::EventKind::DidValidateMemoizedValue { .. }
                ) {
                    sink.lock().unwrap().push(format!("{:?}", event.kind));
                }
            }))),
            events,
        }
    }
}
fn selected(public: &[PublicPathsRow], roots: &[String]) -> Vec<PublicPathsRow> {
    public
        .iter()
        .filter(|p| cpg_schema::catalog::under_roots(&p.access_path, roots))
        .cloned()
        .collect()
}
fn bytes(contracts: &Contracts) -> Vec<u8> {
    // This comparison encoding is not an external format or a canonical publication digest.
    format!("{contracts:?}").into_bytes()
}
#[salsa::tracked]
fn catalog_bytes(db: &dyn salsa::Database, input: Inputs) -> Vec<u8> {
    let pinned = input.pinned(db);
    let roots = input.roots(db);
    let public = selected(&pinned.public, roots);
    bytes(
        &pinned
            .catalog
            .derive(Id::from_hex(input.snapshot(db)).unwrap(), roots, &public)
            .unwrap(),
    )
}
fn comparison_digest(bytes: &[u8]) -> String {
    cpg_schema::IdHasher::new("pr2-probe-comparison")
        .bytes(bytes)
        .finish_digest()
        .hex()
}
#[salsa::tracked]
fn output_digest(db: &dyn salsa::Database, input: Inputs) -> String {
    comparison_digest(catalog_bytes(db, input))
}
fn rss() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:")
                .and_then(|n| n.split_whitespace().next())
                .and_then(|n| n.parse().ok())
        })
        .unwrap_or(0)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (_, ctx) = cpg_core::snapshot::published(
        std::path::Path::new(&args[1]),
        Id::from_hex(&args[2]).unwrap(),
    )
    .await?
    .ok_or("published snapshot missing")?;
    let facts = cpg_core::catalog::load_facts(&ctx).await?;
    let public: Vec<PublicPathsRow> = cpg_core::sql::fetch(
        &ctx,
        &cpg_schema::public::public_paths(),
        cpg_core::sql::Params::new().texts("roots", [&args[3]]),
    )
    .await?;
    let start = Instant::now();
    let prepared = Arc::new(PreparedCatalog::new(facts));
    let prepare_us = start.elapsed().as_micros();
    let snapshot = Id::from_hex(&args[2]).unwrap();
    let mut db = Database::default();
    let input = Inputs::new(
        &db,
        Pinned {
            catalog: prepared.clone(),
            public: Arc::new(public.clone()),
        },
        vec![args[3].clone()],
        snapshot.hex(),
    );
    let mut records = Vec::new();
    let narrow = format!("{}.Options", args[3]);
    for (label, roots) in [
        ("initial", vec![args[3].clone()]),
        ("unchanged", vec![args[3].clone()]),
        ("redundant_setter", vec![args[3].clone()]),
        ("narrow", vec![narrow.clone()]),
        ("expand", vec![args[3].clone()]),
        ("restore", vec![narrow.clone()]),
        (
            "absent_lookup",
            vec![narrow.clone(), "unavailable_root".into()],
        ),
        ("unchanged_again", vec![narrow, "unavailable_root".into()]),
    ] {
        let paths = selected(&public, &roots);
        let start = Instant::now();
        let clean =
            cpg_core::catalog::derive_contracts(prepared.facts(), snapshot, &roots, &paths)?;
        let clean_bytes = bytes(&clean);
        let clean_digest = comparison_digest(&clean_bytes);
        let clean_us = start.elapsed().as_micros();
        let start = Instant::now();
        let indexed = prepared.derive(snapshot, &roots, &paths)?;
        let indexed_digest = comparison_digest(&bytes(&indexed));
        let indexed_us = start.elapsed().as_micros();
        assert_eq!(clean, indexed);
        assert_eq!(clean_digest, indexed_digest);
        if input.roots(&db) != &roots || label == "redundant_setter" {
            input.set_roots(&mut db).to(roots.clone());
        }
        db.events.lock().unwrap().clear();
        let start = Instant::now();
        let digest = output_digest(&db, input).clone();
        let salsa_us = start.elapsed().as_micros();
        assert_eq!(catalog_bytes(&db, input), &clean_bytes);
        assert_eq!(digest, clean_digest);
        records.push(serde_json::json!({"case":label,"roots":roots,"clean_us":clean_us,"indexed_us":indexed_us,"salsa_us":salsa_us,"rss_kib":rss(),"equal":true,"comparison_digest":digest,"events":db.events.lock().unwrap().clone()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"salsa":"0.28.2","scope":"same pinned facts; changed roots only","prepare_us":prepare_us,"records":records})
        )?
    );
    Ok(())
}
