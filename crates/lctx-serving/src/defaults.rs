//! Readable declaration/context closure over the already hydrated canonical scope.
//! Runtime override/effective-value claims are never inferred from source/default IDs.
use crate::records::{need, rows, wire, Prepared,PacketRows};
use lctx_model::domain::{resources::ResourceBudget, serving::*, *};
use lctx_surrealdb::NativeReader;
use std::collections::{BTreeMap, BTreeSet};
fn unavailable(reason: &str) -> Result<Availability, ModelError> {
    Ok(Availability::Unavailable {
        reason: Name::new(reason).map_err(wire)?,
    })
}
fn partial(reason: &str) -> Result<Availability, ModelError> {
    Ok(Availability::Partial {
        reason: Name::new(reason).map_err(wire)?,
    })
}
fn scalar(value: &value::Literal) -> Result<Option<String>, ModelError> {
    Ok(match value {
        value::Literal::None => Some("None".into()),
        value::Literal::Bool { value } => Some(if *value { "True" } else { "False" }.into()),
        value::Literal::Integer { decimal } => Some(decimal.clone()),
        value::Literal::String { value } => {
            Some(serde_json::to_string(value.as_str()).map_err(ModelError::codec)?)
        }
        value::Literal::Bytes { value } => Some(format!("bytes(hex={})", hex::encode(&value.0))),
        value::Literal::Float { bits } => {
            let v = f64::from_bits(*bits as u64);
            if v.is_finite() {
                Some(format!("{v} (IEEE754 bits={:016x})", *bits as u64))
            } else {
                None
            }
        }
    })
}
fn readable_predicate(
    predicate: &value::Predicate,
    literals: &PacketRows<value::Literal>,
) -> Result<Option<String>, ModelError> {
    Ok(match predicate {
        value::Predicate::IsNone => Some("is None".into()),
        value::Predicate::Truthy => Some("is truthy".into()),
        value::Predicate::IsValue { value } => {
            scalar(need(literals, *value)?)?.map(|v| format!("is {v}"))
        }
        value::Predicate::Equals { value } => {
            scalar(need(literals, *value)?)?.map(|v| format!("equals {v}"))
        }
        // class_expression currently has no admitted operand-source binding. Its opaque
        // producer spelling/identity is not a readable captured class operand.
        value::Predicate::IsInstance { .. } | value::Predicate::TypeIs { .. } => None,
        _ => None,
    })
}
fn context(row: &attribution::AnalysisContext) -> Result<AnalysisContextPacket, ModelError> {
    Ok(AnalysisContextPacket {
        analysis: row.id(),
        python_version: Name::new(row.python_version.clone()).map_err(wire)?,
        python_platform: Name::new(row.python_platform.clone()).map_err(wire)?,
        search_path: row
            .search_path
            .iter()
            .cloned()
            .map(Text::new)
            .collect::<Result<_, _>>()
            .map_err(wire)?,
        site_package_path: row
            .site_package_path
            .iter()
            .cloned()
            .map(Text::new)
            .collect::<Result<_, _>>()
            .map_err(wire)?,
        config_digest: row.config_digest,
        environment_digest: row.environment_digest,
        lock_digest: Nullable(row.lock_digest),
    })
}
fn excerpt(
    source: &Prepared<'_>,
    occurrence: Id<source::Occurrence>,
    analysis: Id<attribution::AnalysisContext>,
    release: Id<input::Release>,
) -> Result<OriginalExcerpt, ModelError> {
    Ok(OriginalExcerpt {
        original: crate::originals::range(
            source,
            &OriginalReference::Occurrence { occurrence },
            Some(analysis),
            Some(release),
        )?,
        text: Nullable(None),
        availability: unavailable("original_reference_not_yet_read")?,
    })
}
/// Collect scope-bound declaration alternatives before choosing a displayed scalar default.
/// A source-parameter observation in a different analysis is never a compatible default.
pub fn closure(
    source: &Prepared<'_>,
    core: &OperationCore,
) -> Result<InterpretationClosure, ModelError> {
    let options = rows::<catalog::CatalogOption>(source)?.select_for("member",&[core.member])?;
    let subjects = rows::<catalog::CatalogOptionSubject>(source)?;
    let evidence = rows::<catalog::CatalogOptionEvidence>(source)?;
    let syntax = rows::<syntax::ParameterSyntaxObservation>(source)?;
    let defaults = rows::<catalog::CatalogDefault>(source)?;
    let literals = rows::<value::Literal>(source)?;
    let quals = rows::<assertion::AssertionQualification>(source)?;
    let native_signatures = rows::<types::NativeSignatureObservation>(source)?;
    let mut declarations = vec![];
    let mut qids = BTreeSet::new();
    let mut analyses = BTreeSet::new();
    for signature in &core.signatures {
        analyses.insert(signature.analysis);
        qids.extend(signature.typing.iter().map(|t| t.qualification));
        for parameter in &signature.parameters {
            for option in &options {
                let relevant = match need(&subjects, option.subject)? {
                    catalog::CatalogOptionSubject::Parameter { slot } => {
                        parameter.slot.0 == Some(*slot)
                    }
                    catalog::CatalogOptionSubject::SourceParameter { parameter: formal } => {
                        parameter.formals.contains(formal)
                    }
                    catalog::CatalogOptionSubject::Field { .. } => false,
                };
                if !relevant {
                    continue;
                }
                let (declaration, qualification, source_default) =
                    match need(&evidence, option.evidence)? {
                        catalog::CatalogOptionEvidence::Parameter { syntax: id, .. }
                        | catalog::CatalogOptionEvidence::SourceParameter { syntax: id, .. } => {
                            let row = need(&syntax, *id)?;
                            let q = need(&quals, row.qualification)?;
                            if q.context != signature.analysis {
                                continue;
                            }
                            (
                                DefaultDeclaration::SourceParameter,
                                Some(row.qualification),
                                row.default,
                            )
                        }
                        catalog::CatalogOptionEvidence::NativeParameter { slot } => {
                            if parameter.slot.0 != Some(*slot) {
                                return Err(ModelError::Conflict("native parameter default slot"));
                            }
                            let qualification = signature
                                .native
                                .0
                                .map(|id| need(&native_signatures, id).map(|row| row.qualification))
                                .transpose()?;
                            if qualification.is_some_and(|id| {
                                quals
                                    .iter()
                                    .any(|q| q.id() == id && q.context != signature.analysis)
                            }) {
                                continue;
                            }
                            (DefaultDeclaration::NativeSignature, qualification, None)
                        }
                        catalog::CatalogOptionEvidence::DeclaredField { .. } => {
                            (DefaultDeclaration::DeclaredField, None, None)
                        }
                        catalog::CatalogOptionEvidence::NativeField { .. } => {
                            (DefaultDeclaration::NativeField, None, None)
                        }
                    };
                let value = DefaultValue::from_canonical(need(&defaults, option.default)?);
                let occurrence = match &value {
                    DefaultValue::Expression { expression }
                    | DefaultValue::Factory { expression } => Some(*expression),
                    _ => source_default,
                };
                let readable = match &value {
                    DefaultValue::Literal { literal } => scalar(need(&literals, *literal)?)?,
                    DefaultValue::Absent {} => Some("no declared default".into()),
                    _ => None,
                };
                let availability = if readable.is_some() {
                    Availability::Available {}
                } else {
                    unavailable("declared_value_requires_original_or_is_unknown")?
                };
                if let Some(q) = qualification {
                    qids.insert(q);
                }
                declarations.push(DefaultInterpretation {
                    signature: Nullable(Some(signature.signature)),
                    variant: Nullable(Some(signature.variant)),
                    analysis: signature.analysis,
                    parameter: Nullable(Some(parameter.parameter)),
                    field: Nullable(None),
                    subject_name: parameter.name.clone(),
                    option: Nullable(Some(option.id())),
                    declaration,
                    value,
                    readable: Nullable(readable.map(Text::new).transpose().map_err(wire)?),
                    original: Nullable(
                        occurrence
                            .map(|o| excerpt(source, o, signature.analysis, core.release.release))
                            .transpose()?,
                    ),
                    qualification: Nullable(qualification),
                    availability,
                    effective_override: unavailable(
                        "runtime_effective_value_and_caller_override_not_observed",
                    )?,
                });
            }
        }
    }
    let fields = rows::<normalized::entities::FieldEntity>(source)?;
    let links = rows::<normalized::entities::FieldDeclarationLink>(source)?;
    let field_syntax = rows::<syntax::ClassFieldSyntaxObservation>(source)?;
    let native_fields = rows::<types::RecordFieldObservation>(source)?;
    let native_links = rows::<normalized::entities::FieldEntityLink>(source)?;
    for option in &options {
        let catalog::CatalogOptionSubject::Field { field } = need(&subjects, option.subject)?
        else {
            continue;
        };
        let field_row = need(&fields, *field)?;
        let (declaration, qualification, source_default) = match need(&evidence, option.evidence)? {
            catalog::CatalogOptionEvidence::DeclaredField { declaration, .. } => {
                let link = need(&links, *declaration)?;
                if link.field != *field {
                    return Err(ModelError::Conflict("field default declaration target"));
                }
                let row = need(&field_syntax, link.declaration)?;
                (
                    DefaultDeclaration::DeclaredField,
                    row.qualification,
                    row.value,
                )
            }
            catalog::CatalogOptionEvidence::NativeField { link, observation } => {
                let relation = need(&native_links, *link)?;
                if relation.field != *field || relation.observation != *observation {
                    return Err(ModelError::Conflict("native field default target"));
                }
                let row = need(&native_fields, *observation)?;
                (
                    DefaultDeclaration::NativeField,
                    row.qualification,
                    row.declaration,
                )
            }
            _ => return Err(ModelError::Conflict("field option has parameter evidence")),
        };
        let q = need(&quals, qualification)?;
        analyses.insert(q.context);
        qids.insert(qualification);
        let value = DefaultValue::from_canonical(need(&defaults, option.default)?);
        let occurrence = match &value {
            DefaultValue::Expression { expression } | DefaultValue::Factory { expression } => {
                Some(*expression)
            }
            _ => source_default,
        };
        let readable = match &value {
            DefaultValue::Literal { literal } => scalar(need(&literals, *literal)?)?,
            DefaultValue::Absent {} => Some("no declared default".into()),
            _ => None,
        };
        declarations.push(DefaultInterpretation {
            signature: Nullable(None),
            variant: Nullable(None),
            analysis: q.context,
            parameter: Nullable(None),
            field: Nullable(Some(*field)),
            subject_name: Nullable(Some(Name::new(field_row.name.as_str()).map_err(wire)?)),
            option: Nullable(Some(option.id())),
            declaration,
            value,
            readable: Nullable(
                readable
                    .as_ref()
                    .map(|v| Text::new(v.clone()))
                    .transpose()
                    .map_err(wire)?,
            ),
            original: Nullable(
                occurrence
                    .map(|o| excerpt(source, o, q.context, core.release.release))
                    .transpose()?,
            ),
            qualification: Nullable(Some(qualification)),
            availability: if readable.is_some() {
                Availability::Available {}
            } else {
                unavailable("field_default_requires_original_or_is_unknown")?
            },
            effective_override: unavailable(
                "runtime_effective_field_value_and_override_not_observed",
            )?,
        });
    }
    let mut result = qualified(source, analyses, qids, core.release.release)?;
    result.defaults = declarations;
    Ok(result)
}
pub fn qualified(
    source: &Prepared<'_>,
    analyses: BTreeSet<Id<attribution::AnalysisContext>>,
    qids: BTreeSet<Id<assertion::AssertionQualification>>,
    release: Id<input::Release>,
) -> Result<InterpretationClosure, ModelError> {
    let contexts = rows::<attribution::AnalysisContext>(source)?;
    let qualifications = rows::<assertion::AssertionQualification>(source)?;
    let conditions = rows::<conditions::Condition>(source)?;
    let nodes = rows::<conditions::ConditionNode>(source)?;
    let atoms = rows::<conditions::EvaluationAtom>(source)?;
    let predicates = rows::<value::Predicate>(source)?;
    let literals = rows::<value::Literal>(source)?;
    let claims = source.claims()?;
    let mut values = vec![];
    for id in qids {
        let q = need(&qualifications, id)?;
        if !analyses.contains(&q.context) {
            return Err(ModelError::Conflict("interpretation qualification context"));
        }
        let condition = need(&conditions, q.condition)?;
        let diagram = conditions::Diagram::from_records(condition, &nodes)?;
        let rendered = diagram
            .render_terms(16)
            .map_err(|e| ModelError::Invalid(format!("readable condition boundary:{e:?}")))?;
        let mut terms = vec![];
        for term in rendered.terms {
            let mut readable = vec![];
            for (id, value) in term {
                let atom = need(&atoms, id)?;
                if atom.context != q.context {
                    return Err(ModelError::Conflict("readable condition atom context"));
                }
                let predicate = readable_predicate(need(&predicates, atom.predicate)?, &literals)?;
                readable.push(ReadableConditionAtom {
                    atom: id,
                    analysis: atom.context,
                    predicate: Nullable(predicate.map(Text::new).transpose().map_err(wire)?),
                    evaluation: Nullable(Some(excerpt(
                        source,
                        atom.evaluation,
                        atom.context,
                        release,
                    )?)),
                    value,
                    availability: unavailable("condition_source_not_yet_read_or_predicate_opaque")?,
                });
            }
            terms.push(readable);
        }
        let constant = if diagram.is_true() {
            Some(true)
        } else if diagram.is_false() {
            Some(false)
        } else {
            None
        };
        values.push(QualificationInterpretation {
            qualification: q.id(),
            analysis: q.context,
            scope: q.scope,
            condition: q.condition,
            constant: Nullable(constant),
            terms,
            truncated: rendered.truncated,
            modality: Name::new(match q.modality {
                attribution::Modality::Definite => "definite",
                attribution::Modality::Candidate => "candidate",
                attribution::Modality::Potential => "potential",
            })
            .map_err(wire)?,
            approximation: Name::new(match q.approximation {
                assertion::Approximation::Exact => "exact",
                assertion::Approximation::Over => "over",
                assertion::Approximation::Under => "under",
                assertion::Approximation::Mixed => "mixed",
                assertion::Approximation::Unknown => "unknown",
            })
            .map_err(wire)?,
            claim_basis: claims.basis(q.id())?,
            availability: if constant.is_some() && !rendered.truncated {
                Availability::Available {}
            } else {
                partial("condition_readability_pending")?
            },
        });
    }
    let availability = if !analyses.is_empty()
        && values
            .iter()
            .all(|q| matches!(q.availability, Availability::Available {}))
    {
        Availability::Available {}
    } else {
        partial("interpretation_has_missing_context_or_opaque_conditions")?
    };
    Ok(InterpretationClosure {
        contexts: analyses
            .iter()
            .map(|id| context(need(&contexts, *id)?))
            .collect::<Result<_, _>>()?,
        defaults: vec![],
        qualifications: values,
        availability,
    })
}
/// Read all nominated exact originals in bounded unions. No per-default/atom network loop.
pub async fn read_originals(
    reader: &NativeReader,
    closure: &mut InterpretationClosure,
    request: &Request,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut ranges: BTreeMap<(Id<source::SourceArtifact>, u64, u64), OriginalRange> =
        BTreeMap::new();
    for e in closure
        .defaults
        .iter()
        .filter_map(|d| d.original.0.as_ref())
        .chain(closure.qualifications.iter().flat_map(|q| {
            q.terms
                .iter()
                .flatten()
                .filter_map(|a| a.evaluation.0.as_ref())
        }))
    {
        ranges.insert(
            (e.original.artifact, e.original.start, e.original.end),
            e.original.clone(),
        );
    }
    let limit = if request.page().expanded {
        64 * 1024
    } else {
        8 * 1024
    };
    let mut total = 0;
    let mut selected = vec![];
    let mut keys = vec![];
    for (key, range) in ranges {
        let length = usize::try_from(range.end - range.start).map_err(ModelError::codec)?;
        if length > limit || total + length > limit {
            continue;
        }
        total += length;
        keys.push(key);
        selected.push((graph::EntityId::of(range.artifact), range.start, length));
    }
    let _charge = budget.reserve(
        "readable-interpretation-originals",
        total.saturating_add(selected.len() * 65536),
    )?;
    let mut text = BTreeMap::new();
    for (batch_keys, batch_ranges) in keys.chunks(32).zip(selected.chunks(32)) {
        for (key, bytes) in batch_keys
            .iter()
            .zip(reader.original_bytes_batch(batch_ranges).await?)
        {
            if let Ok(value) = String::from_utf8(bytes)
                && let Ok(value) = Text::new(value)
            {
                text.insert(*key, value);
            }
        }
    }
    fn hydrate(
        e: &mut OriginalExcerpt,
        text: &BTreeMap<(Id<source::SourceArtifact>, u64, u64), Text<0, 262144>>,
    ) -> Result<(), ModelError> {
        let key = (e.original.artifact, e.original.start, e.original.end);
        e.text = Nullable(text.get(&key).cloned());
        e.availability = if e.text.0.is_some() {
            Availability::Available {}
        } else {
            partial("original_expandable_or_not_utf8")?
        };
        Ok(())
    }
    for d in &mut closure.defaults {
        if let Some(e) = &mut d.original.0 {
            hydrate(e, &text)?;
            if matches!(
                d.value,
                DefaultValue::Expression { .. } | DefaultValue::Factory { .. }
            ) {
                d.readable = e.text.clone();
                d.availability = e.availability.clone();
            }
        }
    }
    for q in &mut closure.qualifications {
        for atom in q.terms.iter_mut().flatten() {
            if let Some(e) = &mut atom.evaluation.0 {
                hydrate(e, &text)?;
            }
            atom.availability = if atom.predicate.0.is_some()
                && atom
                    .evaluation
                    .0
                    .as_ref()
                    .is_some_and(|e| e.text.0.is_some())
            {
                Availability::Available {}
            } else {
                partial("opaque_or_expandable_condition_atom")?
            };
        }
        q.availability = if !q.truncated
            && q.terms
                .iter()
                .flatten()
                .all(|a| matches!(a.availability, Availability::Available {}))
        {
            Availability::Available {}
        } else {
            partial("opaque_or_expandable_qualification")?
        };
    }
    closure.availability = if !closure.contexts.is_empty()
        && closure
            .defaults
            .iter()
            .all(|d| matches!(d.availability, Availability::Available {}))
        && closure
            .qualifications
            .iter()
            .all(|q| matches!(q.availability, Availability::Available {}))
    {
        Availability::Available {}
    } else {
        partial("interpretation_closure_has_opaque_or_expandable_parts")?
    };
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_class_operand_never_becomes_readable_predicate() {
        let literals=PacketRows::<value::Literal>::new(vec![],&ResourceBudget::fixed(1024).unwrap()).unwrap();
        assert_eq!(
            readable_predicate(
                &value::Predicate::IsInstance {
                    class_expression: "opaque-id-0123456789".into()
                },
                &literals
            )
            .unwrap(),
            None
        );
        assert_eq!(
            readable_predicate(
                &value::Predicate::TypeIs {
                    class_expression: "opaque-id-0123456789".into()
                },
                &literals
            )
            .unwrap(),
            None
        );
        assert_eq!(
            readable_predicate(&value::Predicate::Truthy, &literals).unwrap(),
            Some("is truthy".into())
        );
    }
    #[test]
    fn scalar_meanings_preserve_none_string_and_nonfinite_uncertainty() {
        assert_eq!(scalar(&value::Literal::None).unwrap(), Some("None".into()));
        assert_eq!(
            scalar(&value::Literal::String {
                value: "None\nquoted".into()
            })
            .unwrap(),
            Some("\"None\\nquoted\"".into())
        );
        assert_eq!(
            scalar(&value::Literal::String {
                value: "α\"".into()
            })
            .unwrap(),
            Some("\"α\\\"\"".into())
        );
        assert!(
            serde_json::from_str::<Utf8Text>("[255]").is_err(),
            "invalid UTF8 cannot become a readable string literal"
        );
        assert_eq!(
            scalar(&value::Literal::Bytes {
                value: EvidenceBytes(vec![255])
            })
            .unwrap(),
            Some("bytes(hex=ff)".into())
        );
        assert_eq!(
            scalar(&value::Literal::Float {
                bits: f64::NAN.to_bits() as i64
            })
            .unwrap(),
            None
        );
        assert_eq!(
            scalar(&value::Literal::Bool { value: false }).unwrap(),
            Some("False".into())
        );
    }
}
