//! Bounded wire projections of one immutable completed contributor. Its serialized
//! specification is repeated inside the descriptor; neither encoding is discarded.
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use surrealdb::types::{Object, RecordId, Value};

// Keep every native field for independent validation, but do not transmit both
// serialized copies and their native query projection in one aggregate row.
// Completion makes these fields immutable for this retained native source.
pub(super) const CONTRIBUTION_SQL: &str = "SELECT * OMIT spec, descriptor FROM $owner WHERE completed=true; SELECT spec FROM $owner WHERE completed=true; SELECT descriptor FROM $owner WHERE completed=true";

pub(super) struct ContributionRead {
    owner: RecordId,
    row: Option<Object>,
    parts: usize,
    charge: StateCharge,
}
impl ContributionRead {
    pub(super) fn new(owner: RecordId, budget: &ResourceBudget) -> Self {
        Self {
            owner,
            row: None,
            parts: 0,
            charge: StateCharge::new(budget, "completed contributor projected read"),
        }
    }
    pub(super) fn push(&mut self, value: Value) -> Result<(), ModelError> {
        // The transport owns the current row; this grant retains all earlier
        // projections across the next poll and covers subsequent typed decoding.
        self.charge.grow(
            crate::loader::native_bytes(&value)
                .saturating_mul(4)
                .saturating_add(4096),
        )?;
        let Value::Object(mut object) = value else {
            return Err(ModelError::Schema("completed contributor projected object"));
        };
        match self.parts {
            0 => {
                if object.get("id") != Some(&Value::RecordId(self.owner.clone()))
                    || object.contains_key("spec")
                    || object.contains_key("descriptor")
                {
                    return Err(ModelError::Conflict(
                        "completed contributor projected metadata",
                    ));
                }
                self.row = Some(object);
            }
            1 | 2 => {
                let field = if self.parts == 1 {
                    "spec"
                } else {
                    "descriptor"
                };
                if object.len() != 1 {
                    return Err(ModelError::Conflict(
                        "completed contributor projected payload inventory",
                    ));
                }
                let Some(value @ Value::Bytes(_)) = object.remove(field) else {
                    return Err(ModelError::Schema(
                        "completed contributor projected payload bytes",
                    ));
                };
                self.row
                    .as_mut()
                    .ok_or(ModelError::Conflict(
                        "completed contributor projected order",
                    ))?
                    .insert(field, value);
            }
            _ => {
                return Err(ModelError::Conflict(
                    "duplicate completed contributor projected row",
                ));
            }
        }
        self.parts += 1;
        Ok(())
    }
    pub(super) fn finish(mut self) -> Result<(Value, StateCharge), ModelError> {
        if self.parts != 3 {
            return Err(ModelError::Conflict(
                "missing completed contributor projected row",
            ));
        }
        Ok((
            Value::Object(self.row.take().ok_or(ModelError::Conflict(
                "missing completed contributor projected metadata",
            ))?),
            self.charge,
        ))
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    fn metadata(owner: RecordId) -> Value {
        let mut row = Object::new();
        row.insert("id", owner);
        row.insert("completed", true);
        row.insert(
            "unexpected_native_field",
            "must remain visible to validation",
        );
        Value::Object(row)
    }
    fn payload(field: &str, byte: u8) -> Value {
        let mut row = Object::new();
        row.insert(field, surrealdb::types::Bytes::from(vec![byte; 600 * 1024]));
        Value::Object(row)
    }
    #[test]
    fn repeated_metadata_payloads_are_bounded_individually_and_preserved_exactly() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let owner = RecordId::new("compiler_contribution", "test");
        let parts = [
            metadata(owner.clone()),
            payload("spec", 11),
            payload("descriptor", 23),
        ];
        assert!(
            parts
                .iter()
                .all(|row| crate::loader::native_bytes(row) < 1 << 20)
        );
        let mut expected = Object::new();
        for part in &parts {
            for (field, value) in part.as_object().unwrap().iter() {
                expected.insert(field, value.clone());
            }
        }
        let expected = Value::Object(expected);
        assert!(crate::loader::native_bytes(&expected) > 1 << 20);
        let mut read = ContributionRead::new(owner, &budget);
        for part in parts {
            read.push(part).unwrap();
        }
        let (actual, charge) = read.finish().unwrap();
        assert_eq!(actual, expected);
        assert!(budget.reserved() >= crate::loader::native_bytes(&actual));
        drop(actual);
        drop(charge);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn projected_metadata_rejects_missing_duplicate_wrong_owner_and_reordered_parts() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let owner = RecordId::new("compiler_contribution", "test");
        assert!(
            ContributionRead::new(owner.clone(), &budget)
                .finish()
                .is_err()
        );
        let mut read = ContributionRead::new(owner.clone(), &budget);
        assert!(
            read.push(metadata(RecordId::new("compiler_contribution", "other")))
                .is_err()
        );
        let mut read = ContributionRead::new(owner.clone(), &budget);
        read.push(metadata(owner.clone())).unwrap();
        assert!(read.push(payload("descriptor", 23)).is_err());
        let mut read = ContributionRead::new(owner.clone(), &budget);
        read.push(metadata(owner.clone())).unwrap();
        read.push(payload("spec", 11)).unwrap();
        assert!(read.finish().is_err());
        let mut read = ContributionRead::new(owner.clone(), &budget);
        read.push(metadata(owner.clone())).unwrap();
        read.push(payload("spec", 11)).unwrap();
        read.push(payload("descriptor", 23)).unwrap();
        assert!(read.push(metadata(owner)).is_err());
    }
    #[test]
    fn reassembled_contributor_still_requires_exact_stored_specification_and_projection() {
        use lctx_model::domain::{self as d, ContentHash, completed::*};
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let attempt = ContentHash::of(b"projected-read-attempt");
        let spec = ContributionSpec {
            captured_binding: None,
            producer: "projected-read-control".into(),
            profile: OutputProfile::Specific(d::stages::Profile::Catalog),
            model: ContentHash::of(b"model"),
            implementation: ContentHash::of(b"implementation"),
            configuration: None,
            inputs: vec![],
            outputs: ["packages".into()].into(),
        };
        let descriptor = CompletedContribution {
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec: spec.clone(),
            outcome: 0,
            outputs: [(
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty"),
                },
            )]
            .into(),
        };
        let owner = RecordId::new(
            "compiler_contribution",
            super::super::attempt_contribution(attempt, spec.identity().unwrap()).hex(),
        );
        let mut native = lctx_native::completed_rows::contribution_projection(&spec);
        native.insert("id", owner.clone());
        native.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
        native.insert("completed", true);
        native.insert("logical", descriptor.identity().unwrap().hex());
        native.insert("outcome", Value::from_t(descriptor.outcome));
        let mut specification = Object::new();
        specification.insert(
            "spec",
            surrealdb::types::Bytes::from(serde_json::to_vec(&spec).unwrap()),
        );
        let mut completed = Object::new();
        completed.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(&descriptor).unwrap()),
        );
        let reconstruct = |native: Object, specification: Object| {
            let mut read = ContributionRead::new(owner.clone(), &budget);
            read.push(Value::Object(native)).unwrap();
            read.push(Value::Object(specification)).unwrap();
            read.push(Value::Object(completed.clone())).unwrap();
            read.finish().unwrap()
        };
        let (actual, _charge) = reconstruct(native.clone(), specification.clone());
        lctx_native::completed_rows::validate_state_row("compiler_contribution", &actual).unwrap();
        let mut changed = spec;
        changed.implementation = ContentHash::of(b"different implementation");
        let mut substituted = Object::new();
        substituted.insert(
            "spec",
            surrealdb::types::Bytes::from(serde_json::to_vec(&changed).unwrap()),
        );
        let (actual, _charge) = reconstruct(native.clone(), substituted);
        assert!(
            lctx_native::completed_rows::validate_state_row("compiler_contribution", &actual)
                .is_err()
        );
        native.insert("implementation", changed.implementation.hex());
        let (actual, _charge) = reconstruct(native, specification);
        assert!(
            lctx_native::completed_rows::validate_state_row("compiler_contribution", &actual)
                .is_err()
        );
    }
}
