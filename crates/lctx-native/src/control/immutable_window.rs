//! Reachability-bounded immutable installation: rows and their holds share one outcome.
use super::*;
use lctx_model::domain::resources::{MAX_ROW_BYTES, Reservation, ResourceBudget, TRANSFER_BYTES};

pub struct ImmutableWindow {
    sql: String,
    bindings: std::collections::BTreeMap<String, Value>,
    bytes: usize,
    rows: usize,
    charges: Vec<Box<dyn Reservation>>,
}
impl ImmutableWindow {
    fn unit(
        row: Value,
        attempt: Option<ContentHash>,
        contribution: Option<ContentHash>,
        owner: Option<&RecordId>,
        unit_references: bool,
        budget: Option<&ResourceBudget>,
    ) -> Result<Self, ModelError> {
        // Reserve expansion before constructing anchors, reference guards or binding arrays.
        // Native values include their container storage; escaped JSON can occupy six bytes
        // per source byte. The bound also covers duplicated guards and fixed SQL overhead.
        let admitted = crate::loader::native_bytes(&row)
            .saturating_mul(32)
            .saturating_add(65536);
        let mut charge = budget
            .map(|b| b.reserve("native immutable construction", admitted))
            .transpose()?;
        crate::loader::validate_native_row(&row)?;
        let Value::Object(object) = &row else {
            return Err(ModelError::Schema("native immutable object"));
        };
        let Some(Value::RecordId(id)) = object.get("id") else {
            return Err(ModelError::Schema("native immutable identity"));
        };
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
        if id.table.as_str() == "original_chunk" {
            let Some(Value::RecordId(blob)) = object.get("blob") else {
                return Err(ModelError::Schema("native original piece parent"));
            };
            references.push((blob.clone(), id.clone()));
        } else if let Some(owner) = owner {
            references.push((owner.clone(), id.clone()));
            let fields: &[&str] = if id.table.as_str() == "original" {
                &["artifact", "blob"]
            } else {
                &["anchor", "in", "out", "unit_node", "source"]
            };
            for field in fields {
                if *field == "unit_node" && !unit_references {
                    continue;
                }
                if let Some(Value::RecordId(target)) = object.get(*field) {
                    references.push((id.clone(), target.clone()));
                }
            }
        }
        let original_blob = if id.table.as_str() == "original" {
            Some(crate::originals::decode_layout(row.clone())?.blob)
        } else {
            None
        };
        rows.push(row);
        let row_count = rows.len();
        let (mut sql, mut bindings) = immutable_row_plan(attempt, contribution, rows)?
            .ok_or(ModelError::Schema("native immutable unit"))?;
        if let Some(blob) = original_blob {
            bindings.insert("original_blobs", vec![blob.value(false)]);
            let mut scope = Variables::new();
            scope.insert(
                "__write_guards",
                vec![guard_id(&Value::RecordId(blob.id()))?],
            );
            scope.insert(
                "__guard_records",
                guard_records(std::iter::once(&blob.id()))?,
            );
            let mut creation = Object::new();
            creation.insert("kind", "original_blob");
            creation.insert("record", blob.id());
            scope.insert("__immutable_creations", vec![Value::Object(creation)]);
            merge_plan_bindings(&mut bindings, scope)?;
            // Declaration and logical layout ownership have one atomic outcome. Completion
            // is mutable, while immutable declaration fields and exact header shape collide.
            sql = format!(
                "FOR $i IN 0..array::len($original_blobs) {{ LET $row=$original_blobs[$i]; LET $blob=$row.id; fn::native_incidence_insert_admitted($row,$__attempt_context); LET $prior=SELECT * FROM ONLY $blob FOR UPDATE; IF $prior=NONE {{ CREATE $blob CONTENT $row RETURN NONE; }} ELSE {{ IF $prior.id!=$row.id OR $prior.content!=$row.content OR $prior.byte_len!=$row.byte_len OR $prior.format!=$row.format OR type::is_bool($prior.completed)=false OR array::len(object::keys($prior))!=5 {{ THROW 'original blob declaration collision'; }}; }}; }}; {sql}"
            );
        }
        if let Some((hold_owner, _, holds)) = hold_plan(attempt, references)? {
            if !matches!(
                hold_owner,
                EffectOwner::OpenAttempt(_) | EffectOwner::Installation
            ) {
                return Err(ModelError::Conflict("native immutable window authority"));
            }
            sql = sql.replace(
                "fn::native_immutable_install($rows,[],$__attempt_context);",
                "fn::native_immutable_install($rows,$holds,$__attempt_context);",
            );
            merge_plan_bindings(&mut bindings, holds)?;
        }
        let bytes = plan_bytes(&sql, &bindings)?;
        if bytes > MAX_ROW_BYTES {
            return Err(ModelError::Limit {
                owner: "native immutable window",
                limit: "expanded unit bytes",
                observed: bytes,
                bound: MAX_ROW_BYTES,
            });
        }
        if bytes > admitted {
            return Err(ModelError::Schema("native immutable construction bound"));
        }
        if let Some(charge) = &mut charge {
            charge.try_resize(bytes)?;
        }
        Ok(Self {
            sql,
            bindings: bindings.into_iter().collect(),
            bytes,
            rows: row_count,
            charges: charge.into_iter().collect(),
        })
    }
    fn coalesce(mut self) -> Result<Self, ModelError> {
        // Units merge their already-admitted plans without rebuilding the growing
        // window. Compact once at emission, while all construction charges remain.
        if let Some(Value::Array(rows)) = self.bindings.remove("rows") {
            self.bindings.insert(
                "rows".into(),
                Value::Array(coalesce_immutable_rows(rows.into_iter().collect())?.into()),
            );
        }
        for key in [
            "__guard_records",
            "__immutable_creations",
            "guards",
            "__write_guards",
            "original_blobs",
            "holds",
            "hold_guards",
        ] {
            if let Some(Value::Array(values)) = self.bindings.get_mut(key) {
                let mut unique = Vec::with_capacity(values.len());
                for value in std::mem::take(values).into_iter() {
                    if !unique.contains(&value) {
                        unique.push(value);
                    }
                }
                *values = unique.into();
            }
        }
        Ok(self)
    }
    fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if self.sql != other.sql {
            return Err(ModelError::Conflict("native immutable window plan"));
        }
        let bytes = self.bytes.saturating_add(other.bytes);
        for (key, value) in other.bindings {
            match (self.bindings.get_mut(&key), value) {
                (Some(Value::Array(left)), Value::Array(right)) => {
                    left.extend(right);
                }
                (Some(left), right) if *left == right => {}
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
    let native = bindings
        .iter()
        .map(|(key, value)| key.len().saturating_add(crate::loader::native_bytes(value)))
        .fold(0usize, usize::saturating_add);
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Counter(0);
    serde_json::to_writer(&mut count, bindings).map_err(ModelError::codec)?;
    let encoded = count.0;
    Ok(native
        .saturating_mul(2)
        .saturating_add(encoded.saturating_mul(3))
        .saturating_add(sql.len().saturating_mul(4))
        .saturating_add(8192))
}
pub type ReservedNativeRow = (Value, Option<Box<dyn Reservation>>);
type NativeRowSource<'a> =
    Box<dyn Iterator<Item = Result<ReservedNativeRow, ModelError>> + Send + 'a>;
pub struct ImmutableWindows<'a> {
    rows: NativeRowSource<'a>,
    pending: Option<ImmutableWindow>,
    attempt: Option<ContentHash>,
    contribution: Option<ContentHash>,
    owner: Option<RecordId>,
    unit_references: bool,
    failed: bool,
    budget: Option<ResourceBudget>,
}
impl<'a> ImmutableWindows<'a> {
    pub fn new(
        rows: Vec<Value>,
        attempt: Option<ContentHash>,
        contribution: Option<ContentHash>,
        owner: Option<RecordId>,
    ) -> Self {
        Self::from_charged_iter(
            rows.into_iter().map(|row| Ok((row, None))),
            attempt,
            contribution,
            owner,
        )
    }
    /// Lazy producers reserve their native representation before copying a row. Both
    /// that reservation and the expanded transaction charge survive acknowledgement.
    pub fn from_charged_iter(
        rows: impl Iterator<Item = Result<ReservedNativeRow, ModelError>> + Send + 'a,
        attempt: Option<ContentHash>,
        contribution: Option<ContentHash>,
        owner: Option<RecordId>,
    ) -> Self {
        Self {
            rows: Box::new(rows),
            pending: None,
            attempt,
            contribution,
            owner,
            unit_references: false,
            failed: false,
            budget: None,
        }
    }
    pub fn with_budget(mut self, budget: Option<&ResourceBudget>) -> Self {
        self.budget = budget.cloned();
        self
    }
    pub fn for_loading(
        rows: Vec<Value>,
        attempt: Option<ContentHash>,
        owner: Option<RecordId>,
    ) -> Self {
        let mut windows = Self::new(rows, attempt, None, owner);
        windows.unit_references = true;
        windows
    }
}
impl Iterator for ImmutableWindows<'_> {
    type Item = Result<ImmutableWindow, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let result = (|| {
            let mut output = self.pending.take();
            for input in self.rows.by_ref() {
                let (row, input_charge) = input?;
                let mut unit = ImmutableWindow::unit(
                    row,
                    self.attempt,
                    self.contribution,
                    self.owner.as_ref(),
                    self.unit_references,
                    self.budget.as_ref(),
                )?;
                unit.charges.extend(input_charge);
                if let Some(window) = &output
                    && (window.sql != unit.sql
                        || window.rows + unit.rows > crate::loader::NATIVE_WINDOW_ROWS
                        || window.bytes.saturating_add(unit.bytes) > TRANSFER_BYTES)
                {
                    self.pending = Some(unit);
                    break;
                }
                match &mut output {
                    Some(window) => window.append(unit)?,
                    None => output = Some(unit),
                }
            }
            output.map(ImmutableWindow::coalesce).transpose()
        })();
        match result {
            Ok(Some(window)) => Some(Ok(window)),
            Ok(None) => None,
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}
pub async fn install_window(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    window: ImmutableWindow,
    budget: Option<&ResourceBudget>,
) -> Result<(), ModelError> {
    let _charge: Option<Box<dyn Reservation>> = if window.charges.is_empty() {
        budget
            .map(|budget| budget.reserve("native immutable window", window.bytes))
            .transpose()?
    } else {
        None
    };
    effect(client, attempt, &window.sql, window.bindings.into()).await
}

/// A frozen derivation can only retain derived children and exact existing canonical
/// external endpoints. It cannot introduce a canonical payload after native freeze.
pub(super) async fn install_staging_window(
    client: &Surreal<Client>,
    staging: &staging::DerivedStaging,
    mut window: ImmutableWindow,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let _charge = if window.charges.is_empty() {
        Some(budget.reserve("publication derived immutable window", window.bytes)?)
    } else {
        None
    };
    let Some(Value::Array(rows)) = window.bindings.get("rows") else {
        return Err(ModelError::Schema("publication immutable rows"));
    };
    for row in rows.iter() {
        let Value::Object(row) = row else {
            return Err(ModelError::Schema("publication derived object"));
        };
        let Some(Value::RecordId(id)) = row.get("id") else {
            return Err(ModelError::Schema("publication derived address"));
        };
        if crate::derivation::FAMILIES.contains(&id.table.as_str()) {
            crate::derivation::require_namespace(row, staging.derivation())?;
        } else if !matches!(
            id.table.as_str(),
            "external" | "entity_anchor" | "assertion_anchor"
        ) {
            return Err(ModelError::Conflict("publication derived immutable family"));
        }
    }
    // The ensuing immutable write and object-incarnation guard share this transaction.
    // Thus canonical endpoint comparison cannot race a retirement/replacement.
    let _guard_charge =
        budget.reserve("publication canonical endpoint guard construction", 8192)?;
    window.sql = format!(
        "FOR $row IN $rows {{ IF record::table($row.id) IN ['external','entity_anchor','assertion_anchor'] {{ LET $canonical_id=$row.id; LET $canonical=SELECT * FROM ONLY $canonical_id FOR UPDATE; IF $canonical=NONE OR $canonical!=$row {{ THROW 'frozen canonical endpoint missing or changed'; }}; }}; }}; {}",
        window.sql
    );
    staging
        .effect(client, &window.sql, window.bindings.into(), budget)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn immutable_control_row(name: &str, size: i64) -> Value {
        let mut target = lctx_model::domain::graph::Target::External {
            provider: lctx_model::domain::graph::EntityId(ContentHash::of(
                b"immutable fixture provider",
            )),
            context: lctx_model::domain::graph::EntityId(ContentHash::of(
                b"immutable fixture context",
            )),
            name: format!("{name}:0"),
            reason: lctx_model::domain::normalized::entities::EntityReason::ProviderExternal,
        };
        let id = crate::reader::target_id(target.clone());
        // The late collision deliberately substitutes different full canonical bytes at
        // an occupied address. The initially installed endpoint has its exact model ID.
        if let lctx_model::domain::graph::Target::External { name: label, .. } = &mut target {
            *label = format!("{name}:{size}");
        }
        let mut row = Object::new();
        row.insert("id", id);
        row.insert(
            "canonical",
            surrealdb::types::Bytes::from(serde_json::to_vec(&target).unwrap()),
        );
        Value::Object(row)
    }
    fn exact_control_recovery(
        client: Arc<Surreal<Client>>,
        label: &'static str,
    ) -> (Arc<RecoveryScope>, crate::signal::SignalRegistration) {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("owned native control runtime"),
        ))
        .unwrap();
        let stop = RetryCancellation::default();
        let cancel = stop.clone();
        let signals = config.signal_owner(label, move || cancel.cancel()).unwrap();
        let scope = RecoveryScope::new(
            client,
            ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap(),
            stop,
            signals.pending_recipient().unwrap(),
            RecoveryTarget {
                generation: config.service_generation,
                namespace: config.namespace.as_str().into(),
                database: config.database.as_str().into(),
                endpoint: Some(config.endpoint.clone()),
            },
        )
        .unwrap();
        (scope, signals)
    }
    #[test]
    fn windows_account_for_guards_and_holds_and_bound_complete_units() {
        let attempt = ContentHash::of(b"window owner");
        let owner = RecordId::new("native_attempt", attempt.hex());
        let rows = (0..300)
            .map(|n| immutable_control_row(&format!("item-{n}"), 0))
            .collect();
        let windows = ImmutableWindows::new(rows, Some(attempt), None, Some(owner))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(windows.len() > 1);
        assert_eq!(windows.iter().map(|window| window.rows).sum::<usize>(), 300);
        for window in windows {
            assert!(window.rows <= crate::loader::NATIVE_WINDOW_ROWS);
            assert!(window.bytes <= TRANSFER_BYTES);
            let Some(Value::Array(rows)) = window.bindings.get("rows") else {
                panic!("rows");
            };
            let Some(Value::Array(holds)) = window.bindings.get("holds") else {
                panic!("holds");
            };
            assert_eq!(rows.len(), holds.len());
            assert!(window.bytes > serde_json::to_vec(&window.bindings).unwrap().len());
        }
    }
    #[test]
    fn grouped_window_coalesces_equal_rows_holds_and_guards_and_refuses_unequal() {
        let attempt = ContentHash::of(b"grouped window control");
        let owner = RecordId::new("native_attempt", attempt.hex());
        let row = immutable_control_row("equal-window", 0);
        let window = ImmutableWindows::new(
            vec![row.clone(), row.clone()],
            Some(attempt),
            None,
            Some(owner.clone()),
        )
        .next()
        .unwrap()
        .unwrap();
        for key in ["rows", "holds", "guards", "__immutable_creations"] {
            let Some(Value::Array(values)) = window.bindings.get(key) else {
                panic!("{key}")
            };
            assert_eq!(values.len(), 1, "exact duplicate {key} coalesces");
        }
        assert_eq!(
            window
                .sql
                .matches("fn::native_immutable_install($rows,$holds,$__attempt_context)")
                .count(),
            1
        );
        assert!(!window.sql.contains("UPSERT"));
        assert!(matches!(
            ImmutableWindows::new(
                vec![row, immutable_control_row("equal-window", 1)],
                Some(attempt),
                None,
                Some(owner)
            )
            .next()
            .unwrap(),
            Err(ModelError::Conflict("native immutable address collision"))
        ));
    }
    #[test]
    fn grouped_immutable_and_hold_sql_parses_with_actual_schema() {
        surrealdb_syn::parse(include_str!("schema.surql")).unwrap();
        let (sql, _) =
            immutable_row_plan(None, None, vec![immutable_control_row("parse-group", 0)])
                .unwrap()
                .unwrap();
        surrealdb_syn::parse(&format!("BEGIN; {sql} COMMIT;")).unwrap();
        let owner = RecordId::new("native_attempt", "parse-owner");
        let target = RecordId::new("external", "parse-target");
        let (_, sql, variables) =
            hold_plan(None, vec![(owner.clone(), target.clone()), (owner, target)])
                .unwrap()
                .unwrap();
        surrealdb_syn::parse(&format!("BEGIN; {sql} COMMIT;")).unwrap();
        let Some(Value::Array(holds)) = variables.get("holds") else {
            panic!("holds")
        };
        assert_eq!(holds.len(), 1);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_grouped_window_mixed_rows_relations_and_repeated_blob_parent() {
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned RocksDB campaign"),
        ))
        .unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let local_config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").unwrap(),
        ))
        .unwrap();
        let stop = RetryCancellation::default();
        let cancel = stop.clone();
        let signals = local_config
            .signal_owner("grouped immutable control writer", move || cancel.cancel())
            .unwrap();
        let scope = RecoveryScope::new(
            client.clone(),
            ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap(),
            stop,
            signals.pending_recipient().unwrap(),
            RecoveryTarget {
                generation: local_config.service_generation,
                namespace: local_config.namespace.as_str().into(),
                database: local_config.database.as_str().into(),
                endpoint: Some(local_config.endpoint.clone()),
            },
        )
        .unwrap();
        let nonce = store.attempt().hex();
        let owner = RecordId::new("native_attempt", nonce.clone());
        let external = immutable_control_row(&format!("{nonce}-mixed"), 0);
        let external_id = external.as_object().unwrap().get("id").unwrap().clone();
        let anchor = RecordId::new("entity_anchor", format!("{nonce}-entity"));
        let entity = RecordId::new("entity", format!("{nonce}-entity"));
        let mut row = Object::new();
        row.insert("id", entity.clone());
        row.insert("anchor", anchor);
        row.insert("semantic_type", "grouped-control");
        row.insert("semantic_key", nonce.clone());
        row.insert("kind", 0i64);
        row.insert("subtype", Value::Null);
        row.insert("content", nonce.clone());
        row.insert("canonical", surrealdb::types::Bytes::from(Vec::<u8>::new()));
        row.insert("body", Object::new());
        row.insert("scope_keys", Vec::<String>::new());
        let mut relation = Object::new();
        relation.insert("id", RecordId::new("reference", format!("{nonce}-edge")));
        relation.insert("in", entity);
        relation.insert("out", external_id);
        relation.insert("derivation", "grouped-control");
        relation.insert("field", "control");
        relation.insert("role", 0i64);
        relation.insert("position", Value::Null);
        let result: Result<(), ModelError> = with_recovery_scope(scope.clone(), async {
            let rows = vec![
                Value::Object(row),
                external.clone(),
                Value::Object(relation.clone()),
                external,
            ];
            let mut windows = ImmutableWindows::new(rows, Some(store.attempt()), None, Some(owner));
            let window = windows.next().unwrap()?;
            assert!(windows.next().is_none());
            install_window(&client, Some(store.attempt()), window, None).await?;
            let edge = relation.get("id").unwrap().clone();
            let mut response = client
                .query("SELECT * FROM $edge")
                .bind(("edge", edge))
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let actual: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
            assert_eq!(actual, vec![Value::Object(relation)]);
            let bytes = vec![17u8; 2 * crate::originals::CHUNK_BYTES];
            let blob = crate::originals::Blob::new(ContentHash::of(&bytes), bytes.len() as u64)?;
            ensure_rows(&client, Some(store.attempt()), vec![blob.value(false)]).await?;
            let pieces = vec![
                blob.piece(0, &bytes[..crate::originals::CHUNK_BYTES])?,
                blob.piece(
                    crate::originals::CHUNK_BYTES as u64,
                    &bytes[crate::originals::CHUNK_BYTES..],
                )?,
            ];
            let mut windows =
                ImmutableWindows::new(pieces.clone(), Some(store.attempt()), None, None);
            let window = windows.next().unwrap()?;
            assert!(
                windows.next().is_none(),
                "both valid pieces share one parent preparation"
            );
            install_window(&client, Some(store.attempt()), window, None).await?;
            let mut response = client
                .query("SELECT VALUE revision FROM ONLY type::record('native_predicate',[$blob])")
                .bind(("blob", blob.id()))
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let revision: Option<i64> = response.take(0).map_err(ModelError::codec)?;
            assert_eq!(
                revision,
                Some(2),
                "one declaration and one grouped child/hold incidence"
            );
            let mut vars = Variables::new();
            vars.insert("blob", blob.id());
            guarded_effect(
                &client,
                Some(store.attempt()),
                vec![blob.id()],
                "UPDATE $blob SET completed=true RETURN NONE",
                vars,
            )
            .await?;
            ensure_rows(&client, Some(store.attempt()), pieces).await?;
            let mut missing = blob
                .piece(0, &bytes[..crate::originals::CHUNK_BYTES])?
                .as_object()
                .unwrap()
                .clone();
            missing.insert(
                "id",
                RecordId::new("original_chunk", format!("{nonce}-forbidden")),
            );
            let error = ensure_rows(&client, Some(store.attempt()), vec![Value::Object(missing)])
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("completed original blob refuses missing or changed piece"),
                "{error}"
            );
            Ok(())
        })
        .await;
        assert_eq!(
            scope.pending_count().unwrap(),
            0,
            "unsettled recovery protects this exact test client"
        );
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "grouped control session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        completion.step("grouped control owner abandon", store.abandon().await);
        let finality = signals.complete(&completion).await;
        completion.step("grouped control signal owner", finality);
        lctx_model::domain::completion::complete(result, completion).unwrap();
    }
    fn control_address(row: &Value) -> RecordId {
        let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
            panic!("control row address")
        };
        id.clone()
    }
    async fn control_guard(client: &Surreal<Client>, record: &RecordId) -> Object {
        let guard = guard_id(&Value::RecordId(record.clone())).unwrap();
        let mut response = client
            .query("SELECT * FROM $guard")
            .bind(("guard", guard))
            .await
            .unwrap()
            .check()
            .unwrap();
        let rows: Vec<Object> = response.take(0).unwrap();
        assert_eq!(rows.len(), 1);
        rows.into_iter().next().unwrap()
    }
    async fn control_packet(
        client: &Surreal<Client>,
        attempt: ContentHash,
        sql: &str,
        mut bindings: Variables,
        avoid: Option<i64>,
    ) -> (NativeRequest, authorization::OwnerEffectPlan) {
        coordination::prepare_expectations(client, &mut bindings)
            .await
            .unwrap();
        let digest = ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).unwrap());
        let mut request = NativeRequest::issue(client, digest).await.unwrap();
        while avoid == Some(coordination::attempt_slot(request.operation)) {
            request.operation = fresh_identity("immutable-overlap-distinct-slot").unwrap();
        }
        let plan = authorization::owner_effect_plan(
            request,
            &EffectOwner::OpenAttempt(attempt),
            sql,
            bindings,
            false,
        )
        .unwrap();
        (request, plan)
    }
    // Explicit transactions establish both payload snapshots before either commit.
    // There is no wall-clock threshold: a shared attempt guard write makes one commit
    // fail, while independent participant stripes permit both complete row/hold effects.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB protocol campaign: direct commit-order packets also touch installation admission stripes"]
    async fn native_attempt_immutable_rows_and_hold_only_windows_overlap_both_commit_orders() {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("native control runtime"),
        ))
        .unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let (scope, signals) =
            exact_control_recovery(client.clone(), "immutable actual overlap control");
        with_recovery_scope(scope.clone(),async {
            let era=IssuanceEra::capture(&client).await.unwrap();
            for hold_only in [false,true] {
                for reverse in [false,true] {
                    let attempt=fresh_identity("immutable-overlap-attempt").unwrap();
                    authorization::begin_attempt_in(&client,attempt,era).await.unwrap();
                    let owner=RecordId::new("native_attempt",attempt.hex());
                    let rows=[immutable_control_row(&format!("{}-left",attempt.hex()),0),immutable_control_row(&format!("{}-right",attempt.hex()),0)];
                    let ids=rows.iter().map(control_address).collect::<Vec<_>>();
                    if hold_only { ensure_rows(&client,Some(attempt),rows.to_vec()).await.unwrap(); }
                    let before=control_guard(&client,&owner).await;
                    let target_before=if hold_only { Some([control_guard(&client,&ids[0]).await,control_guard(&client,&ids[1]).await]) } else { None };
                    let mut packets=Vec::new();
                    for (index,row) in rows.iter().enumerate() {
                        let (sql,bindings)=if hold_only {
                            let (_,sql,bindings)=hold_plan(Some(attempt),vec![(owner.clone(),ids[index].clone())]).unwrap().unwrap();
                            (sql,bindings)
                        } else {
                            let window=ImmutableWindows::new(vec![row.clone()],Some(attempt),None,Some(owner.clone())).next().unwrap().unwrap();
                            (window.sql,window.bindings.into())
                        };
                        let avoid=packets.first().map(|(request,_):&(NativeRequest,authorization::OwnerEffectPlan)|coordination::attempt_slot(request.operation));
                        packets.push(control_packet(&client,attempt,&sql,bindings,avoid).await);
                    }
                    for (_,plan) in &packets { checked_transaction(client.query(plan.intent.clone()).bind(plan.intent_bindings.clone()).await.unwrap()).unwrap(); }
                    assert_eq!(control_guard(&client,&owner).await,before,"intent participation leaves attempt physical guard unchanged");
                    let body=|query:&str|query.strip_prefix("BEGIN;").unwrap().strip_suffix("COMMIT;").unwrap().to_owned();
                    let left=client.as_ref().clone().begin().await.unwrap();
                    let right=client.as_ref().clone().begin().await.unwrap();
                    checked_transaction(left.query(body(&packets[0].1.query)).bind(packets[0].1.bindings.clone()).await.unwrap()).unwrap();
                    checked_transaction(right.query(body(&packets[1].1.query)).bind(packets[1].1.bindings.clone()).await.unwrap()).unwrap();
                    if reverse {
                        right.commit().await.unwrap().invalidate().await.unwrap();
                        left.commit().await.unwrap().invalidate().await.unwrap();
                    } else {
                        left.commit().await.unwrap().invalidate().await.unwrap();
                        right.commit().await.unwrap().invalidate().await.unwrap();
                    }
                    assert_eq!(control_guard(&client,&owner).await,before,"ordinary same-attempt row/hold payloads do not serialize on its physical guard");
                    for (index,id) in ids.iter().enumerate() {
                        let mut response=client.query("SELECT * FROM $record; SELECT * FROM $hold").bind(("record",id.clone())).bind(("hold",hold_record_id(&owner,id).unwrap())).await.unwrap().check().unwrap();
                        let actual:Vec<Value>=response.take(0).unwrap(); let holds:Vec<Object>=response.take(1).unwrap();
                        assert_eq!(actual,vec![rows[index].clone()]); assert_eq!(holds.len(),1);
                        assert_eq!(holds[0].get("owner"),Some(&Value::RecordId(owner.clone())));
                        assert_eq!(holds[0].get("object"),Some(&Value::RecordId(id.clone())));
                        if let Some(target_before)=&target_before {
                            let after=control_guard(&client,id).await;
                            assert_eq!(authorization::int_field(&after,"revision").unwrap(),authorization::int_field(&target_before[index],"revision").unwrap()+1,"non-attempt hold endpoint still advances exact guard revision");
                            assert_eq!(after.get("incarnation"),target_before[index].get("incarnation"));
                        }
                    }
                    for (request,_) in packets { super::super::history::release_outcome(&client,request,&request.operation.hex(),"recovery").await.unwrap(); }
                    close_attempt(&client,attempt,"abandoned").await.unwrap();
                }
            }
        }).await;
        assert_eq!(scope.pending_count().unwrap(), 0);
        client.invalidate().await.unwrap();
        signals
            .complete(&lctx_model::domain::completion::Completion::default())
            .await
            .unwrap();
    }
    // These exact logical attempts are intentionally corrupted. The owning disposable
    // campaign must discard them after native/session drainage; never select on shared
    // validation or repair a damaged incarnation backwards merely for test cleanup.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_attempt_immutable_guard_refusals_leave_rows_and_holds_absent() {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let (scope, signals) =
            exact_control_recovery(client.clone(), "immutable guard refusal control");
        with_recovery_scope(scope.clone(),async {
            let era=IssuanceEra::capture(&client).await.unwrap();
            for damage in ["missing","retiring","incarnation","stale_owner"] {
                let attempt=fresh_identity("immutable-guard-refusal").unwrap();
                authorization::begin_attempt_in(&client,attempt,era).await.unwrap();
                let owner=RecordId::new("native_attempt",attempt.hex());
                let row=immutable_control_row(&format!("{}-{damage}",attempt.hex()),0); let id=control_address(&row);
                let mut window=ImmutableWindows::new(vec![row],Some(attempt),None,Some(owner.clone())).next().unwrap().unwrap();
                let mut bindings:Variables=window.bindings.into(); coordination::prepare_expectations(&client,&mut bindings).await.unwrap(); window.bindings=bindings.into_iter().collect();
                let guard=guard_id(&Value::RecordId(owner.clone())).unwrap();
                let mutation=match damage {
                    "missing"=>"DELETE $guard RETURN NONE;",
                    "retiring"=>"UPDATE $guard SET phase='retiring' RETURN NONE;",
                    "incarnation"=>"UPDATE $guard SET incarnation+=1 RETURN NONE;",
                    "stale_owner"=>"fn::native_attempt_cut($owner,$generation,$era,1,'close'); UPDATE $owner SET state='closing',revision+=1 RETURN NONE;",
                    _=>unreachable!(),
                };
                client.query(format!("BEGIN; {mutation} COMMIT;")).bind(("guard",guard)).bind(("owner",owner.clone())).bind(("generation",era.generation.hex())).bind(("era",era.era)).await.unwrap().check().unwrap();
                let error=install_window(&client,Some(attempt),window,None).await.unwrap_err();
                assert!(error.to_string().contains(if damage=="stale_owner" {"native owner fenced"} else {"native expected active incarnation changed"}),"{damage}: {error}");
                assert!(error.permits_storage_cleanup(),"acknowledged refusal must settle exact request: {error}");
                let mut response=client.query("SELECT * FROM $record; SELECT * FROM $hold").bind(("record",id.clone())).bind(("hold",hold_record_id(&owner,&id).unwrap())).await.unwrap().check().unwrap();
                let rows:Vec<Value>=response.take(0).unwrap(); let holds:Vec<Value>=response.take(1).unwrap();
                assert!(rows.is_empty() && holds.is_empty(),"{damage} refuses before rows/holds commit");
            }
        }).await;
        assert_eq!(scope.pending_count().unwrap(), 0);
        client.invalidate().await.unwrap();
        signals
            .complete(&lctx_model::domain::completion::Completion::default())
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn budget_refusal_precedes_native_effect_and_releases_reservation() {
        let attempt = ContentHash::of(b"window owner");
        let window = ImmutableWindows::new(
            vec![immutable_control_row("budget-refusal", 0)],
            Some(attempt),
            None,
            Some(RecordId::new("native_attempt", attempt.hex())),
        )
        .next()
        .unwrap()
        .unwrap();
        let budget = ResourceBudget::fixed(window.bytes - 1).unwrap();
        assert!(matches!(
            install_window(&Surreal::init(), Some(attempt), window, Some(&budget)).await,
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn construction_is_admitted_and_lookahead_stays_charged_until_drop() {
        let budget = ResourceBudget::fixed(4 * 1024 * 1024).unwrap();
        let rows = (0..300)
            .map(|n| immutable_control_row(&format!("charged-{n}"), 0))
            .collect();
        let mut windows = ImmutableWindows::new(rows, None, None, None).with_budget(Some(&budget));
        let first = windows.next().unwrap().unwrap();
        assert!(
            budget.reserved() > first.bytes,
            "one pending complete unit remains charged"
        );
        drop(first);
        assert!(budget.reserved() > 0);
        drop(windows);
        assert_eq!(budget.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            ImmutableWindows::new(vec![immutable_control_row("refuse", 0)], None, None, None)
                .with_budget(Some(&tiny))
                .next()
                .unwrap(),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn frozen_immutable_creation_joins_only_equal_planned_successor_and_keeps_holds_atomic() {
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"),
        ))
        .unwrap();
        let first = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let second = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            first.namespace().as_str(),
            first.database().as_str(),
        )
        .await
        .unwrap();
        let (scope, signals) = exact_control_recovery(client.clone(), "immutable creation control");
        async fn freeze(
            client: &Surreal<Client>,
            mut window: ImmutableWindow,
        ) -> Result<ImmutableWindow, ModelError> {
            let mut bindings: Variables = window.bindings.into();
            coordination::prepare_expectations(client, &mut bindings).await?;
            window.bindings = bindings.into_iter().collect();
            Ok(window)
        }
        fn address(value: &Value) -> RecordId {
            let Some(Value::RecordId(id)) = value.as_object().and_then(|row| row.get("id")) else {
                panic!("control record")
            };
            id.clone()
        }
        let owner1 = RecordId::new("native_attempt", first.attempt().hex());
        let owner2 = RecordId::new("native_attempt", second.attempt().hex());
        let result:Result<(),ModelError>=with_recovery_scope(scope.clone(),async {
            for retired in [false,true] {
                let row=immutable_control_row(&format!("{}-equal-{retired}",first.attempt().hex()),0);let record=address(&row);
                if retired {ensure_rows(&client,None,vec![row.clone()]).await?;retire(&client,record.clone()).await?;}
                // Both requests capture the SAME vacant/tombstoned boundary before a peer
                // activates it. This deterministically exposes the real planning race.
                let a=freeze(&client,ImmutableWindows::new(vec![row.clone()],Some(first.attempt()),None,Some(owner1.clone())).next().unwrap()?).await?;
                let b=freeze(&client,ImmutableWindows::new(vec![row.clone()],Some(second.attempt()),None,Some(owner2.clone())).next().unwrap()?).await?;
                let digest=ContentHash::of(&serde_json::to_vec(&(&a.sql,&a.bindings)).map_err(ModelError::codec)?);
                install_window(&client,Some(second.attempt()),b,None).await?;
                assert_eq!(digest,ContentHash::of(&serde_json::to_vec(&(&a.sql,&a.bindings)).map_err(ModelError::codec)?),"peer execution never refreshes the original request");
                install_window(&client,Some(first.attempt()),a,None).await?;
                let mut response=client.query("SELECT * FROM $record; SELECT owner FROM native_hold WITH INDEX object_holds WHERE object=$record; SELECT incarnation,tombstone_revision FROM $guard")
                    .bind(("record",record.clone())).bind(("guard",guard_id(&Value::RecordId(record))?)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;let holds:Vec<Object>=response.take(1).map_err(ModelError::codec)?;let guards:Vec<Object>=response.take(2).map_err(ModelError::codec)?;
                assert_eq!(rows,vec![row]);assert_eq!(holds.len(),2);assert!(holds.iter().any(|row|row.get("owner")==Some(&Value::RecordId(owner1.clone()))));assert!(holds.iter().any(|row|row.get("owner")==Some(&Value::RecordId(owner2.clone()))));
                assert_eq!(guards[0].get("incarnation"),Some(&test_value(if retired {2i64}else{1i64})));assert_eq!(guards[0].get("tombstone_revision"),Some(&test_value(if retired {1i64}else{0i64})));
            }
            let name=format!("{}-collision",first.attempt().hex());let row=immutable_control_row(&name,0);let absent=immutable_control_row(&format!("{}-rollback",first.attempt().hex()),0);
            let a=freeze(&client,ImmutableWindows::new(vec![absent.clone(),immutable_control_row(&name,1)],Some(first.attempt()),None,Some(owner1.clone())).next().unwrap()?).await?;
            let record=address(&row);ensure_rows(&client,Some(second.attempt()),vec![row]).await?;hold(&client,Some(second.attempt()),owner2.clone(),vec![record]).await?;
            let error=install_window(&client,Some(first.attempt()),a,None).await.unwrap_err();assert!(error.to_string().contains("native immutable address collision"),"{error}");assert!(error.permits_storage_cleanup(),"collision is acknowledged and reconciled: {error}");
            let mut response=client.query("SELECT * FROM $record; SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$record").bind(("record",address(&absent))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;let holds:Vec<RecordId>=response.take(1).map_err(ModelError::codec)?;assert!(rows.is_empty() && holds.is_empty(),"late comparison rolls back preceding guard/row/hold work");
            let row=immutable_control_row(&format!("{}-later-incarnation",first.attempt().hex()),0);let record=address(&row);
            let a=freeze(&client,ImmutableWindows::new(vec![row.clone()],Some(first.attempt()),None,Some(owner1)).next().unwrap()?).await?;
            ensure_rows(&client,None,vec![row.clone()]).await?;retire(&client,record.clone()).await?;ensure_rows(&client,None,vec![row]).await?;hold(&client,Some(second.attempt()),owner2.clone(),vec![record]).await?;
            let error=install_window(&client,Some(first.attempt()),a,None).await.unwrap_err();assert!(error.to_string().contains("native expected vacant guard changed"),"equal bytes at a later incarnation remain fenced: {error}");assert!(error.permits_storage_cleanup(),"{error}");
            let row=immutable_control_row(&format!("{}-strict-generic",first.attempt().hex()),0);let record=address(&row);let mut vars=Variables::new();vars.insert("record",record.clone());
            let sql=guarded_effect_plan(vec![record.clone()],"UPDATE $record SET forbidden=true RETURN NONE",&mut vars)?;coordination::prepare_expectations(&client,&mut vars).await?;
            ensure_rows(&client,None,vec![row.clone()]).await?;hold(&client,Some(second.attempt()),owner2.clone(),vec![record.clone()]).await?;
            let error=effect(&client,None,&sql,vars).await.unwrap_err();assert!(error.to_string().contains("native expected vacant guard changed"),"generic mutable writes cannot adopt immutable alternatives: {error}");assert!(error.permits_storage_cleanup(),"{error}");
            let mut response=client.query("SELECT * FROM $record").bind(("record",record)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;assert_eq!(rows,vec![row],"refused mutable request has no payload effect");
            Ok(())
        }).await;
        assert_eq!(
            scope.pending_count().unwrap(),
            0,
            "unsettled recovery retains the exact creation-control session"
        );
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "creation control session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        completion.step("first creation owner abandon", first.abandon().await);
        completion.step("second creation owner abandon", second.abandon().await);
        let finality = signals.complete(&completion).await;
        completion.step("creation control signal owner", finality);
        lctx_model::domain::completion::complete(result, completion).unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_late_collision_rolls_back_rows_and_holds_together() {
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign fixture"),
        ))
        .unwrap();
        let store = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let (scope, signals) =
            exact_control_recovery(client.clone(), "immutable late collision control");
        with_recovery_scope(scope.clone(), async {
            let attempt = store.attempt();
            let owner = RecordId::new("native_attempt", attempt.hex());
            let existing = format!("{}-existing", attempt.hex());
            let absent = format!("{}-absent", attempt.hex());
            let first = ImmutableWindows::new(
                vec![immutable_control_row(&existing, 0)],
                Some(attempt),
                None,
                Some(owner.clone()),
            )
            .next()
            .unwrap()
            .unwrap();
            install_window(&client, Some(attempt), first, None)
                .await
                .unwrap();
            let mut windows = ImmutableWindows::new(
                vec![
                    immutable_control_row(&absent, 0),
                    immutable_control_row(&existing, 1),
                ],
                Some(attempt),
                None,
                Some(owner),
            );
            let collision = windows.next().unwrap().unwrap();
            assert!(windows.next().is_none());
            let result = install_window(&client, Some(attempt), collision, None).await;
            assert!(result.is_err(), "full-value collision must refuse");
            let mut response = client
                .query("SELECT * FROM $absent; SELECT * FROM native_hold WHERE object=$absent")
                .bind((
                    "absent",
                    match immutable_control_row(&absent, 0) {
                        Value::Object(row) => row.get("id").unwrap().clone(),
                        _ => unreachable!(),
                    },
                ))
                .await
                .unwrap()
                .check()
                .unwrap();
            let rows: Vec<Value> = response.take(0).unwrap();
            let holds: Vec<Value> = response.take(1).unwrap();
            assert!(rows.is_empty());
            assert!(holds.is_empty());
        })
        .await;
        assert_eq!(
            scope.pending_count().unwrap(),
            0,
            "unsettled recovery retains the exact late-collision session"
        );
        client.invalidate().await.unwrap();
        store.abandon().await.unwrap();
        signals
            .complete(&lctx_model::domain::completion::Completion::default())
            .await
            .unwrap();
    }
}
