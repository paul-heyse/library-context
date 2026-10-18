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
/// Cleanup has rechecked exact owner/origin identity and its full hold page,
/// completed distinct attempt cuts and scalar-locked ordinary predicate records.
/// Validate every nomination before the shared bulk predicate increment; then
/// create all roots/items before the caller deletes any actual hold ID. These
/// phases remain in ONE effect transaction and retain no partial setup job.
/// Revision churn on the same active incarnation remains legitimate reuse.
pub(super) const CLEANUP_NOMINATE_VALIDATE: &str = r#"
IF array::len($holds)>0 {
    IF $n09_job=NONE OR $n09_root_count!=array::len($n09_roots) OR $n09_root_count!=array::len($holds) OR array::len(array::distinct($n09_roots.map(|$root| $root.item)))!=$n09_root_count { THROW 'native cleanup nomination complete root count'; };
    LET $n09_existing_job=SELECT * FROM ONLY $n09_job FOR UPDATE;
    IF $n09_existing_job!=NONE { THROW 'native cleanup nomination job collision'; };
    FOR $n09_root IN $n09_roots {
        LET $n09_guard_id=$n09_root.guard;
        LET $n09_physical=SELECT * FROM ONLY $n09_guard_id FOR UPDATE;
        IF $n09_physical=NONE OR $n09_physical.phase!='active' OR $n09_physical.retired!=false OR $n09_root.incarnation<=0 OR $n09_physical.incarnation!=$n09_root.incarnation { THROW 'native cleanup nominated incarnation changed'; };
        LET $n09_item_id=$n09_root.item;
        IF (SELECT VALUE id FROM ONLY $n09_item_id)!=NONE { THROW 'native cleanup nomination item collision'; };
    };
};
"#;
pub(super) const CLEANUP_NOMINATE_WRITE: &str = r#"
IF array::len($holds)>0 {
    CREATE $n09_job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$n09_root_digest,lineage=$n09_job,state='open',root_count=$n09_root_count,roots_registered=$n09_root_count,examined=0,retired=0,revision=0 RETURN NONE;
    LET $n09_item_rows=$n09_roots.map(|$root| {id:$root.item,job:$n09_job,object:$root.object,incarnation:$root.incarnation,state:'pending'});
    INSERT INTO native_retirement_item $n09_item_rows RETURN NONE;
};
"#;

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

/// One canonical, row/byte-bounded candidate payload. Discovery is only a hint;
/// every immutable identity and mutable state is checked again inside the effect.
fn candidate_bindings(
    job: &RecordId,
    lineage: &RecordId,
    items: Vec<Object>,
) -> Result<(Variables, usize), ModelError> {
    if items.is_empty() || items.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("retirement candidate page width"));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for item in items {
        let (Some(Value::RecordId(id)), Some(Value::RecordId(object))) =
            (item.get("id"), item.get("object"))
        else {
            return Err(ModelError::Schema("retirement candidate identity"));
        };
        let incarnation = int_field(&item, "incarnation")?;
        if incarnation <= 0
            || item.get("job") != Some(&Value::RecordId(job.clone()))
            || id != &item_id(lineage, object, incarnation)?
            || !seen.insert(id.clone())
            || item
                .get("successor")
                .is_some_and(|value| *value != Value::None)
            || !matches!(item.get("state"), Some(Value::String(state)) if matches!(state.as_str(), "pending" | "retained" | "retiring"))
        {
            return Err(ModelError::Conflict("retirement candidate correspondence"));
        }
        let mut row = Object::new();
        row.insert("id", id.clone());
        row.insert("object", object.clone());
        row.insert("incarnation", incarnation);
        row.insert("state", item.get("state").cloned().unwrap());
        row.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
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
    bindings.insert("rows", rows);
    Ok((bindings, count))
}

// Scalar ONLY locks are deliberate: the streaming engine does not lock record-ID
// arrays. Classify the whole bounded page before changing any item. The shared
// backup gate and full native effect protocol are acquired once for this page.
const RETIRE_CLAIM_PAGE: &str = r#"
IF $__owner.lineage!=$lineage { THROW 'native retirement lineage changed'; };
fn::native_backup_clear();
LET $classified=$rows.map(|$row| {
    LET $id=$row.id;
    LET $guard=$row.guard;
    LET $object=$row.object;
    LET $queued=SELECT * FROM ONLY $id FOR UPDATE;
    LET $before=SELECT * FROM ONLY $guard FOR UPDATE;
    IF $queued.job!=$job OR $queued.object!=$object OR $queued.incarnation!=$row.incarnation OR $queued.successor!=NONE OR $queued.state!=$row.state OR $queued.state NOT IN ['pending','retained'] { THROW 'native retirement candidate changed'; };
    LET $held=array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0;
    LET $unreleased=record::table($object)='native_product' AND (SELECT VALUE released FROM ONLY $object FOR UPDATE)!=true;
    RETURN {id:$id,object:$object,guard:$guard,before:$before,superseded:($before.incarnation!=$queued.incarnation OR $before.phase='retired'),retained:($before.phase='retiring' OR $unreleased OR $held)};
});
FOR $row IN $classified {
    fn::native_touch($row.id);
    fn::native_touch($row.object);
    IF $row.superseded {
        UPDATE $row.id SET state='superseded' RETURN NONE;
    } ELSE IF $row.retained {
        UPDATE $row.id SET state='retained' RETURN NONE;
    } ELSE {
        UPDATE $row.guard SET phase='retiring',retiring_item=$row.id,revision+=1 RETURN NONE;
        UPDATE $row.id SET state='retiring',cutoff=$row.before.tombstone_revision RETURN NONE;
    };
};
LET $retained_count=array::len($classified.filter(|$row| !$row.superseded AND $row.retained));
LET $advanced_count=array::len($classified.filter(|$row| !$row.superseded));
IF $advanced_count>0 { UPDATE $job SET examined+=$retained_count,revision+=$advanced_count RETURN NONE; };
"#;
async fn claim_page(
    client: &Surreal<Client>,
    job: &RecordId,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(
        client,
        EffectOwner::Retirement(job.clone()),
        RETIRE_CLAIM_PAGE,
        bindings,
    )
    .await
}

