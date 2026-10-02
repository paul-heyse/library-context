//! Explicit, disposable exact-vector realization. This cache holds no semantic/context/text copies.
use super::{Error, GenerationGuard, GenerationId, GenerationLease, GenerationStore};
use lctx_model::domain::{
    *, normalized::Rows, resources::{ResourceBudget, Reservation},
    embedding::{EmbeddingSpec, Spec, analytic::{AnalysisEmbeddingUse,VectorAvailability},
        text::TextWindow, consumption::Winners, value},
    retrieval::{Fragment, consumption::RetrievalEmbeddingUse},
};
use pgvector::Vector;
use std::{collections::BTreeMap, sync::Arc};
type ArtifactManifestRow = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, i16, Vec<u8>, i64);
type PhysicalColumnRow = (String, String, String, String, i32, bool, bool, String, String);
const DIMENSIONS: u32 = 1024;
const CACHE_DDL: &str = include_str!("../../migrations/202610020015_serving_vector_artifact.sql");
pub struct ScoredVectors {
    values: Vec<(Id<RetrievalEmbeddingUse>,f64)>,
    _charge: Box<dyn Reservation>,
}
impl ScoredVectors { pub fn values(&self) -> &[(Id<RetrievalEmbeddingUse>,f64)] { &self.values } }
pub struct VectorArtifact {
    pub key: ContentHash,
    pub generation: GenerationId,
    pub model: ContentHash,
    pub source: ContentHash,
    pub specification: Spec,
    physical: ContentHash,
    rows: BTreeMap<Id<RetrievalEmbeddingUse>, Vector>,
    _charge: Box<dyn Reservation>,
    guard: Option<GenerationGuard>,
}
struct Inputs {
    model:Arc<ValidatedModel>,
    specifications: Rows<EmbeddingSpec>, fragments: Rows<Fragment>, uses: Rows<RetrievalEmbeddingUse>,
    analytic: Rows<AnalysisEmbeddingUse>, windows: Rows<TextWindow>,
}
async fn load<R: Record>(lease: &mut GenerationLease) -> Result<Rows<R>, Error> {
    let mut rows = Rows::new(&lease.budget);
    lease.visit_verified::<R>(|batch| {
        for row in batch.rows() { rows.insert(row.clone())?; }
        Ok(())
    }).await?;
    Ok(rows)
}
impl Inputs {
    async fn load(lease: &mut GenerationLease) -> Result<Self,Error> {
        if lease.frontier != admission::Frontier::Catalog { return Err(Error::Frontier("vector preparation requires Catalog".into())); }
        let specifications:Rows<EmbeddingSpec>=load(lease).await?;
        if specifications.len()!=1 { return Err(Error::Frontier("vector preparation requires one selected embedding specification".into())); }
        if specifications.iter().next().ok_or(Error::Contract)?.configuration()?.dimensions!=DIMENSIONS {
            return Err(Error::Frontier("unsupported exact vector profile; select lexical-only explicitly".into()));
        }
        Ok(Self { model:lease.model.clone(), specifications, fragments:load(lease).await?, uses:load(lease).await?,
            analytic:load(lease).await?, windows:load(lease).await? })
    }
    fn derive(self, generation:GenerationId, model:ContentHash, physical:ContentHash, budget:&ResourceBudget) -> Result<VectorArtifact,Error> {
        if self.specifications.len()!=1 { return Err(Error::Contract); }
        let selected = self.specifications.iter().next().ok_or(Error::Contract)?;
        let spec = selected.configuration()?;
        if spec.dimensions != DIMENSIONS { return Err(Error::Frontier("unsupported exact vector profile; select lexical-only explicitly".into())); }
        let mut winner = Winners::new(budget);
        for row in self.analytic.iter().filter(|r|r.availability==VectorAvailability::Available) {
            if row.specification != selected.id() { return Err(Error::Contract); }
            let window = self.windows.get(row.window).ok_or(Error::Contract)?;
            drop(winner.replay(&spec,window.text.as_str(),row.receipt()?)?);
        }
        let relation = Relation::of::<RetrievalEmbeddingUse>();
        let mut content = relation.content();
        let mut rows = BTreeMap::new();
        let mut charge = budget.reserve("serving-vector-artifact",size_of::<VectorArtifact>()+spec.heap_bytes())?;
        for row in self.uses.iter() {
            let batch = Batch::new(&self.model,vec![row.clone()],budget)?;
            relation.hash_rows(batch.arrow(),&mut content)?;
            if row.specification != selected.id() { return Err(Error::Contract); }
            if row.availability != VectorAvailability::Available { continue; }
            let fragment = self.fragments.get(row.fragment).ok_or(Error::Contract)?;
            let decoded = winner.replay(&spec,fragment.text.as_str(),row.receipt()?)?;
            charge.try_resize(charge.size().checked_add(decoded.values().len()*4+size_of::<(Id<RetrievalEmbeddingUse>,Vector)>()+64).ok_or(Error::Contract)?)?;
            rows.insert(row.id(),Vector::from(decoded.values().to_vec()));
        }
        let source = content.finish().1;
        let mut key = KeySink::new("serving-vector-artifact/v1");
        key.part(b"generation",generation.bytes()); model.encode(&mut key); source.encode(&mut key);
        spec.hash().encode(&mut key); physical.encode(&mut key);
        Ok(VectorArtifact {key:key.finish(),generation,model,source,specification:spec,physical,rows,_charge:charge,guard:None})
    }
}
impl GenerationStore {
    /// Explicit owner effect. Preparation pins canonical inputs; never selects or calls an embedder.
    pub async fn prepare_vector_artifact(&self, reader:&sqlx::PgPool, generation:GenerationId, budget:ResourceBudget) -> Result<ContentHash,Error> {
        let mut lease = self.pin(reader,generation,budget.clone()).await?;
        let physical=inspect_cache(&mut lease.connection,&budget).await?;
        let inputs = Inputs::load(&mut lease).await?;
        let artifact = inputs.derive(generation,self.model.digest(),physical,&budget)?;
        let mut tx = self.owner.begin().await?;
        if inspect_cache(&mut tx,&budget).await?!=physical {return Err(Error::Contract);}
        let lock = i64::from_le_bytes(artifact.key.0[..8].try_into().expect("digest width"));
        sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(lock).execute(&mut *tx).await?;
        // Explicit preparation replaces wrong-but-valid numerical content atomically.
        sqlx::query("DELETE FROM lctx_cache.serving_vector_artifacts WHERE artifact_key=$1").bind(artifact.key.0.to_vec()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO lctx_cache.serving_vector_artifacts(artifact_key,generation_id,model_digest,source_content,spec_hash,codec,physical_digest,row_count) VALUES($1,$2,$3,$4,$5,1,$6,$7)")
            .bind(artifact.key.0.to_vec()).bind(generation.0.to_vec()).bind(artifact.model.0.to_vec()).bind(artifact.source.0.to_vec())
            .bind(artifact.specification.hash().0.to_vec()).bind(physical.0.to_vec()).bind(i64::try_from(artifact.rows.len()).map_err(|_|Error::Contract)?)
            .execute(&mut *tx).await?;
        let _readback=budget.reserve("serving-vector-preparation-readback",DIMENSIONS as usize*8+512)?;
        for (id, vector) in &artifact.rows {
            let readback:Vector=sqlx::query_scalar("INSERT INTO lctx_cache.serving_vectors(artifact_key,retrieval_use,value) VALUES($1,$2,$3) RETURNING value")
                .bind(artifact.key.0.to_vec()).bind(id.bytes().to_vec()).bind(vector).fetch_one(&mut *tx).await?;
            if !same_bits(readback.as_slice(),vector.as_slice()) {return Err(Error::Contract);}
        }
        tx.commit().await?;
        artifact.check(&mut lease).await?;
        lease.release().await?;
        Ok(artifact.key)
    }
}
impl GenerationGuard {
    /// Read-only admission derives expected canonical content, validates cache manifest and
    /// numerical readback, then retains this same original process guard with charged vectors.
    pub async fn vector_artifact(&self) -> Result<Arc<VectorArtifact>,Error> {
        self.check().await?;
        let (inputs,model,physical,budget) = {
            let mut locked = self.state.lease.lock().await;
            let lease = locked.as_mut().ok_or(Error::State)?;
            let physical=inspect_cache(&mut lease.connection,&lease.budget).await?;
            (Inputs::load(lease).await?,lease.model.digest(),physical,lease.budget.clone())
        };
        let mut artifact = inputs.derive(self.generation(),model,physical,&budget)?;
        {
            let mut locked = self.state.lease.lock().await;
            artifact.check(locked.as_mut().ok_or(Error::State)?).await?;
        }
        self.check().await?;
        artifact.guard=Some(self.clone());
        Ok(Arc::new(artifact))
    }
}
impl VectorArtifact {
    async fn check(&self, lease:&mut GenerationLease) -> Result<(),Error> {
        if inspect_cache(&mut lease.connection,&lease.budget).await?!=self.physical {return Err(Error::Contract);}
        self.check_manifest(&mut lease.connection).await?;
        // The selected typmod is already inspected. Charge one decoded numerical row before fetch.
        let _readback=lease.budget.reserve("serving-vector-admission-readback",DIMENSIONS as usize*8+512)?;
        use futures::TryStreamExt;
        let mut values = sqlx::query_as::<_,(Vec<u8>,Vector)>("SELECT retrieval_use,value FROM lctx_cache.serving_vectors WHERE artifact_key=$1 ORDER BY retrieval_use")
            .bind(self.key.0.to_vec()).fetch(&mut *lease.connection);
        let mut count=0;
        while let Some((key,vector))=values.try_next().await? {
            let id=decode_use(key)?;
            let expected=self.rows.get(&id).ok_or(Error::Contract)?;
            if !same_bits(vector.as_slice(),expected.as_slice()) { return Err(Error::Contract); }
            count+=1;
        }
        if count!=self.rows.len() {return Err(Error::Contract);}
        Ok(())
    }
    async fn check_manifest(&self, connection:&mut sqlx::PgConnection) -> Result<(),Error> {
        let manifest: Option<ArtifactManifestRow> = sqlx::query_as("SELECT generation_id,model_digest,source_content,spec_hash,codec,physical_digest,row_count FROM lctx_cache.serving_vector_artifacts WHERE artifact_key=$1")
            .bind(self.key.0.to_vec()).fetch_optional(connection).await?;
        if manifest != Some((self.generation.0.to_vec(),self.model.0.to_vec(),self.source.0.to_vec(),self.specification.hash().0.to_vec(),value::VALUE_CODEC,self.physical.0.to_vec(),i64::try_from(self.rows.len()).map_err(|_|Error::Contract)?)) { return Err(Error::Contract); }
        Ok(())
    }
    /// Exact scan over the complete caller-admitted eligible use domain; no candidate limit.
    pub async fn score(self:&Arc<Self>, execution:&super::RequestExecution, eligible:Vec<Id<RetrievalEmbeddingUse>>, query:Vec<f32>) -> Result<ScoredVectors,Error> {
        let guard=self.guard.as_ref().ok_or(Error::State)?;
        if execution.generation()!=self.generation || !execution.shares_guard(guard) {return Err(Error::Contract);}
        guard.check().await?;
        embedding::check_vector(&query,DIMENSIONS).map_err(ModelError::Invalid)?;
        if eligible.iter().any(|id|!self.rows.contains_key(id)) {return Err(Error::Contract);}
        let output_charge=execution.budget().reserve("serving-vector-scores",eligible.len().checked_mul(128).and_then(|n|n.checked_add(size_of::<ScoredVectors>())).ok_or(Error::Contract)?)?;
        let input_bytes=eligible.len().checked_mul(128).and_then(|n|n.checked_add(query.len()*8+512)).ok_or(Error::Contract)?;
        let input=execution.budget().reserve("serving-vector-score-input",input_bytes)?;
        let unique = eligible.iter().collect::<std::collections::BTreeSet<_>>();
        if unique.len()!=eligible.len() {return Err(Error::Contract);}
        drop(unique);
        let key=self.key.0.to_vec();
        let ids=eligible.iter().map(|id|id.bytes().to_vec()).collect::<Vec<_>>();
        let vector=Vector::from(query);
        let retained=self.clone();
        let rows=execution.query(move |lease| Box::pin(async move {
            let _input=input;
            if inspect_cache(&mut lease.connection,&lease.budget).await?!=retained.physical {return Err(Error::Contract);}
            retained.check_manifest(&mut lease.connection).await?;
            let _readback=lease.budget.reserve("serving-vector-scoring-readback",DIMENSIONS as usize*8+512)?;
            use futures::TryStreamExt;
            let mut stream=sqlx::query_as::<_,(Vec<u8>,Vector,f64)>("SELECT retrieval_use,value,1-(value OPERATOR(lctx_ext.<=>) $2) FROM lctx_cache.serving_vectors WHERE artifact_key=$1 AND retrieval_use=ANY($3) ORDER BY retrieval_use")
                .bind(key).bind(vector).bind(ids).fetch(&mut *lease.connection);
            let mut result=Vec::new();
            while let Some((id,vector,score))=stream.try_next().await? {
                if !score.is_finite() {return Err(Error::Contract);}
                let id=decode_use(id)?;
                let expected=retained.rows.get(&id).ok_or(Error::Contract)?;
                if !same_bits(vector.as_slice(),expected.as_slice()) {return Err(Error::Contract);}
                result.push((id,score));
            }
            Ok(ScoredVectors {values:result,_charge:output_charge})
        })).await?;
        if rows.values.len()!=eligible.len() {return Err(Error::Contract);}
        guard.check().await?;
        Ok(rows)
    }
}
fn same_bits(a:&[f32],b:&[f32])->bool { a.len()==b.len() && a.iter().zip(b).all(|(a,b)|a.to_bits()==b.to_bits()) }
fn decode_use(bytes:Vec<u8>)->Result<Id<RetrievalEmbeddingUse>,Error> {
    Ok(serde_json::from_value(serde_json::to_value(bytes).map_err(ModelError::codec)?).map_err(ModelError::codec)?)
}

/// Inspect this migration's two numerical tables, not the retained cache/operations services.
/// Bounded catalog reads refuse extra columns/constraints/indexes before any vector scan.
async fn inspect_cache(connection:&mut sqlx::PgConnection,budget:&ResourceBudget)->Result<ContentHash,Error> {
    let _charge=budget.reserve("serving-vector-physical-inspection",64*1024)?;
    let extension:Option<(String,String)>=sqlx::query_as("SELECT e.extversion,n.nspname::text FROM pg_extension e JOIN pg_namespace n ON n.oid=e.extnamespace WHERE e.extname='vector'")
        .fetch_optional(&mut *connection).await?;
    if extension!=Some(("0.8.6".into(),"lctx_ext".into())) {return Err(Error::Codec("serving vector cache extension version/namespace differs".into()));}
    let cache_owner:Option<String>=sqlx::query_scalar("SELECT r.rolname::text FROM pg_namespace n JOIN pg_roles r ON r.oid=n.nspowner WHERE n.nspname='lctx_cache'")
        .fetch_optional(&mut *connection).await?;
    if cache_owner.as_deref()!=Some("lctx_migrator") {return Err(Error::Codec("serving vector cache schema owner differs".into()));}
    let tables:Vec<(String,String,String,String,bool,bool,bool)>=sqlx::query_as("SELECT c.relname::text,r.rolname::text,c.relkind::text,c.relpersistence::text,c.relrowsecurity,c.relforcerowsecurity,c.relispartition FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_roles r ON r.oid=c.relowner WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') ORDER BY c.relname LIMIT 3")
        .fetch_all(&mut *connection).await?;
    let expected_tables=vec![
        ("serving_vector_artifacts".into(),"lctx_migrator".into(),"r".into(),"p".into(),false,false,false),
        ("serving_vectors".into(),"lctx_migrator".into(),"r".into(),"p".into(),false,false,false)];
    if tables!=expected_tables {return Err(Error::Codec("serving vector cache table ownership/flags differs".into()));}
    let columns:Vec<PhysicalColumnRow>=sqlx::query_as("SELECT c.relname::text,a.attname::text,tn.nspname::text,t.typname::text,a.atttypmod,a.attnotnull,a.atthasdef,a.attgenerated::text,a.attidentity::text FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_type t ON t.oid=a.atttypid JOIN pg_namespace tn ON tn.oid=t.typnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') AND a.attnum>0 AND NOT a.attisdropped ORDER BY c.relname,a.attnum LIMIT 13")
        .fetch_all(&mut *connection).await?;
    let fields=[
        ("serving_vector_artifacts","artifact_key","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","generation_id","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","model_digest","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","source_content","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","spec_hash","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","codec","pg_catalog","int2",-1),
        ("serving_vector_artifacts","physical_digest","pg_catalog","bytea",-1),
        ("serving_vector_artifacts","row_count","pg_catalog","int8",-1),
        ("serving_vectors","artifact_key","pg_catalog","bytea",-1),
        ("serving_vectors","retrieval_use","pg_catalog","bytea",-1),
        ("serving_vectors","value","lctx_ext","vector",DIMENSIONS as i32)];
    let expected_columns:Vec<_>=fields.into_iter().map(|(table,column,namespace,ty,modifier)|
        (table.to_owned(),column.to_owned(),namespace.to_owned(),ty.to_owned(),modifier,true,false,String::new(),String::new())).collect();
    if columns!=expected_columns {return Err(Error::Codec("serving vector cache columns/types/modifiers differs".into()));}
    // Definitions are bounded at the server; a forged long expression cannot inflate admission.
    let constraints:Vec<(String,String,String,bool,bool,bool)>=sqlx::query_as("SELECT c.relname::text,k.contype::text,left(pg_get_constraintdef(k.oid),512),k.convalidated,k.condeferrable,k.condeferred FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') ORDER BY c.relname,k.contype,pg_get_constraintdef(k.oid) LIMIT 25")
        .fetch_all(&mut *connection).await?;
    let normalize=|definition:&str| definition.chars().filter(|c|!c.is_whitespace() && *c!='(' && *c!=')').collect::<String>();
    let checks=[
        ("serving_vector_artifacts","c","CHECK (octet_length(artifact_key)=32)"),
        ("serving_vector_artifacts","c","CHECK (octet_length(generation_id)=16)"),
        ("serving_vector_artifacts","c","CHECK (octet_length(model_digest)=32)"),
        ("serving_vector_artifacts","c","CHECK (octet_length(source_content)=32)"),
        ("serving_vector_artifacts","c","CHECK (octet_length(spec_hash)=32)"),
        ("serving_vector_artifacts","c","CHECK (codec=1)"),
        ("serving_vector_artifacts","c","CHECK (octet_length(physical_digest)=32)"),
        ("serving_vector_artifacts","c","CHECK (row_count>=0)"),
        ("serving_vector_artifacts","p","PRIMARY KEY (artifact_key)"),
        ("serving_vectors","c","CHECK (octet_length(retrieval_use)=16)"),
        ("serving_vectors","p","PRIMARY KEY (artifact_key,retrieval_use)"),
        ("serving_vectors","f","FOREIGN KEY (artifact_key) REFERENCES lctx_cache.serving_vector_artifacts(artifact_key) ON DELETE CASCADE")];
    let mut expected_constraints:Vec<_>=checks.into_iter().map(|(table,kind,definition)|
        (table.to_owned(),kind.to_owned(),normalize(definition),true,false,false)).collect();
    // PostgreSQL18 catalogs NOT NULL as native constraints, including primary-key columns.
    expected_constraints.extend(fields.iter().map(|(table,column,_,_,_)|
        (table.to_string(),"n".to_owned(),normalize(&format!("NOT NULL {column}")),true,false,false)));
    let mut normalized:Vec<_>=constraints.iter().map(|(table,kind,definition,valid,defer,deferred)|
        (table.clone(),kind.clone(),normalize(definition),*valid,*defer,*deferred)).collect();
    expected_constraints.sort();normalized.sort();
    if normalized!=expected_constraints {return Err(Error::Codec("serving vector cache constraints differs".into()));}
    let indexes:Vec<(String,String,bool,bool)>=sqlx::query_as("SELECT c.relname::text,left(pg_get_indexdef(i.indexrelid),512),i.indisvalid,i.indisready FROM pg_index i JOIN pg_class c ON c.oid=i.indrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') ORDER BY c.relname,pg_get_indexdef(i.indexrelid) LIMIT 5")
        .fetch_all(&mut *connection).await?;
    let expected_indexes=vec![
        ("serving_vector_artifacts".into(),"CREATE INDEX serving_vector_generation ON lctx_cache.serving_vector_artifacts USING btree (generation_id)".into(),true,true),
        ("serving_vector_artifacts".into(),"CREATE UNIQUE INDEX serving_vector_artifacts_pkey ON lctx_cache.serving_vector_artifacts USING btree (artifact_key)".into(),true,true),
        ("serving_vectors".into(),"CREATE UNIQUE INDEX serving_vectors_pkey ON lctx_cache.serving_vectors USING btree (artifact_key, retrieval_use)".into(),true,true)];
    if indexes!=expected_indexes {return Err(Error::Codec("serving vector cache indexes differs".into()));}
    let extra:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') AND NOT t.tgisinternal) OR EXISTS(SELECT 1 FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors')) OR EXISTS(SELECT 1 FROM pg_rewrite w JOIN pg_class c ON c.oid=w.ev_class JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors'))")
        .fetch_one(&mut *connection).await?;
    if extra {return Err(Error::Codec("serving vector cache triggers/policies/rules differs".into()));}
    let acl:Vec<(String,String,String,String,bool)>=sqlx::query_as("WITH objects AS (SELECT c.relname::text name,c.relowner owner,coalesce(c.relacl,acldefault('r',c.relowner)) acl FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') UNION ALL SELECT n.nspname::text,n.nspowner,coalesce(n.nspacl,acldefault('n',n.nspowner)) FROM pg_namespace n WHERE n.nspname='lctx_cache') SELECT o.name,coalesce(r.rolname::text,'PUBLIC'),g.rolname::text,a.privilege_type,a.is_grantable FROM objects o CROSS JOIN LATERAL aclexplode(o.acl) a LEFT JOIN pg_roles r ON r.oid=a.grantee JOIN pg_roles g ON g.oid=a.grantor ORDER BY 1,2,3,4,5 LIMIT 48")
        .fetch_all(&mut *connection).await?;
    let expected_acl:Vec<(String,String,String,String,bool)>=sqlx::query_as("WITH objects AS (SELECT c.relname::text name,c.relowner owner,acldefault('r',c.relowner) acl FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_cache' AND c.relname IN ('serving_vector_artifacts','serving_vectors') UNION ALL SELECT n.nspname::text,n.nspowner,acldefault('n',n.nspowner) FROM pg_namespace n WHERE n.nspname='lctx_cache') SELECT o.name,r.rolname::text,g.rolname::text,a.privilege_type,a.is_grantable FROM objects o CROSS JOIN LATERAL aclexplode(o.acl) a JOIN pg_roles r ON r.oid=a.grantee JOIN pg_roles g ON g.oid=a.grantor UNION ALL SELECT * FROM (VALUES ('lctx_cache','lctx_app','lctx_migrator','USAGE',false),('lctx_cache','lctx_serving','lctx_migrator','USAGE',false),('serving_vector_artifacts','lctx_serving','lctx_migrator','SELECT',false),('serving_vectors','lctx_serving','lctx_migrator','SELECT',false)) grants ORDER BY 1,2,3,4,5 LIMIT 48")
        .fetch_all(&mut *connection).await?;
    if acl!=expected_acl {return Err(Error::Codec("serving vector cache ACL differs".into()));}
    let mut digest=KeySink::new("serving-vector-physical/v1");
    digest.part(b"migration",CACHE_DDL.as_bytes());
    digest.part(b"conversion",b"pgvector-0.8.6/exact-cosine-1024/shared-f32-le-codec-1");
    for (name,bytes) in [(b"tables".as_slice(),serde_json::to_vec(&tables)),(b"columns".as_slice(),serde_json::to_vec(&columns)),
        (b"constraints".as_slice(),serde_json::to_vec(&normalized)),(b"indexes".as_slice(),serde_json::to_vec(&indexes)),
        (b"acl".as_slice(),serde_json::to_vec(&acl))] {digest.part(name,&bytes.map_err(ModelError::codec)?);}
    Ok(digest.finish())
}
