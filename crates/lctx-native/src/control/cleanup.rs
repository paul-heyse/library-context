//! Durable exact terminal cleanup, with a distinct current-era maintenance successor.
use super::*;

pub(super) struct CleanupHolds {
    identity: ContentHash,
}
impl CleanupHolds {
    pub(super) async fn begin(
        client: &Surreal<Client>,
        authorization: TerminalAttempt,
        owners: Vec<RecordId>,
        unadmitted: bool,
    ) -> Result<Self, ModelError> {
        if owners
            .iter()
            .any(|owner| owner.table.as_str() != "native_attempt")
        {
            return Err(ModelError::Conflict(
                "attempt cleanup cannot release retained product ownership",
            ));
        }
        let mut response = client
            .query("SELECT generation,era FROM $attempt")
            .bind((
                "attempt",
                RecordId::new("native_attempt", authorization.identity.hex()),
            ))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [origin] = rows.as_slice() else {
            return Err(ModelError::Conflict(
                "native cleanup original attempt missing",
            ));
        };
        let issuance = IssuanceEra {
            generation: authorization::hash_field(origin, "generation")?,
            era: authorization::int_field(origin, "era")?,
        };
        let identity = fresh_identity("cleanup-obligation")?;
        let mut bindings = Variables::new();
        bindings.insert("cleanup", RecordId::new("native_cleanup", identity.hex()));
        let expected = cleanup_owner_expectations(client, &owners).await?;
        bindings.insert("owners", owners);
        bindings.insert("owner_incarnations", expected);
        bindings.insert("unadmitted", unadmitted);
        effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"IF $unadmitted AND $__owner.admitted { THROW 'native cleanup admitted branch'; }; fn::native_touch($cleanup); fn::native_touch($__attempt); FOR $expected IN $owner_incarnations { fn::native_touch($expected.record); LET $physical_id=$expected.guard; LET $physical=SELECT * FROM ONLY $physical_id FOR UPDATE; IF ($physical.incarnation ?? 0)!=$expected.incarnation OR ($physical.phase ?? 'vacant')!=$expected.phase { THROW 'native cleanup initial owner identity changed'; }; }; CREATE $cleanup SET attempt=$__attempt,generation=$__generation,era=$__era,epoch=$__epoch,owners=$owners,owner_incarnations=$owner_incarnations,unadmitted=$unadmitted,state='open',lineage=$cleanup RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("cleanup obligation",identity,issuance,error))?;
        Ok(Self { identity })
    }
    pub(super) async fn step(&self, client: &Surreal<Client>) -> Result<bool, ModelError> {
        let cleanup = RecordId::new("native_cleanup", self.identity.hex());
        let mut pending=client.query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND owner=$cleanup LIMIT 1").bind(("cleanup",cleanup.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let pending: Vec<RecordId> = pending.take(0).map_err(ModelError::codec)?;
        if !pending.is_empty() {
            return Err(ModelError::Conflict(
                "native cleanup previous page unresolved",
            ));
        }
        let mut response = client
            .query("SELECT * FROM $cleanup")
            .bind(("cleanup", cleanup.clone()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native cleanup obligation missing"));
        };
        if row.get("state") == Some(&Value::String("done".into())) {
            return Ok(true);
        }
        let Some(Value::Array(owners)) = row.get("owners") else {
            return Err(ModelError::Schema("native cleanup exact owners"));
        };
        let owner = owners.first().cloned().unwrap_or(Value::None);
        let scope = match row.get("owner_incarnations") {
            Some(Value::Array(values)) => values.first().cloned().unwrap_or(Value::None),
            _ if owners.is_empty() => Value::None,
            _ => {
                return Err(ModelError::Conflict(
                    "native cleanup original owner incarnation missing",
                ));
            }
        };
        let expected = if let Value::RecordId(owner_id) = &owner {
            if owner_id.table.as_str() != "native_attempt" {
                return Err(ModelError::Conflict(
                    "retained product cleanup requires its retirement owner",
                ));
            }
            let Value::Object(scope) = &scope else {
                return Err(ModelError::Conflict(
                    "native cleanup original owner identity missing",
                ));
            };
            if scope.get("record") != Some(&owner) {
                return Err(ModelError::Conflict("native cleanup owner identity order"));
            }
            let observed = cleanup_owner_expectations(client, std::slice::from_ref(owner_id))
                .await?
                .into_iter()
                .next()
                .ok_or(ModelError::Schema("native cleanup current owner identity"))?;
            let Value::Object(current) = &observed else {
                unreachable!()
            };
            if current.get("incarnation") != scope.get("incarnation") {
                return Err(ModelError::Conflict(
                    "native cleanup original owner incarnation changed",
                ));
            }
            observed
        } else {
            Value::None
        };
        let mut response = client
            .query("SELECT * FROM $origin")
            .bind((
                "origin",
                row.get("attempt")
                    .cloned()
                    .ok_or(ModelError::Schema("native cleanup origin identity"))?,
            ))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let origins: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [origin] = origins.as_slice() else {
            return Err(ModelError::Conflict(
                "native cleanup terminal origin missing",
            ));
        };
        validate_cleanup_origin(row, origin)?;
        let nomination = match &owner {
            Value::RecordId(owner) => retirement::prepare_cleanup_nomination(client, owner).await?,
            Value::None if owners.is_empty() => retirement::empty_cleanup_nomination(),
            _ => return Err(ModelError::Schema("native cleanup original owner record")),
        };
        let mut bindings = nomination;
        bindings.insert("cleanup", cleanup.clone());
        bindings.insert("owner", owner);
        bindings.insert("expected_owner", expected);
        bindings.insert("cleanup_observed", row.clone());
        bindings.insert("cleanup_origin", origin.clone());
        let sql = cleanup_step_sql();
        effect_for_owner(client, EffectOwner::Cleanup(cleanup), &sql, bindings).await?;
        let mut response = client
            .query("SELECT VALUE state FROM $cleanup")
            .bind((
                "cleanup",
                RecordId::new("native_cleanup", self.identity.hex()),
            ))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let states: Vec<String> = response.take(0).map_err(ModelError::codec)?;
        Ok(states == ["done"])
    }
}
fn validate_cleanup_origin(cleanup: &Object, origin: &Object) -> Result<(), ModelError> {
    let epoch = authorization::int_field(origin, "epoch")?;
    let original_era = authorization::int_field(origin, "era")?;
    if cleanup.get("attempt") != origin.get("id")
        || cleanup.get("generation") != origin.get("generation")
        || epoch <= 0
        || epoch != authorization::int_field(cleanup, "epoch")?
        || original_era <= 0
        || original_era > authorization::int_field(cleanup, "era")?
        || !matches!(origin.get("state"),Some(Value::String(state))if matches!(state.as_str(),"closing"|"closed"|"abandoned"|"maintenance_fenced"))
        || cleanup.get("unadmitted") == Some(&Value::Bool(true))
            && origin.get("admitted") != Some(&Value::Bool(false))
    {
        return Err(ModelError::Conflict(
            "native cleanup exact terminal origin changed",
        ));
    }
    Ok(())
}
fn cleanup_step_sql() -> String {
    format!(
        "fn::native_incidence($__owner); IF $__owner!=$cleanup_observed {{ THROW 'native cleanup original page changed'; }}; LET $origin_id=$__owner.attempt; LET $origin=SELECT * FROM ONLY $origin_id FOR UPDATE; IF $origin=NONE OR $origin!=$cleanup_origin OR $origin.generation!=$__owner.generation OR $origin.epoch<=0 OR $origin.epoch!=$__owner.epoch OR $origin.era<=0 OR $origin.era>$__owner.era OR $origin.state NOT IN ['closing','closed','abandoned','maintenance_fenced'] OR ($__owner.unadmitted AND $origin.admitted) {{ THROW 'native cleanup origin changed'; }}; IF array::len($__owner.owners)=0 {{ UPDATE $cleanup SET state='done',predecessor=NONE RETURN NONE; }} ELSE {{ IF record::table($owner)!='native_attempt' {{ THROW 'retained product cleanup requires retirement owner'; }}; IF $__owner.owners[0]!=$owner OR $__owner.owner_incarnations[0].record!=$owner OR $__owner.owner_incarnations[0].incarnation!=$expected_owner.incarnation {{ THROW 'native cleanup owner raced'; }}; fn::native_touch($owner); LET $physical_id=$expected_owner.guard; LET $physical=SELECT * FROM ONLY $physical_id FOR UPDATE; IF ($physical.incarnation ?? 0)!=$expected_owner.incarnation OR ($physical.phase ?? 'vacant')!=$expected_owner.phase {{ THROW 'native cleanup owner incarnation changed'; }}; LET $holds=SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner ORDER BY object LIMIT {}; IF $holds!=$n09_holds {{ THROW 'native cleanup prepared hold page changed'; }}; {} FOR $hold_row IN $holds {{ fn::native_incidence($hold_row); DELETE $hold_row.id RETURN NONE; }}; IF array::len($holds)=0 {{ UPDATE $cleanup SET owners=array::slice(owners,1),owner_incarnations=array::slice(owner_incarnations,1) RETURN NONE; }}; }};",
        crate::loader::NATIVE_WINDOW_ROWS,
        retirement::CLEANUP_NOMINATE_PAGE
    )
}
pub async fn resume_cleanup(
    client: &Surreal<Client>,
    identity: ContentHash,
) -> Result<(), ModelError> {
    let cleanup = CleanupHolds { identity };
    while !cleanup.step(client).await? {}
    Ok(())
}
/// This is a new operation over only the predecessor's persisted remaining owners.
pub async fn recover_cleanup(
    client: &Surreal<Client>,
    identity: ContentHash,
) -> Result<ContentHash, ModelError> {
    let issuance = IssuanceEra::capture(client).await?;
    let old = RecordId::new("native_cleanup", identity.hex());
    let mut response = client
        .query("SELECT successor FROM $old")
        .bind(("old", old.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    if let Some(Value::RecordId(successor)) = rows.first().and_then(|row| row.get("successor")) {
        let mut response = client
            .query("SELECT era,generation FROM $successor")
            .bind(("successor", successor.clone()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("cleanup successor missing"));
        };
        if authorization::int_field(row, "era")? != issuance.era
            || authorization::hash_field(row, "generation")? != issuance.generation
        {
            return Err(ModelError::Conflict(
                "recover named cleanup successor first",
            ));
        }
        let surrealdb::types::RecordIdKey::String(key) = &successor.key else {
            return Err(ModelError::Schema("cleanup successor identity"));
        };
        return Ok(ContentHash(
            hex::decode(key)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("cleanup successor width"))?,
        ));
    }
    let successor = fresh_identity("cleanup-maintenance-successor")?;
    let next = RecordId::new("native_cleanup", successor.hex());
    let mut bindings = Variables::new();
    bindings.insert("old", old);
    bindings.insert("next", next);
    bindings.insert("current_era", issuance.era);
    coordination::bind_all_stripes(&mut bindings);
    let recovery = format!(
        "{} {}",
        coordination::ALL_STRIPES_GUARD,
        "IF $__era!=$current_era OR $__installation.admission_open { THROW 'cleanup recovery requires original current-era drained maintenance'; }; LET $before=SELECT * FROM ONLY $old FOR UPDATE; IF $before=NONE OR $before.state!='open' OR $before.generation!=$__generation OR $before.epoch<=0 OR $before.successor!=NONE OR $before.era>($__installation.closed_through ?? 0) { THROW 'cleanup predecessor not fenced'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 { THROW 'cleanup recovery has unresolved effects'; }; fn::native_incidence($before); fn::native_touch($next); CREATE $next SET attempt=$before.attempt,generation=$__generation,era=$current_era,epoch=$before.epoch,owners=$before.owners,owner_incarnations=$before.owner_incarnations,unadmitted=$before.unadmitted,state='open',lineage=($before.lineage ?? $old),predecessor=$old RETURN NONE; UPDATE $old SET successor=$next,state='superseded' RETURN NONE"
    );
    effect(client, None, &recovery, bindings).await?;
    Ok(successor)
}

async fn cleanup_owner_expectations(
    client: &Surreal<Client>,
    owners: &[RecordId],
) -> Result<Vec<Value>, ModelError> {
    if owners.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Conflict("native cleanup owner scope window"));
    }
    let guards = owners
        .iter()
        .map(|id| guard_id(&Value::RecordId(id.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    let mut response = client
        .query("SELECT * FROM $guards")
        .bind(("guards", guards.clone()))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let mut actual = std::collections::BTreeMap::new();
    for row in rows {
        let Some(Value::RecordId(id)) = row.get("id") else {
            return Err(ModelError::Schema("native cleanup guard identity"));
        };
        if !guards.contains(id) || actual.insert(id.clone(), row.clone()).is_some() {
            return Err(ModelError::Conflict("native cleanup guard cardinality"));
        }
    }
    owners
        .iter()
        .zip(guards)
        .map(|(owner, guard)| {
            let (incarnation, phase) = if let Some(row) = actual.get(&guard) {
                let incarnation = authorization::int_field(row, "incarnation")?;
                let Some(Value::String(phase)) = row.get("phase") else {
                    return Err(ModelError::Schema("native cleanup guard phase"));
                };
                if incarnation <= 0 || !matches!(phase.as_str(), "active" | "retiring" | "retired")
                {
                    return Err(ModelError::Conflict("native cleanup guard incarnation"));
                }
                (incarnation, phase.clone())
            } else {
                (0, "vacant".into())
            };
            let mut expected = Object::new();
            expected.insert("record", owner.clone());
            expected.insert("guard", guard);
            expected.insert("incarnation", incarnation);
            expected.insert("phase", phase);
            Ok(Value::Object(expected))
        })
        .collect()
}

#[cfg(test)]
mod cleanup_origin_controls {
    use super::*;
    async fn fixture_client() -> Arc<Surreal<Client>> {
        let path = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("owned persistent native fixture");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        crate::installation::check_installation(&config)
            .await
            .unwrap()
    }
    fn active_record(id: RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", id);
        row.insert("revision", 0i64);
        row.insert("phase", "active");
        row.insert("incarnation", 1i64);
        row.insert("retired", false);
        row.insert("tombstone_revision", 0i64);
        Value::Object(row)
    }
    async fn cleanup_fixture(
        client: &Surreal<Client>,
    ) -> (ContentHash, CleanupHolds, Vec<RecordId>) {
        let attempt = fresh_identity("cleanup-nomination-control").unwrap();
        begin_attempt_in(client, attempt, IssuanceEra::capture(client).await.unwrap())
            .await
            .unwrap();
        let owner = RecordId::new("native_attempt", attempt.hex());
        let roots = (0..2)
            .map(|index| {
                RecordId::new(
                    "native_guard",
                    format!("cleanup_root_{}_{index}", attempt.hex()),
                )
            })
            .collect::<Vec<_>>();
        ensure_rows(
            client,
            None,
            roots.iter().cloned().map(active_record).collect(),
        )
        .await
        .unwrap();
        super::super::hold(client, Some(attempt), owner.clone(), roots.clone())
            .await
            .unwrap();
        let mut bindings = Variables::new();
        bindings.insert("attempt", owner.clone());
        effect(client,Some(attempt),"fn::native_touch($attempt); UPDATE $attempt SET state='closing',revision+=1 RETURN NONE",bindings).await.unwrap();
        let cleanup = CleanupHolds::begin(
            client,
            TerminalAttempt {
                identity: attempt,
                epoch: 1,
            },
            vec![owner],
            true,
        )
        .await
        .unwrap();
        (attempt, cleanup, roots)
    }
    async fn prepared_cleanup_page(
        client: &Surreal<Client>,
        attempt: ContentHash,
        cleanup: &CleanupHolds,
    ) -> (RecordId, Variables) {
        let id = RecordId::new("native_cleanup", cleanup.identity.hex());
        let owner = RecordId::new("native_attempt", attempt.hex());
        let mut response = client
            .query("SELECT * FROM $cleanup; SELECT * FROM $owner")
            .bind(("cleanup", id.clone()))
            .bind(("owner", owner.clone()))
            .await
            .unwrap()
            .check()
            .unwrap();
        let rows: Vec<Object> = response.take(0).unwrap();
        let origins: Vec<Object> = response.take(1).unwrap();
        let [row] = rows.as_slice() else {
            panic!("exact cleanup")
        };
        let [origin] = origins.as_slice() else {
            panic!("exact original owner")
        };
        validate_cleanup_origin(row, origin).unwrap();
        let expected = cleanup_owner_expectations(client, std::slice::from_ref(&owner))
            .await
            .unwrap()
            .pop()
            .unwrap();
        let mut bindings = retirement::prepare_cleanup_nomination(client, &owner)
            .await
            .unwrap();
        bindings.insert("cleanup", id.clone());
        bindings.insert("owner", owner);
        bindings.insert("expected_owner", expected);
        bindings.insert("cleanup_observed", row.clone());
        bindings.insert("cleanup_origin", origin.clone());
        (id, bindings)
    }
    async fn page_snapshot(
        client: &Surreal<Client>,
        attempt: ContentHash,
        job: &RecordId,
    ) -> Vec<Vec<Object>> {
        let mut response=client.query("SELECT * FROM $job; SELECT * FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job ORDER BY id; SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner ORDER BY object")
            .bind(("job",job.clone())).bind(("owner",RecordId::new("native_attempt",attempt.hex()))).await.unwrap().check().unwrap();
        (0..3).map(|index| response.take(index).unwrap()).collect()
    }
    async fn retire_owned(client: &Surreal<Client>, roots: Vec<RecordId>) {
        let mut progress = retirement::retire_reachable(client, roots, 16)
            .await
            .unwrap();
        while !progress.remaining.is_empty() {
            assert!(
                progress.retained.is_empty(),
                "owned control roots remain held"
            );
            progress = retirement::resume_retirement(client, progress.identity, 16)
                .await
                .unwrap();
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn cleanup_nomination_accepts_same_incarnation_revision_churn_and_independent_holder() {
        let client = fixture_client().await;
        let (attempt, cleanup, roots) = cleanup_fixture(&client).await;
        let (id, bindings) = prepared_cleanup_page(&client, attempt, &cleanup).await;
        let Some(Value::RecordId(job)) = bindings.get("n09_job") else {
            panic!("bounded nomination job")
        };
        let job = job.clone();
        let guard = guard_id(&Value::RecordId(roots[0].clone())).unwrap();
        let mut response = client
            .query("SELECT * FROM $guard")
            .bind(("guard", guard.clone()))
            .await
            .unwrap()
            .check()
            .unwrap();
        let before: Vec<Object> = response.take(0).unwrap();
        // Real guarded hold insertion advances the shared physical revision. It is
        // legitimate reuse of the same active incarnation, not an injected mutation.
        let peer = RecordId::new("native_guard", format!("cleanup_peer_{}", attempt.hex()));
        ensure_rows(&client, None, vec![active_record(peer.clone())])
            .await
            .unwrap();
        super::super::hold(&client, None, peer.clone(), roots.clone())
            .await
            .unwrap();
        let mut response = client
            .query("SELECT * FROM $guard")
            .bind(("guard", guard))
            .await
            .unwrap()
            .check()
            .unwrap();
        let after: Vec<Object> = response.take(0).unwrap();
        assert_eq!(before[0].get("incarnation"), after[0].get("incarnation"));
        assert!(
            authorization::int_field(&after[0], "revision").unwrap()
                > authorization::int_field(&before[0], "revision").unwrap()
        );
        effect_for_owner(
            &client,
            EffectOwner::Cleanup(id),
            &cleanup_step_sql(),
            bindings,
        )
        .await
        .unwrap();
        let snapshot = page_snapshot(&client, attempt, &job).await;
        assert_eq!(snapshot[0].len(), 1);
        assert_eq!(snapshot[1].len(), 2);
        assert!(snapshot[2].is_empty());
        for item in &snapshot[1] {
            assert_eq!(item.get("incarnation"), before[0].get("incarnation"));
        }
        let mut response=client.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$peer ORDER BY object; SELECT VALUE id FROM $roots")
            .bind(("peer",peer.clone())).bind(("roots",roots.clone())).await.unwrap().check().unwrap();
        assert_eq!(response.take::<Vec<Object>>(0).unwrap().len(), 2);
        assert_eq!(response.take::<Vec<RecordId>>(1).unwrap().len(), 2);
        resume_cleanup(&client, cleanup.identity).await.unwrap();
        close_attempt(&client, attempt, "abandoned").await.unwrap();
        retire_owned(&client, vec![peer]).await;
        let surrealdb::types::RecordIdKey::String(job_key) = &job.key else {
            panic!("retirement identity")
        };
        let job_identity = ContentHash(hex::decode(job_key).unwrap().try_into().unwrap());
        let settled = retirement::resume_retirement(&client, job_identity, 16)
            .await
            .unwrap();
        assert!(settled.remaining.is_empty());
        client.invalidate().await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn cleanup_nomination_refuses_changed_phase_incarnation_and_hold_page_atomically() {
        let client = fixture_client().await;
        for defect in ["incarnation", "phase", "retired", "missing", "hold_page"] {
            let (attempt, cleanup, roots) = cleanup_fixture(&client).await;
            let (id, bindings) = prepared_cleanup_page(&client, attempt, &cleanup).await;
            let Some(Value::RecordId(job)) = bindings.get("n09_job") else {
                panic!("bounded nomination job")
            };
            let job = job.clone();
            let Some(Value::Array(holds)) = bindings.get("n09_holds") else {
                panic!("exact held roots")
            };
            let Value::Object(last) = holds.last().unwrap() else {
                panic!("last incoming hold")
            };
            let Some(Value::RecordId(root)) = last.get("object") else {
                panic!("last root")
            };
            let root = root.clone();
            let guard = guard_id(&Value::RecordId(root.clone())).unwrap();
            let mut response = client
                .query("SELECT * FROM $guard")
                .bind(("guard", guard.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let original: Vec<Object> = response.take(0).unwrap();
            let [original] = original.as_slice() else {
                panic!("actual guard")
            };
            let mut vars = Variables::new();
            vars.insert("guard", guard.clone());
            vars.insert("hold", last.clone());
            // Explicit qualification mutations are confined to these fresh owned
            // records, after production preparation; they model forbidden stale premises.
            let sql = match defect {
                "incarnation" => "UPDATE $guard SET incarnation+=1,revision+=1 RETURN NONE",
                "phase" => "UPDATE $guard SET phase='retiring',revision+=1 RETURN NONE",
                "retired" => "UPDATE $guard SET retired=true,revision+=1 RETURN NONE",
                "missing" => "DELETE $guard RETURN NONE",
                _ => "fn::native_incidence($hold); DELETE $hold.id RETURN NONE",
            };
            effect(&client, None, sql, vars).await.unwrap();
            let before = page_snapshot(&client, attempt, &job).await;
            assert!(before[0].is_empty() && before[1].is_empty());
            let error = effect_for_owner(
                &client,
                EffectOwner::Cleanup(id),
                &cleanup_step_sql(),
                bindings,
            )
            .await
            .unwrap_err();
            let expected = if defect == "hold_page" {
                "native cleanup prepared hold page changed"
            } else {
                "native cleanup nominated incarnation changed"
            };
            assert!(error.to_string().contains(expected), "{defect}: {error}");
            assert_eq!(
                page_snapshot(&client, attempt, &job).await,
                before,
                "{defect}: late refusal must roll back all nominations and hold deletions"
            );
            let mut vars = Variables::new();
            vars.insert("guard", guard);
            vars.insert("physical", original.clone());
            client
                .query("UPSERT $guard CONTENT $physical RETURN NONE")
                .bind(vars)
                .await
                .unwrap()
                .check()
                .unwrap();
            if defect == "hold_page" {
                super::super::hold(
                    &client,
                    None,
                    RecordId::new("native_attempt", attempt.hex()),
                    vec![root.clone()],
                )
                .await
                .unwrap();
            }
            resume_cleanup(&client, cleanup.identity).await.unwrap();
            close_attempt(&client, attempt, "abandoned").await.unwrap();
            retire_owned(&client, roots).await;
        }
        client.invalidate().await.unwrap();
    }
    #[test]
    fn successor_cleanup_retains_original_terminal_owner_epoch_and_generation() {
        let id = RecordId::new("native_attempt", "original");
        let generation = ContentHash::of(b"same installation").hex();
        let mut origin = Object::new();
        origin.insert("id", id.clone());
        origin.insert("generation", generation.clone());
        origin.insert("epoch", 17i64);
        origin.insert("era", 3i64);
        origin.insert("state", "abandoned");
        origin.insert("admitted", false);
        let mut successor = Object::new();
        successor.insert("attempt", id);
        successor.insert("generation", generation);
        successor.insert("epoch", 17i64);
        successor.insert("era", 8i64);
        successor.insert("unadmitted", true);
        validate_cleanup_origin(&successor, &origin).unwrap();
        for (field, value) in [
            ("epoch", crate::control::test_value(18i64)),
            ("era", crate::control::test_value(9i64)),
            (
                "generation",
                crate::control::test_value(ContentHash::of(b"different installation").hex()),
            ),
            ("state", crate::control::test_value("open")),
            ("state", crate::control::test_value("frozen")),
            ("admitted", crate::control::test_value(true)),
        ] {
            let mut changed = origin.clone();
            changed.insert(field, value);
            assert!(validate_cleanup_origin(&successor, &changed).is_err());
        }
        successor.insert("unadmitted", false);
        origin.insert("admitted", true);
        validate_cleanup_origin(&successor, &origin).unwrap();
    }
}
