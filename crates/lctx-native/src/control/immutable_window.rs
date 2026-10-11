//! Reachability-bounded immutable installation: rows and their holds share one outcome.
use super::*;
use lctx_model::domain::resources::{ResourceBudget, Reservation, TRANSFER_BYTES, MAX_ROW_BYTES};

pub struct ImmutableWindow {
    sql: String,
    bindings: std::collections::BTreeMap<String, Value>,
    bytes: usize,
    rows: usize,
    charges: Vec<Box<dyn Reservation>>,
}
impl ImmutableWindow {
    fn unit(row: Value, attempt: Option<ContentHash>, contribution: Option<ContentHash>,
        owner: Option<&RecordId>, unit_references: bool, budget: Option<&ResourceBudget>) -> Result<Self, ModelError> {
        // Reserve expansion before constructing anchors, reference guards or binding arrays.
        // Native values include their container storage; escaped JSON can occupy six bytes
        // per source byte. The bound also covers duplicated guards and fixed SQL overhead.
        let admitted = crate::loader::native_bytes(&row).saturating_mul(32).saturating_add(65536);
        let mut charge = budget.map(|b| b.reserve("native immutable construction", admitted)).transpose()?;
        crate::loader::validate_native_row(&row)?;
        let Value::Object(object) = &row else { return Err(ModelError::Schema("native immutable object")); };
        let Some(Value::RecordId(id)) = object.get("id") else { return Err(ModelError::Schema("native immutable identity")); };
        let mut rows = Vec::new();
        if let Some(Value::RecordId(anchor)) = object.get("anchor") {
            if !matches!(anchor.table.as_str(), "entity_anchor" | "assertion_anchor") {
                return Err(ModelError::Schema("native anchor family"));
            }
            let surrealdb::types::RecordIdKey::String(nominal) = &anchor.key else {
                return Err(ModelError::Schema("native anchor identity"));
            };
            let mut value = Object::new();
            value.insert("id", anchor.clone());
            value.insert("family", anchor.table.as_str().trim_end_matches("_anchor"));
            value.insert("nominal", nominal.clone());
            rows.push(Value::Object(value));
        }
        let mut references = Vec::new();
        if id.table.as_str()=="original_chunk" {
            let Some(Value::RecordId(blob))=object.get("blob") else { return Err(ModelError::Schema("native original piece parent")); };
            references.push((blob.clone(),id.clone()));
        } else if let Some(owner) = owner {
            references.push((owner.clone(), id.clone()));
            let fields: &[&str] = if id.table.as_str()=="original" { &["artifact","blob"] } else { &["anchor","in","out","unit_node","source"] };
            for field in fields {
                if *field == "unit_node" && !unit_references { continue; }
                if let Some(Value::RecordId(target)) = object.get(*field) {
                    references.push((id.clone(), target.clone()));
                }
            }
        }
        let original_blob=if id.table.as_str()=="original" {Some(crate::originals::decode_layout(row.clone())?.blob)} else {None};
        rows.push(row);
        let row_count = rows.len();
        let (mut sql, mut bindings) = immutable_row_plan(attempt, contribution, rows)?
            .ok_or(ModelError::Schema("native immutable unit"))?;
        if let Some(blob)=original_blob {
            bindings.insert("original_blobs",vec![blob.value(false)]);
            let mut scope=Variables::new();scope.insert("__write_guards",vec![guard_id(&Value::RecordId(blob.id()))?]);scope.insert("__guard_records",guard_records(std::iter::once(&blob.id()))?);
            merge_plan_bindings(&mut bindings,scope)?;
            // Declaration and logical layout ownership have one atomic outcome. Completion
            // is mutable, while immutable declaration fields and exact header shape collide.
            sql=format!("FOR $i IN 0..array::len($original_blobs) {{ LET $row=$original_blobs[$i]; LET $blob=$row.id; fn::native_incidence_insert($row); LET $prior=SELECT * FROM ONLY $blob FOR UPDATE; IF $prior=NONE {{ CREATE $blob CONTENT $row RETURN NONE; }} ELSE {{ IF $prior.id!=$row.id OR $prior.content!=$row.content OR $prior.byte_len!=$row.byte_len OR $prior.format!=$row.format OR type::is::bool($prior.completed)=false OR array::len(object::keys($prior))!=5 {{ THROW 'original blob declaration collision'; }}; }}; }}; {sql}");
        }
        if let Some((hold_owner, hold_sql, holds)) = hold_plan(attempt, references)? {
            if !matches!(hold_owner, EffectOwner::OpenAttempt(_) | EffectOwner::Installation) {
                return Err(ModelError::Conflict("native immutable window authority"));
            }
            sql.push(';'); sql.push_str(&hold_sql);
            merge_plan_bindings(&mut bindings,holds)?;
        }
        let bytes = plan_bytes(&sql, &bindings)?;
        if bytes > MAX_ROW_BYTES {
            return Err(ModelError::Limit { owner: "native immutable window", limit: "expanded unit bytes", observed: bytes, bound: MAX_ROW_BYTES });
        }
        if bytes > admitted { return Err(ModelError::Schema("native immutable construction bound")); }
        if let Some(charge) = &mut charge { charge.try_resize(bytes)?; }
        Ok(Self { sql, bindings: bindings.into_iter().collect(), bytes, rows: row_count, charges: charge.into_iter().collect() })
    }
    fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if self.sql != other.sql { return Err(ModelError::Conflict("native immutable window plan")); }
        let bytes = self.bytes.saturating_add(other.bytes);
        for (key, value) in other.bindings {
            match (self.bindings.get_mut(&key), value) {
                (Some(Value::Array(left)), Value::Array(right)) => {
                    left.extend(right);
                }
                (Some(left), right) if *left == right => {},
                _ => return Err(ModelError::Conflict("native immutable window binding")),
            }
        }
        self.charges.extend(other.charges);
        self.rows += other.rows;
        // Summed complete-unit charges conservatively retain their repeated SQL/map
        // overhead, avoiding a serialization of the growing window after each row.
        self.bytes = bytes;
        Ok(())
    }
}
pub(super) fn plan_bytes(sql: &str, bindings: &Variables) -> Result<usize, ModelError> {
    // Full native containers, serialization, request hashing and intent/payload controls
    // coexist. Include guard/hold expansion rather than budgeting only the input row.
    let native = bindings.iter().map(|(key, value)| key.len().saturating_add(crate::loader::native_bytes(value))).fold(0usize, usize::saturating_add);
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> { self.0 = self.0.saturating_add(bytes.len()); Ok(bytes.len()) }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut count = Counter(0);
    serde_json::to_writer(&mut count, bindings).map_err(ModelError::codec)?;
    let encoded = count.0;
    Ok(native.saturating_mul(2).saturating_add(encoded.saturating_mul(3)).saturating_add(sql.len().saturating_mul(4)).saturating_add(8192))
}
pub struct ImmutableWindows<'a> {
    rows: Box<dyn Iterator<Item = Result<(Value, Option<Box<dyn Reservation>>), ModelError>> + Send + 'a>,
    pending: Option<ImmutableWindow>,
    attempt: Option<ContentHash>,
    contribution: Option<ContentHash>,
    owner: Option<RecordId>,
    unit_references: bool,
    failed: bool,
    budget: Option<ResourceBudget>,
}
impl<'a> ImmutableWindows<'a> {
    pub fn new(rows: Vec<Value>, attempt: Option<ContentHash>, contribution: Option<ContentHash>, owner: Option<RecordId>) -> Self {
        Self::from_charged_iter(rows.into_iter().map(|row| Ok((row, None))), attempt, contribution, owner)
    }
    /// Lazy producers reserve their native representation before copying a row. Both
    /// that reservation and the expanded transaction charge survive acknowledgement.
    pub fn from_charged_iter(rows: impl Iterator<Item = Result<(Value, Option<Box<dyn Reservation>>), ModelError>> + Send + 'a,
        attempt: Option<ContentHash>, contribution: Option<ContentHash>, owner: Option<RecordId>) -> Self {
        Self { rows: Box::new(rows), pending: None, attempt, contribution, owner, unit_references: false, failed: false, budget: None }
    }
    pub fn with_budget(mut self, budget: Option<&ResourceBudget>) -> Self {
        self.budget = budget.cloned(); self
    }
    pub fn for_loading(rows: Vec<Value>, attempt: Option<ContentHash>, owner: Option<RecordId>) -> Self {
        let mut windows = Self::new(rows, attempt, None, owner);
        windows.unit_references = true;
        windows
    }
}
impl Iterator for ImmutableWindows<'_> {
    type Item = Result<ImmutableWindow, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed { return None; }
        let result = (|| {
            let mut output = self.pending.take();
            for input in self.rows.by_ref() {
                let (row, input_charge) = input?;
                let mut unit = ImmutableWindow::unit(row, self.attempt, self.contribution, self.owner.as_ref(), self.unit_references, self.budget.as_ref())?;
                unit.charges.extend(input_charge);
                if let Some(window) = &output {
                    if window.sql!=unit.sql || window.rows + unit.rows > crate::loader::NATIVE_WINDOW_ROWS || window.bytes.saturating_add(unit.bytes) > TRANSFER_BYTES {
                        self.pending = Some(unit); break;
                    }
                }
                match &mut output { Some(window) => window.append(unit)?, None => output = Some(unit) }
            }
            Ok(output)
        })();
        match result { Ok(Some(window)) => Some(Ok(window)), Ok(None) => None, Err(error) => { self.failed = true; Some(Err(error)) } }
    }
}
pub async fn install_window(client: &Surreal<Client>, attempt: Option<ContentHash>, window: ImmutableWindow,
    budget: Option<&ResourceBudget>) -> Result<(), ModelError> {
    let _charge: Option<Box<dyn Reservation>> = if window.charges.is_empty() { budget.map(|budget| budget.reserve("native immutable window", window.bytes)).transpose()? } else { None };
    effect(client, attempt, &window.sql, window.bindings.into()).await
}


