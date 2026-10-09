//! Disposable native compilation products. These bytes never confer completed-view authority.
//! Local filesystem leases coordinate every process using the same managed local database.
use crate::{RuntimeConfig, ReuseConfig, reader};
use lctx_model::domain::{ModelError, compilation_product::{ProductRequest, PortableProduct}, resources::{ResourceBudget, Reservation}};
use std::{fs::{File, OpenOptions}, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicU64, Ordering}}, io::Write};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Bytes, RecordId, SurrealValue, Variables}};
const VERSION: u32 = 5;
const CHUNK: usize = lctx_model::domain::resources::TRANSFER_BYTES;
static NONCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone)]
pub struct NativeProductCache { client: Arc<Surreal<Client>>, config: ReuseConfig, directory: PathBuf, generation: String }
/// A borrowed product retains its lifecycle pin and allocation until the final consumer drops it.
pub struct LeasedProduct { product: PortableProduct, _entry: File, _lifecycle: File, _charge: Box<dyn Reservation> }
impl LeasedProduct { pub fn product(&self) -> &PortableProduct { &self.product } }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Header { version: u32, directory: String, generation: String }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct CoordinationOwner { directory: String, generation: String }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Quota { used:u64 }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Entry { id: RecordId, key: String, created: u64, request: Bytes, digest: String, bytes: u64, chunks: u32, staging: String, generation: String }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Stage { id: RecordId, staging: String, bytes: u64 }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Inventory { id: RecordId, key: String, bytes: u64, chunks: u32, staging: String }
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Chunk { id: RecordId, staging: String, bytes: Bytes }
#[derive(serde::Serialize,serde::Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
struct AcknowledgedEntry {generation:String,key:String,staging:String,digest:String}
fn coordination_path(directory:&Path,kind:&str,identity:&str)->PathBuf {
    let stripe=lctx_model::domain::ContentHash::of(identity.as_bytes()).hex();
    directory.join(format!("{kind}_{}.lock", &stripe[..3]))
}
fn fenced_sql(body:&str)->String {
    format!("BEGIN; IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $generation {{ THROW 'retired product generation'; }}; {body}; COMMIT;")
}
const PRODUCT_SCHEMA: &[&str] = &[
            "DEFINE TABLE IF NOT EXISTS product_header TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS version ON product_header TYPE int",
            "DEFINE FIELD IF NOT EXISTS directory ON product_header TYPE string",
            "DEFINE FIELD IF NOT EXISTS generation ON product_header TYPE string",
            "DEFINE TABLE IF NOT EXISTS product_quota TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS used ON product_quota TYPE int ASSERT $value >= 0",
            "DEFINE TABLE IF NOT EXISTS product_entry TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS key ON product_entry TYPE string",
            "DEFINE FIELD IF NOT EXISTS created ON product_entry TYPE int",
            "DEFINE FIELD IF NOT EXISTS request ON product_entry TYPE bytes",
            "DEFINE FIELD IF NOT EXISTS digest ON product_entry TYPE string",
            "DEFINE FIELD IF NOT EXISTS bytes ON product_entry TYPE int ASSERT $value > 0",
            "DEFINE FIELD IF NOT EXISTS chunks ON product_entry TYPE int ASSERT $value > 0",
            "DEFINE FIELD IF NOT EXISTS staging ON product_entry TYPE string",
            "DEFINE FIELD IF NOT EXISTS generation ON product_entry TYPE string",
            "DEFINE TABLE IF NOT EXISTS product_stage TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS staging ON product_stage TYPE string",
            "DEFINE FIELD IF NOT EXISTS bytes ON product_stage TYPE int ASSERT $value > 0",
            "DEFINE TABLE IF NOT EXISTS product_chunk TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS bytes ON product_chunk TYPE bytes",
            "DEFINE FIELD IF NOT EXISTS staging ON product_chunk TYPE string",
        ];
