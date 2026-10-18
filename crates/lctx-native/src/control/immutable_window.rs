//! Reachability-bounded immutable installation: rows and their holds share one outcome.
use super::*;
use lctx_model::domain::resources::{MAX_ROW_BYTES, Reservation, ResourceBudget, TRANSFER_BYTES};
use lctx_tracing::InContext;

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
        let admission = if charge.is_some() {
            CoalescingAdmission::ExistingConstruction
        } else {
            CoalescingAdmission::ActiveRecovery
        };
        let (mut sql, mut bindings) =
            immutable_row_plan_in(attempt, contribution, rows, admission)?
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
            // The native window classifies declaration, layout and holds together.
            // Existing blob completion is mutable; immutable header equality remains strict.
        }
        if let Some((hold_owner, _, holds)) = hold_plan_in(attempt, references, admission)? {
            if !matches!(
                hold_owner,
                EffectOwner::OpenAttempt(_) | EffectOwner::Installation
            ) {
                return Err(ModelError::Conflict("native immutable window authority"));
            }
            sql = sql.replace(
                "fn::native_immutable_install($rows,[],$__attempt_context,$__deferred_guards,($original_blobs ?? []));",
                "fn::native_immutable_install($rows,$holds,$__attempt_context,$__deferred_guards,($original_blobs ?? []));",
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
    fn coalesce(mut self, budget: Option<&ResourceBudget>) -> Result<Self, ModelError> {
        // Reserve all candidate backing before its first allocation. Existing unit
        // charges retain their input bodies until compaction and acknowledgement.
        let index_bytes = self
            .bindings
            .values()
            .filter_map(|value| {
                if let Value::Array(values) = value {
                    Some(values)
                } else {
                    None
                }
            })
            .flatten()
            .map(exact_index::entry_workspace)
            .fold(0usize, usize::saturating_add);
        let scope = capture_recovery_scope();
        let budget = budget.or_else(|| scope.as_ref().map(|scope| scope.budget()));
        let _index_charge = budget
            .map(|budget| budget.reserve("native immutable exact coalescing", index_bytes))
            .transpose()?;
        // Units merge their already-admitted plans without rebuilding the growing
        // window. Compact once at emission, while all construction charges remain.
        if let Some(Value::Array(rows)) = self.bindings.remove("rows") {
            self.bindings.insert(
                "rows".into(),
                Value::Array(
                    coalesce_immutable_rows_in(
                        rows.into_iter().collect(),
                        CoalescingAdmission::ExistingConstruction,
                    )?
                    .into(),
                ),
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
                let field = match key {
                    "__guard_records" => Some("guard"),
                    "__immutable_creations" => Some("record"),
                    "original_blobs" | "holds" => Some("id"),
                    _ => None,
                };
                let unique =
                    exact_index::stable_unique(std::mem::take(values).into_iter().collect(), field);
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
    // Full native containers, serialization, request hashing and atomic controls
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
            output
                .map(|window| window.coalesce(self.budget.as_ref()))
                .transpose()
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
/// Invocation-local immutable writer. Only the owner's original issuance is reused;
/// every window captures its own physical expectations and checks current eligibility.
pub struct WindowWriter<'a> {
    client: &'a Surreal<Client>,
    budget: Option<&'a ResourceBudget>,
    owner: EffectOwner,
    issuance: Option<IssuanceEra>,
    control_bytes: Option<usize>,
    operation:Option<Arc<operation_pages::OperationWriter>>,
}
impl<'a> WindowWriter<'a> {
    pub fn new(
        client: &'a Surreal<Client>,
        attempt: Option<ContentHash>,
        budget: Option<&'a ResourceBudget>,
    ) -> Self {
        Self::for_owner(
            client,
            attempt
                .map(EffectOwner::OpenAttempt)
                .unwrap_or(EffectOwner::Installation),
            budget,
        )
    }
    pub(super) fn for_owner(
        client: &'a Surreal<Client>,
        owner: EffectOwner,
        budget: Option<&'a ResourceBudget>,
    ) -> Self {
        Self {
            client,
            budget,
            owner,
            issuance: None,
            control_bytes: None,
            operation:None,
        }
    }
    /// Attach the durable root of the actual producer; windows remain native-only pages.
    pub fn with_operation(mut self,operation:Arc<operation_pages::OperationWriter>)->Self{
        self.operation=Some(operation);self
    }
    pub(super) fn set_operation(&mut self,operation:Arc<operation_pages::OperationWriter>){self.operation=Some(operation);self.control_bytes=None;}
    /// Retain exact physical endpoints through the same producer operation root.
    pub async fn hold_many(&mut self,references:Vec<(RecordId,RecordId)>)->Result<(),ModelError>{
        if let Some((_,sql,bindings))=hold_plan_in(None,references,CoalescingAdmission::ExistingConstruction)?{self.effect(&sql,bindings).await?;}Ok(())
    }
    /// Acquire a bounded set using its independently observed full bytes and original
    /// incarnation. Ordinary immutable-hold lowering still supplies the scalar locks;
    /// the set observation cannot silently refresh an intervening recreation.
    pub async fn hold_expected_window(&mut self, owner:RecordId, expected:Vec<ExpectedRecord>)->Result<(),ModelError>{
        if expected.is_empty(){return Ok(());}
        if expected.len()>crate::loader::NATIVE_WINDOW_ROWS{return Err(ModelError::Schema("expected hold window bound"));}
        let _metadata=self.budget.map(|budget|budget.reserve("expected hold window metadata",expected.len().saturating_mul(2048).saturating_add(4096))).transpose()?;
        let mut references=Vec::with_capacity(expected.len());
        let mut rows=Vec::with_capacity(expected.len());
        let mut charges=Vec::with_capacity(expected.len());
        for value in expected{
            let record=value.record().clone();
            let mut row=Object::new();row.insert("record",record.clone());
            row.insert("guard",guard_id(&Value::RecordId(record.clone()))?);
            row.insert("incarnation",value.incarnation());
            let (bytes,charge)=value.into_owned_row();row.insert("bytes",bytes);
            rows.push(Value::Object(row));charges.push(charge);references.push((owner.clone(),record));
        }
        let (_,hold_sql,mut bindings)=hold_plan_in(None,references,CoalescingAdmission::ExistingConstruction)?.ok_or(ModelError::Schema("expected hold window plan"))?;
        bindings.insert("captured_expected_rows",rows);
        let sql=format!("LET $captured_ids=$captured_expected_rows.map(|$row|$row.record); LET $captured_guard_ids=$captured_expected_rows.map(|$row|$row.guard); LET $captured_actual=SELECT * FROM $captured_ids; LET $captured_guards=SELECT id,phase,incarnation FROM $captured_guard_ids; LET $captured_actual_by_id=object::from_entries($captured_actual.map(|$row|[<string>$row.id,$row])); LET $captured_guards_by_id=object::from_entries($captured_guards.map(|$row|[<string>$row.id,$row])); FOR $expected IN $captured_expected_rows {{ LET $guard=$captured_guards_by_id[<string>$expected.guard]; IF $guard.phase!='active' OR $guard.incarnation!=$expected.incarnation OR $captured_actual_by_id[<string>$expected.record]!=$expected.bytes {{ THROW 'native expected window incarnation or bytes changed'; }}; }}; {hold_sql}");
        self.effect(&sql,bindings).await?;drop(charges);Ok(())
    }
    /// Install fused contribution locators without making them physical hold/guard targets.
    /// Existing equal logical rows retain their first placement, even under new packing.
    pub async fn install_compiler_packet(&mut self,contribution:ContentHash,blocks:crate::compiler_blocks::PreparedBlocks,graph:Vec<Value>)->Result<(),ModelError>{
        let (blocks,locators,_packet)=blocks.into_parts();
        let budget=self.budget.ok_or(ModelError::Conflict("compiler locator installation budget"))?;
        let _grouping=budget.reserve("compiler packet unique grouping",graph.iter().map(crate::loader::native_bytes).sum::<usize>().saturating_mul(8).saturating_add(blocks.len().saturating_mul(4096)))?;
        let mut grouped=std::collections::BTreeMap::<RecordId,Vec<Value>>::new();
        for locator in locators{let physical=crate::compiler_blocks::validate_locator(&locator)?.record().clone();grouped.entry(physical).or_default().push(locator);}
        let mut packets=blocks.into_iter().map(|block|{
            let id=match block.as_object().and_then(|row|row.get("id")){Some(Value::RecordId(id))=>id.clone(),_=>unreachable!("prepared block identity")};
            (vec![block],grouped.remove(&id).expect("prepared block locators"))
        }).collect::<Vec<_>>();
        assert!(grouped.is_empty(),"prepared locators all belong to exact blocks");
        packets.extend(graph.chunks(crate::loader::NATIVE_WINDOW_ROWS).map(|page|(Vec::new(),page.to_vec())));
        for (rows,page) in packets{
            let budget=self.budget.ok_or(ModelError::Conflict("compiler locator installation budget"))?;
            let parent=RecordId::new("compiler_contribution",contribution.hex());
            for locator in &page{
                crate::compiler_blocks::validate_locator(locator)?;
                if locator.as_object().and_then(|row|row.get("contribution"))!=Some(&Value::RecordId(parent.clone())){return Err(ModelError::Conflict("compiler packet exact locator contribution"));}
            }
            let _charge=budget.reserve("compiler fused locator native packet",page.iter().map(crate::loader::native_bytes).sum::<usize>().saturating_mul(32).saturating_add(rows.iter().map(crate::loader::native_bytes).sum::<usize>().saturating_mul(4)).saturating_add(65536))?;
            let references=rows.iter().map(|row|Ok((parent.clone(),match row.as_object().and_then(|r|r.get("id")){Some(Value::RecordId(id))=>id.clone(),_=>return Err(ModelError::Schema("compiler block packet identity"))}))).collect::<Result<Vec<_>,ModelError>>()?;
            let (mut sql,mut bindings)=if let Some((sql,vars))=immutable_row_plan_in(None,None,rows,CoalescingAdmission::ExistingConstruction)?{(sql,vars)}else{(String::new(),Variables::new())};
            if let Some((_,hold_sql,holds))=hold_plan_in(None,references,CoalescingAdmission::ExistingConstruction)?{
                sql=if sql.is_empty(){hold_sql}else{join_immutable_holds(&sql,&hold_sql)};
                merge_plan_bindings(&mut bindings,holds)?;
            }
            // An entirely graph-only packet still protects the actual parent guard.
            let mut owner=Variables::new();owner.insert("__guard_records",guard_records(std::iter::once(&parent))?);merge_plan_bindings(&mut bindings,owner)?;
            bindings.insert("__compiler_locator_packet",true);
            bindings.insert("locator_parent",parent);bindings.insert("locator_rows",page);
            if bindings.get("rows").is_none(){bindings.insert("rows",Vec::<Value>::new());}
            if bindings.get("holds").is_none(){bindings.insert("holds",Vec::<Value>::new());}
            let suffix="IF array::len($new_locators)>0 { fn::native_touch_admitted($locator_parent,$__attempt_context); INSERT INTO compiler_membership $new_locators RETURN NONE; };";
            self.effect(&format!("{sql} {suffix}"),bindings).await?;
        }
        Ok(())
    }
    /// Reuse belongs to one exact client, effect owner and admission pool.
    pub fn ensure_context(
        &self,
        client: &Surreal<Client>,
        attempt: Option<ContentHash>,
        budget: Option<&ResourceBudget>,
    ) -> Result<(), ModelError> {
        let same_owner = match (&self.owner, attempt) {
            (EffectOwner::Installation, None) => true,
            (EffectOwner::OpenAttempt(current), Some(attempt)) => *current == attempt,
            _ => false,
        };
        let same_budget = match (self.budget, budget) {
            (Some(current), Some(budget)) => current.shares_pool(budget),
            (None, None) => true,
            _ => false,
        };
        if !std::ptr::eq(self.client, client) || !same_owner || !same_budget {
            return Err(ModelError::Conflict(
                "native immutable writer context changed",
            ));
        }
        Ok(())
    }
    pub async fn install(&mut self, window: ImmutableWindow) -> Result<(), ModelError> {
        let _charge: Option<Box<dyn Reservation>> = if window.charges.is_empty() {
            self.budget
                .map(|budget| budget.reserve("native immutable window", window.bytes))
                .transpose()?
        } else {
            None
        };
        let span = tracing::info_span!(target: "lctx_phase", "native_immutable_window", rows = window.rows, bytes = window.bytes);
        async {
            let phase = crate::phase::Phase::begin("native_immutable_window");
            let result = self.effect(&window.sql, window.bindings.into()).await;
            phase.finish_result(&result);
            result
        }
        .with_tracing_span(span)
        .await
    }
    pub(super) async fn effect(
        &mut self,
        sql: &str,
        mut bindings: Variables,
    ) -> Result<(), ModelError> {
        // Retain the payload/construction charges while preparing the complete protocol.
        // The same prepared request is sized and executed, without a second lowering.
        // Admit the fixed-only sizing plan separately, before lowering it. The two
        // model-owned guard bodies dominate its text; plan_bytes also retains their
        // native/serialization/container allowance. No row or endpoint is cloned here.
        let sizing = if self.control_bytes.is_none() {
            let sizing_bytes = plan_bytes(coordination::ERA_GUARD, &Variables::new())?
                .saturating_add(plan_bytes(
                    coordination::EXPECTED_OBJECT_GUARD,
                    &Variables::new(),
                )?);
            self.budget
                .map(|budget| budget.reserve("native immutable sizing templates", sizing_bytes))
                .transpose()?
        } else {
            None
        };
        let controls = self.control_workspace_bytes()?;
        let mut workspace_bytes = plan_bytes(sql, &bindings)?.saturating_add(controls);
        if bindings.get("__compiler_locator_packet")==Some(&Value::Bool(true)){workspace_bytes=workspace_bytes.saturating_add(plan_bytes(coordination::COMPILER_LOCATOR_PREPARATION,&Variables::new())?);}
        let mut workspace = self
            .budget
            .map(|budget| budget.reserve("native immutable complete preparation", workspace_bytes))
            .transpose()?;
        // The initial raw/control charge admits the fixed sizing templates. Derive
        // the growing expectation envelope from the actual planned identities before
        // prepare_expectations allocates/maps/reads their physical observations.
        workspace_bytes =
            workspace_bytes.saturating_add(Self::expectation_workspace_bytes(&bindings)?);
        if let Some(workspace) = &mut workspace {
            workspace.try_resize(workspace_bytes)?;
        }
        drop(sizing);
        if let Some(operation)=&self.operation{
            let budget=self.budget.ok_or(ModelError::Conflict("native operation requires resource owner"))?;
            coordination::prepare_expectations(self.client,&mut bindings).await?;
            return operation.apply(self.client,sql,bindings,budget).await;
        }
        if self.issuance.is_none() {
            self.issuance = Some(match self.owner {
                EffectOwner::Installation => IssuanceEra::capture(self.client).await?,
                _ => authorization::owner_issuance(self.client, &self.owner).await?,
            });
        }
        let prepared = authorization::prepare_owner_effect_admitted(
            self.client,
            &self.owner,
            self.issuance
                .expect("captured exact immutable owner issuance"),
            sql,
            bindings,
        )
        .await?;
        if prepared.bytes()? > workspace_bytes {
            return Err(ModelError::Schema(
                "native immutable complete preparation bound",
            ));
        }
        // Recovery retains this request's identity. Neither retry nor a later window
        // refreshes the original owner's generation/era after an eligibility cut.
        authorization::execute_prepared_owner_effect(self.client, prepared).await
    }
    fn control_workspace_bytes(&mut self) -> Result<usize, ModelError> {
        if let Some(bytes) = self.control_bytes {
            return Ok(bytes);
        }
        if let Some(operation)=&self.operation{
            let bytes=operation.workspace_bytes()?;self.control_bytes=Some(bytes);return Ok(bytes);
        }
        // Fixed-width hashes and the widest positive era bound every actual request.
        // Reuse the production lowering rather than copying its wrapper SQL/controls.
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash([0; 32]),
                era: i64::MAX,
            },
            operation: ContentHash([0; 32]),
            request: ContentHash([0; 32]),
        };
        let mut plan =
            authorization::owner_effect_plan(request, &self.owner, "", Variables::new(), false)?;
        {
            let bindings = &mut plan.bindings;
            bindings.insert("__stripe", RecordId::new("native_admission_stripe", 63i64));
            if bindings.get("__attempt_slot") != Some(&Value::None) {
                bindings.insert("__attempt_slot", 63i64);
            }
        }
        let bytes = plan_bytes(&plan.query, &plan.bindings)?;
        self.control_bytes = Some(bytes);
        Ok(bytes)
    }
    pub(super) fn expectation_workspace_bytes(bindings: &Variables) -> Result<usize, ModelError> {
        fn encoded(value: &impl serde::Serialize) -> Result<usize, ModelError> {
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
            let mut counter = Counter(0);
            serde_json::to_writer(&mut counter, value).map_err(ModelError::codec)?;
            Ok(counter.0)
        }
        // Candidate indexes are temporary but coexist with the complete request.
        // Count all source entries, including equal candidates retained for later
        // compound membership queries, before any index allocates.
        let index_bytes = [
            "rows",
            "original_blobs",
            "__immutable_creations",
            "__mutable_write_guards",
            "hold_guards",
            "guards",
            "__write_guards",
        ]
        .into_iter()
        .filter_map(|name| match bindings.get(name) {
            Some(Value::Array(values)) => Some(values),
            _ => None,
        })
        .flatten()
        .map(exact_index::entry_workspace)
        .fold(0usize, usize::saturating_add);
        if let Some(Value::Array(expected)) = bindings.get("__object_expectations") {
            // Supplied physical observations are immutable. Metadata is attached
            // locally; a caller may have supplied the pre-metadata envelope.
            return Ok(index_bytes.saturating_add(expected.len().saturating_mul(256)));
        }
        // Every selected guard occurs in this map. Counting extra entries is safe;
        // no identity is shortened to a digest or a fixed assumed string width.
        let records = match bindings.get("__guard_records") {
            Some(Value::Array(records)) => Some(records),
            None => None,
            _ => return Err(ModelError::Schema("native expected record map")),
        };
        let placeholder = RecordId::new("native_guard", "");
        let expected = coordination::ExpectedObject {
            record: placeholder.clone(),
            incarnation: i64::MAX,
            phase: "retired".into(),
            tombstone_revision_if_vacant: Some(i64::MAX),
        };
        let Value::Object(mut fields) = expected.binding(placeholder, None) else {
            unreachable!()
        };
        fields.remove("record");
        fields.remove("guard");
        let field_native = fields
            .iter()
            .map(|(key, value)| {
                key.len()
                    .saturating_add(32)
                    .saturating_add(crate::loader::native_bytes(value))
            })
            .fold(0usize, usize::saturating_add);
        let field_encoded = fields.iter().try_fold(0usize, |bytes, (key, value)| {
            // Added fields contribute a comma, a JSON key, a colon and their value.
            Ok::<_, ModelError>(
                bytes
                    .saturating_add(encoded(key)?)
                    .saturating_add(encoded(value)?)
                    .saturating_add(2),
            )
        })?;
        let mut native = "__object_expectations"
            .len()
            .saturating_add(std::mem::size_of::<Value>());
        let mut serialized = encoded(&"__object_expectations")?
            .saturating_add(encoded(&Value::Array(Vec::<Value>::new().into()))?)
            .saturating_add(2);
        if let Some(records) = records {
            for record in records.iter() {
                native = native
                    .saturating_add(crate::loader::native_bytes(record))
                    .saturating_add(field_native);
                serialized = serialized
                    .saturating_add(encoded(record)?)
                    .saturating_add(field_encoded);
            }
            serialized = serialized.saturating_add(records.len().saturating_sub(1));
        }
        // creation_indices stores only {kind,index}, never cloned immutable payloads.
        // At most one expected guard receives each planned creation declaration.
        if let Some(Value::Array(creations)) = bindings.get("__immutable_creations") {
            for creation in creations.iter() {
                let Some(Value::String(kind)) =
                    creation.as_object().and_then(|row| row.get("kind"))
                else {
                    return Err(ModelError::Schema("native immutable creation envelope"));
                };
                let source = match kind.as_str() {
                    "immutable" => "rows",
                    "original_blob" => "original_blobs",
                    _ => return Err(ModelError::Schema("native immutable creation kind")),
                };
                let Some(Value::Array(rows)) = bindings.get(source) else {
                    return Err(ModelError::Schema("native immutable creation payload"));
                };
                let index =
                    i64::try_from(rows.len().saturating_sub(1)).map_err(ModelError::codec)?;
                let mut reference = Object::new();
                reference.insert("kind", kind.as_str());
                reference.insert("index", index);
                let reference = Value::Object(reference);
                native = native.saturating_add(
                    crate::loader::native_bytes(&reference)
                        .saturating_sub(crate::loader::native_bytes(&Value::None)),
                );
                serialized = serialized
                    .saturating_add(encoded(&reference)?.saturating_sub(encoded(&Value::None)?));
            }
        }
        // Match plan_bytes' resident-container and serialization workspace model.
        // This is not the protobuf wire bound; production preflight checks that separately.
        Ok(index_bytes.saturating_add(
            native
                .saturating_mul(2)
                .saturating_add(serialized.saturating_mul(3)),
        ))
    }
}

#[cfg(test)]
mod writer_workspace_controls {
    use super::*;

    #[test]
    fn coalescing_workspace_refuses_before_candidate_growth_and_releases_at_last_use() {
        let row = |payload: i64| {
            let mut row = Object::new();
            row.insert("id", RecordId::new("entity", "index-admission"));
            row.insert("payload", payload);
            Value::Object(row)
        };
        let make = || ImmutableWindow::unit(row(1), None, None, None, false, None).unwrap();
        let mut collision = make();
        collision
            .bindings
            .insert("rows".into(), Value::Array(vec![row(1), row(2)].into()));
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(
            matches!(
                collision.coalesce(Some(&tiny)),
                Err(ModelError::Resource { .. })
            ),
            "index reservation precedes collision scanning and backing growth"
        );
        assert_eq!(tiny.reserved(), 0);
        let budget = ResourceBudget::fixed(4 * 1024 * 1024).unwrap();
        let window = make().coalesce(Some(&budget)).unwrap();
        assert!(budget.peak().unwrap() > 0, "temporary index was admitted");
        assert_eq!(
            budget.reserved(),
            0,
            "index dies after compaction, not acknowledgement"
        );
        drop(window);
    }
    #[test]
    fn complete_workspace_covers_distinct_endpoint_guards_and_creation_indices() {
        let attempt = ContentHash::of(b"immutable workspace control");
        let owner = RecordId::new("native_attempt", attempt.hex());
        let rows = (0..crate::loader::NATIVE_WINDOW_ROWS)
            .map(|index| {
                let mut row = Object::new();
                row.insert("id", RecordId::new("reference", format!("row-{index}")));
                for field in ["in", "out", "unit_node", "source"] {
                    row.insert(
                        field,
                        RecordId::new("entity", format!("endpoint-{field}-{index}-\"🦀")),
                    );
                }
                Value::Object(row)
            })
            .collect();
        let mut windows = ImmutableWindows::for_loading(rows, Some(attempt), Some(owner));
        let window = windows.next().unwrap().unwrap();
        assert!(windows.next().is_none(), "full 128-unit window");
        let mut bindings: Variables = window.bindings.into();
        // Exercise the second model-owned creation-reference kind as well.
        let blob = crate::originals::Blob::new(ContentHash::of(b"x"), 1).unwrap();
        let mut extra = Variables::new();
        extra.insert("original_blobs", vec![blob.value(false)]);
        extra.insert(
            "__guard_records",
            guard_records(std::iter::once(&blob.id())).unwrap(),
        );
        extra.insert(
            "__write_guards",
            vec![guard_id(&Value::RecordId(blob.id())).unwrap()],
        );
        let mut creation = Object::new();
        creation.insert("kind", "original_blob");
        creation.insert("record", blob.id());
        extra.insert("__immutable_creations", vec![Value::Object(creation)]);
        merge_plan_bindings(&mut bindings, extra).unwrap();
        let client = Surreal::init();
        let mut writer = WindowWriter::new(&client, Some(attempt), None);
        let raw = plan_bytes(&window.sql, &bindings).unwrap();
        let capacity = raw
            .saturating_add(writer.control_workspace_bytes().unwrap())
            .saturating_add(WindowWriter::expectation_workspace_bytes(&bindings).unwrap());
        let Some(Value::Array(mapping)) = bindings.get("__guard_records") else {
            panic!("guard map")
        };
        assert_eq!(mapping.len(), 5 * crate::loader::NATIVE_WINDOW_ROWS + 2);
        let Some(Value::Array(creations)) = bindings.get("__immutable_creations") else {
            panic!("creations")
        };
        for phase in ["active", "vacant", "retired"] {
            let expected = mapping
                .iter()
                .map(|value| {
                    let row = value.as_object().unwrap();
                    let Value::RecordId(record) = &row["record"] else {
                        panic!("record")
                    };
                    let Value::RecordId(guard) = &row["guard"] else {
                        panic!("guard")
                    };
                    let creation = creations.iter().find_map(|value| {
                        let row = value.as_object().unwrap();
                        if row.get("record") != Some(&Value::RecordId(record.clone())) {
                            return None;
                        }
                        let Value::String(kind) = &row["kind"] else {
                            panic!("kind")
                        };
                        let source = if kind == "immutable" {
                            "rows"
                        } else {
                            "original_blobs"
                        };
                        let Some(Value::Array(rows)) = bindings.get(source) else {
                            panic!("source")
                        };
                        let index = rows
                            .iter()
                            .position(|row| {
                                row.as_object().unwrap().get("id")
                                    == Some(&Value::RecordId(record.clone()))
                            })
                            .unwrap();
                        let mut reference = Object::new();
                        reference.insert("kind", kind.as_str());
                        reference.insert("index", index as i64);
                        Some(Value::Object(reference))
                    });
                    coordination::ExpectedObject {
                        record: record.clone(),
                        incarnation: if phase == "vacant" { 0 } else { i64::MAX },
                        phase: phase.into(),
                        tombstone_revision_if_vacant: match phase {
                            "active" => None,
                            "vacant" => Some(0),
                            _ => Some(i64::MAX),
                        },
                    }
                    .binding(guard.clone(), creation.as_ref())
                })
                .collect::<Vec<_>>();
            let mut observed = bindings.clone();
            observed.insert("__object_expectations", expected);
            let request = NativeRequest {
                issuance: IssuanceEra {
                    generation: ContentHash([0; 32]),
                    era: i64::MAX,
                },
                operation: ContentHash([0; 32]),
                request: ContentHash([0; 32]),
            };
            // Complete lowering includes the independent protobuf wire admission.
            let plan = authorization::owner_effect_plan(
                request,
                &writer.owner,
                &window.sql,
                observed.clone(),
                false,
            )
            .unwrap();
            let actual = plan_bytes(&plan.query, &plan.bindings).unwrap();
            assert!(
                actual > raw + 131072,
                "old fixed allowance fails for {phase}: raw={raw} actual={actual}"
            );
            assert!(
                actual <= capacity,
                "derived capacity covers {phase}: capacity={capacity} actual={actual}"
            );
            assert!(
                WindowWriter::expectation_workspace_bytes(&observed).unwrap() > 0,
                "supplied envelopes still require metadata and candidate scratch admission"
            );
        }
    }
}
pub async fn install_window(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    window: ImmutableWindow,
    budget: Option<&ResourceBudget>,
) -> Result<(), ModelError> {
    WindowWriter::new(client, attempt, budget)
        .install(window)
        .await
}

/// A frozen derivation can only retain derived children and exact existing canonical
/// external endpoints. It cannot introduce a canonical payload after native freeze.
pub(super) async fn install_staging_window(
    writer: &mut WindowWriter<'_>,
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
        .effect(writer, &window.sql, window.bindings.into(), budget)
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
    fn logical_compiler_ids_cannot_be_raw_physical_guards_holds_or_installations(){
        for table in ["compiler_record","compiler_membership"]{
            let id=RecordId::new(table,"virtual");
            assert!(guard_id(&Value::RecordId(id.clone())).is_err());
            assert!(hold_plan(None,vec![(RecordId::new("native_pin","owner"),id.clone())]).is_err());
            let mut row=Object::new();row.insert("id",id);
            assert!(ImmutableWindows::new(vec![Value::Object(row)],None,None,None).next().unwrap().is_err());
        }
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
                .matches("fn::native_immutable_install($rows,$holds,$__attempt_context,$__deferred_guards,($original_blobs ?? []))")
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
            let selected = rows.iter().map(control_address).collect::<Vec<_>>();
            let mut windows = ImmutableWindows::new(
                rows.clone(),
                Some(store.attempt()),
                None,
                Some(owner.clone()),
            );
            let window = windows.next().unwrap()?;
            assert!(windows.next().is_none());
            let mut writer = WindowWriter::new(&client, Some(store.attempt()), None);
            assert!(
                writer.issuance.is_none(),
                "unused writer earns no native issuance"
            );
            writer.install(window).await?;
            let issuance = writer.issuance;
            let before = control_guard(&client, &selected[0]).await;
            let replay = ImmutableWindows::new(rows, Some(store.attempt()), None, Some(owner))
                .next()
                .unwrap()?;
            writer.install(replay).await?;
            assert_eq!(
                writer.issuance, issuance,
                "later windows retain the exact original issuance"
            );
            assert_eq!(
                control_guard(&client, &selected[0]).await,
                before,
                "equal rows and equal holds do not churn physical guards"
            );
            let (_, sql, variables) = hold_plan(
                Some(store.attempt()),
                vec![(selected[1].clone(), selected[0].clone())],
            )?
            .unwrap();
            effect(&client, Some(store.attempt()), &sql, variables).await?;
            let after = control_guard(&client, &selected[0]).await;
            assert_eq!(
                authorization::int_field(&after, "revision")?,
                authorization::int_field(&before, "revision")? + 1,
                "equal bytes with a new owner hold are changed incidence"
            );
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
    async fn full_physical_envelopes_join_equal_frozen_successors_and_concurrent_windows() {
        use futures::FutureExt;
        use lctx_model::domain::{self as d, Record};
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("ordinary owned fixture"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(d::resources::DEFAULT_MEMORY_BYTES).unwrap();
        let begin = || async {
            let factory =
                lctx_compiler_store::compiler::recovery_factory(&config, budget.clone()).unwrap();
            lctx_compiler_store::compiler::NativeCompilerStore::begin(
                &config,
                d::admission::Frontier::Facts,
                &factory,
            )
            .await
            .unwrap()
        };
        let first = begin().await;
        let second = begin().await;
        let specification = d::completed::ContributionSpec {
            captured_binding: None,
            producer: "full-physical-envelope-control".into(),
            profile: d::completed::OutputProfile::Specific(d::stages::Profile::Behavioral),
            model: d::model().unwrap().digest(),
            implementation: first.attempt(),
            configuration: None,
            inputs: vec![],
            outputs: [
                d::source::SourceArtifact::NAME.into(),
                d::analytics::QualityStep::NAME.into(),
            ]
            .into(),
        };
        let contribution1 = first
            .begin_contribution(specification.clone())
            .await
            .unwrap();
        let contribution2 = second.begin_contribution(specification).await.unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            first.namespace().as_str(),
            first.database().as_str(),
        )
        .await
        .unwrap();
        let (scope, signals) =
            exact_control_recovery(client.clone(), "full physical equal successor control");
        async fn freeze(
            client: &Surreal<Client>,
            mut window: ImmutableWindow,
        ) -> Result<ImmutableWindow, ModelError> {
            let mut bindings: Variables = window.bindings.into();
            coordination::prepare_expectations(client, &mut bindings).await?;
            window.bindings = bindings.into_iter().collect();
            Ok(window)
        }
        let result = with_recovery_scope(scope.clone(), std::panic::AssertUnwindSafe(async {
            for concurrent in [false, true] {
                let mut nonce = KeySink::new("full-physical-envelope-control/v1");
                first.attempt().encode(&mut nonce);
                concurrent.encode(&mut nonce);
                let nonce = nonce.finish();
                let revision = d::input::InputRevision { manifest: nonce };
                let source = d::source::SourceArtifact::from_bytes(revision.id(), format!("{}.py", nonce.hex()), b"def fresh():\n    return 7\n")?;
                let entity = d::graph::Entity::from(source.clone());
                let source_view = crate::codec::entity_views(std::slice::from_ref(&entity))?.remove(0);
                let graph_row = crate::adapter::physical_row(crate::loader::entity_payload_id(&entity)?, entity.content(), serde_json::to_vec(&entity).map_err(ModelError::codec)?, Some((entity.kind() as i64, entity.subtype())), source_view)?;
                let run = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(nonce.0[..16].iter().copied())).map_err(ModelError::codec)?;
                let quality = d::analytics::QualityStep { run, ordinal: 0, value: d::FiniteF64::new(0.75)? };
                let quality_view = crate::codec::views(std::slice::from_ref(&quality))?.remove(0);
                let canonical = serde_json::to_vec(&quality_view.body).map_err(ModelError::codec)?;
                let content = ContentHash::of(&canonical);
                let compiler_row = crate::adapter::physical_row(crate::loader::payload_id("compiler_record", d::analytics::QualityStep::NAME, quality.id().bytes(), content)?, content, canonical, None, quality_view)?;
                let mut rows = crate::loader::entity_endpoint_values(&entity)?;
                rows.push(graph_row.clone());
                let window = |attempt, contribution: ContentHash| ImmutableWindows::new(rows.clone(), Some(attempt), Some(contribution), Some(RecordId::new("compiler_contribution", contribution.hex()))).next().unwrap();
                // Freeze both actual creation expectations before either peer can
                // activate the same fresh graph/compiler payload addresses.
                let a = freeze(&client, window(first.attempt(), contribution1)?).await?;
                let b = freeze(&client, window(second.attempt(), contribution2)?).await?;
                if concurrent {
                    let (a, b) = tokio::join!(install_window(&client, Some(first.attempt()), a, None), install_window(&client, Some(second.attempt()), b, None));
                    let mut terminal = d::completion::Completion::default();
                    terminal.step("second full-row concurrent window", b);
                    d::completion::complete(a, terminal)?;
                } else {
                    install_window(&client, Some(second.attempt()), b, None).await?;
                    install_window(&client, Some(first.attempt()), a, None).await?;
                }
                let packet1=crate::compiler_blocks::PreparedBlocks::prepare(contribution1,vec![compiler_row.clone()],&budget)?;
                let packet2=crate::compiler_blocks::PreparedBlocks::prepare(contribution2,vec![compiler_row.clone()],&budget)?;
                let expected_block=packet1.blocks()[0].clone();
                let locator=packet1.locators()[0].clone();
                let first_locator=locator.clone();
                for table in ["compiler_record","compiler_membership"]{
                    let raw=RecordId::new(table,nonce.hex());
                    assert!(client.query("RETURN fn::native_touch($raw)").bind(("raw",raw.clone())).await.map_err(ModelError::codec)?.check().is_err(),"raw SQL cannot manufacture virtual incidence");
                    let mut value=Object::new();value.insert("id",raw);
                    assert!(client.query("RETURN fn::native_immutable_install($values,[],NONE,[],[])").bind(("values",vec![Value::Object(value)])).await.map_err(ModelError::codec)?.check().is_err(),"raw SQL cannot install a virtual payload/locator as a physical object");
                }

                let install1=async{WindowWriter::new(&client,Some(first.attempt()),Some(&budget)).install_compiler_packet(contribution1,packet1,vec![]).await};
                let install2=async{WindowWriter::new(&client,Some(second.attempt()),Some(&budget)).install_compiler_packet(contribution2,packet2,vec![]).await};
                if concurrent{let (a,b)=tokio::join!(install1,install2);a?;b?;}else{install2.await?;install1.await?;}
                // An occupied physical address requires full equal container bytes.
                let mut corrupted=expected_block.clone();
                if let Value::Object(object)=&mut corrupted{object.insert("relation","changed-relation");}
                let changed=ImmutableWindows::new(vec![corrupted],Some(first.attempt()),Some(contribution1),Some(RecordId::new("compiler_contribution",contribution1.hex()))).with_budget(Some(&budget)).next().unwrap()?;
                assert!(WindowWriter::new(&client,Some(first.attempt()),Some(&budget)).install(changed).await.is_err());
                for expected in [&graph_row, &compiler_row] {
                    let object = expected.as_object().ok_or(ModelError::Schema("full envelope object"))?;
                    let Value::RecordId(id) = object.get("id").ok_or(ModelError::Schema("full envelope identity"))? else { return Err(ModelError::Schema("full envelope identity")); };
                    let physical=if id.table.as_str()=="compiler_record"{crate::compiler_blocks::PhysicalLocation::from_locator(&locator)?.record().clone()}else{id.clone()};
                    let mut response = client.query("SELECT * FROM ONLY $record; SELECT VALUE owner FROM native_hold WITH INDEX object_holds WHERE object=$record ORDER BY owner").bind(("record", physical)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                    let actual: Value = response.take(0).map_err(ModelError::codec)?;
                    let actual=if id.table.as_str()=="compiler_record"{assert_eq!(actual,expected_block);crate::compiler_blocks::validate_block(&actual,&budget)?;crate::compiler_blocks::selected_row(&actual,&locator)?.clone()}else{actual};
                    assert_eq!(&actual, expected, "exact complete stored envelope differs for {id:?}");
                    if id.table.as_str() == "compiler_record" {
                        crate::completed_rows::validate_state_row(id.table.as_str(), &actual)?;
                        crate::completed_rows::validate_backing_batch(std::slice::from_ref(&actual), &budget)?;
                    }
                    let owners: Vec<RecordId> = response.take(1).map_err(ModelError::codec)?;
                    assert_eq!(owners.len(), 2);
                    assert!(owners.contains(&RecordId::new("compiler_contribution", contribution1.hex())));
                    assert!(owners.contains(&RecordId::new("compiler_contribution", contribution2.hex())));
                    let actual = actual.as_object().ok_or(ModelError::Schema("stored full envelope object"))?;
                    let Some(Value::Object(mut body)) = actual.get("body").cloned() else { return Err(ModelError::Schema("stored full envelope body")); };
                    body.insert("id", actual.get("semantic_key").ok_or(ModelError::Schema("stored full envelope nominal key"))?.clone());
                    let body = Value::Object(body);
                    if id.table.as_str() == "entity" {
                        let Value::Bytes(canonical) = actual.get("canonical").ok_or(ModelError::Schema("stored graph canonical"))? else { return Err(ModelError::Schema("stored graph canonical")); };
                        let cold: d::graph::Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
                        assert_eq!(cold, entity);
                        let batch = crate::codec::decode_bodies(&d::Relation::of::<d::source::SourceArtifact>(), vec![body], &budget)?;
                        assert_eq!(d::source::SourceArtifact::decode(&batch)?, vec![source.clone()]);
                    } else {
                        let batch = crate::codec::decode_bodies(&d::Relation::of::<d::analytics::QualityStep>(), vec![body], &budget)?;
                        assert_eq!(d::analytics::QualityStep::decode(&batch)?, vec![quality.clone()]);
                    }
                }
                let mut extra=quality.clone();extra.ordinal=1;
                let extra_view=crate::codec::views(std::slice::from_ref(&extra))?.remove(0);
                let canonical=serde_json::to_vec(&extra_view.body).map_err(ModelError::codec)?;let content=ContentHash::of(&canonical);
                let extra_row=crate::adapter::physical_row(crate::loader::payload_id("compiler_record",d::analytics::QualityStep::NAME,extra.id().bytes(),content)?,content,canonical,None,extra_view)?;
                let alternate=crate::compiler_blocks::PreparedBlocks::prepare(contribution1,vec![compiler_row.clone(),extra_row.clone()],&budget)?;
                let alternate_id=crate::compiler_blocks::PhysicalLocation::from_locator(&alternate.locators()[0])?.record().clone();
                WindowWriter::new(&client,Some(first.attempt()),Some(&budget)).install_compiler_packet(contribution1,alternate,vec![]).await?;
                let first_id=first_locator.as_object().and_then(|row|row.get("id")).cloned().ok_or(ModelError::Schema("first locator"))?;
                let mut response=client.query("SELECT * FROM ONLY $locator; SELECT * FROM ONLY $block").bind(("locator",first_id)).bind(("block",alternate_id.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let retained:Value=response.take(0).map_err(ModelError::codec)?;assert_eq!(retained,first_locator,"equal nominal retains first physical placement");
                let actual:Value=response.take(1).map_err(ModelError::codec)?;crate::compiler_blocks::validate_block(&actual,&budget)?;
                let Some(Value::Array(slots))=actual.as_object().and_then(|row|row.get("rows"))else{return Err(ModelError::Schema("alternate block slots"));};assert_eq!(slots.len(),2,"an unselected slot remains legitimate shared physical content");
            }
            Ok(())
        }).catch_unwind()).await.unwrap_or_else(|panic| Err(ModelError::Cause(Box::new(d::completion::ThreadPanic::new("full physical equal successor control", panic)))));
        let mut terminal = d::completion::Completion::default();
        if scope.pending_count().unwrap() == 0
            && result
                .as_ref()
                .err()
                .is_none_or(ModelError::permits_storage_cleanup)
        {
            terminal.step(
                "full row observer close",
                client.invalidate().await.map_err(ModelError::codec),
            );
            terminal.step("first full row attempt cleanup", first.abandon().await);
            terminal.step("second full row attempt cleanup", second.abandon().await);
        } else {
            terminal.local = d::completion::LocalState::Outstanding;
            terminal.remote = d::completion::RemoteState::Unknown;
        }
        let observed = signals.complete(&terminal).await;
        terminal.step("full row observer signal owner", observed);
        d::completion::complete(result, terminal).unwrap();
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
            let (request, plan) = control_packet(
                &client,
                attempt,
                &collision.sql,
                collision.bindings.into(),
                None,
            )
            .await;
            let result =
                checked_transaction(client.query(plan.query).bind(plan.bindings).await.unwrap());
            assert!(
                matches!(
                    authorization::reconcile_request(&client, request)
                        .await
                        .unwrap(),
                    EffectDisposition::FencedUncommitted
                ),
                "failed atomic mixed window has no committed outcome"
            );
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
            let prior_id = control_address(&immutable_control_row(&existing, 0));
            let mut response = client
                .query("SELECT * FROM $prior")
                .bind(("prior", prior_id))
                .await
                .unwrap()
                .check()
                .unwrap();
            let prior: Vec<Value> = response.take(0).unwrap();
            assert_eq!(
                prior,
                vec![immutable_control_row(&existing, 0)],
                "earlier committed window survives later atomic collision"
            );
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
