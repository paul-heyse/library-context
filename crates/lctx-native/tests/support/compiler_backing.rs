//! Exact physical placement for deliberate cold-validation corruption controls.
use lctx_model::domain::{ContentHash, ModelError, resources::ResourceBudget};
use lctx_native::{
    compiler_blocks,
    surrealdb::{
        Surreal,
        engine::remote::grpc::Client,
        types::{RecordId, Value, Variables},
    },
};
use std::sync::Arc;

fn record(value: &Value) -> &RecordId {
    let Value::RecordId(record) = value else {
        panic!("physical or logical record identity")
    };
    record
}

pub struct Backing {
    pub locator: Value,
    pub row: Value,
}
impl Backing {
    pub async fn read(
        client: &Arc<Surreal<Client>>,
        contribution: ContentHash,
        budget: &ResourceBudget,
    ) -> Vec<Self> {
        Self::try_read(client, contribution, budget).await.unwrap()
    }
    /// An independent physical/typed receiving operation, without borrowing a
    /// producer's cached identity or poisoning its native stream admission.
    pub async fn try_read(
        client: &Arc<Surreal<Client>>,
        contribution: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Vec<Self>, ModelError> {
        let reader = lctx_native::NativeReader::private(client.clone()).with_budget(budget);
        let result = async {
            let mut vars = Variables::new();
            vars.insert("owner", RecordId::new("compiler_contribution", contribution.hex()));
            let members: Vec<Value> = reader.query_native("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND storage='block'", vars).await?;
            let mut output = Vec::new();
            for members in members.chunks(lctx_native::loader::NATIVE_WINDOW_ROWS) {
                let backing = compiler_blocks::inspect_backing_window_read(&reader, members, budget).await?;
                for locator in members {
                    let node = record(locator.as_object().unwrap().get("node").unwrap());
                    let row = backing.get(node)?.row();
                    lctx_native::completed_rows::validate_backing_batch(std::slice::from_ref(row), budget)?;
                    output.push(Self { locator: locator.clone(), row: row.clone() });
                }
            }
            Ok::<_, ModelError>(output)
        }.await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "exact compiler block control reader close",
            reader.close().await,
        );
        lctx_model::domain::completion::complete(result, terminal)
    }
    /// A new content-addressed container keeps the framing honest, so the cold validator
    /// reaches the deliberately malformed logical row instead of stopping at a stale hash.
    pub async fn replace(
        &self,
        client: &Arc<Surreal<Client>>,
        attempt: ContentHash,
        contribution: ContentHash,
        row: Value,
        budget: &ResourceBudget,
    ) -> Replacement {
        let packet =
            compiler_blocks::PreparedBlocks::prepare(contribution, vec![row.clone()], budget)
                .unwrap();
        assert_eq!(packet.blocks().len(), 1);
        let block = packet.blocks()[0].clone();
        let block_id = record(block.as_object().unwrap().get("id").unwrap()).clone();
        let owner = RecordId::new("compiler_contribution", contribution.hex());
        let source_attempt = RecordId::new("native_attempt", attempt.hex());
        // Source controls may already have frozen/exported their content. Own a
        // distinct ordinary open attempt; never reopen or mutate their lifecycle.
        let config = lctx_native::RuntimeConfig::read(&std::path::PathBuf::from(
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("owned persistent native fixture"),
        ))
        .unwrap();
        let factory =
            lctx_compiler_store::compiler::recovery_factory(&config, budget.clone()).unwrap();
        let cleanup = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &factory,
        )
        .await
        .unwrap();
        let scope = cleanup.recovery_scope().unwrap();
        let lifecycle = scope.client().clone();
        let attempt = cleanup.attempt();
        let attempt_owner = RecordId::new("native_attempt", attempt.hex());
        lctx_native::control::with_recovery_scope(
            scope.clone(),
            lctx_native::control::ensure_rows(&lifecycle, Some(attempt), vec![block]),
        )
        .await
        .unwrap();
        // Ordinary protection belongs to the actual open attempt. The completed
        // contribution cannot acquire changed incidence through the production API.
        let attempt_hold = lctx_native::control::hold_record_id(&attempt_owner, &block_id).unwrap();
        let mut prior = client
            .query("SELECT * FROM $hold")
            .bind(("hold", attempt_hold.clone()))
            .await
            .unwrap()
            .check()
            .unwrap();
        let prior: Vec<Value> = prior.take(0).unwrap();
        assert!(
            prior.is_empty(),
            "temporary block protection has no preexisting consumer"
        );
        lctx_native::control::with_recovery_scope(
            scope.clone(),
            lctx_native::control::hold(
                &lifecycle,
                Some(attempt),
                attempt_owner.clone(),
                vec![block_id.clone()],
            ),
        )
        .await
        .unwrap();
        let Value::Object(mut locator) = self.locator.clone() else {
            unreachable!()
        };
        let actual = packet.locators()[0].as_object().unwrap();
        // Preserve the original nominal identity, even in a false-key control.
        for name in [
            "block",
            "slot",
            "node",
            "content",
            "scope_keys",
            "scope_lookup",
        ] {
            locator.insert(name, actual.get(name).unwrap().clone());
        }
        let locator = Value::Object(locator);
        let mut edge = lctx_native::surrealdb::types::Object::new();
        edge.insert(
            "id",
            lctx_native::control::hold_record_id(&owner, &block_id).unwrap(),
        );
        edge.insert("owner", owner.clone());
        edge.insert("object", block_id.clone());
        let edge = Value::Object(edge);
        let guard = |record: &RecordId| {
            RecordId::new(
                "native_guard",
                ContentHash::of(&serde_json::to_vec(&Value::RecordId(record.clone())).unwrap())
                    .hex(),
            )
        };
        let mut vars = Variables::new();
        vars.insert(
            "id",
            self.locator.as_object().unwrap().get("id").unwrap().clone(),
        );
        vars.insert("saved", self.locator.clone());
        vars.insert("row", locator.clone());
        vars.insert("edge", edge.clone());
        vars.insert("owner", owner.clone());
        vars.insert("attempt", attempt_owner);
        vars.insert("source_attempt", source_attempt);
        vars.insert("block", block_id.clone());
        vars.insert("owner_guard", guard(&owner));
        vars.insert("block_guard", guard(&block_id));
        // This is deliberately privileged test corruption, not ordinary completed-parent
        // mutation. Both real endpoint guards and the exact old locator are checked.
        client.query("BEGIN; LET $og=SELECT * FROM ONLY $owner_guard FOR UPDATE; LET $bg=SELECT * FROM ONLY $block_guard FOR UPDATE; LET $parent=SELECT * FROM ONLY $owner FOR UPDATE; LET $active=SELECT * FROM ONLY $attempt FOR UPDATE; LET $existing=SELECT * FROM ONLY $edge.id; IF $og.phase!='active' OR $bg.phase!='active' OR $og.retired OR $bg.retired OR $parent.completed!=true OR $parent.attempt!=$source_attempt OR $active.state!='open' OR $existing!=NONE OR (SELECT * FROM ONLY $id)!=$saved OR (SELECT VALUE id FROM ONLY $block)=NONE { THROW 'owned block corruption setup changed'; }; fn::native_incidence($edge); CREATE $edge.id CONTENT $edge RETURN NONE; UPDATE $id CONTENT $row RETURN NONE; COMMIT;")
            .bind(vars).await.unwrap().check().unwrap();
        let mut changed = client
            .query("SELECT * FROM $id")
            .bind((
                "id",
                self.locator.as_object().unwrap().get("id").unwrap().clone(),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let changed: Vec<Value> = changed.take(0).unwrap();
        assert_eq!(
            changed,
            vec![locator.clone()],
            "one exact fused locator redirects to a real block"
        );
        Replacement {
            saved: self.locator.clone(),
            locator,
            row,
            block: block_id,
            owner,
            attempt,
            attempt_hold,
            edge,
            cleanup,
            scope,
        }
    }
}
pub struct Replacement {
    saved: Value,
    pub locator: Value,
    pub row: Value,
    pub block: RecordId,
    owner: RecordId,
    attempt: ContentHash,
    attempt_hold: RecordId,
    edge: Value,
    cleanup: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    scope: Arc<lctx_native::control::RecoveryScope>,
}
impl Replacement {
    pub async fn restore(self, client: &Arc<Surreal<Client>>) {
        let slot = self.locator.as_object().unwrap().get("slot").unwrap();
        let Value::Number(lctx_native::surrealdb::types::Number::Int(slot)) = slot else {
            panic!("replacement slot");
        };
        let mut physical = client
            .query("SELECT * FROM $block")
            .bind(("block", self.block.clone()))
            .await
            .unwrap()
            .check()
            .unwrap();
        let blocks: Vec<Value> = physical.take(0).unwrap();
        assert_eq!(blocks.len(), 1);
        let Some(Value::Array(rows)) = blocks[0].as_object().unwrap().get("rows") else {
            panic!("replacement block slots");
        };
        assert_eq!(
            &rows[usize::try_from(*slot).unwrap()],
            &self.row,
            "assert deliberate corruption reached actual stored slot"
        );
        let id = self.saved.as_object().unwrap().get("id").unwrap().clone();
        let guard = |record: &RecordId| {
            RecordId::new(
                "native_guard",
                ContentHash::of(&serde_json::to_vec(&Value::RecordId(record.clone())).unwrap())
                    .hex(),
            )
        };
        let mut vars = Variables::new();
        vars.insert("id", id.clone());
        vars.insert("row", self.saved.clone());
        vars.insert("changed", self.locator.clone());
        vars.insert("edge", self.edge);
        vars.insert("owner_guard", guard(&self.owner));
        vars.insert("block_guard", guard(&self.block));
        // A failed restoration leaves both the exact contribution edge and the ordinary
        // attempt protection intact; no potentially referenced replacement is retired.
        client.query("BEGIN; LET $og=SELECT * FROM ONLY $owner_guard FOR UPDATE; LET $bg=SELECT * FROM ONLY $block_guard FOR UPDATE; IF $og.phase!='active' OR $bg.phase!='active' OR $og.retired OR $bg.retired OR (SELECT * FROM ONLY $id)!=$changed OR (SELECT * FROM ONLY $edge.id)!=$edge { THROW 'owned block corruption restoration changed'; }; UPDATE $id CONTENT $row RETURN NONE; fn::native_incidence($edge); DELETE $edge.id RETURN NONE; COMMIT;")
            .bind(vars).await.unwrap().check().unwrap();
        // Release only the helper-created attempt edge. Native retirement then
        // owns any claimed incarnation until this exact job reaches terminal EOF.
        let mut expected = lctx_native::surrealdb::types::Object::new();
        expected.insert("id", self.attempt_hold);
        expected.insert("object", self.block.clone());
        expected.insert("owner", RecordId::new("native_attempt", self.attempt.hex()));
        client.query("BEGIN; LET $guard=SELECT * FROM ONLY $block_guard FOR UPDATE; LET $edge_id=$edge.id; LET $actual=SELECT * FROM ONLY $edge_id FOR UPDATE; IF $guard.phase!='active' OR $guard.retired OR $actual!=$edge { THROW 'temporary block attempt protection changed'; }; fn::native_incidence($edge); DELETE $edge.id RETURN NONE; COMMIT;")
            .bind(("block_guard", guard(&self.block))).bind(("edge", Value::Object(expected))).await.unwrap().check().unwrap();
        let lifecycle = self.scope.client().clone();
        let mut pending_job = None;
        let retirement = lctx_native::control::with_recovery_scope(self.scope.clone(), async {
            let mut progress =
                lctx_native::control::retire_reachable(&lifecycle, vec![self.block.clone()], 1)
                    .await?;
            pending_job = Some(progress.identity);
            // One unit may only claim the physical incarnation. Continue that exact
            // job; a retiring object must never be re-held as an ordinary writable row.
            while !progress.remaining.is_empty() && progress.retained.is_empty() {
                progress = lctx_native::control::resume_retirement(
                    &lifecycle,
                    progress.identity,
                    lctx_native::loader::NATIVE_WINDOW_ROWS,
                )
                .await?;
            }
            if !progress.remaining.is_empty() || !progress.retained.is_empty() {
                return Err(ModelError::Conflict(
                    "temporary malformed block retirement retained",
                ));
            }
            Ok::<_, ModelError>(())
        })
        .await;
        if let Err(primary) = retirement {
            let mut finality = lctx_model::domain::completion::Completion::default();
            if let Some(identity) = pending_job {
                use lctx_native::surrealdb::types::ToSql;
                // The authenticated retirement job owns pending/retiring incarnation
                // drainage. Preserve that actual obligation instead of inventing a hold.
                finality.committed("temporary malformed block exact retirement", identity.hex());
                finality
                    .storage
                    .push(lctx_model::domain::completion::StorageState::Orphan(
                        RecordId::new("native_retirement", identity.hex()).to_sql(),
                    ));
            } else {
                let retained = lctx_native::control::with_recovery_scope(
                    self.scope.clone(),
                    lctx_native::control::hold(
                        &lifecycle,
                        Some(self.attempt),
                        RecordId::new("native_attempt", self.attempt.hex()),
                        vec![self.block.clone()],
                    ),
                )
                .await;
                if retained.is_err() {
                    use lctx_native::surrealdb::types::ToSql;
                    finality
                        .storage
                        .push(lctx_model::domain::completion::StorageState::Orphan(
                            self.block.to_sql(),
                        ));
                }
                finality.step(
                    "temporary malformed block retention after unconfirmed retirement setup",
                    retained,
                );
            }
            lctx_model::domain::completion::complete(Err::<(), _>(primary), finality).unwrap();
        }
        self.cleanup.abandon().await.unwrap();
        let mut readback = client
            .query("SELECT * FROM $locator; SELECT * FROM $block")
            .bind(("locator", id))
            .bind(("block", self.block))
            .await
            .unwrap()
            .check()
            .unwrap();
        let locators: Vec<Value> = readback.take(0).unwrap();
        let blocks: Vec<Value> = readback.take(1).unwrap();
        assert_eq!(locators, vec![self.saved]);
        assert!(blocks.is_empty());
    }
}