/// A frozen derivation can only retain derived children and exact existing canonical
/// external endpoints. It cannot introduce a canonical payload after native freeze.
pub(super) async fn install_staging_window(client:&Surreal<Client>,staging:&staging::DerivedStaging,
    mut window:ImmutableWindow,budget:&ResourceBudget)->Result<(),ModelError>{
    let _charge=if window.charges.is_empty(){Some(budget.reserve("publication derived immutable window",window.bytes)?)}else{None};
    let Some(Value::Array(rows))=window.bindings.get("rows") else{return Err(ModelError::Schema("publication immutable rows"));};
    for row in rows.iter(){
        let Value::Object(row)=row else{return Err(ModelError::Schema("publication derived object"));};
        let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("publication derived address"));};
        if crate::derivation::FAMILIES.contains(&id.table.as_str()){
            crate::derivation::require_namespace(row,staging.derivation())?;
        }else if !matches!(id.table.as_str(),"external"|"entity_anchor"|"assertion_anchor"){
            return Err(ModelError::Conflict("publication derived immutable family"));
        }
    }
    // The ensuing immutable write and object-incarnation guard share this transaction.
    // Thus canonical endpoint comparison cannot race a retirement/replacement.
    let _guard_charge=budget.reserve("publication canonical endpoint guard construction",8192)?;
    window.sql=format!("FOR $row IN $rows {{ IF record::table($row.id) IN ['external','entity_anchor','assertion_anchor'] {{ LET $canonical_id=$row.id; LET $canonical=SELECT * FROM ONLY $canonical_id FOR UPDATE; IF $canonical=NONE OR $canonical!=$row {{ THROW 'frozen canonical endpoint missing or changed'; }}; }}; }}; {}",window.sql);
    staging.effect(client,&window.sql,window.bindings.into(),budget).await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn immutable_control_row(name: &str, size: i64) -> Value {
        let mut target = lctx_model::domain::graph::Target::External {
            provider:lctx_model::domain::graph::EntityId(ContentHash::of(b"immutable fixture provider")),
            context:lctx_model::domain::graph::EntityId(ContentHash::of(b"immutable fixture context")),
            name:format!("{name}:0"), reason:lctx_model::domain::normalized::entities::EntityReason::ProviderExternal,
        };
        let id=crate::reader::target_id(target.clone());
        // The late collision deliberately substitutes different full canonical bytes at
        // an occupied address. The initially installed endpoint has its exact model ID.
        if let lctx_model::domain::graph::Target::External{name:label,..}=&mut target{*label=format!("{name}:{size}");}
        let mut row = Object::new();row.insert("id",id);
        row.insert("canonical",surrealdb::types::Bytes::from(serde_json::to_vec(&target).unwrap()));
        Value::Object(row)
    }
    #[test]
    fn windows_account_for_guards_and_holds_and_bound_complete_units() {
        let attempt = ContentHash::of(b"window owner");
        let owner = RecordId::new("native_attempt", attempt.hex());
        let rows = (0..300).map(|n| immutable_control_row(&format!("item-{n}"), 0)).collect();
        let windows = ImmutableWindows::new(rows, Some(attempt), None, Some(owner)).collect::<Result<Vec<_>, _>>().unwrap();
        assert!(windows.len() > 1);
        assert_eq!(windows.iter().map(|window| window.rows).sum::<usize>(), 300);
        for window in windows {
            assert!(window.rows <= crate::loader::NATIVE_WINDOW_ROWS);
            assert!(window.bytes <= TRANSFER_BYTES);
            let Some(Value::Array(rows)) = window.bindings.get("rows") else { panic!("rows"); };
            let Some(Value::Array(holds)) = window.bindings.get("holds") else { panic!("holds"); };
            assert_eq!(rows.len(), holds.len());
            assert!(window.bytes > serde_json::to_vec(&window.bindings).unwrap().len());
        }
    }
    #[tokio::test]
    async fn budget_refusal_precedes_native_effect_and_releases_reservation() {
        let attempt = ContentHash::of(b"window owner");
        let window = ImmutableWindows::new(vec![immutable_control_row("budget-refusal", 0)], Some(attempt), None,
            Some(RecordId::new("native_attempt", attempt.hex()))).next().unwrap().unwrap();
        let budget = ResourceBudget::fixed(window.bytes - 1).unwrap();
        assert!(matches!(install_window(&Surreal::init(), Some(attempt), window, Some(&budget)).await, Err(ModelError::Resource { .. })));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn construction_is_admitted_and_lookahead_stays_charged_until_drop() {
        let budget = ResourceBudget::fixed(4 * 1024 * 1024).unwrap();
        let rows = (0..300).map(|n| immutable_control_row(&format!("charged-{n}"), 0)).collect();
        let mut windows = ImmutableWindows::new(rows, None, None, None).with_budget(Some(&budget));
        let first = windows.next().unwrap().unwrap();
        assert!(budget.reserved() > first.bytes, "one pending complete unit remains charged");
        drop(first); assert!(budget.reserved() > 0);
        drop(windows); assert_eq!(budget.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(ImmutableWindows::new(vec![immutable_control_row("refuse", 0)], None, None, None)
            .with_budget(Some(&tiny)).next().unwrap(), Err(ModelError::Resource { .. })));
        assert_eq!(tiny.reserved(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_late_collision_rolls_back_rows_and_holds_together() {
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"))).unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts).await.unwrap();
        let client = lctx_compiler_store::reader::connect(&config.endpoint, &config.writer_credentials(), store.namespace().as_str(), store.database().as_str()).await.unwrap();
        let attempt = store.attempt(); let owner = RecordId::new("native_attempt", attempt.hex());
        let existing = format!("{}-existing", attempt.hex()); let absent = format!("{}-absent", attempt.hex());
        let first = ImmutableWindows::new(vec![immutable_control_row(&existing, 0)], Some(attempt), None, Some(owner.clone())).next().unwrap().unwrap();
        install_window(&client, Some(attempt), first, None).await.unwrap();
        let mut windows = ImmutableWindows::new(vec![immutable_control_row(&absent, 0), immutable_control_row(&existing, 1)], Some(attempt), None, Some(owner));
        let collision = windows.next().unwrap().unwrap(); assert!(windows.next().is_none());
        let result = install_window(&client, Some(attempt), collision, None).await;
        assert!(result.is_err(), "full-value collision must refuse");
        let mut response = client.query("SELECT * FROM $absent; SELECT * FROM native_hold WHERE object=$absent")
            .bind(("absent", match immutable_control_row(&absent,0){Value::Object(row)=>row.get("id").unwrap().clone(),_=>unreachable!()})).await.unwrap().check().unwrap();
        let rows: Vec<Value> = response.take(0).unwrap(); let holds: Vec<Value> = response.take(1).unwrap();
        assert!(rows.is_empty()); assert!(holds.is_empty());
        client.invalidate().await.unwrap(); store.abandon().await.unwrap();
    }
}