/// Refresh a bounded hint page in one crossing, retaining its canonical order.
/// A terminal state is skipped, but identity or successor drift is never ignored.
async fn refresh_candidates(
    client: &Surreal<Client>,
    job: &RecordId,
    candidates: Vec<Object>,
    discovered_pending: bool,
) -> Result<Vec<Object>, ModelError> {
    let ids = candidates
        .iter()
        .map(|row| match row.get("id") {
            Some(Value::RecordId(id)) => Ok(id.clone()),
            _ => Err(ModelError::Schema("retirement item identity")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut response = client
        .query("SELECT * FROM $ids")
        .bind(("ids", ids))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let mut actual = std::collections::BTreeMap::new();
    for row in rows {
        let Some(Value::RecordId(id)) = row.get("id") else {
            return Err(ModelError::Schema("retirement item identity"));
        };
        if actual.insert(id.clone(), row).is_some() {
            return Err(ModelError::Conflict(
                "retirement duplicate refreshed candidate",
            ));
        }
    }
    let mut refreshed = Vec::new();
    for candidate in candidates {
        let Some(Value::RecordId(id)) = candidate.get("id") else {
            unreachable!()
        };
        let item = actual
            .remove(id)
            .ok_or(ModelError::Conflict("native retirement identity missing"))?;
        if item.get("job") != Some(&Value::RecordId(job.clone()))
            || item.get("object") != candidate.get("object")
            || item.get("incarnation") != candidate.get("incarnation")
            || item
                .get("successor")
                .is_some_and(|value| *value != Value::None)
        {
            return Err(ModelError::Conflict(
                "native retirement discovered candidate changed",
            ));
        }
        let eligible = matches!(item.get("state"), Some(Value::String(state))
            if if discovered_pending { matches!(state.as_str(), "pending" | "retiring") }
               else { matches!(state.as_str(), "retained" | "retiring") });
        if eligible {
            refreshed.push(item);
        }
    }
    if !actual.is_empty() {
        return Err(ModelError::Conflict(
            "retirement foreign refreshed candidate",
        ));
    }
    Ok(refreshed)
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
    if holds.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("retirement hold page width"));
    }
    let children = holds
        .iter()
        .map(|hold| match hold.get("object") {
            Some(Value::RecordId(id)) => Ok(id.clone()),
            _ => Err(ModelError::Schema("retirement child")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let incarnations = incarnations(client, &children).await?;
    retirement_page_bindings(job, lineage, item, holds, &incarnations)
}
fn retirement_page_bindings(
    job: &RecordId,
    lineage: &RecordId,
    item: &Object,
    holds: Vec<Object>,
    incarnations: &std::collections::BTreeMap<RecordId, i64>,
) -> Result<(Variables, usize), ModelError> {
    use std::collections::{BTreeMap, BTreeSet};
    if holds.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("retirement hold page width"));
    }
    let Some(Value::RecordId(object)) = item.get("object") else {
        return Err(ModelError::Schema("retirement object"));
    };
    let Some(Value::RecordId(root_item)) = item.get("id") else {
        return Err(ModelError::Schema("retirement item"));
    };
    let mut rows = Vec::new();
    let mut seen_holds = BTreeSet::new();
    for hold in holds {
        let Some(Value::RecordId(hold_id)) = hold.get("id") else {
            return Err(ModelError::Schema("retirement hold identity"));
        };
        if hold_id.table.as_str() != "native_hold"
            || hold.get("owner") != Some(&Value::RecordId(object.clone()))
            || !seen_holds.insert(hold_id.clone())
        {
            return Err(ModelError::Conflict("retirement hold page correspondence"));
        }
        let Some(Value::RecordId(child)) = hold.get("object") else {
            return Err(ModelError::Schema("retirement child"));
        };
        let guard = guard_id(&Value::RecordId(child.clone()))?;
        let incarnation = *incarnations
            .get(&guard)
            .ok_or(ModelError::Schema("retirement child incarnation absent"))?;
        if incarnation <= 0 {
            return Err(ModelError::Schema("retirement child incarnation"));
        }
        let child_item = item_id(lineage, child, incarnation)?;
        let mut row = hold.clone();
        row.insert("item", child_item);
        row.insert("guard", guard);
        row.insert("incarnation", incarnation);
        rows.push(Value::Object(row));
    }
    // Preserve the existing row/byte-selected prefix before deriving any
    // supplemental effect set. The unselected tail remains held for a later page.
    let rows = crate::loader::NativeWindows::new(rows)
        .next()
        .transpose()?
        .unwrap_or_default();
    let count = rows.len();
    let mut guard_children = BTreeMap::new();
    let mut item_children = BTreeMap::new();
    for value in &rows {
        let Value::Object(row) = value else {
            return Err(ModelError::Schema("retirement prepared row"));
        };
        let (
            Some(Value::RecordId(child)),
            Some(Value::RecordId(guard)),
            Some(Value::RecordId(child_item)),
        ) = (row.get("object"), row.get("guard"), row.get("item"))
        else {
            return Err(ModelError::Schema("retirement prepared correspondence"));
        };
        let incarnation = int_field(row, "incarnation")?;
        if guard_children
            .insert(guard, (child, incarnation))
            .is_some_and(|prior| prior != (child, incarnation))
        {
            return Err(ModelError::Conflict(
                "retirement child guard correspondence",
            ));
        }
        if item_children
            .insert(child_item, (child, incarnation))
            .is_some_and(|prior| prior != (child, incarnation))
        {
            return Err(ModelError::Conflict("retirement child item correspondence"));
        }
    }
    drop((guard_children, item_children));
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert("lineage", lineage.clone());
    bindings.insert("item", root_item.clone());
    bindings.insert("object", object.clone());
    bindings.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
    bindings.insert("rows", rows);
    Ok((bindings, count))
}
// The streaming executor locks scalar record targets, not record-ID arrays.
// Map each bounded distinct ID to one ONLY locked read, filtering absent NONE
// results; the subsequent comparisons run only over these local object values.
// All page checks precede the batched mutations, so
// a late changed child still rejects every nomination and hold deletion.
// Recovery moves unfinished items to the successor, but terminal items may keep
// an earlier job. Their immutable object/incarnation and actual job lineage must
// match; their existing job, state and successor are never reset by nomination.
const RETIRE_HOLD_PAGE: &str = r#"
IF $__owner.lineage!=$lineage { THROW 'native retirement lineage changed'; };
fn::native_backup_clear();
fn::native_touch($item);
fn::native_touch($object);
LET $queued=SELECT * FROM ONLY $item FOR UPDATE;
LET $before=SELECT * FROM ONLY $guard FOR UPDATE;
IF $queued.job!=$job OR $queued.object!=$object OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$item OR $before.incarnation!=$queued.incarnation { THROW 'native retirement page incarnation fenced'; };
LET $hold_ids=$rows.map(|$row| $row.id);
LET $child_guard_ids=array::distinct($rows.map(|$row| $row.guard));
LET $child_item_ids=array::distinct($rows.map(|$row| $row.item));
LET $guard_expectations=array::distinct($rows.map(|$row| {id:$row.guard,incarnation:$row.incarnation}));
LET $guard_rows=$child_guard_ids.map(|$id| {id:$id,revision:0,retired:false,tombstone_revision:0,phase:'active',incarnation:1});
LET $item_rows=array::distinct($rows.map(|$row| {id:$row.item,job:$job,object:$row.object,incarnation:$row.incarnation,state:'pending'}));
LET $predicate_targets=array::distinct(array::flatten($rows.map(|$row| [$row.id,$row.object,$row.item]))).filter(|$record| $record NOT IN [$job,$lineage,$object,$item]);
LET $predicate_rows=$predicate_targets.map(|$record| {id:type::record('native_predicate',[$record]),revision:1});
LET $predicate_ids=$predicate_rows.map(|$row| $row.id);
LET $locked_holds=$hold_ids.map(|$id| (SELECT id,owner,object FROM ONLY $id FOR UPDATE)).filter(|$row| $row!=NONE);
LET $actual_holds=SELECT id,owner,object FROM $locked_holds ORDER BY id;
LET $expected_holds=SELECT id,owner,object FROM $rows ORDER BY id;
IF $actual_holds!=$expected_holds { THROW 'native retirement page ownership changed'; };
LET $actual_guards=$child_guard_ids.map(|$id| (SELECT id,incarnation FROM ONLY $id FOR UPDATE)).filter(|$row| $row!=NONE);
LET $existing_items=$child_item_ids.map(|$id| (SELECT id,job,object,incarnation FROM ONLY $id FOR UPDATE)).filter(|$row| $row!=NONE);
LET $existing_job_ids=array::distinct($existing_items.map(|$row| $row.job));
LET $existing_jobs=$existing_job_ids.map(|$id| (SELECT id,lineage FROM ONLY $id FOR UPDATE)).filter(|$row| $row!=NONE);
LET $actual_predicates=SELECT id,revision FROM $predicate_ids;
LET $existing_predicate_ids=$actual_predicates.map(|$row|$row.id);
LET $missing_predicate_rows=$predicate_rows.filter(|$row|$row.id NOT IN $existing_predicate_ids);
LET $actual_guard_ids=$actual_guards.map(|$row| $row.id);
LET $existing_item_ids=$existing_items.map(|$row| $row.id);
IF array::len(SELECT * FROM $actual_guards WHERE {id:id,incarnation:incarnation} NOT IN $guard_expectations)>0 OR array::len(SELECT * FROM $guard_expectations WHERE incarnation!=1 AND id NOT IN $actual_guard_ids)>0 { THROW 'native retirement child incarnation changed'; };
LET $expected_items=SELECT id,object,incarnation FROM $item_rows;
IF array::len(SELECT * FROM $existing_items WHERE {id:id,object:object,incarnation:incarnation} NOT IN $expected_items)>0 { THROW 'native retirement child item changed'; };
IF array::len($existing_jobs)!=array::len($existing_job_ids) OR array::len(SELECT * FROM $existing_jobs WHERE lineage!=$lineage)>0 { THROW 'native retirement child item lineage changed'; };
IF array::len(SELECT * FROM $actual_predicates WHERE revision>=9223372036854775807)>0 { THROW 'native predicate revision exhausted'; };
LET $new_guards=SELECT * FROM $guard_rows WHERE id NOT IN $actual_guard_ids;
LET $new_items=SELECT * FROM $item_rows WHERE id NOT IN $existing_item_ids;
IF array::len($existing_predicate_ids)>0 { UPDATE $existing_predicate_ids SET revision+=1 RETURN NONE; };
IF array::len($missing_predicate_rows)>0 { INSERT INTO native_predicate $missing_predicate_rows RETURN NONE; };
IF array::len($new_guards)>0 { INSERT INTO native_guard $new_guards RETURN NONE; };
IF array::len($new_items)>0 { INSERT INTO native_retirement_item $new_items RETURN NONE; };
DELETE $hold_ids RETURN NONE;
UPDATE $job SET revision+=1 RETURN NONE;
"#;
async fn commit_page(
    client: &Surreal<Client>,
    job: &RecordId,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(
        client,
        EffectOwner::Retirement(job.clone()),
        RETIRE_HOLD_PAGE,
        bindings,
    )
    .await
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
// Every selected incarnation and both hold directions are checked before ANY
// terminal deletion. A late changed row refuses the entire page atomically.
const RETIRE_FINALIZE_PAGE: &str = r#"
IF $__owner.lineage!=$lineage { THROW 'native retirement lineage changed'; };
fn::native_backup_clear();
LET $classified=$rows.map(|$row| {
    LET $id=$row.id;
    LET $guard=$row.guard;
    LET $object=$row.object;
    LET $queued=SELECT * FROM ONLY $id FOR UPDATE;
    LET $before=SELECT * FROM ONLY $guard FOR UPDATE;
    IF $row.state!='retiring' OR $queued.job!=$job OR $queued.object!=$object OR $queued.incarnation!=$row.incarnation OR $queued.state!='retiring' OR $queued.successor!=NONE OR $before.phase!='retiring' OR $before.retiring_item!=$id OR $before.incarnation!=$queued.incarnation OR $queued.cutoff=NONE { THROW 'native retirement finalize incarnation fenced'; };
    IF array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0 { THROW 'native retirement final holds remain'; };
    IF record::table($object)='compiler_contribution' AND array::len(SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$object LIMIT 1)>0 { THROW 'native contribution logical inventory remains'; };
    // Keep only the complete native_incidence input, never compiler/original bodies.
    LET $physical=SELECT id,attempt,owner,object,effect,contribution,completion,job,release_job,lineage,predecessor,successor,recovery_source FROM ONLY $object FOR UPDATE;
    RETURN {id:$id,guard:$guard,object:$object,physical:$physical};
});
FOR $row IN $classified {
    fn::native_touch($row.id);
    fn::native_touch($row.object);
    IF $row.physical!=NONE { fn::native_incidence($row.physical); };
    DELETE $row.object RETURN NONE;
    UPDATE $row.guard SET phase='retired',retired=true,retiring_item=NONE,tombstone_revision+=1,revision+=1 RETURN NONE;
    UPDATE $row.id SET state='done' RETURN NONE;
};
UPDATE $job SET examined+=array::len($rows),retired+=array::len($rows),revision+=array::len($rows) RETURN NONE;
"#;
async fn finalize_page(
    client: &Surreal<Client>,
    job: &RecordId,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(
        client,
        EffectOwner::Retirement(job.clone()),
        RETIRE_FINALIZE_PAGE,
        bindings,
    )
    .await
}

/// Locators are a contribution-owned inventory, removed in bounded pages under
/// its claimed physical parent. They never acquire independent retirement items.
async fn retire_locator_page(
    client: &Surreal<Client>,
    job: &RecordId,
    lineage: &RecordId,
    item: &Object,
    limit: usize,
) -> Result<usize, ModelError> {
    let Some(Value::RecordId(object)) = item.get("object") else {
        return Err(ModelError::Schema("locator retirement parent"));
    };
    if object.table.as_str() != "compiler_contribution" {
        return Ok(0);
    }
    let mut response=client.query("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$parent ORDER BY relation,semantic_key LIMIT $limit").bind(("parent",object.clone())).bind(("limit",limit.min(crate::loader::NATIVE_WINDOW_ROWS))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let locators: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
    if locators.is_empty() {
        return Ok(0);
    }
    let count = locators.len();
    let (mut bindings, _) = candidate_bindings(job, lineage, vec![item.clone()])?;
    bindings.insert("locator_page", locators);
    let sql = "LET $row=$rows[0]; LET $object=$row.object; LET $guard=$row.guard; LET $item=$row.id; LET $actual=SELECT * FROM ONLY $guard FOR UPDATE; LET $queued=SELECT * FROM ONLY $item FOR UPDATE; LET $parent=SELECT * FROM ONLY $object FOR UPDATE; IF $actual.phase!='retiring' OR $actual.retiring_item!=$item OR $actual.incarnation!=$row.incarnation OR $queued.job!=$job OR $queued.state!='retiring' OR $queued.object!=$object OR $__owner.lineage!=$lineage { THROW 'native locator retirement parent changed'; }; LET $ids=$locator_page.map(|$entry|$entry.id); LET $current=SELECT * FROM $ids; IF array::len($current)!=array::len($locator_page) OR array::len($current.filter(|$entry|$entry.contribution!=$object OR $entry NOT IN $locator_page))>0 { THROW 'native locator retirement inventory changed'; }; fn::native_touch($object); DELETE $ids RETURN NONE; UPDATE $job SET examined+=array::len($locator_page),revision+=1 RETURN NONE;";
    effect_for_owner(client, EffectOwner::Retirement(job.clone()), sql, bindings).await?;
    Ok(count)
}
/// This read classifies useful work only; terminal authorization still checks the
/// exact item, guard and both hold directions in RETIRE_FINALIZE_PAGE.
async fn leaf_candidates(
    client: &Surreal<Client>,
    items: &[Object],
) -> Result<std::collections::BTreeSet<RecordId>, ModelError> {
    let objects = items
        .iter()
        .map(|item| match item.get("object") {
            Some(Value::RecordId(object)) => Ok(object.clone()),
            _ => Err(ModelError::Schema("retirement object")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut response = client.query("RETURN $objects.filter(|$object| array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object LIMIT 1)=0 AND (record::table($object)!='compiler_contribution' OR array::len(SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$object LIMIT 1)=0))")
        .bind(("objects", objects.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let leaves: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    let expected = objects
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected = std::collections::BTreeSet::new();
    for leaf in leaves {
        if !expected.contains(&leaf) || !selected.insert(leaf) {
            return Err(ModelError::Conflict("retirement leaf correspondence"));
        }
    }
    Ok(selected)
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
        .into_iter()
        .map(|row| match row.get("object") {
            Some(Value::RecordId(object)) => Ok(object.clone()),
            _ => Err(ModelError::Schema("retirement progress object")),
        })
        .collect::<Result<Vec<_>, _>>()?;
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
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let pending: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let retiring: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    bounded_pending_head(pending, retiring, limit)
}

fn bounded_pending_head(
    pending: Vec<Object>,
    retiring: Vec<Object>,
    limit: usize,
) -> Result<std::collections::VecDeque<Object>, ModelError> {
    let rows = merge_pending_head(pending, retiring, limit)?;
    let rows = crate::loader::NativeWindows::new(rows.into_iter().map(Value::Object).collect())
        .next()
        .transpose()?
        .unwrap_or_default();
    rows.into_iter()
        .map(|value| match value {
            Value::Object(row) => Ok(row),
            _ => Err(ModelError::Schema("retirement pending discovery")),
        })
        .collect()
}

fn merge_pending_head(
    pending: Vec<Object>,
    retiring: Vec<Object>,
    limit: usize,
) -> Result<Vec<Object>, ModelError> {
    if pending.len() > limit || retiring.len() > limit {
        return Err(ModelError::Schema(
            "retirement discovery exceeds exact state limit",
        ));
    }
    // At most 2k moved payloads, not a retained queue. A transition between the
    // two observations can report the same item in both states; neither state
    // observation authorizes an effect. The caller still refetches the item.
    let mut head = std::collections::BTreeMap::new();
    for rows in [pending, retiring] {
        for window in
            crate::loader::NativeWindows::new(rows.into_iter().map(Value::Object).collect())
        {
            for value in window? {
                let Value::Object(row) = value else {
                    return Err(ModelError::Schema("retirement pending discovery"));
                };
                let Some(Value::RecordId(id)) = row.get("id") else {
                    return Err(ModelError::Schema("retirement item identity"));
                };
                match head.entry(id.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(row);
                    }
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        if ["job", "object", "incarnation"]
                            .into_iter()
                            .any(|field| entry.get().get(field) != row.get(field))
                        {
                            return Err(ModelError::Conflict(
                                "retirement duplicate discovery changed",
                            ));
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
        Some(Value::RecordId(lineage)) if lineage.table.as_str() == "native_retirement" => {
            Ok(lineage)
        }
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
        // Reserve useful room for terminal work while allowing a one-unit claim.
        // This is a bounded wave, never a retained complete retirement queue.
        let width = (limit - spent)
            .div_ceil(2)
            .min(crate::loader::NATIVE_WINDOW_ROWS);
        let candidates = if discovered_pending {
            pending
                .drain(..pending.len().min(width))
                .collect::<Vec<_>>()
        } else {
            // Retained priority/cursor semantics stay singleton (ADR-0170).
            let mut response=client.query("SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job AND state='retained' AND ($after=NONE OR id>$after) ORDER BY id LIMIT 1")
                .bind(("job",job.clone())).bind(("after",retained_after.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
            let Some(item) = rows.into_iter().next() else {
                break;
            };
            retained_after = item
                .get("id")
                .cloned()
                .ok_or(ModelError::Schema("retirement item identity"))?;
            vec![item]
        };
        let candidates = refresh_candidates(client, &job, candidates, discovered_pending).await?;
        let claims = candidates
            .iter()
            .filter(|item| item.get("state") != Some(&Value::String("retiring".into())))
            .cloned()
            .collect::<Vec<_>>();
        if !claims.is_empty() {
            let (bindings, count) = candidate_bindings(&job, &lineage, claims)?;
            claim_page(client, &job, bindings).await?;
            spent += count;
        }
        // Reobserve the bounded wave after claim. A byte-limited claim prefix may
        // leave unclaimed candidates; they remain durable at the next head.
        let items = refresh_candidates(client, &job, candidates, discovered_pending)
            .await?
            .into_iter()
            .filter(|item| item.get("state") == Some(&Value::String("retiring".into())))
            .collect::<Vec<_>>();
        if spent == limit || items.is_empty() {
            continue;
        }
        let leaves = leaf_candidates(client, &items).await?;
        let terminal = items.iter().filter(|item| matches!(item.get("object"), Some(Value::RecordId(object)) if leaves.contains(object)))
            .take(limit - spent).cloned().collect::<Vec<_>>();
        let mut finalized = std::collections::BTreeSet::new();
        if !terminal.is_empty() {
            let (bindings, count) = candidate_bindings(&job, &lineage, terminal)?;
            let Some(Value::Array(rows)) = bindings.get("rows") else {
                unreachable!()
            };
            for row in rows.iter() {
                let Value::RecordId(id) = row.get("id") else {
                    unreachable!()
                };
                finalized.insert(id.clone());
            }
            finalize_page(client, &job, bindings).await?;
            spent += count;
        }
        for item in items {
            if matches!(item.get("id"), Some(Value::RecordId(id)) if finalized.contains(id)) {
                continue;
            }
            let Some(Value::RecordId(object)) = item.get("object") else {
                return Err(ModelError::Schema("retirement object"));
            };
            if object.table.as_str() == "compiler_contribution" {
                while spent < limit {
                    let count =
                        retire_locator_page(client, &job, &lineage, &item, limit - spent).await?;
                    spent += count;
                    if count == 0 {
                        break;
                    }
                }
                if spent >= limit {
                    break 'resume;
                }
            }
            let mut nominated = false;
            loop {
                if spent >= limit {
                    break;
                }
                let allowance = ((limit - spent) / 2).min(crate::loader::NATIVE_WINDOW_ROWS);
                // One edge unit comprises a durable child nomination and one exact hold deletion.
                let holds = outgoing_page(client, object, allowance.max(1)).await?;
                if holds.is_empty() {
                    let (bindings, count) = candidate_bindings(&job, &lineage, vec![item.clone()])?;
                    finalize_page(client, &job, bindings).await?;
                    spent += count;
                    break;
                }
                if allowance == 0 {
                    break 'resume;
                }
                spent += page(client, &job, &lineage, &item, holds).await? * 2;
                // Children may sort before every buffered candidate. Their durable
                // nominations are authoritative; these discovery hints are discarded.
                pending.clear();
                nominated = true;
            }
            if nominated {
                // Resume from the newly durable head before other wave hints.
                break;
            }
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
        let ids = |rows: Vec<Object>| {
            rows.into_iter()
                .map(|row| row.get("id").cloned().unwrap())
                .collect::<Vec<_>>()
        };
        let expected = |names: &[&str]| {
            names
                .iter()
                .map(|name| Value::RecordId(RecordId::new("native_retirement_item", *name)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(merge_pending_head(
                vec![candidate("b", "pending", 1), candidate("d", "pending", 1)],
                vec![
                    candidate("a", "retiring", 1),
                    candidate("c", "retiring", 1),
                    candidate("d", "retiring", 1)
                ],
                3,
            )
            .unwrap()),
            expected(&["a", "b", "c"])
        );
        // Each state must supply k: dividing its allowance could omit this head.
        assert_eq!(
            ids(merge_pending_head(
                vec![
                    candidate("a", "pending", 1),
                    candidate("b", "pending", 1),
                    candidate("c", "pending", 1)
                ],
                vec![candidate("z", "retiring", 1)],
                3,
            )
            .unwrap()),
            expected(&["a", "b", "c"])
        );
        assert!(
            merge_pending_head(
                vec![candidate("a", "pending", 1)],
                vec![candidate("a", "retiring", 2)],
                1,
            )
            .is_err()
        );
        assert!(merge_pending_head(vec![candidate("a", "pending", 1)], Vec::new(), 0).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pending_head_discovers_lower_child_and_stops_before_unaffordable_edge() {
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let identity = fresh_identity("retirement-head-control").unwrap();
        let job = RecordId::new("native_retirement", identity.hex());
        let parent = RecordId::new("native_guard", format!("head_parent_{}", identity.hex()));
        let parent_item = item_id(&job, &parent, 1).unwrap();
        let child = (0..)
            .map(|index| {
                RecordId::new(
                    "native_guard",
                    format!("head_child_{}_{index}", identity.hex()),
                )
            })
            .find(|child| item_id(&job, child, 1).unwrap() < parent_item)
            .unwrap();
        ensure_rows(&client, None, vec![guard(&parent), guard(&child)])
            .await
            .unwrap();
        hold(&client, None, parent.clone(), vec![child.clone()])
            .await
            .unwrap();
        retire_named_for_owner(
            &client,
            EffectOwner::Installation,
            identity,
            None,
            vec![parent.clone()],
            1,
            RetirementMode::RegisterTerminal,
        )
        .await
        .unwrap();

        let first = resume_retirement(&client, identity, 2).await.unwrap();
        assert_eq!(first.retired, 0);
        assert!(
            !first.remaining.is_empty(),
            "one remaining unit cannot nominate and delete an edge"
        );
        assert_eq!(outgoing_page(&client, &parent, 1).await.unwrap().len(), 1);
        let final_progress = resume_retirement(&client, identity, 5).await.unwrap();
        assert_eq!(
            final_progress.retired, 2,
            "new lower-key child must advance in the same resume"
        );
        assert!(final_progress.remaining.is_empty());
        assert!(final_progress.retained.is_empty());
        assert_eq!(
            record(&client, &job).await.unwrap().get("state"),
            Some(&crate::control::test_value("done"))
        );
        client.invalidate().await.unwrap();
    }
}

#[cfg(test)]
mod incarnation_controls {
    use super::*;
    use futures::FutureExt;
    fn page_fixture() -> (
        RecordId,
        RecordId,
        Object,
        Vec<Object>,
        std::collections::BTreeMap<RecordId, i64>,
    ) {
        let job = RecordId::new("native_retirement", "page-job");
        let lineage = RecordId::new("native_retirement", "page-origin");
        let parent = RecordId::new("native_guard", "page-parent");
        let mut item = Object::new();
        item.insert("id", item_id(&lineage, &parent, 1).unwrap());
        item.insert("object", parent.clone());
        // One child overlaps a predicate already touched by the owner wrapper.
        // The other keeps a composite native key, never a string rendering.
        let children = [
            lineage.clone(),
            RecordId::new(
                "native_guard",
                surrealdb::types::RecordIdKey::Array(
                    vec![Value::RecordId(parent.clone()), test_value(7i64)].into(),
                ),
            ),
        ];
        let mut incarnations = std::collections::BTreeMap::new();
        let holds = children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                incarnations.insert(
                    guard_id(&Value::RecordId(child.clone())).unwrap(),
                    index as i64 + 1,
                );
                let mut hold = Object::new();
                hold.insert("id", hold_record_id(&parent, &child).unwrap());
                hold.insert("owner", parent.clone());
                hold.insert("object", child);
                hold
            })
            .collect();
        (job, lineage, item, holds, incarnations)
    }
    fn bound_rows(bindings: &Variables) -> &[Value] {
        let Some(Value::Array(values)) = bindings.get("rows") else {
            panic!("canonical native row payload");
        };
        values.as_slice()
    }
    #[test]
    fn retirement_page_prepares_one_nominal_payload_and_preserves_recovery_lineage() {
        let (job, lineage, item, holds, incarnations) = page_fixture();
        let (bindings, count) =
            retirement_page_bindings(&job, &lineage, &item, holds.clone(), &incarnations).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            bindings
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["job", "lineage", "item", "object", "guard", "rows"]
                .into_iter()
                .collect(),
            "one canonical payload crosses the wire, without repeated supplemental ID/row sets"
        );
        let rows = bound_rows(&bindings);
        assert_eq!(rows.len(), holds.len());
        for (actual, hold) in rows.iter().zip(&holds) {
            let Value::Object(actual) = actual else {
                panic!("canonical row");
            };
            for field in ["id", "owner", "object"] {
                assert_eq!(actual.get(field), hold.get(field));
            }
            let Some(Value::RecordId(child)) = actual.get("object") else {
                panic!("nominal child");
            };
            let guard = guard_id(&Value::RecordId(child.clone())).unwrap();
            assert_eq!(actual.get("guard"), Some(&Value::RecordId(guard.clone())));
            assert_eq!(
                actual.get("incarnation"),
                Some(&test_value(incarnations[&guard]))
            );
            assert_eq!(
                actual.get("item"),
                Some(&Value::RecordId(
                    item_id(&lineage, child, incarnations[&guard]).unwrap()
                ))
            );
        }
        let Value::Object(composite) = &rows[1] else {
            panic!("composite child");
        };
        assert!(
            matches!(composite.get("object"), Some(Value::RecordId(child)) if matches!(&child.key, surrealdb::types::RecordIdKey::Array(_))),
            "composite nominal keys are retained without SQL/string rendering"
        );
        let successor = RecordId::new("native_retirement", "page-successor");
        let (next, next_count) =
            retirement_page_bindings(&successor, &lineage, &item, holds, &incarnations).unwrap();
        assert_eq!(next_count, count);
        assert_eq!(
            bound_rows(&next),
            rows,
            "recovery preserves lineage-derived child identities"
        );
        assert_ne!(
            next.get("job"),
            bindings.get("job"),
            "the current job remains an explicit fresh binding"
        );
        surrealdb_syn::parse(RETIRE_HOLD_PAGE).unwrap();
    }
    #[test]
    fn retirement_page_supplemental_effects_follow_only_the_existing_byte_selected_prefix() {
        let (job, lineage, item, _, _) = page_fixture();
        let Some(Value::RecordId(parent)) = item.get("object") else {
            panic!("fixture parent");
        };
        let mut incarnations = std::collections::BTreeMap::new();
        let holds = (0..2)
            .map(|index| {
                let child = RecordId::new(
                    "large_child",
                    format!(
                        "{index}{}",
                        "x".repeat(lctx_model::domain::resources::TRANSFER_BYTES / 2)
                    ),
                );
                incarnations.insert(guard_id(&Value::RecordId(child.clone())).unwrap(), 1);
                let mut hold = Object::new();
                hold.insert("id", hold_record_id(parent, &child).unwrap());
                hold.insert("owner", parent.clone());
                hold.insert("object", child);
                hold
            })
            .collect::<Vec<_>>();
        let first_hold = holds[0].get("id").cloned().unwrap();
        let second_hold = holds[1].get("id").cloned().unwrap();
        let second_child = holds[1].get("object").cloned().unwrap();
        let (mut bindings, count) =
            retirement_page_bindings(&job, &lineage, &item, holds, &incarnations).unwrap();
        assert_eq!(
            count, 1,
            "the unchanged byte window, rather than row count, selects this page"
        );
        let [Value::Object(row)] = bound_rows(&bindings) else {
            panic!("one selected row");
        };
        assert_eq!(row.get("id"), Some(&first_hold));
        assert_ne!(row.get("id"), Some(&second_hold));
        assert_ne!(
            row.get("object"),
            Some(&second_child),
            "the tail stays held without supplemental effects"
        );
        // The real lowering preflights the complete atomic request with protocol
        // controls and the pinned protobuf codec. Binding-only size is not proof
        // that a single wide ID can advance through the actual owner request.
        bindings.insert("__object_expectations", Vec::<Value>::new());
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash::of(b"retirement-wire-control"),
                era: 1,
            },
            operation: ContentHash::of(b"retirement-wire-operation"),
            request: ContentHash::of(&serde_json::to_vec(&(RETIRE_HOLD_PAGE, &bindings)).unwrap()),
        };
        authorization::owner_effect_plan(
            request,
            &EffectOwner::Retirement(job),
            RETIRE_HOLD_PAGE,
            bindings,
            false,
        )
        .unwrap();
    }
    #[test]
    fn retirement_page_refuses_duplicate_or_foreign_holds_missing_incarnations_and_excessive_rows()
    {
        let (job, lineage, item, holds, incarnations) = page_fixture();
        assert!(
            retirement_page_bindings(
                &job,
                &lineage,
                &item,
                vec![holds[0].clone(), holds[0].clone()],
                &incarnations
            )
            .is_err()
        );
        let mut foreign = holds[0].clone();
        foreign.insert("owner", job.clone());
        assert!(
            retirement_page_bindings(&job, &lineage, &item, vec![foreign], &incarnations).is_err()
        );
        assert!(
            retirement_page_bindings(
                &job,
                &lineage,
                &item,
                holds.clone(),
                &std::collections::BTreeMap::new()
            )
            .is_err()
        );
        let invalid = incarnations
            .keys()
            .cloned()
            .map(|guard| (guard, 0))
            .collect();
        assert!(retirement_page_bindings(&job, &lineage, &item, holds.clone(), &invalid).is_err());
        assert!(
            retirement_page_bindings(
                &job,
                &lineage,
                &item,
                vec![holds[0].clone(); crate::loader::NATIVE_WINDOW_ROWS + 1],
                &incarnations
            )
            .is_err()
        );
    }
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
        let mut response=client.query("SELECT * FROM $job; SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$nominated_owner ORDER BY object; SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job ORDER BY id")
            .bind(("job",job.clone())).bind(("nominated_owner",parent.clone())).await.unwrap().check().unwrap();
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
        let outcome = std::panic::AssertUnwindSafe(async {
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
        let Some(Value::RecordId(first_child)) = holds[0].get("object") else { panic!("first owned child"); };
        let existing_item = item_id(&job, first_child, 1).unwrap();
        let mut existing = Object::new();
        existing.insert("id", existing_item.clone());
        existing.insert("job", job.clone());
        existing.insert("object", first_child.clone());
        existing.insert("incarnation", 1i64);
        existing.insert("state", "retained");
        ensure_rows(&client, None, vec![Value::Object(existing)]).await.unwrap();
        // One prior nomination must retain its state while the other child is
        // created. Corrupt immutable correspondence in this nonce-owned control
        // first: no earlier valid child may be installed or lose its hold.
        let (prepared, _) = nominate_page(&client, &job, &job, &root_item, holds.clone()).await.unwrap();
        for (field, invalid, original, message) in [
            ("object", Value::RecordId(parent.clone()), Value::RecordId(first_child.clone()), "native retirement child item changed"),
            ("incarnation", test_value(99i64), test_value(1i64), "native retirement child item changed"),
            ("job", Value::RecordId(RecordId::new("native_retirement", "absent_foreign_job")), Value::RecordId(job.clone()), "native retirement child item lineage changed"),
        ] {
            let sql = format!("UPDATE $item SET {field}=$value RETURN NONE");
            client.query(sql.clone()).bind(("item", existing_item.clone())).bind(("value", invalid)).await.unwrap().check().unwrap();
            let before = nominated_page_snapshot(&client, &job, &parent).await;
            let error = commit_page(&client, &job, prepared.clone()).await.unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
            assert_eq!(nominated_page_snapshot(&client, &job, &parent).await, before,
                "wrong existing correspondence refuses the whole mixed page");
            client.query(sql).bind(("item", existing_item.clone())).bind(("value", original)).await.unwrap().check().unwrap();
        }
        let predicate = RecordId::new("native_predicate", surrealdb::types::RecordIdKey::Array(
            vec![Value::RecordId(first_child.clone())].into(),
        ));
        let original_predicate = record(&client, &predicate).await.unwrap();
        client.query("UPDATE $predicate SET revision=9223372036854775807 RETURN NONE")
            .bind(("predicate", predicate.clone())).await.unwrap().check().unwrap();
        let before_exhaustion = nominated_page_snapshot(&client, &job, &parent).await;
        let error = commit_page(&client, &job, prepared).await.unwrap_err();
        assert!(error.to_string().contains("native predicate revision exhausted"), "{error}");
        assert_eq!(nominated_page_snapshot(&client, &job, &parent).await, before_exhaustion,
            "predicate exhaustion refuses every nomination and deletion");
        client.query("UPSERT $predicate CONTENT $row RETURN NONE")
            .bind(("predicate", predicate)).bind(("row", original_predicate)).await.unwrap().check().unwrap();
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
            2,
            "the root and existing retained child precede the missing child nomination"
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
        let existing = snapshot[2].iter().find(|row| row.get("id") == Some(&Value::RecordId(existing_item.clone()))).unwrap();
        assert_eq!(existing.get("state"), Some(&test_value("retained")),
            "nomination must not reset an independently retained existing item");
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
        }).catch_unwind().await;
        let cleanup = client.invalidate().await;
        if let Err(panic) = outcome {
            if let Err(error) = cleanup {
                eprintln!("mixed retirement page session cleanup after assertion panic: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn full_retirement_hold_page_rechecks_all_children_before_atomic_mutation() {
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let outcome = std::panic::AssertUnwindSafe(async {
            let nonce = fresh_identity("full-retirement-hold-page-control").unwrap();
            let parent = RecordId::new("native_guard", format!("full_parent_{}", nonce.hex()));
            let children = (0..crate::loader::NATIVE_WINDOW_ROWS)
                .map(|index| {
                    RecordId::new(
                        "native_guard",
                        format!("full_child_{}_{index:03}", nonce.hex()),
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
            let job = RecordId::new("native_retirement", first.identity.hex());
            let root_item = record(&client, &item_id(&job, &parent, 1).unwrap())
                .await
                .unwrap();
            let holds = outgoing_page(&client, &parent, crate::loader::NATIVE_WINDOW_ROWS)
                .await
                .unwrap();
            let (prepared, count) = nominate_page(&client, &job, &job, &root_item, holds.clone())
                .await
                .unwrap();
            assert_eq!(count, crate::loader::NATIVE_WINDOW_ROWS);
            let Some(Value::RecordId(last)) = holds.last().unwrap().get("object") else {
                panic!("last owned child");
            };
            let last_guard = guard_id(&Value::RecordId(last.clone())).unwrap();
            client
                .query("UPDATE $guard SET incarnation+=1,revision+=1 RETURN NONE")
                .bind(("guard", last_guard.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let before = nominated_page_snapshot(&client, &job, &parent).await;
            let error = commit_page(&client, &job, prepared).await.unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("native retirement child incarnation changed"),
                "{error}"
            );
            assert_eq!(
                nominated_page_snapshot(&client, &job, &parent).await,
                before,
                "the last changed child refuses every effect across the full page"
            );
            let parent_predicate = RecordId::new(
                "native_predicate",
                surrealdb::types::RecordIdKey::Array(vec![Value::RecordId(parent.clone())].into()),
            );
            let before_revision = int_field(
                &record(&client, &parent_predicate).await.unwrap(),
                "revision",
            )
            .unwrap();
            let holds = outgoing_page(&client, &parent, crate::loader::NATIVE_WINDOW_ROWS)
                .await
                .unwrap();
            assert_eq!(
                page(&client, &job, &job, &root_item, holds).await.unwrap(),
                crate::loader::NATIVE_WINDOW_ROWS
            );
            let after_revision = int_field(
                &record(&client, &parent_predicate).await.unwrap(),
                "revision",
            )
            .unwrap();
            assert_eq!(
                after_revision,
                before_revision + 1,
                "the common parent predicate is touched once, not again for each deleted hold"
            );
            let after = nominated_page_snapshot(&client, &job, &parent).await;
            assert!(after[1].is_empty());
            assert_eq!(after[2].len(), crate::loader::NATIVE_WINDOW_ROWS + 1);
            assert!(after[2].iter().any(
                |row| row.get("id") == Some(&Value::RecordId(item_id(&job, last, 2).unwrap()))
            ));
        })
        .catch_unwind()
        .await;
        let cleanup = client.invalidate().await;
        if let Err(panic) = outcome {
            if let Err(error) = cleanup {
                eprintln!("full retirement page session cleanup after assertion panic: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
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

#[cfg(test)]
mod candidate_page_controls {
    use super::*;
    use futures::FutureExt;

    fn candidate(job: &RecordId, lineage: &RecordId, object: &RecordId, state: &str) -> Object {
        let mut row = Object::new();
        row.insert("id", item_id(lineage, object, 1).unwrap());
        row.insert("job", job.clone());
        row.insert("object", object.clone());
        row.insert("incarnation", 1i64);
        row.insert("state", state);
        row
    }
    fn guard(object: &RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", object.clone());
        row.insert("revision", 0i64);
        row.insert("retired", false);
        row.insert("phase", "active");
        row.insert("incarnation", 1i64);
        row.insert("tombstone_revision", 0i64);
        Value::Object(row)
    }
    fn selected(bindings: &Variables) -> &[Value] {
        let Some(Value::Array(rows)) = bindings.get("rows") else {
            panic!("canonical page");
        };
        rows.as_slice()
    }
    #[test]
    fn candidate_pages_prepare_canonical_bounded_identities_and_parse_full_protocol() {
        let job = RecordId::new("native_retirement", "candidate-job");
        let lineage = RecordId::new("native_retirement", "candidate-origin");
        let objects = [
            RecordId::new("native_guard", "leaf"),
            RecordId::new("native_guard", "parent"),
        ];
        let rows = objects
            .iter()
            .map(|object| candidate(&job, &lineage, object, "pending"))
            .collect();
        let (bindings, count) = candidate_bindings(&job, &lineage, rows).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            bindings.len(),
            3,
            "one canonical page, job and exact lineage"
        );
        for (row, object) in selected(&bindings).iter().zip(&objects) {
            assert_eq!(row.get("object"), &Value::RecordId(object.clone()));
            assert_eq!(
                row.get("guard"),
                &Value::RecordId(guard_id(&Value::RecordId(object.clone())).unwrap())
            );
            assert_eq!(
                row.get("id"),
                &Value::RecordId(item_id(&lineage, object, 1).unwrap())
            );
        }
        for sql in [RETIRE_CLAIM_PAGE, RETIRE_FINALIZE_PAGE] {
            surrealdb_syn::parse(sql).unwrap();
            let mut payload = bindings.clone();
            payload.insert("__object_expectations", Vec::<Value>::new());
            let request = NativeRequest {
                issuance: IssuanceEra {
                    generation: ContentHash::of(b"candidate-page-wire"),
                    era: 1,
                },
                operation: ContentHash::of(sql.as_bytes()),
                request: ContentHash::of(&serde_json::to_vec(&(sql, &payload)).unwrap()),
            };
            authorization::owner_effect_plan(
                request,
                &EffectOwner::Retirement(job.clone()),
                sql,
                payload,
                false,
            )
            .unwrap();
        }
    }
    #[test]
    fn terminal_page_projects_complete_incidence_without_retaining_content_bodies() {
        let schema = include_str!("schema.surql");
        let incidence = schema
            .split("DEFINE FUNCTION fn::native_incidence_targets($row: object) {")
            .nth(1)
            .unwrap()
            .split("DEFINE FUNCTION fn::native_backup_clear()")
            .next()
            .unwrap();
        let required = incidence
            .split("$row.")
            .skip(1)
            .map(|tail| {
                tail.chars()
                    .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                    .collect::<String>()
            })
            .collect::<std::collections::BTreeSet<_>>();
        let projection = RETIRE_FINALIZE_PAGE
            .split("LET $physical=SELECT ")
            .nth(1)
            .unwrap()
            .split(" FROM ONLY $object FOR UPDATE;")
            .next()
            .unwrap()
            .split(',')
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            projection, required,
            "the page must retain every incidence field and no unrelated payload"
        );
        for payload in ["body", "canonical", "bytes", "content"] {
            assert!(
                !projection.contains(payload),
                "large {payload} values must never accumulate in terminal classification"
            );
        }
        assert!(
            RETIRE_FINALIZE_PAGE
                .contains("IF $row.physical!=NONE { fn::native_incidence($row.physical); }"),
            "an absent original keeps its NONE/skip behavior"
        );
    }
    #[test]
    fn candidate_pages_refuse_ambiguous_identity_and_keep_byte_selected_prefix() {
        let job = RecordId::new("native_retirement", "candidate-job");
        let object = RecordId::new("native_guard", "candidate-object");
        let item = candidate(&job, &job, &object, "pending");
        assert!(candidate_bindings(&job, &job, vec![item.clone(), item.clone()]).is_err());
        for (field, value) in [
            (
                "job",
                Value::RecordId(RecordId::new("native_retirement", "foreign")),
            ),
            (
                "id",
                Value::RecordId(RecordId::new("native_retirement_item", "foreign")),
            ),
            ("incarnation", test_value(2i64)),
            ("state", test_value("done")),
            ("successor", Value::RecordId(job.clone())),
        ] {
            let mut invalid = item.clone();
            invalid.insert(field, value);
            assert!(
                candidate_bindings(&job, &job, vec![invalid]).is_err(),
                "{field}"
            );
        }
        assert!(candidate_bindings(&job, &job, Vec::new()).is_err());
        assert!(
            candidate_bindings(
                &job,
                &job,
                vec![item; crate::loader::NATIVE_WINDOW_ROWS + 1]
            )
            .is_err()
        );
        let objects = (0..2)
            .map(|index| {
                RecordId::new(
                    "large_candidate",
                    format!(
                        "{index}{}",
                        "x".repeat(lctx_model::domain::resources::TRANSFER_BYTES / 2)
                    ),
                )
            })
            .collect::<Vec<_>>();
        let rows = objects
            .iter()
            .map(|object| candidate(&job, &job, object, "pending"))
            .collect();
        let (bindings, count) = candidate_bindings(&job, &job, rows).unwrap();
        assert_eq!(
            count, 1,
            "byte bounds select a prefix before effect preparation"
        );
        assert_eq!(
            selected(&bindings)[0].get("object"),
            &Value::RecordId(objects[0].clone())
        );
    }

    async fn snapshot(
        client: &Surreal<Client>,
        job: &RecordId,
        objects: &[RecordId],
    ) -> Vec<Vec<Object>> {
        let guards = objects
            .iter()
            .map(|object| guard_id(&Value::RecordId(object.clone())).unwrap())
            .collect::<Vec<_>>();
        let mut response = client.query("SELECT * FROM $job; SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job ORDER BY id; SELECT * FROM $guards ORDER BY id; SELECT * FROM $objects ORDER BY id")
            .bind(("job", job.clone())).bind(("guards", guards)).bind(("objects", objects.to_vec())).await.unwrap().check().unwrap();
        (0..4)
            .map(|index| response.take::<Vec<Object>>(index).unwrap())
            .collect()
    }
    async fn register(
        client: &Surreal<Client>,
        identity: ContentHash,
        roots: Vec<RecordId>,
    ) -> RecordId {
        retire_named_for_owner(
            client,
            EffectOwner::Installation,
            identity,
            None,
            roots,
            1,
            RetirementMode::RegisterTerminal,
        )
        .await
        .unwrap();
        RecordId::new("native_retirement", identity.hex())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn candidate_pages_refuse_late_identity_drift_atomically_and_resume_committed_prefix() {
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let outcome = std::panic::AssertUnwindSafe(async {
            let nonce = fresh_identity("candidate-page-atomic-control").unwrap();
            let objects = (0..2)
                .map(|index| {
                    RecordId::new(
                        "native_guard",
                        format!("candidate_atomic_{}_{index}", nonce.hex()),
                    )
                })
                .collect::<Vec<_>>();
            ensure_rows(&client, None, objects.iter().map(guard).collect())
                .await
                .unwrap();
            let job = register(&client, nonce, objects.clone()).await;
            let items = objects
                .iter()
                .map(|object| candidate(&job, &job, object, "pending"))
                .collect::<Vec<_>>();
            let (prepared, count) = candidate_bindings(&job, &job, items.clone()).unwrap();
            assert_eq!(count, 2);
            let last = item_id(&job, &objects[1], 1).unwrap();
            client
                .query("UPDATE $id SET incarnation=2 RETURN NONE")
                .bind(("id", last.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let before = snapshot(&client, &job, &objects).await;
            let error = claim_page(&client, &job, prepared.clone())
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("native retirement candidate changed"),
                "{error}"
            );
            assert_eq!(
                snapshot(&client, &job, &objects).await,
                before,
                "late invalid candidate refuses the entire claim page"
            );
            client
                .query("UPDATE $id SET incarnation=1 RETURN NONE")
                .bind(("id", last))
                .await
                .unwrap()
                .check()
                .unwrap();
            claim_page(&client, &job, prepared).await.unwrap();
            let refreshed = refresh_candidates(&client, &job, items, true)
                .await
                .unwrap();
            assert!(
                refreshed
                    .iter()
                    .all(|item| item.get("state") == Some(&test_value("retiring")))
            );
            let (prepared, count) = candidate_bindings(&job, &job, refreshed).unwrap();
            assert_eq!(count, 2);
            let last_guard = guard_id(&Value::RecordId(objects[1].clone())).unwrap();
            client
                .query("UPDATE $id SET incarnation=2,revision+=1 RETURN NONE")
                .bind(("id", last_guard.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let before = snapshot(&client, &job, &objects).await;
            let error = finalize_page(&client, &job, prepared.clone())
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("native retirement finalize incarnation fenced"),
                "{error}"
            );
            assert_eq!(
                snapshot(&client, &job, &objects).await,
                before,
                "last-row guard drift refuses every terminal deletion"
            );
            client
                .query("UPDATE $id SET incarnation=1,revision+=1 RETURN NONE")
                .bind(("id", last_guard))
                .await
                .unwrap()
                .check()
                .unwrap();
            // An interrupted caller resumes the durable claimed prefix. One unit
            // finalizes only one leaf; repeating resume never repeats its deletion.
            let first = resume_retirement(&client, nonce, 1).await.unwrap();
            assert_eq!(first.retired, 1);
            assert_eq!(first.remaining.len(), 1);
            let done = resume_retirement(&client, nonce, 1).await.unwrap();
            assert_eq!(done.retired, 2);
            assert!(done.remaining.is_empty());
            let repeated = resume_retirement(&client, nonce, 1).await.unwrap();
            assert_eq!(repeated.retired, done.retired);
            assert_eq!(repeated.examined, done.examined);
            assert_eq!(snapshot(&client, &job, &objects).await[3].len(), 0);
        })
        .catch_unwind()
        .await;
        let cleanup = client.invalidate().await;
        if let Err(panic) = outcome {
            if let Err(error) = cleanup {
                eprintln!("candidate atomic page cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn candidate_pages_preserve_partial_budget_held_objects_and_high_degree_progress() {
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let outcome = std::panic::AssertUnwindSafe(async {
            let nonce = fresh_identity("candidate-page-progress-control").unwrap();
            let leaves = (0..3)
                .map(|index| {
                    RecordId::new(
                        "native_guard",
                        format!("candidate_leaf_{}_{index}", nonce.hex()),
                    )
                })
                .collect::<Vec<_>>();
            let holder = RecordId::new("native_guard", format!("candidate_holder_{}", nonce.hex()));
            let parent = RecordId::new("native_guard", format!("candidate_parent_{}", nonce.hex()));
            let children = (0..crate::loader::NATIVE_WINDOW_ROWS + 1)
                .map(|index| {
                    RecordId::new(
                        "native_guard",
                        format!("candidate_child_{}_{index:03}", nonce.hex()),
                    )
                })
                .collect::<Vec<_>>();
            let values = leaves
                .iter()
                .chain([&holder, &parent])
                .chain(children.iter())
                .map(guard)
                .collect();
            ensure_rows(&client, None, values).await.unwrap();
            hold(&client, None, holder.clone(), vec![leaves[0].clone()])
                .await
                .unwrap();
            hold(&client, None, parent.clone(), children.clone())
                .await
                .unwrap();
            let job = register(&client, nonce, leaves.clone()).await;
            let first = resume_retirement(&client, nonce, 3).await.unwrap();
            assert!(
                first.retired <= 1,
                "three units cannot claim and finalize two fresh leaves"
            );
            let second = resume_retirement(&client, nonce, 8).await.unwrap();
            assert_eq!(second.retired, 2);
            assert!(second.remaining.is_empty());
            assert_eq!(second.retained.len(), 1);
            assert_eq!(
                record(
                    &client,
                    &guard_id(&Value::RecordId(leaves[0].clone())).unwrap()
                )
                .await
                .unwrap()
                .get("phase"),
                Some(&test_value("active"))
            );
            // Release the held leaf by retiring its real owner, then revisit the
            // original retained cursor without resetting its durable item state.
            retire(&client, holder).await.unwrap();
            let third = resume_retirement(&client, nonce, 2).await.unwrap();
            assert!(third.retained.is_empty());
            assert!(third.remaining.is_empty());
            assert!(third.retired >= 2);
            assert_eq!(
                record(&client, &job).await.unwrap().get("state"),
                Some(&test_value("done"))
            );

            let parent_identity = fresh_identity("candidate-high-degree-control").unwrap();
            let parent_job = register(&client, parent_identity, vec![parent.clone()]).await;
            let claimed = resume_retirement(&client, parent_identity, 1)
                .await
                .unwrap();
            assert_eq!(claimed.retired, 0);
            let after_page = resume_retirement(
                &client,
                parent_identity,
                crate::loader::NATIVE_WINDOW_ROWS * 2,
            )
            .await
            .unwrap();
            assert_eq!(after_page.retired, 0);
            assert_eq!(
                outgoing_page(&client, &parent, 2).await.unwrap().len(),
                1,
                "one complete hold page consumes exactly two units per edge"
            );
            let mut progress = after_page;
            for _ in 0..8 {
                if progress.remaining.is_empty() {
                    break;
                }
                progress = resume_retirement(
                    &client,
                    parent_identity,
                    crate::loader::NATIVE_WINDOW_ROWS * 4,
                )
                .await
                .unwrap();
            }
            assert_eq!(progress.retired, (children.len() + 1) as u64);
            assert!(progress.remaining.is_empty());
            assert!(progress.retained.is_empty());
            assert_eq!(
                record(&client, &parent_job).await.unwrap().get("state"),
                Some(&test_value("done"))
            );
        })
        .catch_unwind()
        .await;
        let cleanup = client.invalidate().await;
        if let Err(panic) = outcome {
            if let Err(error) = cleanup {
                eprintln!("candidate progress page cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
}

#[cfg(test)]
mod backup_gate_controls {
    use super::*;
    #[test]
    fn retirement_backup_gate_is_checked_without_mutation() {
        let schema = include_str!("schema.surql");
        let gate = schema
            .split("DEFINE FUNCTION fn::native_backup_clear() {")
            .nth(1)
            .unwrap()
            .split("\n};")
            .next()
            .unwrap();
        surrealdb_syn::parse(schema).unwrap();
        assert!(gate.contains("FROM ONLY native_backup_gate:current FOR UPDATE"));
        assert!(gate.contains("type::is_int($gate.active_count)=false"));
        assert!(gate.contains("type::is_int($gate.revision)=false"));
        assert!(!gate.contains("UPDATE "));
        assert!(!gate.contains("UPSERT "));
        assert!(
            !gate.contains("9223372036854775807"),
            "read-only exclusion still works at the final valid revision"
        );
    }
    // This control deliberately owns the database-wide backup gate and explicit
    // transaction sessions. Run alone on a task-owned RocksDB campaign; ordinary
    // shared-service verification must never select it.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_backup_gate_checked_observation_commit_orders() {
        let cfg = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("exclusive owned RocksDB campaign"),
        ))
        .unwrap();
        let client = crate::installation::check_installation(&cfg).await.unwrap();
        let stop = RetryCancellation::default();
        let cancel = stop.clone();
        let signals = cfg
            .signal_owner("backup gate control writer", move || cancel.cancel())
            .unwrap();
        let scope = RecoveryScope::new(
            client.clone(),
            lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .unwrap(),
            stop,
            signals.pending_recipient().unwrap(),
            RecoveryTarget {
                generation: cfg.service_generation,
                namespace: cfg.namespace.as_str().into(),
                database: cfg.database.as_str().into(),
                endpoint: Some(cfg.endpoint.clone()),
            },
        )
        .unwrap();
        with_recovery_scope(scope.clone(), async {
            // Two read-only retirement exclusions do not serialize each other's commits.
            let left = client.as_ref().clone().begin().await.unwrap();
            let right = client.as_ref().clone().begin().await.unwrap();
            left.query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            right
                .query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            left.commit().await.unwrap().invalidate().await.unwrap();
            right.commit().await.unwrap().invalidate().await.unwrap();
            // A backup writer committed after an older retirement snapshot invalidates
            // that observation, even though retirement does not write the gate.
            let retirement = client.as_ref().clone().begin().await.unwrap();
            retirement
                .query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            let hold = acquire_backup_hold(&client).await.unwrap();
            assert!(
                retirement.commit().await.is_err(),
                "checked read must refuse a changed gate"
            );
            let active = client.as_ref().clone().begin().await.unwrap();
            assert!(
                active
                    .query("fn::native_backup_clear()")
                    .await
                    .unwrap()
                    .check()
                    .is_err(),
                "active backup refuses retirement"
            );
            active.cancel_ref().await.unwrap();
            active.invalidate_session().await.unwrap();
            release_backup_hold(&client, hold).await.unwrap();
            // Reader-first order succeeds; subsequent acquisition/release remains live.
            let retirement = client.as_ref().clone().begin().await.unwrap();
            retirement
                .query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            retirement
                .commit()
                .await
                .unwrap()
                .invalidate()
                .await
                .unwrap();
            let hold = acquire_backup_hold(&client).await.unwrap();
            release_backup_hold(&client, hold).await.unwrap();
            // Even repeated release writes its gate; an older checked reader conflicts.
            let retirement = client.as_ref().clone().begin().await.unwrap();
            retirement
                .query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            release_backup_hold(&client, hold).await.unwrap();
            assert!(retirement.commit().await.is_err());
            let retirement = client.as_ref().clone().begin().await.unwrap();
            retirement
                .query("fn::native_backup_clear()")
                .await
                .unwrap()
                .check()
                .unwrap();
            retirement
                .commit()
                .await
                .unwrap()
                .invalidate()
                .await
                .unwrap();
            release_backup_hold(&client, hold).await.unwrap();
        })
        .await;
        assert_eq!(scope.pending_count().unwrap(), 0);
        client.invalidate().await.unwrap();
        signals
            .complete(&lctx_model::domain::completion::Completion::default())
            .await
            .unwrap();
    }
}
