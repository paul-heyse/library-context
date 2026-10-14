//! A claimed incarnation excludes hold insertion/reactivation while bounded pages drain.
use super::*;
use authorization::{hash_field, int_field};
use lctx_model::domain::completed::RetirementProgress;

pub(super) fn item_id(
    job: &RecordId,
    object: &RecordId,
    incarnation: i64,
) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "native_retirement_item",
        ContentHash::of(
            &serde_json::to_vec(&(job, object, incarnation)).map_err(ModelError::codec)?,
        )
        .hex(),
    ))
}
async fn record(client: &Surreal<Client>, id: &RecordId) -> Result<Object, ModelError> {
    let mut response = client
        .query("SELECT * FROM $id")
        .bind(("id", id.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict("native retirement identity missing"));
    };
    Ok(row.clone())
}
async fn incarnations(
    client: &Surreal<Client>,
    objects: &[RecordId],
) -> Result<std::collections::BTreeMap<RecordId, i64>, ModelError> {
    let guards = objects
        .iter()
        .map(|object| guard_id(&Value::RecordId(object.clone())))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let mut response = client
        .query("SELECT id,incarnation FROM $guards")
        .bind(("guards", guards.iter().cloned().collect::<Vec<_>>()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    decode_incarnations(guards, rows)
}
fn decode_incarnations(
    guards: std::collections::BTreeSet<RecordId>,
    rows: Vec<Object>,
) -> Result<std::collections::BTreeMap<RecordId, i64>, ModelError> {
    let mut values = guards
        .into_iter()
        .map(|guard| (guard, 1))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        let Some(Value::RecordId(id)) = row.get("id") else {
            return Err(ModelError::Schema("native incarnation guard identity"));
        };
        let value = int_field(&row, "incarnation")?;
        if value <= 0 || !seen.insert(id.clone()) || !values.contains_key(id) {
            return Err(ModelError::Conflict("native object incarnation"));
        }
        values.insert(id.clone(), value);
    }
    Ok(values)
}
/// One actual outgoing hold page. Registration is composed into the cleanup owner's
/// payload BEFORE those exact holds are removed; it performs no retirement advancement.
/// Unlike old generic registration, a missing physical root is never incarnation one.
pub(super) async fn prepare_cleanup_nomination(
    client: &Surreal<Client>,
    owner: &RecordId,
) -> Result<Variables, ModelError> {
    let mut response = client
        .query(cleanup_hold_page_sql())
        .bind(("owner", owner.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let holds = one_cleanup_window(response.take::<Vec<Value>>(0).map_err(ModelError::codec)?)?;
    let mut guards = std::collections::BTreeSet::new();
    for hold in &holds {
        let Some(Value::RecordId(object)) = hold.get("object") else {
            return Err(ModelError::Schema("native cleanup nominated object"));
        };
        guards.insert(guard_id(&Value::RecordId(object.clone()))?);
    }
    let physical = if guards.is_empty() {
        Vec::new()
    } else {
        let mut response = client
            .query("SELECT * FROM $n09_guards")
            .bind(("n09_guards", guards.into_iter().collect::<Vec<_>>()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        one_cleanup_window(response.take::<Vec<Value>>(0).map_err(ModelError::codec)?)?
    };
    let job = if holds.is_empty() {
        None
    } else {
        Some(RecordId::new(
            "native_retirement",
            fresh_identity("cleanup-page-retirement")?.hex(),
        ))
    };
    cleanup_nomination_bindings(owner, holds, physical, job)
}
fn cleanup_hold_page_sql() -> String {
    format!(
        "SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner ORDER BY object LIMIT {}",
        crate::loader::NATIVE_WINDOW_ROWS
    )
}
fn one_cleanup_window(rows: Vec<Value>) -> Result<Vec<Object>, ModelError> {
    let mut windows = crate::loader::NativeWindows::new(rows);
    let rows = windows.next().transpose()?.unwrap_or_default();
    if windows.next().is_some() {
        return Err(ModelError::Conflict("native cleanup nomination window"));
    }
    rows.into_iter()
        .map(|value| match value {
            Value::Object(row) => Ok(row),
            _ => Err(ModelError::Schema("native cleanup nomination row")),
        })
        .collect()
}
pub(super) fn empty_cleanup_nomination() -> Variables {
    let mut bindings = Variables::new();
    bindings.insert("n09_holds", Vec::<Object>::new());
    bindings.insert("n09_roots", Vec::<Object>::new());
    bindings.insert("n09_job", Value::None);
    bindings.insert("n09_root_count", 0i64);
    bindings.insert("n09_root_digest", Value::None);
    bindings
}
fn cleanup_nomination_bindings(
    owner: &RecordId,
    holds: Vec<Object>,
    physical: Vec<Object>,
    job: Option<RecordId>,
) -> Result<Variables, ModelError> {
    if holds.len() > crate::loader::NATIVE_WINDOW_ROWS
        || physical.len() > crate::loader::NATIVE_WINDOW_ROWS
    {
        return Err(ModelError::Conflict("native cleanup nomination width"));
    }
    if holds.is_empty() {
        if !physical.is_empty() || job.is_some() {
            return Err(ModelError::Conflict(
                "native cleanup empty nomination scope",
            ));
        }
        return Ok(empty_cleanup_nomination());
    }
    let job = job.ok_or(ModelError::Schema("native cleanup nomination job missing"))?;
    if job.table.as_str() != "native_retirement" {
        return Err(ModelError::Schema("native cleanup nomination job family"));
    }
    let mut actual = std::collections::BTreeMap::new();
    for guard in physical {
        let Some(Value::RecordId(id)) = guard.get("id") else {
            return Err(ModelError::Schema(
                "native cleanup nomination guard identity",
            ));
        };
        if id.table.as_str() != "native_guard" || actual.insert(id.clone(), guard.clone()).is_some()
        {
            return Err(ModelError::Conflict(
                "native cleanup nomination duplicate/foreign guard",
            ));
        }
    }
    let mut root_ids = std::collections::BTreeSet::new();
    let mut hold_ids = std::collections::BTreeSet::new();
    let mut roots = Vec::new();
    for hold in &holds {
        let (Some(Value::RecordId(id)), Some(Value::RecordId(object))) =
            (hold.get("id"), hold.get("object"))
        else {
            return Err(ModelError::Schema(
                "native cleanup nomination hold identity",
            ));
        };
        if id.table.as_str() != "native_hold"
            || hold.get("owner") != Some(&Value::RecordId(owner.clone()))
            || !hold_ids.insert(id.clone())
            || !root_ids.insert(object.clone())
        {
            return Err(ModelError::Conflict(
                "native cleanup nomination actual hold scope",
            ));
        }
        let guard = guard_id(&Value::RecordId(object.clone()))?;
        let physical = actual.remove(&guard).ok_or(ModelError::Conflict(
            "native cleanup nomination original incarnation missing",
        ))?;
        let incarnation = int_field(&physical, "incarnation")?;
        if incarnation <= 0
            || physical.get("phase") != Some(&Value::String("active".into()))
            || physical.get("retired") != Some(&Value::Bool(false))
            || int_field(&physical, "revision")? < 0
            || int_field(&physical, "tombstone_revision")? < 0
        {
            return Err(ModelError::Conflict(
                "native cleanup nomination original incarnation not active",
            ));
        }
        let item = item_id(&job, object, incarnation)?;
        let mut root = Object::new();
        root.insert("object", object.clone());
        root.insert("guard", guard);
        root.insert("incarnation", incarnation);
        root.insert("item", item);
        roots.push(root);
    }
    if !actual.is_empty() {
        return Err(ModelError::Conflict(
            "native cleanup nomination extra guard",
        ));
    }
    let root_ids = root_ids.into_iter().collect::<Vec<_>>();
    let digest = ContentHash::of(
        &serde_json::to_vec(&("native-retirement/v4", &root_ids)).map_err(ModelError::codec)?,
    );
    let mut bindings = Variables::new();
    bindings.insert("n09_holds", holds);
    bindings.insert("n09_roots", roots);
    bindings.insert("n09_job", job);
    bindings.insert("n09_root_count", root_ids.len());
    bindings.insert("n09_root_digest", digest.hex());
    Ok(bindings)
}
/// The caller has already rechecked exact cleanup/origin/owner identity and full hold
/// page. Its incoming holds remain live until this payload nominates every exact
/// incarnation and then deletes those holds. Independent holders can advance a
/// guard's revision without changing the active incarnation consumed by the item.
/// All roots/items are created in this ONE payload before any hold deletion.
/// A lost acknowledgement replays the original native effect, not a partial setup job.
pub(super) const CLEANUP_NOMINATE_PAGE: &str = "IF array::len($holds)>0 { IF $n09_job=NONE OR $n09_root_count!=array::len($n09_roots) OR $n09_root_count!=array::len($holds) { THROW 'native cleanup nomination complete root count'; }; fn::native_touch($n09_job); LET $n09_existing_job=SELECT * FROM ONLY $n09_job FOR UPDATE; IF $n09_existing_job!=NONE { THROW 'native cleanup nomination job collision'; }; CREATE $n09_job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$n09_root_digest,lineage=$n09_job,state='open',root_count=$n09_root_count,roots_registered=$n09_root_count,examined=0,retired=0,revision=0 RETURN NONE; FOR $n09_root IN $n09_roots { fn::native_touch($n09_root.object); fn::native_touch($n09_root.item); LET $n09_guard_id=$n09_root.guard; LET $n09_physical=SELECT * FROM ONLY $n09_guard_id FOR UPDATE; IF $n09_physical=NONE OR $n09_physical.phase!='active' OR $n09_physical.retired!=false OR $n09_root.incarnation<=0 OR $n09_physical.incarnation!=$n09_root.incarnation { THROW 'native cleanup nominated incarnation changed'; }; LET $n09_item_id=$n09_root.item; IF (SELECT VALUE id FROM ONLY $n09_item_id)!=NONE { THROW 'native cleanup nomination item collision'; }; CREATE $n09_item_id SET job=$n09_job,object=$n09_root.object,incarnation=$n09_root.incarnation,state='pending' RETURN NONE; }; };";
async fn enqueue_roots(
    client: &Surreal<Client>,
    job: &RecordId,
    objects: &[RecordId],
    registered: i64,
) -> Result<(), ModelError> {
    let incarnations = incarnations(client, objects).await?;
    let mut rows = Vec::new();
    for object in objects {
        let guard = guard_id(&Value::RecordId(object.clone()))?;
        let current = incarnations[&guard];
        let mut row = Object::new();
        row.insert("id", item_id(job, object, current)?);
        row.insert("object", object.clone());
        row.insert("guard", guard);
        row.insert("incarnation", current);
        rows.push(Value::Object(row));
    }
    let mut registered = registered;
    for rows in crate::loader::NativeWindows::new(rows) {
        let rows = rows?;
        let count = i64::try_from(rows.len()).map_err(ModelError::codec)?;
        let mut bindings = Variables::new();
        bindings.insert("job", job.clone());
        bindings.insert("rows", rows);
        bindings.insert("registered", registered);
        effect_for_owner(client,EffectOwner::RetirementSetup(job.clone()),"IF $__owner.state!='registering' OR $__owner.roots_registered!=$registered { THROW 'native retirement registration raced'; }; FOR $row IN $rows { LET $guard_id=$row.guard; LET $before=SELECT * FROM ONLY $guard_id FOR UPDATE; IF ($before.incarnation ?? 1)!=$row.incarnation { THROW 'native retirement root incarnation changed'; }; IF $before=NONE { CREATE $guard_id SET revision=0,retired=false,tombstone_revision=0,phase='active',incarnation=1 RETURN NONE; }; fn::native_touch($job); fn::native_touch($row.object); fn::native_touch($row.id); LET $item_id=$row.id; LET $item=SELECT * FROM ONLY $item_id FOR UPDATE; IF $item=NONE { CREATE $item_id SET job=$job,object=$row.object,incarnation=$row.incarnation,state='pending' RETURN NONE; }; }; UPDATE $job SET roots_registered+=array::len($rows),revision+=1 RETURN NONE;",bindings).await?;
        registered += count;
    }
    Ok(())
}
/// Register the requested roots in bounded non-destructive setup transactions, then spend
/// `limit` on destructive advancement: one claim/finalize unit or two writes per outgoing
/// edge (durable child nomination plus exact hold deletion). Root registration is operation
/// setup, not permission to remove more edges than this pass admits.
pub async fn retire_reachable(
    client: &Surreal<Client>,
    roots: Vec<RecordId>,
    limit: usize,
) -> Result<RetirementProgress, ModelError> {
    retire_roots_for_owner(
        client,
        EffectOwner::Installation,
        roots,
        limit,
        RetirementMode::Advance,
    )
    .await
}
/// The caller durably records this identity before the first effect. It is an address,
/// not authority: native generation, era and the exact canonical root set are checked
/// on every retry, including after partial registration or destructive progress.
pub async fn retire_reachable_named(
    client: &Surreal<Client>,
    identity: ContentHash,
    expected_generation: ContentHash,
    roots: Vec<RecordId>,
    limit: usize,
) -> Result<RetirementProgress, ModelError> {
    retire_named_for_owner(
        client,
        EffectOwner::Installation,
        identity,
        Some(expected_generation),
        roots,
        limit,
        RetirementMode::Advance,
    )
    .await
}
/// Terminal cleanup registers its still-held exact roots during closed admission.
/// Destructive advancement remains with the retirement owner after hold drainage;
/// registration must also succeed while an independent cold-backup gate is held.
/// The original terminal attempt state/epoch/era remains the authority; a cut refuses
/// this owner, requiring an explicit successor over already-persisted remaining scope.
pub(super) async fn retire_terminal_roots(
    client: &Surreal<Client>,
    owner: TerminalAttempt,
    roots: Vec<RecordId>,
    limit: usize,
) -> Result<RetirementProgress, ModelError> {
    retire_roots_for_owner(
        client,
        EffectOwner::TerminalAttempt(owner),
        roots,
        limit,
        RetirementMode::RegisterTerminal,
    )
    .await
}
enum RetirementMode {
    Advance,
    RegisterTerminal,
}
async fn retire_roots_for_owner(
    client: &Surreal<Client>,
    owner: EffectOwner,
    roots: Vec<RecordId>,
    limit: usize,
    mode: RetirementMode,
) -> Result<RetirementProgress, ModelError> {
    retire_named_for_owner(
        client,
        owner,
        fresh_identity("retirement-invocation")?,
        None,
        roots,
        limit,
        mode,
    )
    .await
}
async fn retire_named_for_owner(
    client: &Surreal<Client>,
    owner: EffectOwner,
    identity: ContentHash,
    expected_generation: Option<ContentHash>,
    mut roots: Vec<RecordId>,
    limit: usize,
    mode: RetirementMode,
) -> Result<RetirementProgress, ModelError> {
    if limit == 0 {
        return Err(ModelError::Invalid(
            "bounded retirement effect limit".into(),
        ));
    }
    roots.sort();
    roots.dedup();
    let issuance = IssuanceEra::capture(client).await?;
    if expected_generation.is_some_and(|expected| expected != issuance.generation) {
        return Err(ModelError::Conflict(
            "native retirement expected installation generation",
        ));
    }
    let job = RecordId::new("native_retirement", identity.hex());
    let digest = ContentHash::of(
        &serde_json::to_vec(&("native-retirement/v4", &roots)).map_err(ModelError::codec)?,
    );
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert("root_digest", digest.hex());
    bindings.insert("issuance", issuance.era);
    bindings.insert("issuance_generation", issuance.generation.hex());
    bindings.insert("root_count", roots.len());
    bindings.insert("require_admission", matches!(mode, RetirementMode::Advance));
    effect_for_owner(client,owner,"IF $__generation!=$issuance_generation OR $__era!=$issuance OR ($require_admission AND !$__admission.admission_open) { THROW 'native retirement issuance changed'; }; fn::native_touch($job); LET $prior=SELECT * FROM ONLY $job FOR UPDATE; IF $prior=NONE { CREATE $job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$root_digest,lineage=$job,state='registering',root_count=$root_count,roots_registered=0,examined=0,retired=0,revision=0 RETURN NONE; } ELSE IF $prior.generation!=$__generation OR $prior.era!=$__era OR $prior.epoch<=0 OR $prior.root_digest!=$root_digest OR $prior.root_count!=$root_count OR $prior.lineage!=$job OR $prior.successor!=NONE OR $prior.state NOT IN ['registering','open','done'] { THROW 'native retirement exact invocation changed'; };",bindings).await.map_err(|error|authorization::lifecycle_error("retirement invocation",identity,issuance,error))?;
    let result = async {
        if record(client, &job).await?.get("state") == Some(&Value::String("done".into())) {
            return progress(client, identity).await;
        }
        register_retirement_roots(client, identity, roots).await?;
        if matches!(mode, RetirementMode::Advance) {
            resume_retirement(client, identity, limit).await
        } else {
            progress(client, identity).await
        }
    }
    .await;
    if result.is_err() {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.committed("native retirement invocation", identity.hex());
        return lctx_model::domain::completion::complete(result, completion);
    }
    result
}

/// Retry only the exact original canonical root set. Until all bounded registration pages
/// commit, resume_retirement cannot claim or delete any object in the partial invocation.
pub async fn register_retirement_roots(
    client: &Surreal<Client>,
    identity: ContentHash,
    mut roots: Vec<RecordId>,
) -> Result<(), ModelError> {
    roots.sort();
    roots.dedup();
    let job = RecordId::new("native_retirement", identity.hex());
    let row = record(client, &job).await?;
    let digest = ContentHash::of(
        &serde_json::to_vec(&("native-retirement/v4", &roots)).map_err(ModelError::codec)?,
    );
    if hash_field(&row, "root_digest")? != digest
        || usize::try_from(int_field(&row, "root_count")?).map_err(ModelError::codec)?
            != roots.len()
    {
        return Err(ModelError::Conflict(
            "native retirement original roots changed",
        ));
    }
    if row.get("state") == Some(&Value::String("open".into())) {
        return Ok(());
    }
    if row.get("state") != Some(&Value::String("registering".into())) {
        return Err(ModelError::Conflict(
            "native retirement registration fenced",
        ));
    }
    let mut registered =
        usize::try_from(int_field(&row, "roots_registered")?).map_err(ModelError::codec)?;
    if registered > roots.len() {
        return Err(ModelError::Schema("native retirement registration count"));
    }
    for window in roots[registered..].chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        enqueue_roots(
            client,
            &job,
            window,
            i64::try_from(registered).map_err(ModelError::codec)?,
        )
        .await?;
        registered += window.len();
    }
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    effect_for_owner(client,EffectOwner::RetirementSetup(job),"IF $__owner.state!='registering' OR $__owner.roots_registered!=$__owner.root_count { THROW 'native retirement roots incomplete'; }; UPDATE $job SET state='open',revision+=1 RETURN NONE",bindings).await
}

async fn claim(client: &Surreal<Client>, job: &RecordId, item: &Object) -> Result<(), ModelError> {
    let Some(Value::RecordId(object)) = item.get("object") else {
        return Err(ModelError::Schema("retirement object"));
    };
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert(
        "item",
        item.get("id")
            .cloned()
            .ok_or(ModelError::Schema("retirement item"))?,
    );
    bindings.insert("object", object.clone());
    bindings.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"fn::native_backup_clear(); fn::native_touch($item); fn::native_touch($object); LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.object!=$object OR $queued.successor!=NONE OR $queued.state NOT IN ['pending','retained'] { THROW 'native retirement item not claimable'; }; IF $before.incarnation!=$queued.incarnation OR $before.phase='retired' { UPDATE $item SET state='superseded' RETURN NONE; } ELSE { IF $before.phase='retiring' OR (record::table($object)='native_product' AND (SELECT VALUE released FROM ONLY $object)!=true) OR array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0 { UPDATE $item SET state='retained' RETURN NONE; UPDATE $job SET examined+=1,revision+=1 RETURN NONE; } ELSE { UPDATE $guard SET phase='retiring',retiring_item=$item,revision+=1 RETURN NONE; UPDATE $item SET state='retiring',cutoff=$before.tombstone_revision RETURN NONE; UPDATE $job SET revision+=1 RETURN NONE; }; };",bindings).await
}

async fn outgoing_page(
    client: &Surreal<Client>,
    object: &RecordId,
    limit: usize,
) -> Result<Vec<Object>, ModelError> {
    let mut response=client.query("SELECT id,owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$object ORDER BY object LIMIT $limit")
        .bind(("object",object.clone())).bind(("limit",limit)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let rows = crate::loader::NativeWindows::new(rows.into_iter().map(Value::Object).collect())
        .next()
        .transpose()?
        .unwrap_or_default();
    rows.into_iter()
        .map(|value| match value {
            Value::Object(row) => Ok(row),
            _ => Err(ModelError::Schema("retirement outgoing hold")),
        })
        .collect()
}
async fn nominate_page(
    client: &Surreal<Client>,
    job: &RecordId,
    lineage: &RecordId,
    item: &Object,
    holds: Vec<Object>,
) -> Result<(Variables, usize), ModelError> {
    let Some(Value::RecordId(object)) = item.get("object") else {
        return Err(ModelError::Schema("retirement object"));
    };
    let children = holds
        .iter()
        .map(|hold| match hold.get("object") {
            Some(Value::RecordId(id)) => Ok(id.clone()),
            _ => Err(ModelError::Schema("retirement child")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let incarnations = incarnations(client, &children).await?;
    let mut rows = Vec::new();
    for hold in holds {
        let Some(Value::RecordId(child)) = hold.get("object") else {
            return Err(ModelError::Schema("retirement child"));
        };
        let guard = guard_id(&Value::RecordId(child.clone()))?;
        let incarnation = incarnations[&guard];
        let mut row = hold.clone();
        row.insert("item", item_id(lineage, child, incarnation)?);
        row.insert("guard", guard);
        row.insert("incarnation", incarnation);
        rows.push(Value::Object(row));
    }
    let rows = crate::loader::NativeWindows::new(rows)
        .next()
        .transpose()?
        .unwrap_or_default();
    let count = rows.len();
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert("lineage", lineage.clone());
    bindings.insert(
        "item",
        item.get("id")
            .cloned()
            .ok_or(ModelError::Schema("retirement item"))?,
    );
    bindings.insert("object", object.clone());
    bindings.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
    bindings.insert("rows", rows);
    Ok((bindings, count))
}
async fn commit_page(
    client: &Surreal<Client>,
    job: &RecordId,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"IF $__owner.lineage!=$lineage { THROW 'native retirement lineage changed'; }; fn::native_backup_clear(); fn::native_touch($item); fn::native_touch($object); LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.object!=$object OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$item OR $before.incarnation!=$queued.incarnation { THROW 'native retirement page incarnation fenced'; }; FOR $row IN $rows { LET $hold_id=$row.id; LET $hold=SELECT * FROM ONLY $hold_id FOR UPDATE; IF $hold=NONE OR $hold.owner!=$object OR $hold.object!=$row.object { THROW 'native retirement page ownership changed'; }; fn::native_incidence($hold); fn::native_touch($job); fn::native_touch($row.item); LET $child_guard=$row.guard; LET $child=SELECT * FROM ONLY $child_guard FOR UPDATE; IF ($child.incarnation ?? 1)!=$row.incarnation { THROW 'native retirement child incarnation changed'; }; IF $child=NONE { CREATE $child_guard SET revision=0,retired=false,tombstone_revision=0,phase='active',incarnation=1 RETURN NONE; }; LET $child_item=$row.item; LET $existing=SELECT * FROM ONLY $child_item FOR UPDATE; IF $existing=NONE { CREATE $child_item SET job=$job,object=$row.object,incarnation=$row.incarnation,state='pending' RETURN NONE; }; DELETE $hold_id RETURN NONE; }; UPDATE $job SET revision+=1 RETURN NONE;",bindings).await
}
async fn page(
    client: &Surreal<Client>,
    job: &RecordId,
    lineage: &RecordId,
    item: &Object,
    holds: Vec<Object>,
) -> Result<usize, ModelError> {
    let (bindings, count) = nominate_page(client, job, lineage, item, holds).await?;
    commit_page(client, job, bindings).await?;
    Ok(count)
}
async fn finalize(
    client: &Surreal<Client>,
    job: &RecordId,
    item: &Object,
) -> Result<(), ModelError> {
    let Some(Value::RecordId(object)) = item.get("object") else {
        return Err(ModelError::Schema("retirement object"));
    };
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert(
        "item",
        item.get("id")
            .cloned()
            .ok_or(ModelError::Schema("retirement item"))?,
    );
    bindings.insert("object", object.clone());
    bindings.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
    effect_for_owner(client,EffectOwner::Retirement(job.clone()),"fn::native_backup_clear(); fn::native_touch($item); fn::native_touch($object); LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $queued.job!=$job OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$item OR $before.incarnation!=$queued.incarnation OR $queued.cutoff=NONE { THROW 'native retirement finalize incarnation fenced'; }; IF array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0 { THROW 'native retirement final holds remain'; }; LET $object_row=SELECT * FROM ONLY $object FOR UPDATE; IF $object_row!=NONE { fn::native_incidence($object_row); }; DELETE $object RETURN NONE; UPDATE $guard SET phase='retired',retired=true,retiring_item=NONE,tombstone_revision+=1,revision+=1 RETURN NONE; UPDATE $item SET state='done' RETURN NONE; UPDATE $job SET examined+=1,retired+=1,revision+=1 RETURN NONE;",bindings).await
}
async fn progress(
    client: &Surreal<Client>,
    identity: ContentHash,
) -> Result<RetirementProgress, ModelError> {
    let job = RecordId::new("native_retirement", identity.hex());
    let mut response=client.query(format!("SELECT examined,retired,state FROM $job; {PENDING_HEAD_SQL}; SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retained' ORDER BY id LIMIT 128"))
        .bind(("job",job)).bind(("limit",crate::loader::NATIVE_WINDOW_ROWS)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict("native retirement progress"));
    };
    let identities = |values: Vec<RecordId>| {
        values
            .into_iter()
            .map(|value| serde_json::to_string(&value).map_err(ModelError::codec))
            .collect::<Result<Vec<_>, _>>()
    };
    let pending: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    let retiring: Vec<Object> = response.take(2).map_err(ModelError::codec)?;
    let remaining = bounded_pending_head(pending, retiring, crate::loader::NATIVE_WINDOW_ROWS)?
        .into_iter().map(|row| match row.get("object") {
            Some(Value::RecordId(object)) => Ok(object.clone()),
            _ => Err(ModelError::Schema("retirement progress object")),
        }).collect::<Result<Vec<_>, _>>()?;
    let retained: Vec<RecordId> = response.take(3).map_err(ModelError::codec)?;
    if remaining.is_empty()
        && retained.is_empty()
        && row.get("state") == Some(&Value::String("open".into()))
    {
        let job = RecordId::new("native_retirement", identity.hex());
        let mut vars = Variables::new();
        vars.insert("job", job.clone());
        effect_for_owner(client,EffectOwner::Retirement(job),"IF array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state IN ['pending','retained','retiring'] LIMIT 1)>0 { THROW 'retirement completion raced'; }; UPDATE $job SET state='done',revision+=1 RETURN NONE",vars).await?;
    }
    Ok(RetirementProgress {
        identity,
        examined: u64::try_from(int_field(row, "examined")?).map_err(ModelError::codec)?,
        retired: u64::try_from(int_field(row, "retired")?).map_err(ModelError::codec)?,
        remaining: identities(remaining)?,
        retained: identities(retained)?,
    })
}

/// Discovery is a bounded hint, not authorization. Pending work always restarts at
/// the head; a retained cursor must never skip a newly nominated pending child.
const PENDING_HEAD_SQL: &str = "SELECT id,job,object,incarnation,state,successor FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='pending' ORDER BY id LIMIT $limit; SELECT id,job,object,incarnation,state,successor FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retiring' ORDER BY id LIMIT $limit";

async fn pending_page(
    client: &Surreal<Client>,
    job: &RecordId,
    limit: usize,
) -> Result<std::collections::VecDeque<Object>, ModelError> {
    let limit = limit.min(crate::loader::NATIVE_WINDOW_ROWS);
    let mut response = client
        // Exact equality prefixes let the native index supply order and LIMIT.
        // The combined IN predicate instead scans all job states and sorts them.
        .query(PENDING_HEAD_SQL)
        .bind(("job", job.clone()))
        .bind(("limit", limit))
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let pending: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let retiring: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    bounded_pending_head(pending, retiring, limit)
}

fn bounded_pending_head(pending: Vec<Object>, retiring: Vec<Object>, limit: usize)
    -> Result<std::collections::VecDeque<Object>, ModelError> {
    let rows = merge_pending_head(pending, retiring, limit)?;
    let rows = crate::loader::NativeWindows::new(rows.into_iter().map(Value::Object).collect())
        .next().transpose()?.unwrap_or_default();
    rows.into_iter().map(|value| match value {
        Value::Object(row) => Ok(row),
        _ => Err(ModelError::Schema("retirement pending discovery")),
    }).collect()
}

fn merge_pending_head(
    pending: Vec<Object>,
    retiring: Vec<Object>,
    limit: usize,
) -> Result<Vec<Object>, ModelError> {
    if pending.len() > limit || retiring.len() > limit {
        return Err(ModelError::Schema("retirement discovery exceeds exact state limit"));
    }
    // At most 2k moved payloads, not a retained queue. A transition between the
    // two observations can report the same item in both states; neither state
    // observation authorizes an effect. The caller still refetches the item.
    let mut head = std::collections::BTreeMap::new();
    for rows in [pending, retiring] {
        for window in crate::loader::NativeWindows::new(rows.into_iter().map(Value::Object).collect()) {
            for value in window? {
                let Value::Object(row) = value else {
                    return Err(ModelError::Schema("retirement pending discovery"));
                };
                let Some(Value::RecordId(id)) = row.get("id") else {
                    return Err(ModelError::Schema("retirement item identity"));
                };
                match head.entry(id.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => { entry.insert(row); }
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        if ["job", "object", "incarnation"].into_iter()
                            .any(|field| entry.get().get(field) != row.get(field)) {
                            return Err(ModelError::Conflict("retirement duplicate discovery changed"));
                        }
                    }
                }
            }
        }
    }
    Ok(head.into_values().take(limit).collect())
}

fn lineage(row: &Object) -> Result<&RecordId, ModelError> {
    match row.get("lineage") {
        Some(Value::RecordId(lineage)) if lineage.table.as_str() == "native_retirement" => Ok(lineage),
        _ => Err(ModelError::Schema("native retirement invocation lineage")),
    }
}

pub async fn resume_retirement(
    client: &Surreal<Client>,
    identity: ContentHash,
    limit: usize,
) -> Result<RetirementProgress, ModelError> {
    if limit == 0 {
        return Err(ModelError::Invalid(
            "bounded retirement effect limit".into(),
        ));
    }
    let job = RecordId::new("native_retirement", identity.hex());
    let mut row = record(client, &job).await?;
    if row.get("state") == Some(&Value::String("recovering".into())) {
        finish_recovery(client, &job).await?;
        row = record(client, &job).await?;
    }
    if row.get("state") == Some(&Value::String("registering".into())) {
        return Err(ModelError::Conflict(
            "native retirement registration incomplete; retry exact roots",
        ));
    }
    if row.get("state") == Some(&Value::String("done".into())) {
        return progress(client, identity).await;
    }
    let current = IssuanceEra::capture(client).await?;
    if hash_field(&row, "generation")? != current.generation
        || int_field(&row, "era")? != current.era
        || row.get("state") != Some(&Value::String("open".into()))
    {
        return Err(ModelError::Conflict(
            "native retirement needs explicit maintenance successor",
        ));
    }
    let lineage = lineage(&row)?.clone();
    let mut response=client.query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND owner=$job LIMIT 1").bind(("job",job.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let pending: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    if !pending.is_empty() {
        return Err(ModelError::Conflict(
            "native retirement previous page unresolved",
        ));
    }
    let mut spent = 0usize;
    let mut retained_after = Value::None;
    let mut pending = std::collections::VecDeque::new();
    'resume: while spent < limit {
        if pending.is_empty() {
            pending = pending_page(client, &job, limit - spent).await?;
        }
        let discovered_pending = !pending.is_empty();
        let candidate = if let Some(item) = pending.pop_front() {
            item
        } else {
            let mut response=client.query("SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retained' AND ($after=NONE OR id>$after) ORDER BY id LIMIT 1")
                .bind(("job",job.clone())).bind(("after",retained_after.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
            let Some(item) = rows.into_iter().next() else { break; };
            item
        };
        let Some(Value::RecordId(id)) = candidate.get("id") else {
            return Err(ModelError::Schema("retirement item identity"));
        };
        if !discovered_pending {
            retained_after = Value::RecordId(id.clone());
        }
        let mut item = record(client, id).await?;
        if item.get("job") != Some(&Value::RecordId(job.clone()))
            || item.get("object") != candidate.get("object")
            || item.get("incarnation") != candidate.get("incarnation")
            || item.get("successor").is_some_and(|value| *value != Value::None)
        {
            return Err(ModelError::Conflict("native retirement discovered candidate changed"));
        }
        let eligible = match item.get("state") {
            Some(Value::String(state)) if discovered_pending => matches!(state.as_str(), "pending" | "retiring"),
            Some(Value::String(state)) => state == "retained",
            _ => false,
        };
        if !eligible { continue; }
        if item.get("state") != Some(&Value::String("retiring".into())) {
            claim(client, &job, &item).await?;
            spent += 1;
            item = record(client, id).await?;
        }
        if item.get("state") != Some(&Value::String("retiring".into())) {
            continue;
        }
        let Some(Value::RecordId(object)) = item.get("object") else {
            return Err(ModelError::Schema("retirement object"));
        };
        loop {
            if spent >= limit {
                break;
            }
            let allowance = ((limit - spent) / 2).min(crate::loader::NATIVE_WINDOW_ROWS);
            // One edge unit comprises a durable child nomination and one exact hold deletion.
            let holds = outgoing_page(client, object, allowance.max(1)).await?;
            if holds.is_empty() {
                finalize(client, &job, &item).await?;
                spent += 1;
                break;
            }
            if allowance == 0 {
                break 'resume;
            }
            spent += page(client, &job, &lineage, &item, holds).await? * 2;
            // Children may sort before every buffered candidate. Their durable
            // nominations are authoritative; these discovery hints are discarded.
            pending.clear();
        }
    }
    progress(client, identity).await
}
pub async fn retire(client: &Surreal<Client>, object: RecordId) -> Result<(), ModelError> {
    let mut progress =
        retire_reachable(client, vec![object], crate::loader::NATIVE_WINDOW_ROWS).await?;
    loop {
        if !progress.retained.is_empty() {
            return Err(ModelError::Conflict("native retirement reachable"));
        }
        if progress.remaining.is_empty() {
            return Ok(());
        }
        progress =
            resume_retirement(client, progress.identity, crate::loader::NATIVE_WINDOW_ROWS).await?;
    }
}
/// Claim only durable remaining items after the old era is permanently closed. An old
/// invocation cannot regain authority over a reactivated object or reset a done item.
pub async fn recover_retirement(
    client: &Surreal<Client>,
    identity: ContentHash,
) -> Result<ContentHash, ModelError> {
    let issuance = IssuanceEra::capture(client).await?;
    let old = RecordId::new("native_retirement", identity.hex());
    let before = record(client, &old).await?;
    let existing = match before.get("successor") {
        Some(Value::RecordId(id)) => Some(id.clone()),
        _ => None,
    };
    let next = if let Some(id) = &existing {
        let row = record(client, id).await?;
        if hash_field(&row, "generation")? != issuance.generation
            || int_field(&row, "era")? != issuance.era
            || !matches!(row.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"open"|"recovering"))
        {
            return Err(ModelError::Conflict(
                "recover named retirement successor first",
            ));
        }
        hash_record(id)?
    } else {
        fresh_identity("retirement-maintenance-successor")?
    };
    let successor = RecordId::new("native_retirement", next.hex());
    let mut bindings = Variables::new();
    bindings.insert("old", old.clone());
    bindings.insert("next", successor.clone());
    bindings.insert("era", issuance.era);
    coordination::bind_all_stripes(&mut bindings);
    if existing.is_none() {
        let recovery = format!(
            "{} {}",
            coordination::ALL_STRIPES_GUARD,
            "IF $__era!=$era OR $__installation.admission_open { THROW 'retirement recovery requires original current-era drained maintenance'; }; LET $before=SELECT * FROM ONLY $old FOR UPDATE; IF $before=NONE OR $before.state NOT IN ['open','recovering'] OR $before.generation!=$__generation OR $before.epoch<=0 OR $before.successor!=NONE OR $before.era>($__installation.closed_through ?? 0) { THROW 'retirement predecessor not fenced'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 { THROW 'retirement recovery has unresolved effects'; }; fn::native_incidence($before); fn::native_touch($next); CREATE $next SET generation=$__generation,era=$era,epoch=$__epoch,root_digest=$before.root_digest,lineage=$before.lineage,state='recovering',predecessor=$old,recovery_source=$old,root_count=$before.root_count,roots_registered=$before.roots_registered,examined=$before.examined,retired=$before.retired,revision=0 RETURN NONE; UPDATE $old SET successor=$next,state='superseded' RETURN NONE"
        );
        effect(client, None, &recovery, bindings).await?;
    }
    finish_recovery(client, &successor).await.map_err(|error| {
        authorization::lifecycle_error("retirement successor", next, issuance, error)
    })?;
    Ok(next)
}
async fn finish_recovery(client: &Surreal<Client>, successor: &RecordId) -> Result<(), ModelError> {
    // Walk the durable predecessor chain one source at a time. A later era cut may fence
    // a half-rehomed successor; its successor still sees every older remaining source.
    loop {
        let row = record(client, successor).await?;
        if row.get("state") == Some(&Value::String("open".into())) {
            return Ok(());
        }
        if row.get("state") != Some(&Value::String("recovering".into())) {
            return Err(ModelError::Conflict("native retirement recovery fenced"));
        }
        let Some(Value::RecordId(source)) = row.get("recovery_source") else {
            return Err(ModelError::Schema("native retirement recovery source"));
        };
        let mut response=client.query("SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$source AND state IN ['pending','retained','retiring'] LIMIT 128").bind(("source",source.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let items: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        let mut bindings = Variables::new();
        bindings.insert("source", source.clone());
        bindings.insert("next", successor.clone());
        bindings.insert("items", items);
        effect_for_owner(client,EffectOwner::RetirementSetup(successor.clone()),"IF $__owner.state!='recovering' OR $__owner.recovery_source!=$source { THROW 'native retirement recovery source raced'; }; fn::native_touch($source); LET $origin=SELECT * FROM ONLY $source FOR UPDATE; fn::native_incidence($origin); IF $origin=NONE OR $origin.state!='superseded' { THROW 'native retirement recovery source not fenced'; }; FOR $id IN $items { LET $item=SELECT * FROM ONLY $id FOR UPDATE; IF $item.job=$source AND $item.state IN ['pending','retained','retiring'] { fn::native_incidence($item); fn::native_touch($next); UPDATE $id SET job=$next RETURN NONE; }; }; IF array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$source AND state IN ['pending','retained','retiring'] LIMIT 1)=0 { IF $origin.predecessor=NONE { UPDATE $next SET state='open',recovery_source=NONE,predecessor=NONE,revision+=1 RETURN NONE; } ELSE { UPDATE $next SET recovery_source=$origin.predecessor,revision+=1 RETURN NONE; }; };",bindings).await?;
    }
}

fn hash_record(id: &RecordId) -> Result<ContentHash, ModelError> {
    let surrealdb::types::RecordIdKey::String(key) = &id.key else {
        return Err(ModelError::Schema("retirement successor identity"));
    };
    Ok(ContentHash(
        hex::decode(key)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("retirement successor width"))?,
    ))
}

#[cfg(test)]
mod frontier_controls {
    use super::*;

    fn guard(id: &RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", id.clone());
        row.insert("revision", 0i64);
        row.insert("retired", false);
        row.insert("phase", "active");
        row.insert("incarnation", 1i64);
        row.insert("tombstone_revision", 0i64);
        Value::Object(row)
    }

    #[test]
    fn invocation_lineage_is_required_and_nominal() {
        let mut row = Object::new();
        assert!(lineage(&row).is_err());
        row.insert("lineage", RecordId::new("native_guard", "foreign"));
        assert!(lineage(&row).is_err());
        let expected = RecordId::new("native_retirement", "origin");
        row.insert("lineage", expected.clone());
        assert_eq!(lineage(&row).unwrap(), &expected);
    }

    #[test]
    fn pending_head_merges_exact_state_limits_and_checks_duplicate_immutable_hints() {
        let candidate = |name: &str, state: &str, incarnation: i64| {
            let mut row = Object::new();
            row.insert("id", RecordId::new("native_retirement_item", name));
            row.insert("job", RecordId::new("native_retirement", "job"));
            row.insert("object", RecordId::new("native_guard", name));
            row.insert("incarnation", incarnation);
            row.insert("state", state);
            row
        };
        let ids = |rows: Vec<Object>| rows.into_iter().map(|row| row.get("id").cloned().unwrap()).collect::<Vec<_>>();
        let expected = |names: &[&str]| names.iter().map(|name| Value::RecordId(RecordId::new("native_retirement_item", *name))).collect::<Vec<_>>();
        assert_eq!(ids(merge_pending_head(
            vec![candidate("b", "pending", 1), candidate("d", "pending", 1)],
            vec![candidate("a", "retiring", 1), candidate("c", "retiring", 1), candidate("d", "retiring", 1)], 3,
        ).unwrap()), expected(&["a", "b", "c"]));
        // Each state must supply k: dividing its allowance could omit this head.
        assert_eq!(ids(merge_pending_head(
            vec![candidate("a", "pending", 1), candidate("b", "pending", 1), candidate("c", "pending", 1)],
            vec![candidate("z", "retiring", 1)], 3,
        ).unwrap()), expected(&["a", "b", "c"]));
        assert!(merge_pending_head(
            vec![candidate("a", "pending", 1)], vec![candidate("a", "retiring", 2)], 1,
        ).is_err());
        assert!(merge_pending_head(vec![candidate("a", "pending", 1)], Vec::new(), 0).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pending_head_discovers_lower_child_and_stops_before_unaffordable_edge() {
        let path = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config).await.unwrap();
        let identity = fresh_identity("retirement-head-control").unwrap();
        let job = RecordId::new("native_retirement", identity.hex());
        let parent = RecordId::new("native_guard", format!("head_parent_{}", identity.hex()));
        let parent_item = item_id(&job, &parent, 1).unwrap();
        let child = (0..).map(|index| RecordId::new("native_guard", format!("head_child_{}_{index}", identity.hex())))
            .find(|child| item_id(&job, child, 1).unwrap() < parent_item).unwrap();
        ensure_rows(&client, None, vec![guard(&parent), guard(&child)]).await.unwrap();
        hold(&client, None, parent.clone(), vec![child.clone()]).await.unwrap();
        retire_named_for_owner(&client, EffectOwner::Installation, identity, None,
            vec![parent.clone()], 1, RetirementMode::RegisterTerminal).await.unwrap();

        let first = resume_retirement(&client, identity, 2).await.unwrap();
        assert_eq!(first.retired, 0);
        assert!(!first.remaining.is_empty(), "one remaining unit cannot nominate and delete an edge");
        assert_eq!(outgoing_page(&client, &parent, 1).await.unwrap().len(), 1);
        let final_progress = resume_retirement(&client, identity, 5).await.unwrap();
        assert_eq!(final_progress.retired, 2, "new lower-key child must advance in the same resume");
        assert!(final_progress.remaining.is_empty());
        assert!(final_progress.retained.is_empty());
        assert_eq!(record(&client, &job).await.unwrap().get("state"), Some(&crate::control::test_value("done")));
        client.invalidate().await.unwrap();
    }
}

#[cfg(test)]
mod incarnation_controls {
    use super::*;
    #[test]
    fn batch_incarnations_keep_missing_default_and_reject_foreign_duplicate_or_zero_guards() {
        let one = RecordId::new("native_guard", "one");
        let two = RecordId::new("native_guard", "two");
        let mut observed = Object::new();
        observed.insert("id", two.clone());
        observed.insert("incarnation", 7i64);
        let requested = [one.clone(), two.clone()].into_iter().collect();
        let actual = decode_incarnations(requested, vec![observed.clone()]).unwrap();
        assert_eq!(actual[&one], 1);
        assert_eq!(actual[&two], 7);
        assert!(
            decode_incarnations([one.clone()].into_iter().collect(), vec![observed.clone()])
                .is_err()
        );
        assert!(
            decode_incarnations(
                [two.clone()].into_iter().collect(),
                vec![observed.clone(), observed.clone()]
            )
            .is_err()
        );
        observed.insert("incarnation", 0i64);
        assert!(decode_incarnations([two].into_iter().collect(), vec![observed]).is_err());
    }

    async fn nominated_page_snapshot(
        client: &Surreal<Client>,
        job: &RecordId,
        parent: &RecordId,
    ) -> Vec<Vec<Object>> {
        let mut response=client.query("SELECT * FROM $job; SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$parent ORDER BY object; SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job ORDER BY id")
            .bind(("job",job.clone())).bind(("parent",parent.clone())).await.unwrap().check().unwrap();
        (0..3)
            .map(|index| response.take::<Vec<Object>>(index).unwrap())
            .collect()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn changed_child_incarnation_after_batch_nomination_rolls_back_all_child_items_and_hold_deletions()
     {
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let nonce = fresh_identity("retirement-batch-incarnation-control").unwrap();
        let parent = RecordId::new("native_guard", format!("batch_parent_{}", nonce.hex()));
        let children = (0..2)
            .map(|index| {
                RecordId::new(
                    "native_guard",
                    format!("batch_child_{}_{index}", nonce.hex()),
                )
            })
            .collect::<Vec<_>>();
        let values = std::iter::once(parent.clone())
            .chain(children.iter().cloned())
            .map(|id| {
                let mut row = Object::new();
                row.insert("id", id);
                row.insert("revision", 0i64);
                row.insert("retired", false);
                row.insert("phase", "active");
                row.insert("incarnation", 1i64);
                row.insert("tombstone_revision", 0i64);
                Value::Object(row)
            })
            .collect();
        ensure_rows(&client, None, values).await.unwrap();
        hold(&client, None, parent.clone(), children.clone())
            .await
            .unwrap();
        let first = retire_reachable(&client, vec![parent.clone()], 1)
            .await
            .unwrap();
        assert_eq!(first.retired, 0);
        let job = RecordId::new("native_retirement", first.identity.hex());
        let root_item = record(&client, &item_id(&job, &parent, 1).unwrap())
            .await
            .unwrap();
        let holds = outgoing_page(&client, &parent, 2).await.unwrap();
        assert_eq!(holds.len(), 2);
        let before_lineage = nominated_page_snapshot(&client, &job, &parent).await;
        let foreign_lineage = RecordId::new("native_retirement", "foreign_lineage");
        let (foreign_bindings, _) = nominate_page(&client, &job, &foreign_lineage, &root_item, holds.clone()).await.unwrap();
        let error = commit_page(&client, &job, foreign_bindings).await.unwrap_err();
        assert!(error.to_string().contains("native retirement lineage changed"), "{error}");
        assert_eq!(nominated_page_snapshot(&client, &job, &parent).await, before_lineage,
            "foreign prepared lineage cannot change parent, children or holds");
        let (bindings, count) = nominate_page(&client, &job, &job, &root_item, holds.clone())
            .await
            .unwrap();
        assert_eq!(count, 2);
        let Value::RecordId(second) = holds[1].get("object").unwrap() else {
            panic!("second owned child");
        };
        let second_guard = guard_id(&Value::RecordId(second.clone())).unwrap();
        // Qualification-only controlled state change on this freshly owned guard after
        // the actual production batch read. This is not a claim of a legal reactivation
        // while the incoming hold exists, nor a global hook or concurrent-writer model.
        client
            .query("UPDATE $guard SET incarnation+=1,revision+=1 RETURN NONE")
            .bind(("guard", second_guard.clone()))
            .await
            .unwrap()
            .check()
            .unwrap();
        let before = nominated_page_snapshot(&client, &job, &parent).await;
        assert_eq!(
            before[2].len(),
            1,
            "only the root exists before either child nomination"
        );
        let error = commit_page(&client, &job, bindings).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("native retirement child incarnation changed"),
            "the second batched child must reject its stale incarnation: {error}"
        );
        assert_eq!(
            nominated_page_snapshot(&client, &job, &parent).await,
            before,
            "the first child nomination and hold deletion must roll back with the second child's refusal"
        );
        // Retry from an actual new nomination rather than reusing stale batch values.
        let holds = outgoing_page(&client, &parent, 2).await.unwrap();
        assert_eq!(page(&client, &job, &job, &root_item, holds).await.unwrap(), 2);
        let snapshot = nominated_page_snapshot(&client, &job, &parent).await;
        assert!(snapshot[1].is_empty());
        assert_eq!(snapshot[2].len(), 3);
        let second_item = item_id(&job, second, 2).unwrap();
        assert!(snapshot[2].iter().any(|row| row.get("id")
            == Some(&Value::RecordId(second_item.clone()))
            && row.get("incarnation") == Some(&crate::control::test_value(2i64))));
        let mut progress = resume_retirement(&client, first.identity, 16)
            .await
            .unwrap();
        while !progress.remaining.is_empty() {
            progress = resume_retirement(&client, first.identity, 16)
                .await
                .unwrap();
        }
        assert_eq!(progress.retired, 3);
        assert!(progress.retained.is_empty());
        client.invalidate().await.unwrap();
    }
}

#[cfg(test)]
mod cleanup_nomination_controls {
    use super::*;
    fn hold(owner: &RecordId, object: &RecordId, name: &str) -> Object {
        let mut row = Object::new();
        row.insert("id", RecordId::new("native_hold", name));
        row.insert("owner", owner.clone());
        row.insert("object", object.clone());
        row
    }
    fn physical(object: &RecordId, incarnation: i64) -> Object {
        let mut row = Object::new();
        row.insert("id", guard_id(&Value::RecordId(object.clone())).unwrap());
        row.insert("incarnation", incarnation);
        row.insert("phase", "active");
        row.insert("retired", false);
        row.insert("revision", 3i64);
        row.insert("tombstone_revision", 2i64);
        row
    }
    #[test]
    fn nominated_roots_use_actual_complete_canonical_root_set_and_incarnations() {
        let owner = RecordId::new("native_attempt", "failed-no-product");
        let one = RecordId::new("original", "one");
        let two = RecordId::new("compiler_contribution", "incomplete");
        let job = RecordId::new("native_retirement", "owned-job");
        let holds = vec![hold(&owner, &two, "two"), hold(&owner, &one, "one")];
        let guards = vec![physical(&one, 4), physical(&two, 9)];
        let bindings =
            cleanup_nomination_bindings(&owner, holds.clone(), guards.clone(), Some(job.clone()))
                .unwrap();
        assert_eq!(
            bindings.get("n09_root_count"),
            Some(&crate::control::test_value(2i64))
        );
        assert_eq!(
            bindings.get("n09_holds"),
            Some(&crate::control::test_value(holds))
        );
        let mut canonical = vec![one.clone(), two.clone()];
        canonical.sort();
        let digest =
            ContentHash::of(&serde_json::to_vec(&("native-retirement/v4", canonical)).unwrap());
        assert_eq!(
            bindings.get("n09_root_digest"),
            Some(&crate::control::test_value(digest.hex()))
        );
        let Some(Value::Array(roots)) = bindings.get("n09_roots") else {
            panic!("actual root envelope")
        };
        for root in roots {
            let Value::Object(root) = root else {
                panic!("actual root")
            };
            let Some(Value::RecordId(object)) = root.get("object") else {
                panic!("actual object")
            };
            let incarnation = if object == &one { 4 } else { 9 };
            assert_eq!(
                root.get("incarnation"),
                Some(&crate::control::test_value(incarnation))
            );
            assert_eq!(
                root.get("item"),
                Some(&Value::RecordId(
                    item_id(&job, object, incarnation).unwrap()
                ))
            );
            assert_eq!(
                root.get("guard"),
                Some(&Value::RecordId(
                    guard_id(&Value::RecordId(object.clone())).unwrap()
                ))
            );
            assert!(!root.contains_key("physical"));
        }
        let mut duplicate = vec![hold(&owner, &one, "one"), hold(&owner, &one, "other")];
        assert!(
            cleanup_nomination_bindings(
                &owner,
                duplicate.clone(),
                guards.clone(),
                Some(job.clone())
            )
            .is_err()
        );
        duplicate[1] = hold(&RecordId::new("native_attempt", "foreign"), &two, "two");
        assert!(cleanup_nomination_bindings(&owner, duplicate, guards, Some(job)).is_err());
    }
    #[test]
    fn missing_or_nonactive_original_incarnation_is_never_defaulted() {
        let owner = RecordId::new("native_attempt", "owner");
        let object = RecordId::new("original", "partial");
        let holds = vec![hold(&owner, &object, "held")];
        let job = Some(RecordId::new("native_retirement", "job"));
        assert!(cleanup_nomination_bindings(&owner, holds.clone(), vec![], job.clone()).is_err());
        for (field, value) in [
            ("incarnation", crate::control::test_value(0i64)),
            ("phase", crate::control::test_value("retired")),
            ("phase", crate::control::test_value("retiring")),
            ("retired", crate::control::test_value(true)),
        ] {
            let mut guard = physical(&object, 5);
            guard.insert(field, value);
            assert!(
                cleanup_nomination_bindings(&owner, holds.clone(), vec![guard], job.clone())
                    .is_err()
            );
        }
        let empty = cleanup_nomination_bindings(&owner, vec![], vec![], None).unwrap();
        assert_eq!(empty.get("n09_job"), Some(&Value::None));
        assert_eq!(
            empty.get("n09_root_count"),
            Some(&crate::control::test_value(0i64))
        );
        assert!(cleanup_nomination_bindings(&owner, vec![], vec![], job).is_err());
    }
}
