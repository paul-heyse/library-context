//! Receiving-invocation preparation. Local IPC is data, never validity or owner authority.
use super::{AdmissionRuntime, ValidationSession};
use arrow_schema::SchemaRef;
use datafusion::prelude::SessionContext;
use lctx_model::domain::{ContentHash, ModelError, Relation, charged::StateCharge};
use std::collections::BTreeMap;

struct PreparedInput {
    schema: SchemaRef,
    table: String,
}
struct PreparedFamily {
    schema: SchemaRef,
    table: String,
}
/// Original provider/session ownership outlives all local files and queries. Every demanded
/// input is drained once, preserving its full typed rows, before any local comparison uses it.
/// The caller still performs actual current observations and failure-atomic premise publication.
pub(super) struct PreparedReferenceTargets<'a> {
    runtime: &'a AdmissionRuntime,
    original: &'a SessionContext,
    _owner: Option<&'a ValidationSession>,
    local: Option<crate::invocation_ipc::InvocationIpc>,
    inputs: BTreeMap<ContentHash, PreparedInput>,
    families: BTreeMap<Vec<ContentHash>, PreparedFamily>,
    charge: StateCharge,
}
impl<'a> PreparedReferenceTargets<'a> {
    pub(super) fn new(
        runtime: &'a AdmissionRuntime,
        original: &'a SessionContext,
        owner: Option<&'a ValidationSession>,
    ) -> Self {
        Self {
            runtime,
            original,
            _owner: owner,
            local: None,
            inputs: BTreeMap::new(),
            families: BTreeMap::new(),
            charge: StateCharge::new(runtime.budget(), "receiving reference preparation metadata"),
        }
    }
    pub(super) fn session(&self) -> &SessionContext {
        self.local
            .as_ref()
            .expect("reference input has been prepared")
            .session()
    }
    fn initialize(&mut self) -> Result<(), ModelError> {
        if self.local.is_none() {
            self.local = Some(crate::invocation_ipc::InvocationIpc::new(
                &self.runtime.context,
                self.runtime.budget(),
                &self.runtime.cancellation,
                self.runtime._directory.as_ref().map(|parent| parent.path()),
            )?);
        }
        Ok(())
    }
    pub(super) async fn input(
        &mut self,
        identity: ContentHash,
        relation: &Relation,
        table: &str,
    ) -> Result<String, ModelError> {
        self.runtime.cancellation.check()?;
        if let Some(prepared) = self.inputs.get(&identity) {
            if prepared.schema.as_ref() != relation.schema().as_ref() {
                return Err(ModelError::Schema(
                    "reference preparation input schema changed",
                ));
            }
            return Ok(prepared.table.clone());
        }
        self.initialize()?;
        self.charge
            .grow(1024usize.saturating_add(relation.schema().fields().len().saturating_mul(128)))?;
        let name = format!("_reference_input_{}", identity.hex());
        let sql = format!("SELECT * FROM {}", crate::consumed_rows::identifier(table));
        let frame = crate::sql::query(self.original, &sql)
            .await
            .map_err(crate::sql::model_error)?;
        self.materialize(frame, &name, Some(relation.schema()))
            .await?;
        self.inputs.insert(
            identity,
            PreparedInput {
                schema: relation.schema().clone(),
                table: name.clone(),
            },
        );
        Ok(name)
    }
    /// The complete ordered target set and its full schema determine this private family.
    /// Payload conflicts have already been checked before its canonical ID/tag projection.
    pub(super) async fn family(
        &mut self,
        targets: &[(&ContentHash, &(Relation, String))],
    ) -> Result<String, ModelError> {
        let (_, (relation, _)) = targets
            .first()
            .ok_or(ModelError::Schema("empty prepared target family"))?;
        let _key = self.runtime.budget().reserve(
            "reference target family lookup",
            targets
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>())
                .saturating_add(512),
        )?;
        let identities = targets
            .iter()
            .map(|(identity, _)| **identity)
            .collect::<Vec<_>>();
        if targets
            .iter()
            .any(|(_, (other, _))| other.schema().as_ref() != relation.schema().as_ref())
        {
            return Err(ModelError::Schema("reference target family schema differs"));
        }
        if let Some(prepared) = self.families.get(&identities) {
            if prepared.schema.as_ref() != relation.schema().as_ref() {
                return Err(ModelError::Schema("reference target family schema changed"));
            }
            return Ok(prepared.table.clone());
        }
        self.charge.grow(
            targets
                .len()
                .saturating_mul(
                    std::mem::size_of::<ContentHash>() + std::mem::size_of::<String>() + 128,
                )
                .saturating_add(512),
        )?;
        let mut tables = Vec::with_capacity(targets.len());
        for (identity, (relation, table)) in targets {
            tables.push(self.input(**identity, relation, table).await?);
        }
        let quote = crate::consumed_rows::identifier;
        let columns = match relation.sum() {
            Some(sum) => format!("{}, {}", quote("id"), quote(sum.tag)),
            None => quote("id"),
        };
        self.charge.grow(
            tables
                .iter()
                .map(|table| table.len().saturating_add(columns.len()).saturating_add(64))
                .sum(),
        )?;
        let union = tables
            .iter()
            .map(|table| format!("SELECT {columns} FROM {}", quote(table)))
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        let sql = format!("SELECT DISTINCT {columns} FROM ({union}) AS reference_targets");
        let frame = crate::sql::query(self.session(), &sql)
            .await
            .map_err(crate::sql::model_error)?;
        let name = format!("_reference_family_{}", self.families.len());
        self.materialize(frame, &name, None).await?;
        self.families.insert(
            identities,
            PreparedFamily {
                schema: relation.schema().clone(),
                table: name.clone(),
            },
        );
        Ok(name)
    }
    async fn materialize(
        &mut self,
        frame: datafusion::dataframe::DataFrame,
        name: &str,
        expected: Option<&SchemaRef>,
    ) -> Result<(), ModelError> {
        self.local
            .as_mut()
            .expect("reference input has been prepared")
            .materialize(frame, name, expected)
            .await
    }
}