fn installation_body(reset:bool)->String {
    let mut sql=String::from("IF (SELECT VALUE generation FROM product_owner:current FOR UPDATE)[0] != $owner_generation { THROW 'superseded product installation'; }; UPDATE product_owner:current SET generation=$generation; UPDATE product_header:current SET generation=$generation; ");
    if reset {
        for table in ["product_entry","product_chunk","product_stage","product_quota","product_header"] {sql.push_str(&format!("REMOVE TABLE IF EXISTS {table}; "));}
    }
    for statement in PRODUCT_SCHEMA {sql.push_str(statement);sql.push_str("; ");}
    sql.push_str("CREATE product_header:current CONTENT $header; CREATE product_quota:current SET used=0");sql
}
const RETIREMENT_BODY:&str="IF (SELECT VALUE generation FROM product_owner:current FOR UPDATE)[0] != $owner_generation { THROW 'superseded product administration'; }; UPDATE product_owner:current SET generation=$generation; IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $old_generation { THROW 'retired product generation'; }; DELETE product_entry; DELETE product_chunk; DELETE product_stage; UPDATE product_header:current SET generation=$generation; UPDATE product_quota:current SET used=0";
fn nonce() -> String { format!("{}_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos(), NONCE.fetch_add(1, Ordering::Relaxed)) }
fn open_lock(path: &Path) -> Result<File, ModelError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).open(path).map_err(ModelError::codec)
}
async fn lock(path: &Path, shared: bool) -> Result<File, ModelError> {
    let file = open_lock(path)?;
    loop {
        let result = if shared { file.try_lock_shared() } else { file.try_lock() };
        match result {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
            Err(std::fs::TryLockError::Error(error)) => return Err(ModelError::codec(error)),
        }
    }
}
async fn lifecycle(directory: &Path, shared: bool) -> Result<File, ModelError> {
    // Retirement takes exclusive admission before waiting on live borrowed values. New
    // readers cannot indefinitely overtake it by acquiring another shared lifecycle lock.
    let _admission = lock(&directory.join("admission.lock"), shared).await?;
    lock(&directory.join("lifecycle.lock"), shared).await
}
fn remove_generation_assets(directory: &Path) -> Result<(), ModelError> {
    // Only called with exclusive admission/lifecycle ownership. Preserve coordination inodes.
    for entry in std::fs::read_dir(directory).map_err(ModelError::codec)? {
        let entry = entry.map_err(ModelError::codec)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("ack_") || name.starts_with("stage_") || name.starts_with("entry_") {
            std::fs::remove_file(entry.path()).map_err(ModelError::codec)?;
        }
    }
    Ok(())
}
fn read_acknowledgment(path:&Path)->Result<Option<AcknowledgedEntry>,ModelError> {
    let size=match std::fs::metadata(path) {Ok(metadata)=>metadata.len(),Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(error)=>return Err(ModelError::codec(error))};
    if size>2048 {return Ok(None)}
    let bytes=std::fs::read(path).map_err(ModelError::codec)?;
    Ok(serde_json::from_slice(&bytes).ok())
}
fn reconcile_acknowledgments(directory:&Path,generation:&str)->Result<(),ModelError> {
    // The native transaction may commit despite a lost acknowledgment. Its local asset
    // cleanup then has not happened. Reconcile only obsolete/invalid receipts under
    // exclusive lifecycle after the new native administrative fence is confirmed.
    for entry in std::fs::read_dir(directory).map_err(ModelError::codec)? {
        let entry=entry.map_err(ModelError::codec)?;
        if entry.file_name().to_string_lossy().starts_with("ack_")
            && read_acknowledgment(&entry.path())?.is_none_or(|receipt|receipt.generation!=generation) {
            std::fs::remove_file(entry.path()).map_err(ModelError::codec)?;
        }
    }
    Ok(())
}
fn mark_administration(directory:&Path)->Result<(),ModelError> {
    let marker=open_lock(&directory.join("admin.pending"))?;marker.sync_all().map_err(ModelError::codec)?;
    File::open(directory).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)
}
fn clear_administration(directory:&Path)->Result<(),ModelError> {
    match std::fs::remove_file(directory.join("admin.pending")) {
        Ok(())=>{},Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},Err(error)=>return Err(ModelError::codec(error)),
    }
    File::open(directory).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)
}
fn administration_pending(directory:&Path)->Result<bool,ModelError> {directory.join("admin.pending").try_exists().map_err(ModelError::codec)}
fn canonical<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ModelError> { serde_json::to_vec(value).map_err(ModelError::codec) }
struct Counter(usize);
impl Write for Counter { fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> { self.0 = self.0.checked_add(bytes.len()).ok_or_else(|| std::io::Error::other("product size overflow"))?; Ok(bytes.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }
fn encoded_size(value: &PortableProduct) -> Result<usize, ModelError> { let mut counter = Counter(0); serde_json::to_writer(&mut counter, value).map_err(ModelError::codec)?; Ok(counter.0) }
impl NativeProductCache {
    /// Explicit installation only. Ordinary lookup never creates databases or schemas.
    pub async fn install(runtime: &RuntimeConfig) -> Result<Option<Self>, ModelError> {
        let Some(config) = &runtime.reuse else { return Ok(None) };
        config.validate(runtime)?;
        std::fs::create_dir_all(&config.lease_directory).map_err(ModelError::codec)?;
        let directory = std::fs::canonicalize(&config.lease_directory).map_err(ModelError::codec)?;
        let _lifecycle = lifecycle(&directory, false).await?;
        let client = reader::authenticated(&runtime.endpoint, &runtime.root_credentials(), None).await?;
        for statement in [format!("DEFINE NAMESPACE IF NOT EXISTS `{}`", runtime.namespace.as_str()), format!("USE NS `{}`; DEFINE DATABASE IF NOT EXISTS `{}` STRICT", runtime.namespace.as_str(), config.database.as_str())] {
            client.query(statement).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        }
        client.use_ns(runtime.namespace.as_str()).use_db(config.database.as_str()).await.map_err(ModelError::codec)?;
        // This immutable native receipt survives generation/schema reset. Otherwise a
        // foreign directory could claim the database while its header is absent after DDL
        // or an interrupted installation, bypassing the existing owner's filesystem locks.
        for statement in [
            "DEFINE TABLE IF NOT EXISTS product_owner TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS directory ON product_owner TYPE string",
            "DEFINE FIELD IF NOT EXISTS generation ON product_owner TYPE string",
        ] { client.query(statement).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?; }
        let owner: Option<CoordinationOwner> = client.select(("product_owner", "current")).await.map_err(ModelError::codec)?;
        if owner.as_ref().is_some_and(|owner| owner.directory != directory.to_string_lossy()) {
            return Err(ModelError::Conflict("product cache coordination directory"));
        }
        for statement in [
            "DEFINE TABLE IF NOT EXISTS product_header TYPE NORMAL SCHEMAFULL",
            "DEFINE FIELD IF NOT EXISTS version ON product_header TYPE int",
            "DEFINE FIELD IF NOT EXISTS directory ON product_header TYPE string",
            "DEFINE FIELD IF NOT EXISTS generation ON product_header TYPE string",
        ] { client.query(statement).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?; }
        // Read the encoding version before decoding its fields. Explicit installation discards
        // incompatible acceleration state; ordinary connection never migrates it.
        let mut response=client.query("SELECT VALUE version FROM product_header:current; SELECT VALUE directory FROM product_header:current;").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let versions:Vec<u32>=response.take(0).map_err(ModelError::codec)?;
        let directories:Vec<String>=response.take(1).map_err(ModelError::codec)?;
        if directories.first().is_some_and(|stored|stored!=directory.to_string_lossy().as_ref()) {
            return Err(ModelError::Conflict("product cache coordination directory"));
        }
        if owner.is_none() {
            let mut vars = Variables::new(); vars.insert("directory",directory.to_string_lossy().into_owned());vars.insert("generation",nonce());
            // CREATE (not UPSERT) makes simultaneous first installers conflict on the
            // singleton point. A losing installer cannot replace the winning owner.
            client.query("CREATE product_owner:current SET directory=$directory,generation=$generation").bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        }
        let reset=versions.first().is_some_and(|version|*version!=VERSION) || (owner.is_some() && versions.is_empty());
        let owner:CoordinationOwner=client.select::<Option<CoordinationOwner>>(("product_owner","current")).await.map_err(ModelError::codec)?.ok_or(ModelError::Schema("product cache coordination owner receipt"))?;
        if reset || versions.is_empty() {
            let generation=nonce();
            let header=Header{version:VERSION,directory:directory.to_string_lossy().into_owned(),generation:generation.clone()};
            let mut vars=Variables::new();vars.insert("owner_generation",owner.generation);vars.insert("generation",generation);vars.insert("header",header);
            // One checked native transaction fences the administrative effects themselves.
            // A delayed unknown reset cannot remove a later install's rebuilt tables: its
            // persistent owner-point read conflicts with that later generation mutation.
            let sql=format!("BEGIN; {}; COMMIT;",installation_body(reset));
            mark_administration(&directory)?;
            client.query(sql).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
            if reset {remove_generation_assets(&directory)?;}
        } else {
            // Even compatible explicit installation reconciles unknown administration.
            // Rotating the persistent point fences old reset AND retirement effects while
            // retaining compatible product generation and payloads.
            let mut vars=Variables::new();vars.insert("owner_generation",owner.generation);vars.insert("generation",nonce());
            mark_administration(&directory)?;
            client.query("BEGIN; IF (SELECT VALUE generation FROM product_owner:current FOR UPDATE)[0] != $owner_generation { THROW 'superseded product installation'; }; UPDATE product_owner:current SET generation=$generation; COMMIT;").bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        }
        let header: Option<Header> = client.select(("product_header", "current")).await.map_err(ModelError::codec)?;
        let header=header.ok_or(ModelError::Schema("product cache header receipt"))?;
        if header.directory != directory.to_string_lossy() { return Err(ModelError::Conflict("product cache coordination directory")); }
        if header.version != VERSION { return Err(ModelError::Conflict("product cache encoding changed during installation")); }
        let generation=header.generation;
        reconcile_acknowledgments(&directory,&generation)?;
        clear_administration(&directory)?;
        Ok(Some(Self { client, config: config.clone(), directory, generation }))
    }
    pub async fn connect(runtime: &RuntimeConfig) -> Result<Option<Self>, ModelError> {
        let Some(config) = &runtime.reuse else { return Ok(None) };
        config.validate(runtime)?;
        let directory = match std::fs::canonicalize(&config.lease_directory) { Ok(path) => path, Err(error) => { tracing::debug!(%error, "product cache unavailable"); return Ok(None) } };
        let _lifecycle = lifecycle(&directory, true).await?;
        if administration_pending(&directory)? { return Ok(None) }
        let client = match reader::connect(&runtime.endpoint, &runtime.root_credentials(), runtime.namespace.as_str(), config.database.as_str()).await { Ok(client) => client, Err(error) => { tracing::debug!(%error, "product cache unavailable"); return Ok(None) } };
        let owner: Option<CoordinationOwner> = match client.select(("product_owner", "current")).await {
            Ok(value) => value,
            Err(error) => { tracing::debug!(%error, "product cache owner unavailable"); return Ok(None) }
        };
        let Some(owner) = owner else { return Ok(None) };
        if owner.directory != directory.to_string_lossy() { return Err(ModelError::Conflict("product cache coordination directory")); }
        let header: Option<Header> = match client.select(("product_header", "current")).await { Ok(value) => value, Err(error) => { tracing::debug!(%error, "product cache unavailable"); return Ok(None) } };
        let Some(header) = header else { return Ok(None) };
        if header.directory != directory.to_string_lossy() { return Err(ModelError::Conflict("product cache coordination directory")); }
        if header.version != VERSION { return Ok(None) }
        let quota: Option<Quota> = match client.select(("product_quota", "current")).await {
            Ok(value) => value,
            Err(error) => { tracing::debug!(%error, "product cache quota unavailable"); return Ok(None) }
        };
        if quota.is_none_or(|quota| quota.used > config.capacity_bytes) { return Ok(None) }
        Ok(Some(Self { client, config: config.clone(), directory, generation: header.generation }))
    }
    fn acknowledgment_path(&self,entry:&Entry)->PathBuf {self.directory.join(format!("ack_{}.json",entry.staging))}
    fn acknowledged(&self,entry:&Entry)->Result<bool,ModelError> {
        let Some(receipt)=read_acknowledgment(&self.acknowledgment_path(entry))? else{return Ok(false)};
        Ok(receipt==AcknowledgedEntry{generation:entry.generation.clone(),key:entry.key.clone(),staging:entry.staging.clone(),digest:entry.digest.clone()})
    }
    fn acknowledge(&self,entry:&Entry)->Result<(),ModelError> {
        let receipt=AcknowledgedEntry{generation:entry.generation.clone(),key:entry.key.clone(),staging:entry.staging.clone(),digest:entry.digest.clone()};
        let mut staged=tempfile::NamedTempFile::new_in(&self.directory).map_err(ModelError::codec)?;
        serde_json::to_writer(staged.as_file_mut(),&receipt).map_err(ModelError::codec)?;
        staged.as_file().sync_all().map_err(ModelError::codec)?;
        staged.persist(self.acknowledgment_path(entry)).map_err(|e|ModelError::codec(e.error))?;
        File::open(&self.directory).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;Ok(())
    }
    async fn current(&self) -> Result<bool, ModelError> {
        if administration_pending(&self.directory)? {return Ok(false)}
        let header: Option<Header> = self.client.select(("product_header", "current")).await.map_err(ModelError::codec)?;
        Ok(header.is_some_and(|h| h.version == VERSION && h.generation == self.generation && h.directory == self.directory.to_string_lossy()))
    }
    fn entry_path(&self,key:&str)->PathBuf {
        // Collisions restrict optional acceleration only; native identity remains the full key.
        coordination_path(&self.directory,"entry",key)
    }
    fn stage_path(&self,staging:&str)->PathBuf {
        coordination_path(&self.directory,"stage",staging)
    }
    async fn entry(&self, key: &str) -> Result<Option<Entry>, ModelError> { self.client.select(("product_entry", key)).await.map_err(ModelError::codec) }
    async fn decode(&self, entry: &Entry, request: &ProductRequest, budget: &ResourceBudget) -> Result<(PortableProduct, Box<dyn Reservation>), ModelError> {
        if entry.id != RecordId::new("product_entry", request.identity()?.hex()) || entry.key != request.identity()?.hex() || entry.bytes == 0 || entry.request.as_ref() != canonical(request)? || entry.generation != self.generation || entry.bytes > self.config.capacity_bytes || entry.chunks as u64 != entry.bytes.div_ceil(CHUNK as u64) { return Err(ModelError::Conflict("product cache manifest")); }
        let size = usize::try_from(entry.bytes).map_err(ModelError::codec)?;
        let charge = budget.reserve("native-product", size.checked_mul(4).ok_or(ModelError::Schema("product allocation overflow"))?)?;
        let mut bytes = Vec::with_capacity(size);
        for index in 0..entry.chunks {
            let chunk: Option<Chunk> = self.client.select(("product_chunk", format!("{}_{}", entry.staging, index))).await.map_err(ModelError::codec)?;
            let chunk = chunk.ok_or(ModelError::Schema("product payload chunk"))?;
            let expected = (size - bytes.len()).min(CHUNK);
            if chunk.id != RecordId::new("product_chunk", format!("{}_{}", entry.staging, index)) || chunk.staging != entry.staging || chunk.bytes.len() != expected { return Err(ModelError::Schema("product payload length")); }
            bytes.extend_from_slice(chunk.bytes.as_ref());
        }
        let product: PortableProduct = serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
        product.validate()?;
        if &product.request != request || product.identity()?.hex() != entry.digest || canonical(&product)? != bytes { return Err(ModelError::Conflict("product cache canonical payload")); }
        Ok((product, charge))
    }
    pub async fn lookup(&self, request: &ProductRequest, budget: &ResourceBudget) -> Result<Option<LeasedProduct>, ModelError> {
        request.validate()?;
        let key = request.identity()?.hex();
        let lifecycle = lifecycle(&self.directory, true).await?;
        let entry_lease = lock(&self.entry_path(&key), true).await?;
        match self.current().await { Ok(true) => {}, Ok(false) => return Ok(None), Err(error) => { tracing::debug!(%error, "product cache unavailable"); return Ok(None) } }
        let entry = match self.entry(&key).await { Ok(Some(entry)) => entry, Ok(None) => return Ok(None), Err(error) => { tracing::debug!(%error, "product cache unavailable"); return Ok(None) } };
        if !self.acknowledged(&entry)? {return Ok(None)}
        match self.decode(&entry, request, budget).await {
            Ok((product, charge)) => Ok(Some(LeasedProduct { product, _entry: entry_lease, _lifecycle: lifecycle, _charge: charge })),
            Err(ModelError::Resource { .. }) => Ok(None),
            Err(error) => { tracing::warn!(%error, %key, "discarding unusable derived product"); drop(entry_lease); drop(lifecycle); self.invalidate(request).await?; Ok(None) }
        }
    }
    async fn delete(&self, entry: &Entry) -> Result<(), ModelError> { self.delete_parts(entry.id.clone(), &entry.staging, entry.chunks,entry.bytes).await }
    async fn delete_parts(&self, id: RecordId, staging: &str, _chunks: u32,size:u64) -> Result<(), ModelError> {
        let mut vars=Variables::new();vars.insert("id",id);vars.insert("staging",staging.to_owned());vars.insert("generation",self.generation.clone());vars.insert("size",size);
        self.client.query(fenced_sql("IF array::len(SELECT id FROM $id FOR UPDATE) = 1 { DELETE $id; DELETE product_chunk WHERE staging=$staging; UPDATE product_quota:current SET used=used-$size; }" )).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        match std::fs::remove_file(self.directory.join(format!("ack_{staging}.json"))) {
            Ok(())=>{},Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},Err(error)=>return Err(ModelError::codec(error)),
        }
        Ok(())
    }
    /// Data-level caller validation can reject a syntactically valid cached section. Its
    /// invalidation uses the same leases; no live consumer's bytes are silently replaced.
    pub async fn invalidate(&self, request: &ProductRequest) -> Result<(), ModelError> {
        request.validate()?;
        let this = self.clone(); let key = request.identity()?.hex();
        tokio::spawn(async move { this.evict_corrupt(&key).await }).await.map_err(ModelError::codec)?
    }
    async fn evict_corrupt(&self, key: &str) -> Result<(), ModelError> {
        let _life = lifecycle(&self.directory, true).await?;
        self.evict_corrupt_live(key).await
    }
    async fn evict_corrupt_live(&self, key: &str) -> Result<(), ModelError> {
        let _capacity = lock(&self.directory.join("capacity.lock"), false).await?;
        let file = open_lock(&self.entry_path(key))?;
        if file.try_lock().is_ok() && self.current().await? && let Some(entry) = self.entry(key).await? { self.delete(&entry).await?; }
        Ok(())
    }
    /// Insertion owns its task through native terminality, even if the initiating caller cancels.
    pub async fn insert(&self, product: &PortableProduct, budget: &ResourceBudget) -> Result<bool, ModelError> {
        product.validate()?;
        let size = encoded_size(product)?;
        if size as u64 > self.config.capacity_bytes { return Ok(false) }
        let charge = match budget.reserve("native-product-insertion", size.checked_mul(4).ok_or(ModelError::Schema("product allocation overflow"))?) { Ok(charge) => charge, Err(ModelError::Resource { .. }) => return Ok(false), Err(error) => return Err(error) };
        let owned = product.clone(); let this = self.clone(); let budget = budget.clone();
        tokio::spawn(async move {
            let _charge = charge;
            match this.insert_owned(owned, &budget).await {
                Err(error @ (ModelError::Codec(_) | ModelError::Cause(_) | ModelError::Resource { .. })) => { tracing::warn!(%error, "known product-cache refusal; compute fresh"); Ok(false) },
                result => result,
            }
        }).await.map_err(ModelError::codec)?
    }
    async fn insert_owned(&self, product: PortableProduct, budget: &ResourceBudget) -> Result<bool, ModelError> {
        let _life = lifecycle(&self.directory, true).await?;
        if !self.current().await? { return Ok(false) }
        let key = product.request.identity()?.hex();
        // Expensive canonical decoding happens under a shared entry lease, outside capacity
        // coordination. Complete entries are immutable until every reader releases its lease.
        let pin = lock(&self.entry_path(&key), true).await?;
        if let Some(entry) = self.entry(&key).await? {
            let existing = self.decode(&entry, &product.request, budget).await;
            match existing {
                Ok((old, _)) if old == product => {self.acknowledge(&entry)?;return Ok(true)},
                Ok(_) => return Err(ModelError::Conflict("same product request produced different canonical values")),
                Err(ModelError::Resource { .. }) => return Ok(false),
                Err(error) => { tracing::warn!(%error, "replacing corrupt derived product"); drop(pin); return self.replace_corrupt(product, budget, &key).await; }
            }
        }
        drop(pin);
        self.stage_product(&product,budget,&key).await
    }
    async fn stage_product(&self,product:&PortableProduct,budget:&ResourceBudget,key:&str)->Result<bool,ModelError> {
        let bytes = canonical(&product)?;
        let staging = nonce();
        let _stage_pin = lock(&self.stage_path(&staging), true).await?;
        if !self.reserve_stage(&staging, bytes.len() as u64, budget).await? { return Ok(false) }
        let result = self.write_stage(product, key, &staging, &bytes, budget).await;
        // Unknown transport disposition stays quarantined and fails its owner. Cleanup cannot
        // prove rollback while a request might still be executing remotely.
        if !matches!(&result, Err(ModelError::Completion(_))) {
            let cleanup = self.remove_stage(&staging).await;
            if let Err(error) = cleanup { return Err(error) }
        }
        result
    }
    async fn replace_corrupt(&self,product:PortableProduct,budget:&ResourceBudget,key:&str)->Result<bool,ModelError> {
        self.evict_corrupt_live(key).await?;
        // A borrowed stripe may conservatively prevent removal. Do not recursively retry
        // while that unchanged entry remains visible.
        if self.entry(key).await?.is_some() {return Ok(false);}
        self.stage_product(&product,budget,key).await
    }
    async fn reserve_stage(&self, staging: &str, size: u64, budget: &ResourceBudget) -> Result<bool, ModelError> {
        let _capacity = lock(&self.directory.join("capacity.lock"), false).await?;
        if !self.current().await? { return Ok(false) }
        self.reap_stages().await?;
        let mut response = self.client.query("SELECT VALUE math::sum(bytes) FROM product_entry GROUP ALL; SELECT VALUE math::sum(bytes) FROM product_stage GROUP ALL;").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let complete: Vec<u64> = response.take(0).map_err(ModelError::codec)?;
        let staged: Vec<u64> = response.take(1).map_err(ModelError::codec)?;
        let mut used = complete.first().copied().unwrap_or(0).checked_add(staged.first().copied().unwrap_or(0)).ok_or(ModelError::Schema("cache capacity overflow"))?;
        let _inventory_charge = budget.reserve("native-product-inventory", 64 * 512)?;
        let mut offset = 0u64;
        while used.saturating_add(size) > self.config.capacity_bytes {
            let mut vars = Variables::new(); vars.insert("offset", offset);
            let mut response = self.client.query("SELECT id,key,bytes,chunks,staging FROM product_entry ORDER BY created,key LIMIT 64 START $offset").bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let entries: Vec<Inventory> = response.take(0).map_err(ModelError::codec)?;
            if entries.is_empty() { break }
            let mut retained = 0u64;
            for entry in entries {
                if used.saturating_add(size) <= self.config.capacity_bytes { break }
                if entry.id != RecordId::new("product_entry",entry.key.clone()) {
                    tracing::warn!("invalid cache inventory identity; compute fresh");return Ok(false);
                }
                let pin = open_lock(&self.entry_path(&entry.key))?;
                match pin.try_lock() {
                    Ok(()) => { self.delete_parts(entry.id, &entry.staging, entry.chunks,entry.bytes).await?; used = used.saturating_sub(entry.bytes); },
                    Err(std::fs::TryLockError::WouldBlock) => retained += 1,
                    Err(std::fs::TryLockError::Error(error)) => return Err(ModelError::codec(error)),
                }
            }
            offset += retained;
        }
        if used.saturating_add(size) > self.config.capacity_bytes { return Ok(false) }
        let stage = Stage { id: RecordId::new("product_stage", staging), staging: staging.into(), bytes: size };
        let mut vars = Variables::new(); vars.insert("stage", stage); vars.insert("generation", self.generation.clone());vars.insert("capacity",self.config.capacity_bytes);vars.insert("size",size);
        self.client.query(fenced_sql("IF (SELECT VALUE used FROM product_quota:current FOR UPDATE)[0]+$size > $capacity { THROW 'product capacity exhausted'; }; UPDATE product_quota:current SET used=used+$size; CREATE $stage.id CONTENT $stage")).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        Ok(true)
    }
    async fn write_stage(&self, product: &PortableProduct, key: &str, staging: &str, bytes: &[u8], budget: &ResourceBudget) -> Result<bool, ModelError> {
        for (index, chunk) in bytes.chunks(CHUNK).enumerate() {
            let row = Chunk { id: RecordId::new("product_chunk", format!("{staging}_{index}")), staging: staging.into(), bytes: Bytes::from(chunk.to_vec()) };
            let mut vars = Variables::new(); vars.insert("row", row);vars.insert("generation",self.generation.clone());vars.insert("stage",RecordId::new("product_stage",staging));
            self.client.query(fenced_sql("IF array::len(SELECT id FROM $stage FOR UPDATE) != 1 { THROW 'fenced product stage'; }; CREATE $row.id CONTENT $row")).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        }
        let capacity = lock(&self.directory.join("capacity.lock"), false).await?;
        if !self.current().await? { return Ok(false) }
        if let Some(entry) = self.entry(key).await? {
            let pin = lock(&self.entry_path(key), true).await?;
            drop(capacity);
            let (actual, _) = self.decode(&entry, &product.request, budget).await?;
            drop(pin);
            if actual != *product { return Err(ModelError::Conflict("same product request produced different canonical values")); }
            self.acknowledge(&entry)?;return Ok(true)
        }
        let pin = open_lock(&self.entry_path(key))?;
        match pin.try_lock() {
            Ok(())=>{},
            Err(std::fs::TryLockError::WouldBlock)=>return Ok(false),
            Err(std::fs::TryLockError::Error(error))=>return Err(ModelError::codec(error)),
        }
        let entry = Entry { id: RecordId::new("product_entry", key), key: key.into(), created: u64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(ModelError::codec)?.as_millis()).map_err(ModelError::codec)?, request: Bytes::from(canonical(&product.request)?), digest: product.identity()?.hex(), bytes: bytes.len() as u64, chunks: u32::try_from(bytes.len().div_ceil(CHUNK)).map_err(ModelError::codec)?, staging: staging.into(), generation: self.generation.clone() };
        let mut vars = Variables::new(); vars.insert("entry", entry); vars.insert("generation", self.generation.clone()); vars.insert("stage", RecordId::new("product_stage", staging));
        let written = self.client.query(fenced_sql("IF array::len(SELECT id FROM $stage FOR UPDATE) != 1 { THROW 'fenced product stage'; }; CREATE $entry.id CONTENT $entry; DELETE $stage")).bind(vars).await;
        let write_error = match written { Ok(response) => response.check().err().map(ModelError::codec), Err(error) => Some(crate::loader::write_failure(error)) };
        drop(capacity);
        let result = self.reconcile_publication(key, product, write_error, budget).await;
        drop(pin);
        result
    }
    async fn reconcile_publication(&self, key: &str, product: &PortableProduct, write_error: Option<ModelError>, budget: &ResourceBudget) -> Result<bool, ModelError> {
        match self.entry(key).await {
            Ok(Some(entry)) => {
                if let Some(error) = write_error.as_ref() && !matches!(error, ModelError::Completion(_)) { return Err(write_error.expect("observed checked failure")); }
                let (actual, _) = self.decode(&entry, &product.request, budget).await?;
                if actual != *product { return Err(ModelError::Conflict("product publication exact readback")); }
                self.acknowledge(&entry)?;Ok(true)
            },
            Ok(None) => Err(write_error.unwrap_or(ModelError::Schema("product publication missing receipt"))),
            Err(error) => Err(write_error.unwrap_or(error)),
        }
    }
    async fn remove_stage(&self, staging: &str) -> Result<(), ModelError> {
        let _capacity = lock(&self.directory.join("capacity.lock"), false).await?;
        self.remove_stage_locked(staging).await
    }
    async fn remove_stage_locked(&self, staging: &str) -> Result<(), ModelError> {
        let mut vars = Variables::new(); vars.insert("staging", staging.to_owned()); vars.insert("id", RecordId::new("product_stage", staging));vars.insert("generation",self.generation.clone());
        // A complete entry retains these chunks. Only unreachable staging is discarded.
        self.client.query(fenced_sql("LET $reserved=(SELECT VALUE bytes FROM $id FOR UPDATE)[0]; IF $reserved IS NOT NONE { IF array::len(SELECT id FROM product_entry WHERE staging=$staging) = 0 { DELETE product_chunk WHERE staging=$staging; }; DELETE $id; UPDATE product_quota:current SET used=used-$reserved; }")).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        Ok(())
    }
    async fn reap_stages(&self) -> Result<(), ModelError> {
        // An unlocked local lease need not mean a remote request has stopped. Cleanup changes
        // the generation/quota read fence and deletes the point stage, so any older in-flight
        // transaction must conflict rather than resurrect chunks or unaccounted reservations.
        let mut offset = 0u64;
        loop {
            let mut vars = Variables::new(); vars.insert("offset", offset);
            let mut response = self.client.query("SELECT VALUE staging FROM product_stage ORDER BY id LIMIT 64 START $offset").bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let stages: Vec<String> = response.take(0).map_err(ModelError::codec)?;
            if stages.is_empty() { break }
            let mut retained = 0u64;
            for staging in stages {
                let file = open_lock(&self.stage_path(&staging))?;
                match file.try_lock() {
                    Ok(()) => self.remove_stage_locked(&staging).await?,
                    Err(std::fs::TryLockError::WouldBlock) => retained += 1,
                    Err(std::fs::TryLockError::Error(error)) => return Err(ModelError::codec(error)),
                }
            }
            offset += retained;
        }
        Ok(())
    }
    /// Fence admissions and wait for all product leases and owned insertions before deleting.
    /// Existing cache instances retain the old generation and can no longer publish.
    pub async fn retire(&self) -> Result<(), ModelError> {
        let this = self.clone();
        tokio::spawn(async move {
            let _admission = lock(&this.directory.join("admission.lock"), false).await?;
            let _life = lock(&this.directory.join("lifecycle.lock"), false).await?;
            if administration_pending(&this.directory)? {return Err(ModelError::Conflict("product administration requires explicit installation reconciliation"))}
            if !this.current().await? { return Ok(()) }
            let owner:CoordinationOwner=this.client.select::<Option<CoordinationOwner>>(("product_owner","current")).await.map_err(ModelError::codec)?.ok_or(ModelError::Schema("product cache coordination owner receipt"))?;
            let mut vars = Variables::new(); vars.insert("generation", nonce());vars.insert("old_generation",this.generation.clone());vars.insert("owner_generation",owner.generation);
            mark_administration(&this.directory)?;
            this.client.query(format!("BEGIN; {RETIREMENT_BODY}; COMMIT;")).bind(vars).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
            remove_generation_assets(&this.directory)?;
            clear_administration(&this.directory)?;
            Ok(())
        }).await.map_err(ModelError::codec)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{ContentHash, compilation_product::{ProductKind, ProductOutcome, ProductSection}, serving::Name};
    use std::collections::BTreeSet;
    fn product(label: &str) -> PortableProduct {
        PortableProduct { request: ProductRequest { kind: ProductKind::PureRows, operation: label.into(), model: ContentHash::of(b"model"), implementation: ContentHash::of(b"implementation"), policy: ContentHash::of(b"policy"), result_contract: ContentHash::of(b"result"), configuration: None, profile: "catalog".into(), parameters: vec![], dependencies: vec![], outputs: BTreeSet::from(["rows".into()]) }, outcome: ProductOutcome::Complete, sections: vec![ProductSection { name: "rows".into(), rows: 1, bytes: b"canonical rows".to_vec() }] }
    }
    async fn fixture(capacity: u64) -> (NativeProductCache, RuntimeConfig, tempfile::TempDir) {
        let path = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture required");
        let mut runtime = RuntimeConfig::read(Path::new(&path)).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        runtime.reuse = Some(ReuseConfig { database: Name::new(format!("products_{}", nonce())).unwrap(), capacity_bytes: capacity, lease_directory: scratch.path().to_path_buf() });
        let cache = NativeProductCache::install(&runtime).await.unwrap().unwrap();
        (cache, runtime, scratch)
    }
    #[tokio::test]
    async fn native_product_stripe_collision_is_optional_refusal_not_identity_or_self_wait() {
        let (cache,_runtime,_scratch)=fixture(16<<20).await;
        let budget=ResourceBudget::fixed(32<<20).unwrap();
        let mut stripes=std::collections::BTreeMap::new();
        let (first,second)=(0..=4096).find_map(|index| {
            let value=product(&format!("stripe-{index}"));
            let key=value.request.identity().unwrap().hex();
            stripes.insert(cache.entry_path(&key),value.clone()).map(|old|(old,value))
        }).expect("4097 full keys must share one of 4096 lock stripes");
        assert_ne!(first.request.identity().unwrap(),second.request.identity().unwrap());
        assert!(cache.insert(&first,&budget).await.unwrap());
        let borrowed=cache.lookup(&first.request,&budget).await.unwrap().unwrap();
        assert!(!tokio::time::timeout(std::time::Duration::from_secs(10),cache.insert(&second,&budget)).await
            .expect("an optional colliding publication must not await its caller's live borrow").unwrap());
        assert!(cache.lookup(&second.request,&budget).await.unwrap().is_none());
        assert_eq!(borrowed.product(),&first);
        drop(borrowed);
        assert!(cache.insert(&second,&budget).await.unwrap());
        assert_eq!(cache.lookup(&second.request,&budget).await.unwrap().unwrap().product(),&second);
        assert_eq!(cache.lookup(&first.request,&budget).await.unwrap().unwrap().product(),&first);
        cache.retire().await.unwrap();
    }
    #[tokio::test]
    async fn native_product_reopen_exact_winner_and_borrowed_charge() {
        let (cache, runtime, _scratch) = fixture(16 * 1024 * 1024).await;
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        let product = product("roundtrip");
        assert!(cache.lookup(&product.request, &budget).await.unwrap().is_none());
        assert!(cache.insert(&product, &budget).await.unwrap());
        assert_eq!(budget.reserved(), 0);
        let reopened = NativeProductCache::connect(&runtime).await.unwrap().unwrap();
        let borrowed = reopened.lookup(&product.request, &budget).await.unwrap().unwrap();
        assert_eq!(borrowed.product(), &product);
        assert!(budget.reserved() > 0);
        assert!(cache.insert(&product, &budget).await.unwrap());
        let mut different = product.clone(); different.sections[0].bytes.push(7);
        assert!(matches!(cache.insert(&different, &budget).await, Err(ModelError::Conflict(_))));
        drop(borrowed); assert_eq!(budget.reserved(), 0);
        cache.retire().await.unwrap();
        assert!(!cache.insert(&product, &budget).await.unwrap());
        assert!(reopened.lookup(&product.request, &budget).await.unwrap().is_none());
        let next = NativeProductCache::connect(&runtime).await.unwrap().unwrap();
        assert!(next.insert(&product, &budget).await.unwrap());
        next.retire().await.unwrap();
    }
    #[tokio::test]
    async fn native_product_corruption_falls_back_and_retirement_drains_live_lease() {
        let (cache, runtime, _scratch) = fixture(16 * 1024 * 1024).await;
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        let product = product("corruption");
        cache.insert(&product, &budget).await.unwrap();
        let mut vars = Variables::new(); vars.insert("id", RecordId::new("product_entry", product.request.identity().unwrap().hex()));
        cache.client.query("UPDATE $id SET digest='corrupted'").bind(vars).await.unwrap().check().unwrap();
        assert!(cache.lookup(&product.request, &budget).await.unwrap().is_none());
        assert!(cache.insert(&product, &budget).await.unwrap());
        let borrowed = cache.lookup(&product.request, &budget).await.unwrap().unwrap();
        let other = cache.clone(); let retire = tokio::spawn(async move { other.retire().await });
        assert!(tokio::time::timeout(std::time::Duration::from_millis(40), async { while !retire.is_finished() { tokio::task::yield_now().await } }).await.is_err());
        drop(borrowed);
        retire.await.unwrap().unwrap();
        assert_eq!(budget.reserved(), 0);
        assert!(NativeProductCache::connect(&runtime).await.unwrap().unwrap().lookup(&product.request, &budget).await.unwrap().is_none());
    }
    #[test]
    fn native_subprocess_entry_lease() {
        let Some(directory) = std::env::var_os("LCTX_PRODUCT_TEST_DIRECTORY") else { return };
        let directory = PathBuf::from(directory);
        let key = std::env::var("LCTX_PRODUCT_TEST_KEY").unwrap();
        let lifecycle = open_lock(&directory.join("lifecycle.lock")).unwrap(); lifecycle.lock_shared().unwrap();
        let entry = open_lock(&coordination_path(&directory,"entry",&key)).unwrap(); entry.lock_shared().unwrap();
        std::fs::write(directory.join("child.ready"), b"ready").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !directory.join("child.release").exists() {
            assert!(std::time::Instant::now() < deadline, "parent must release process lease");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
    #[tokio::test]
    async fn native_capacity_preserves_leased_entries_and_recovers_after_release() {
        let first = product("a"); let second = product("b");
        let size = encoded_size(&first).unwrap();
        let (cache, _runtime, scratch) = fixture(size as u64).await;
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        assert!(cache.insert(&first, &budget).await.unwrap());
        let borrowed = cache.lookup(&first.request, &budget).await.unwrap().unwrap();
        assert!(!cache.insert(&second, &budget).await.unwrap());
        assert_eq!(borrowed.product(), &first);
        drop(borrowed);
        let key = first.request.identity().unwrap().hex();
        let mut child = ChildGuard(std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "product_cache::tests::native_subprocess_entry_lease", "--nocapture"]).env("LCTX_PRODUCT_TEST_DIRECTORY", scratch.path()).env("LCTX_PRODUCT_TEST_KEY", key).spawn().unwrap());
        tokio::time::timeout(std::time::Duration::from_secs(5), async { while !scratch.path().join("child.ready").exists() { tokio::time::sleep(std::time::Duration::from_millis(10)).await; } }).await.unwrap();
        assert!(!cache.insert(&second, &budget).await.unwrap());
        std::fs::write(scratch.path().join("child.release"), b"release").unwrap();
        assert!(child.0.wait().unwrap().success());
        assert!(cache.insert(&second, &budget).await.unwrap());
        assert!(cache.lookup(&first.request, &budget).await.unwrap().is_none());
        assert!(cache.lookup(&second.request, &budget).await.unwrap().is_some());
        cache.retire().await.unwrap();
    }    #[tokio::test]
    async fn native_bounded_chunks_and_unreachable_stage_recovery() {
        let (cache, _runtime, _scratch) = fixture(32 * 1024 * 1024).await;
        let budget = ResourceBudget::fixed(128 * 1024 * 1024).unwrap();
        let mut product = product("chunked"); product.sections[0].bytes = vec![65; 3 * 1024 * 1024];
        assert!(encoded_size(&product).unwrap() > CHUNK);
        let stage = Stage { id: RecordId::new("product_stage", "orphan"), staging: "orphan".into(), bytes: 3 };
        let mut vars = Variables::new(); vars.insert("stage", stage);
        cache.client.query("BEGIN; CREATE $stage.id CONTENT $stage; UPDATE product_quota:current SET used=used+3; COMMIT;").bind(vars).await.unwrap().check().unwrap();
        let row = Chunk { id: RecordId::new("product_chunk", "orphan_0"), staging: "orphan".into(), bytes: Bytes::from(vec![1,2,3]) };
        let mut vars = Variables::new(); vars.insert("row", row);
        cache.client.query("CREATE $row.id CONTENT $row").bind(vars).await.unwrap().check().unwrap();
        assert!(cache.lookup(&product.request, &budget).await.unwrap().is_none());
        assert!(cache.insert(&product, &budget).await.unwrap());
        let orphan: Option<Chunk> = cache.client.select(("product_chunk", "orphan_0")).await.unwrap(); assert!(orphan.is_none());
        let borrowed = cache.lookup(&product.request, &budget).await.unwrap().unwrap(); assert_eq!(borrowed.product(), &product);
        drop(borrowed); cache.retire().await.unwrap(); assert_eq!(budget.reserved(), 0);
    }
    fn lost_ack() -> ModelError {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
        lctx_model::domain::completion::complete::<()>(Err(ModelError::Codec("injected acknowledgement loss after native send".into())), completion).unwrap_err()
    }
    #[tokio::test]
    async fn native_lost_ack_reconciles_exact_receipt_and_absence_stays_unknown() {
        let (cache, _runtime, _scratch) = fixture(16 * 1024 * 1024).await;
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        let product = product("ack"); let key = product.request.identity().unwrap().hex();
        assert!(cache.insert(&product, &budget).await.unwrap());
        // Controlled boundary injection after an actual native publication, not a fake store.
        assert!(cache.reconcile_publication(&key, &product, Some(lost_ack()), &budget).await.unwrap());
        cache.invalidate(&product.request).await.unwrap();
        let error = cache.reconcile_publication(&key, &product, Some(lost_ack()), &budget).await.unwrap_err();
        assert!(matches!(error, ModelError::Completion(ref failure) if failure.completion.remote == lctx_model::domain::completion::RemoteState::Unknown));
        cache.retire().await.unwrap();
    }
    #[tokio::test]
    async fn native_concurrent_writers_keep_one_equal_product_and_reject_unequal() {
        let (cache, _runtime, _scratch) = fixture(16 * 1024 * 1024).await;
        let budget = ResourceBudget::fixed(16 * 1024 * 1024).unwrap();
        let product = product("concurrent");
        let (a,b) = tokio::join!(cache.insert(&product, &budget), cache.insert(&product, &budget));
        assert!(a.unwrap()); assert!(b.unwrap());
        let mut unequal = product.clone(); unequal.sections[0].bytes.push(1);
        assert!(matches!(cache.insert(&unequal, &budget).await, Err(ModelError::Conflict(_))));
        assert_eq!(cache.lookup(&product.request, &budget).await.unwrap().unwrap().product(), &product);
        cache.retire().await.unwrap();
    }

    #[tokio::test]
    async fn native_delayed_chunk_cannot_resurrect_reaped_stage() {
        let (cache,_runtime,_scratch)=fixture(16<<20).await;
        let budget=ResourceBudget::fixed(16<<20).unwrap();
        assert!(cache.reserve_stage("delayed",3,&budget).await.unwrap());
        // Keep a real remote transaction pending after its guarded write has executed.
        // Losing the local acknowledgment would release its filesystem lease at this point.
        let pending=cache.client.as_ref().clone().begin().await.unwrap();
        let row=Chunk{id:RecordId::new("product_chunk","delayed_0"),staging:"delayed".into(),bytes:Bytes::from(vec![1,2,3])};
        let mut vars=Variables::new();vars.insert("row",row);vars.insert("stage",RecordId::new("product_stage","delayed"));vars.insert("generation",cache.generation.clone());
        pending.query("IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $generation { THROW 'retired'; }; IF array::len(SELECT id FROM $stage FOR UPDATE)!=1 { THROW 'fenced'; }; CREATE $row.id CONTENT $row").bind(vars).await.unwrap().check().unwrap();
        cache.reap_stages().await.unwrap();
        assert!(pending.commit().await.is_err(),"reaped stage must invalidate a delayed remote commit");
        let chunk:Option<Chunk>=cache.client.select(("product_chunk","delayed_0")).await.unwrap();assert!(chunk.is_none());
        let quota:Option<Quota>=cache.client.select(("product_quota","current")).await.unwrap();assert_eq!(quota.unwrap().used,0);
        cache.retire().await.unwrap();
    }

    #[tokio::test]
    async fn native_unacknowledged_late_publication_is_not_a_cache_hit() {
        let (cache,_runtime,_scratch)=fixture(16<<20).await;
        let budget=ResourceBudget::fixed(16<<20).unwrap();let product=product("late-publication");
        let key=product.request.identity().unwrap().hex();
        let error=cache.reconcile_publication(&key,&product,Some(lost_ack()),&budget).await.unwrap_err();
        assert!(matches!(error,ModelError::Completion(_)));
        assert!(cache.insert(&product,&budget).await.unwrap());
        let entry=cache.entry(&key).await.unwrap().unwrap();
        // Native complete bytes arriving after the owning operation's Unknown outcome do not
        // carry the atomic local exact-readback acknowledgment required for visibility.
        std::fs::remove_file(cache.acknowledgment_path(&entry)).unwrap();
        assert!(cache.lookup(&product.request,&budget).await.unwrap().is_none());
        assert!(cache.reconcile_publication(&key,&product,Some(lost_ack()),&budget).await.unwrap());
        assert!(cache.lookup(&product.request,&budget).await.unwrap().is_some());
        cache.retire().await.unwrap();
    }

    #[tokio::test]
    async fn native_retirement_fences_delayed_reservation_and_publication() {
        let (cache,_runtime,_scratch)=fixture(16<<20).await;
        let budget=ResourceBudget::fixed(16<<20).unwrap();let product=product("old-generation");
        let bytes=canonical(&product).unwrap();let key=product.request.identity().unwrap().hex();
        assert!(cache.reserve_stage("publish",bytes.len() as u64,&budget).await.unwrap());
        let publish=cache.client.as_ref().clone().begin().await.unwrap();
        let entry=Entry{id:RecordId::new("product_entry",key.clone()),key:key.clone(),created:0,request:Bytes::from(canonical(&product.request).unwrap()),digest:product.identity().unwrap().hex(),bytes:bytes.len() as u64,chunks:bytes.len().div_ceil(CHUNK) as u32,staging:"publish".into(),generation:cache.generation.clone()};
        let mut vars=Variables::new();vars.insert("entry",entry);vars.insert("stage",RecordId::new("product_stage","publish"));vars.insert("generation",cache.generation.clone());
        publish.query("IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $generation { THROW 'retired'; }; IF array::len(SELECT id FROM $stage FOR UPDATE)!=1 { THROW 'fenced'; }; CREATE $entry.id CONTENT $entry; DELETE $stage").bind(vars).await.unwrap().check().unwrap();
        let reserve=cache.client.as_ref().clone().begin().await.unwrap();
        let mut vars=Variables::new();vars.insert("generation",cache.generation.clone());
        reserve.query("IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $generation { THROW 'retired'; }; UPDATE product_quota:current SET used=used+3; CREATE product_stage:late SET staging='late',bytes=3").bind(vars).await.unwrap().check().unwrap();
        cache.retire().await.unwrap();
        assert!(publish.commit().await.is_err());assert!(reserve.commit().await.is_err());
        assert!(cache.entry(&key).await.unwrap().is_none());
        let quota:Option<Quota>=cache.client.select(("product_quota","current")).await.unwrap();assert_eq!(quota.unwrap().used,0);
        let stages:Vec<Stage>=cache.client.select("product_stage").await.unwrap();assert!(stages.is_empty());
    }
    #[tokio::test]
    async fn native_delayed_administration_cannot_destroy_a_rebuilt_generation() {
        let (cache,runtime,_scratch)=fixture(16<<20).await;
        cache.client.query("UPDATE product_header:current SET version=1").await.unwrap().check().unwrap();
        let owner:CoordinationOwner=cache.client.select::<Option<CoordinationOwner>>(("product_owner","current")).await.unwrap().unwrap();
        let delayed=cache.client.as_ref().clone().begin().await.unwrap();
        let generation=nonce();let mut vars=Variables::new();vars.insert("owner_generation",owner.generation);vars.insert("generation",generation.clone());
        vars.insert("header",Header{version:VERSION,directory:cache.directory.to_string_lossy().into_owned(),generation});
        delayed.query(installation_body(true)).bind(vars).await.unwrap().check().unwrap();
        let rebuilt=NativeProductCache::install(&runtime).await.unwrap().unwrap();
        let budget=ResourceBudget::fixed(16<<20).unwrap();let product=product("new-administrative-generation");
        assert!(rebuilt.insert(&product,&budget).await.unwrap());
        assert!(delayed.commit().await.is_err(),"late reset DDL cannot commit after a newer owner-generation installation");
        assert!(rebuilt.lookup(&product.request,&budget).await.unwrap().is_some());
        let delayed=rebuilt.client.as_ref().clone().begin().await.unwrap();
        let owner:CoordinationOwner=rebuilt.client.select::<Option<CoordinationOwner>>(("product_owner","current")).await.unwrap().unwrap();
        let mut vars=Variables::new();vars.insert("old_generation",rebuilt.generation.clone());vars.insert("generation",nonce());vars.insert("owner_generation",owner.generation);
        delayed.query(RETIREMENT_BODY).bind(vars).await.unwrap().check().unwrap();
        mark_administration(&rebuilt.directory).unwrap();
        assert!(NativeProductCache::connect(&runtime).await.unwrap().is_none(),"unknown administration cannot resume through readonly connect");
        assert!(rebuilt.lookup(&product.request,&budget).await.unwrap().is_none());
        let current=NativeProductCache::install(&runtime).await.unwrap().unwrap();
        assert!(current.insert(&product,&budget).await.unwrap());
        assert!(delayed.commit().await.is_err(),"late retirement cannot delete new-generation products");
        assert!(current.lookup(&product.request,&budget).await.unwrap().is_some());
        let entry=current.entry(&product.request.identity().unwrap().hex()).await.unwrap().unwrap();
        let old_ack=current.acknowledgment_path(&entry);assert!(old_ack.exists());
        let owner:CoordinationOwner=current.client.select::<Option<CoordinationOwner>>(("product_owner","current")).await.unwrap().unwrap();
        let mut vars=Variables::new();vars.insert("old_generation",current.generation.clone());vars.insert("generation",nonce());vars.insert("owner_generation",owner.generation);
        mark_administration(&current.directory).unwrap();
        // Native commit succeeded but its local caller lost the acknowledgment, so
        // generation assets remain. This is distinct from an uncommitted delayed tx.
        current.client.query(format!("BEGIN; {RETIREMENT_BODY}; COMMIT;")).bind(vars).await.unwrap().check().unwrap();
        assert!(old_ack.exists());
        let recovered=NativeProductCache::install(&runtime).await.unwrap().unwrap();
        assert!(!old_ack.exists(),"confirmed reconciliation reclaims old-generation ACK metadata");
        assert!(recovered.lookup(&product.request,&budget).await.unwrap().is_none());recovered.retire().await.unwrap();
    }

    #[tokio::test]
    async fn native_incompatible_reset_requires_the_existing_coordination_owner() {
        let (cache,mut runtime,_scratch)=fixture(16<<20).await;
        let budget=ResourceBudget::fixed(16<<20).unwrap();let product=product("owned-reset");
        cache.insert(&product,&budget).await.unwrap();
        cache.client.query("DEFINE FIELD obsolete_required ON product_header TYPE int; UPDATE product_header:current SET obsolete_required=1;").await.unwrap().check().unwrap();
        cache.client.query("UPDATE product_header:current SET version=1").await.unwrap().check().unwrap();
        let delayed=cache.client.as_ref().clone().begin().await.unwrap();
        let mut vars=Variables::new();vars.insert("generation",cache.generation.clone());
        delayed.query("IF (SELECT VALUE generation FROM product_header:current FOR UPDATE)[0] != $generation { THROW 'retired'; }; UPDATE product_quota:current SET used=used+3; CREATE product_stage:late SET staging='late',bytes=3").bind(vars).await.unwrap().check().unwrap();
        let foreign=tempfile::tempdir().unwrap();runtime.reuse.as_mut().unwrap().lease_directory=foreign.path().into();
        assert!(matches!(NativeProductCache::install(&runtime).await,Err(ModelError::Conflict("product cache coordination directory"))));
        assert!(cache.entry(&product.request.identity().unwrap().hex()).await.unwrap().is_some(),"foreign lifecycle owner cannot discard even incompatible state");
        runtime.reuse.as_mut().unwrap().lease_directory=cache.directory.clone();
        let reset=NativeProductCache::install(&runtime).await.unwrap().unwrap();
        assert!(reset.entry(&product.request.identity().unwrap().hex()).await.unwrap().is_none());
        assert!(delayed.commit().await.is_err(),"explicit reset fences delayed old-generation commit");
        let stages:Vec<Stage>=reset.client.select("product_stage").await.unwrap();assert!(stages.is_empty());
        // An interrupted reset may leave no version header. Its immutable coordination
        // owner still rejects a foreign installer with unrelated filesystem locks.
        reset.client.query("REMOVE TABLE product_header").await.unwrap().check().unwrap();
        runtime.reuse.as_mut().unwrap().lease_directory=foreign.path().into();
        assert!(matches!(NativeProductCache::install(&runtime).await,Err(ModelError::Conflict("product cache coordination directory"))));
        runtime.reuse.as_mut().unwrap().lease_directory=cache.directory.clone();
        let recovered=NativeProductCache::install(&runtime).await.unwrap().unwrap();recovered.retire().await.unwrap();
    }

}
