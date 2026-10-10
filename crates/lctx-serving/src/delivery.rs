//! Actual-page delivery and greedy whole-bundle packing. Maps are observations, not proof.
use lctx_model::domain::{serving::*, *};
use serde_json::Value;
fn name(value: impl Into<String>) -> Result<Name, WireError> {
    Name::new(value)
}
fn unavailable(reason: &str) -> Result<Availability, WireError> {
    Ok(Availability::Unavailable {
        reason: name(reason)?,
    })
}
fn partial(reason: &str) -> Result<Availability, WireError> {
    Ok(Availability::Partial {
        reason: name(reason)?,
    })
}
fn parse<T: serde::de::DeserializeOwned>(value: &Value, key: &str) -> Option<T> {
    serde_json::from_value(value.get(key)?.clone()).ok()
}
fn binding(value: &Value, inherited: &DeliveryBinding) -> DeliveryBinding {
    DeliveryBinding {
        member: Nullable(parse(value, "member").or(inherited.member.0)),
        signature: Nullable(parse(value, "signature").or(inherited.signature.0)),
        variant: Nullable(parse(value, "variant").or(inherited.variant.0)),
        analysis: Nullable(parse(value, "analysis").or(inherited.analysis.0)),
        parameter: Nullable(parse(value, "parameter").or(inherited.parameter.0)),
        field: Nullable(parse(value, "field").or(inherited.field.0)),
    }
}
fn expansion(
    request: &Request,
    cursor: Option<CursorToken>,
) -> Result<DeliveryExpansion, WireError> {
    let mut json: Value = serde_json::from_str(&request.to_json()?)?;
    if let Some(cursor) = cursor {
        json["page"]["cursor"] = serde_json::to_value(cursor)?;
    } else {
        json["page"]["expanded"] = Value::Bool(true);
    }
    Ok(DeliveryExpansion {
        tool: request.tool(),
        arguments: json,
    })
}
struct Scan<'a> {
    request: &'a Request,
    map: PacketEvidenceMap,
    followups: u32,
    maximum_followups: u32,
    libraries: std::collections::BTreeMap<Id<input::Release>, Vec<Name>>,
}
struct FieldEvidence {
    original: Option<OriginalRange>,
    binding: DeliveryBinding,
    qualifications: Vec<Id<assertion::AssertionQualification>>,
    dependencies: Vec<String>,
}
impl Scan<'_> {
    fn field(
        &mut self,
        path: String,
        role: DeliveryRole,
        evidence: FieldEvidence,
        availability: Availability,
    ) -> Result<(), WireError> {
        self.map.fields.push(DeliveredEvidence {
            field: name(path)?,
            role,
            original: Nullable(evidence.original),
            binding: evidence.binding,
            qualifications: evidence.qualifications,
            dependencies: evidence
                .dependencies
                .into_iter()
                .map(name)
                .collect::<Result<_, _>>()?,
            availability,
        });
        Ok(())
    }
    fn omission(
        &mut self,
        path: String,
        availability: Availability,
        expand: Option<DeliveryExpansion>,
    ) -> Result<(), WireError> {
        let expand = if self.followups < self.maximum_followups {
            if expand.is_some() {
                self.followups += 1;
            }
            expand
        } else {
            None
        };
        self.map.omissions.push(DeliveryOmission {
            field: name(path)?,
            availability,
            expand: Nullable(expand),
        });
        Ok(())
    }
    fn visit(
        &mut self,
        value: &Value,
        path: &str,
        inherited: &DeliveryBinding,
        inherited_release: &Option<ReleaseIdentity>,
    ) -> Result<(), WireError> {
        if let Some(values) = value.as_array() {
            for (i, v) in values.iter().enumerate() {
                self.visit(v, &format!("{path}/{i}"), inherited, inherited_release)?;
            }
            return Ok(());
        }
        let Some(object) = value.as_object() else {
            return Ok(());
        };
        let mut binding = binding(value, inherited);
        let release =
            parse::<ReleaseIdentity>(value, "release").or_else(|| inherited_release.clone());
        let qualification: Option<Id<assertion::AssertionQualification>> =
            parse(value, "qualification");
        let quals = qualification.into_iter().collect::<Vec<_>>();
        let original: Option<OriginalRange> = parse(value, "original");
        let availability = parse(value, "availability").unwrap_or(Availability::Available {});
        // An excerpt's text is the exact independently captured source; a null text is a reference.
        if let Some(original) = original.clone() {
            if binding.analysis.0.is_some_and(|id| id != original.context) {
                return Err(WireError::Invalid(
                    "delivered source/context association".into(),
                ));
            }
            binding.analysis = Nullable(Some(original.context));
            if value.get("text").is_some() {
                if value.get("text").is_some_and(Value::is_string) {
                    self.field(
                        format!("{path}/text"),
                        DeliveryRole::Primary,
                        FieldEvidence {
                            original: Some(original.clone()),
                            binding: binding.clone(),
                            qualifications: quals.clone(),
                            dependencies: vec![],
                        },
                        availability.clone(),
                    )?;
                } else {
                    let expand = DeliveryExpansion {
                        tool: Tool::GetEvidence,
                        arguments: serde_json::json!({"source":original.source,"page":{"expanded":true,"evidence_demand":{"facets":["originals","conditions","setup"],"context":{"analysis":original.context},"maximum_followups":0}}}),
                    };
                    self.omission(format!("{path}/text"), availability.clone(), Some(expand))?;
                }
            }
            if let Some(body) = value.get("body") {
                let start = body
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| WireError::Invalid("delivered original body start".into()))?;
                let end = body
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| WireError::Invalid("delivered original body end".into()))?;
                let raw = matches!(original.encoding.as_str(), "raw_bytes" | "utf-8");
                let length = original
                    .end
                    .checked_sub(original.start)
                    .ok_or_else(|| WireError::Invalid("delivered original bounds".into()))?;
                if end < start
                    || (raw && end > length)
                    || body
                        .get("bytes")
                        .and_then(Value::as_array)
                        .is_none_or(|bytes| bytes.len() as u64 != end - start)
                {
                    return Err(WireError::Invalid("delivered original body bounds".into()));
                }
                let source = if raw {
                    let mut actual = original;
                    actual.start += start;
                    actual.end = actual.start + (end - start);
                    Some(actual)
                } else {
                    None
                };
                self.field(
                    format!("{path}/body/bytes"),
                    if raw {
                        DeliveryRole::Primary
                    } else {
                        DeliveryRole::Synthetic
                    },
                    FieldEvidence {
                        original: source,
                        binding: binding.clone(),
                        qualifications: quals.clone(),
                        dependencies: vec![
                            format!("{path}/release"),
                            format!("{path}/interpretation"),
                        ],
                    },
                    Availability::Available {},
                )?;
                if body.get("truncated").and_then(Value::as_bool) == Some(true)
                    || body
                        .get("omitted")
                        .and_then(Value::as_u64)
                        .is_some_and(|n| n > 0)
                {
                    self.omission(
                        format!("{path}/body"),
                        partial("original_body_page_omitted")?,
                        parse::<CursorToken>(body, "continuation")
                            .map(|cursor| expansion(self.request, Some(cursor)))
                            .transpose()?,
                    )?;
                }
            }
        }
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            let omitted = value.get("omitted").and_then(Value::as_u64).unwrap_or(0);
            let truncated = value
                .get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if omitted > 0 || truncated || !matches!(availability, Availability::Available {}) {
                let cursor = parse::<CursorToken>(value, "continuation");
                let expand = if cursor.is_some() {
                    Some(expansion(self.request, cursor)?)
                } else if matches!(availability,Availability::Partial{ref reason} if reason.as_str()=="delivery_envelope_expand")
                {
                    Some(expansion(self.request, None)?)
                } else {
                    None
                };
                self.omission(
                    path.into(),
                    if omitted > 0 || truncated {
                        partial("page_or_delivery_omission")?
                    } else {
                        availability.clone()
                    },
                    expand,
                )?;
            }
            // Items are scanned below, with bindings derived from each actual enclosing item.
            let _ = items;
        }
        // Navigation candidates carry no signature/default closure. Demand yields explicit
        // public expansions pinned to the actual member and analysis, never expected labels.
        if let (Some(demand), Some(member), Some(analysis), Some(releases)) = (
            self.request.page().evidence_demand.0.as_ref(),
            parse::<Id<catalog::CatalogMember>>(value, "member"),
            parse::<Id<attribution::AnalysisContext>>(value, "analysis"),
            value.get("releases").and_then(Value::as_array),
        ) && !demand.facets.is_empty()
        {
            let arguments: Value = serde_json::from_str(&self.request.to_json()?)?;
            if let Some(library) = arguments.get("library") {
                for release in releases {
                    let Some(version) = release.get("version") else {
                        continue;
                    };
                    if demand
                        .context
                        .release
                        .0
                        .as_ref()
                        .is_some_and(|wanted| version.as_str() != Some(wanted.as_str()))
                    {
                        continue;
                    }
                    let mut expansion_demand = demand.clone();
                    expansion_demand.context.analysis = Optional::supplied(analysis);
                    expansion_demand.context.release =
                        Optional::supplied(serde_json::from_value(version.clone())?);
                    let expansion = DeliveryExpansion {
                        tool: Tool::GetOperation,
                        arguments: serde_json::json!({"library":library,"operation":{"kind":"member","member":member},"page":{"expanded":true,"evidence_demand":expansion_demand}}),
                    };
                    self.omission(
                        format!("{path}/interpretation"),
                        partial("navigation_requires_contextual_operation_expansion")?,
                        Some(expansion),
                    )?;
                }
            }
        }
        if object.contains_key("window")
            && parse::<retrieval::PartPurpose>(value, "purpose")
                == Some(retrieval::PartPurpose::Context)
            && ["member", "binding", "subject", "basis"]
                .iter()
                .any(|key| value.get(key).is_some_and(|v| !v.is_null()))
        {
            return Err(WireError::Invalid(
                "context window cannot nominate member/binding".into(),
            ));
        }
        if object.contains_key("window")
            && parse::<retrieval::PartPurpose>(value, "purpose")
                == Some(retrieval::PartPurpose::Primary)
            && let (Some(demand), Some(member), Some(analysis), Some(release)) = (
                self.request.page().evidence_demand.0.as_ref(),
                parse::<Id<catalog::CatalogMember>>(value, "member"),
                parse::<Id<attribution::AnalysisContext>>(value, "analysis"),
                release.as_ref(),
            )
            && !demand.facets.is_empty()
        {
            let libraries = self
                .libraries
                .get(&release.release)
                .cloned()
                .unwrap_or_default();
            for library in libraries {
                let mut demand = demand.clone();
                demand.context.analysis = Optional::supplied(analysis);
                demand.context.release = Optional::supplied(release.version.clone());
                let expand = DeliveryExpansion {
                    tool: Tool::GetOperation,
                    arguments: serde_json::json!({"library":library,"operation":{"kind":"member","member":member},"page":{"expanded":true,"evidence_demand":demand}}),
                };
                self.omission(
                    format!("{path}/operation"),
                    partial("primary_window_member_requires_contextual_operation_expansion")?,
                    Some(expand),
                )?;
            }
        }
        if object.contains_key("window")
            && object.contains_key("source_maps")
            && object.get("text").is_some_and(Value::is_string)
        {
            self.field(
                format!("{path}/text"),
                if parse::<retrieval::PartPurpose>(value, "purpose")
                    == Some(retrieval::PartPurpose::Context)
                {
                    DeliveryRole::Interpretation
                } else {
                    DeliveryRole::Primary
                },
                FieldEvidence {
                    original: None,
                    binding: binding.clone(),
                    qualifications: quals.clone(),
                    dependencies: vec![
                        format!("{path}/source_maps"),
                        format!("{path}/analysis"),
                        format!("{path}/qualification"),
                    ],
                },
                Availability::Available {},
            )?;
        }
        let mut dependencies = vec![];
        if object.get("analysis").is_some_and(|v| !v.is_null())
            && object.get("variant").is_some_and(|v| !v.is_null())
        {
            dependencies.push(format!("{path}/analysis"));
            dependencies.push(format!("{path}/variant"));
        } else if object.contains_key("option")
            && object.get("analysis").is_some_and(|v| !v.is_null())
            && object.get("field").is_some_and(|v| !v.is_null())
        {
            dependencies.push(format!("{path}/analysis"));
            dependencies.push(format!("{path}/field"));
        }
        for key in [
            "name",
            "subject_name",
            "title",
            "rendered",
            "readable",
            "predicate",
            "constant",
            "python_version",
            "python_platform",
            "search_path",
            "site_package_path",
            "distribution",
            "version",
        ] {
            if let Some(v) = object.get(key) {
                if v.is_null() {
                    continue;
                }
                let role = if matches!(key, "name" | "subject_name" | "title" | "rendered") {
                    DeliveryRole::Primary
                } else {
                    DeliveryRole::Synthetic
                };
                let source = if key == "readable"
                    && matches!(
                        value
                            .get("value")
                            .and_then(|v| v.get("kind"))
                            .and_then(Value::as_str),
                        Some("expression" | "factory")
                    ) {
                    value
                        .get("original")
                        .and_then(|e| e.get("original"))
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                } else {
                    None
                };
                self.field(
                    format!("{path}/{key}"),
                    if source.is_some() {
                        DeliveryRole::Primary
                    } else {
                        role
                    },
                    FieldEvidence {
                        original: source,
                        binding: binding.clone(),
                        qualifications: quals.clone(),
                        dependencies: dependencies.clone(),
                    },
                    availability.clone(),
                )?;
            }
        }
        if let Some(default) = object.get("default")
            && default.is_object()
        {
            self.field(
                format!("{path}/default"),
                DeliveryRole::Reference,
                FieldEvidence {
                    original: None,
                    binding: binding.clone(),
                    qualifications: quals.clone(),
                    dependencies: vec![],
                },
                unavailable("default_reference_is_not_readable_value_or_condition")?,
            )?;
        }
        if object.contains_key("source")
            && object.contains_key("artifact")
            && object.contains_key("digest")
            && let Ok(original) = serde_json::from_value::<OriginalRange>(value.clone())
        {
            let mut source_binding = binding.clone();
            if source_binding
                .analysis
                .0
                .is_some_and(|id| id != original.context)
            {
                return Err(WireError::Invalid(
                    "original reference/container analysis".into(),
                ));
            }
            source_binding.analysis = Nullable(Some(original.context));
            self.field(
                path.into(),
                DeliveryRole::Reference,
                FieldEvidence {
                    original: Some(original.clone()),
                    binding: source_binding,
                    qualifications: quals.clone(),
                    dependencies: vec![],
                },
                unavailable("original_reference_without_source_body")?,
            )?;
        }
        for (key, v) in object {
            if key == "delivery" {
                continue;
            }
            let key = key.replace('~', "~0").replace('/', "~1");
            self.visit(v, &format!("{path}/{key}"), &binding, &release)?;
        }
        Ok(())
    }
}
fn evidence_map(request: &Request, response: &Response) -> Result<PacketEvidenceMap, WireError> {
    let ranked = matches!(
        response,
        Response::SearchOperations(_)
            | Response::SearchEvidence(_)
            | Response::SearchCapabilities(_)
    );
    let public: Value = serde_json::from_str(&response.to_json()?)?;
    let mut libraries: std::collections::BTreeMap<Id<input::Release>, Vec<Name>> =
        std::collections::BTreeMap::new();
    for domain in public
        .get("domains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(library) = parse::<Name>(domain, "name") {
            for capture in domain
                .get("captures")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(release) = parse::<ReleaseIdentity>(capture, "release") {
                    let names = libraries.entry(release.release).or_default();
                    if !names.contains(&library) {
                        names.push(library.clone());
                    }
                }
            }
        }
    }
    let mut scan = Scan {
        request,
        libraries,
        map: PacketEvidenceMap {
            fields: vec![],
            omissions: vec![],
            ranked_continuation: Nullable(ranked.then(RankedContinuationPolicy::default)),
            packing_policy: name("exact_mcp_cost_whole_optional_bundles_with_transport_reserve")?,
        },
        followups: 0,
        maximum_followups: request
            .page()
            .evidence_demand
            .0
            .as_ref()
            .map_or(20, |d| d.maximum_followups),
    };
    scan.field(
        "/content/0/text".into(),
        DeliveryRole::Synthetic,
        FieldEvidence {
            original: None,
            binding: DeliveryBinding::default(),
            qualifications: vec![],
            dependencies: vec![],
        },
        Availability::Available {},
    )?;
    scan.visit(
        &public,
        "/structuredContent",
        &DeliveryBinding::default(),
        &None,
    )?;
    Ok(scan.map)
}
fn check_context(request: &Request, response: &Response) -> Result<(), WireError> {
    let Some(demand) = &request.page().evidence_demand.0 else {
        return Ok(());
    };
    match response {
        Response::GetOperation(r) => {
            if let OperationResolution::Unique { packet } = &r.operation {
                let c = &demand.context;
                if c.release
                    .0
                    .as_ref()
                    .is_some_and(|v| v != &packet.core.release.version)
                {
                    return Err(WireError::Invalid(
                        "demand release differs from selected operation".into(),
                    ));
                }
                if c.signature.0.is_none()
                    && c.variant.0.is_none()
                    && c.analysis.0.is_some_and(|id| {
                        !packet
                            .core
                            .interpretation
                            .contexts
                            .iter()
                            .any(|context| context.analysis == id)
                    })
                {
                    return Err(WireError::Invalid(
                        "demand analysis has no operation context".into(),
                    ));
                }
                if (c.signature.0.is_some() || c.variant.0.is_some())
                    && !packet.core.signatures.iter().any(|s| {
                        c.signature.0.is_none_or(|id| id == s.signature)
                            && c.variant.0.is_none_or(|id| id == s.variant)
                            && c.analysis.0.is_none_or(|id| id == s.analysis)
                    })
                {
                    return Err(WireError::Invalid(
                        "demand context has no compatible signature".into(),
                    ));
                }
            }
        }
        Response::GetEvidence(r) => {
            if demand
                .context
                .release
                .0
                .as_ref()
                .is_some_and(|v| v != &r.evidence.release.version)
                || demand
                    .context
                    .analysis
                    .0
                    .is_some_and(|id| id != r.evidence.original.context)
            {
                return Err(WireError::Invalid(
                    "demand differs from original context".into(),
                ));
            }
            if demand.context.signature.0.is_some() || demand.context.variant.0.is_some() {
                return Err(WireError::Invalid(
                    "original bytes do not establish a signature/variant binding".into(),
                ));
            }
        }
        _ => {
            if demand.context.signature.0.is_some() || demand.context.variant.0.is_some() {
                return Err(WireError::Invalid(
                    "signature context requires get_operation".into(),
                ));
            }
        }
    }
    Ok(())
}
fn protected(request: &Request, section: &str) -> bool {
    request.page().evidence_demand.0.as_ref().is_some_and(|d| {
        d.facets.iter().any(|facet| {
            matches!(
                (facet, section),
                (EvidenceFacet::Scenarios, "scenarios")
                    | (EvidenceFacet::Deployment, "deployment")
                    | (EvidenceFacet::Relationships, "relationships")
                    | (EvidenceFacet::Behavior, "behavior")
            )
        })
    })
}
fn omit<T: serde::Serialize>(page: &mut SectionPage<T>, expanded: bool) -> Result<(), WireError> {
    page.omitted = page
        .omitted
        .checked_add(page.items.len() as u64)
        .ok_or_else(|| WireError::Invalid("delivery omission count".into()))?;
    page.items.clear();
    page.truncated = true;
    page.continuation = Optional::default();
    page.availability = if expanded {
        unavailable("optional_bundle_exceeds_expanded_envelope")?
    } else {
        partial("delivery_envelope_expand")?
    };
    Ok(())
}
fn optional_bundle(request: &Request, response: &mut Response) -> Result<bool, WireError> {
    let Response::GetOperation(r) = response else {
        return Ok(false);
    };
    let OperationResolution::Unique { packet } = &mut r.operation else {
        return Ok(false);
    };
    let mut choices = vec![];
    macro_rules! choose {($($field:ident),*)=>{$(if !protected(request,stringify!($field))&&!packet.$field.items.is_empty(){choices.push((serde_json::to_vec(&packet.$field)?.len(),stringify!($field)));})*};}
    choose!(
        access_routes,
        callable_comparison,
        contextual_typing,
        incoming_references,
        scenarios,
        deployment,
        relationships,
        conflicts,
        briefs,
        behavior
    );
    choices.sort();
    let Some((_, selected)) = choices.pop() else {
        return Ok(false);
    };
    macro_rules! remove {($($field:ident),*)=>{match selected{$(stringify!($field)=>omit(&mut packet.$field,request.page().expanded)?,)*_=>unreachable!()}};}
    remove!(
        access_routes,
        callable_comparison,
        contextual_typing,
        incoming_references,
        scenarios,
        deployment,
        relationships,
        conflicts,
        briefs,
        behavior
    );
    Ok(true)
}
fn ranked_tail(request: &Request, response: &mut Response) -> Result<bool, WireError> {
    fn trim<T>(
        page: &mut SectionPage<T>,
        ranking: &mut Vec<ranking::RankedHit>,
        extent: &mut SelectionExtent,
        request: &Request,
        snapshot: &SnapshotHandle,
        channels: &ChannelState,
    ) -> Result<bool, WireError> {
        if page.items.len() != ranking.len() {
            return Err(WireError::Invalid(
                "ranked delivery rows/witnesses mismatch".into(),
            ));
        }
        if page.items.len() <= 1 {
            return Ok(false);
        }
        let token = page.continuation.0.as_ref().ok_or_else(|| {
            WireError::Continuation(
                "retained ranked continuation required for delivery packing".into(),
            )
        })?;
        let expected = crate::pagination::binding(
            request,
            snapshot,
            channels,
            request.tool().name(),
            "results",
            None,
        )?;
        let mut cursor = Cursor::decode(token, &expected)?;
        let CursorPosition::Ranked { offset, .. } = &mut cursor.after else {
            return Err(WireError::Continuation(
                "ranked packing cursor required".into(),
            ));
        };
        *offset = offset
            .checked_sub(1)
            .ok_or_else(|| WireError::Continuation("ranked packing offset underflow".into()))?;
        page.omitted = page
            .omitted
            .checked_add(1)
            .ok_or_else(|| WireError::Invalid("ranked omission overflow".into()))?;
        page.items.pop();
        ranking.pop();
        page.truncated = true;
        page.continuation = Optional::supplied(cursor.encode()?);
        *extent = SelectionExtent::Ranked {
            returned: page.items.len() as u64,
        };
        Ok(true)
    }
    match response {
        Response::SearchOperations(r) => trim(
            &mut r.results,
            &mut r.ranking,
            &mut r.extent,
            request,
            &r.snapshot,
            &r.channels,
        ),
        Response::SearchEvidence(r) => trim(
            &mut r.results,
            &mut r.ranking,
            &mut r.extent,
            request,
            &r.snapshot,
            &r.channels,
        ),
        Response::SearchCapabilities(r) => trim(
            &mut r.results,
            &mut r.ranking,
            &mut r.extent,
            request,
            &r.snapshot,
            &r.channels,
        ),
        _ => Ok(false),
    }
}
/// Shared fresh/resumed-page entry point. Core/signatures/interpretation are never pruned.
/// The actual Python transport additionally admits the complete JSON-RPC envelope and metadata.
pub fn finalize(request: &Request, response: &mut Response) -> Result<(), WireError> {
    if request.tool() != response.tool() {
        return Err(WireError::Invalid("delivery request/response route".into()));
    }
    check_context(request, response)?;
    // Leave transport framing/IDs room; actual transport admission remains authoritative.
    let limit = ResourceLimits::default()
        .response_bytes(request.page().expanded)
        .saturating_sub(1024) as usize;
    loop {
        *response.delivery_mut() = Optional::default();
        let map = evidence_map(request, response)?;
        *response.delivery_mut() = Optional::supplied(map);
        if response.mcp_result_len()? <= limit {
            return Ok(());
        }
        if !optional_bundle(request, response)? && !ranked_tail(request, response)? {
            return Err(WireError::ResourceRefused(
                "indivisible core/closure or demanded bundle exceeds final envelope".into(),
            ));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn request() -> Request {
        decode_request("get_evidence",&serde_json::to_string(&serde_json::json!({"source":{"kind":"occurrence","occurrence":id::<source::Occurrence>(1)},"page":{"evidence_demand":{"facets":["originals"],"maximum_followups":2}}})).unwrap(),&ResourceLimits::default()).unwrap()
    }
    fn original(encoding: &str) -> OriginalRange {
        OriginalRange {
            source: OriginalReference::Occurrence { occurrence: id(1) },
            artifact: id(2),
            start: 10,
            end: 20,
            digest: ContentHash::of(b"0123456789"),
            encoding: Name::new(encoding).unwrap(),
            release: id(3),
            context: id(4),
        }
    }
    fn scan(request: &Request, value: Value) -> Result<PacketEvidenceMap, WireError> {
        let mut scan = Scan {
            request,
            map: PacketEvidenceMap {
                fields: vec![],
                omissions: vec![],
                ranked_continuation: Nullable(None),
                packing_policy: Name::new("test").unwrap(),
            },
            followups: 0,
            maximum_followups: 2,
            libraries: std::collections::BTreeMap::new(),
        };
        scan.visit(
            &value,
            "/structuredContent/evidence",
            &DeliveryBinding::default(),
            &None,
        )?;
        Ok(scan.map)
    }
    #[test]
    fn raw_body_context_and_continuation_are_actual() {
        let request = request();
        let original = original("raw_bytes");
        let map=scan(&request,serde_json::json!({"original":original,"body":{"start":2,"end":5,"bytes":[50,51,52],"omitted":5,"truncated":true,"continuation":"ab"}})).unwrap();
        let field = map
            .fields
            .iter()
            .find(|f| f.field.as_str().ends_with("/body/bytes"))
            .unwrap();
        assert_eq!(field.binding.analysis.0, Some(id(4)));
        assert_eq!(
            field.original.0.as_ref().map(|v| (v.start, v.end)),
            Some((12, 15))
        );
        assert_eq!(
            map.omissions[0].expand.0.as_ref().unwrap().arguments["page"]["cursor"],
            "ab"
        );
        assert_eq!(
            map.omissions[0].expand.0.as_ref().unwrap().arguments["page"]["expanded"],
            false
        );
    }
    #[test]
    fn interpreted_prose_has_no_original_byte_map() {
        let map=scan(&request(),serde_json::json!({"original":original("native_literal_utf8_slice"),"body":{"start":0,"end":30,"bytes":vec![65;30],"truncated":false,"omitted":0}})).unwrap();
        let field = map
            .fields
            .iter()
            .find(|f| f.field.as_str().ends_with("/body/bytes"))
            .unwrap();
        assert!(field.original.0.is_none());
        assert_eq!(field.role, DeliveryRole::Synthetic);
    }
    #[test]
    fn foreign_container_cannot_certify_original_context() {
        assert!(scan(&request(),serde_json::json!({"analysis":id::<attribution::AnalysisContext>(9),"original":original("raw_bytes"),"text":"same substring"})).is_err());
    }
    #[test]
    fn navigation_demand_expands_actual_member_and_context() {
        let request=decode_request("find_operations",r#"{"library":"control","page":{"evidence_demand":{"facets":["defaults","conditions"],"maximum_followups":1}}}"#,&ResourceLimits::default()).unwrap();
        let value = serde_json::json!({"member":id::<catalog::CatalogMember>(5),"analysis":id::<attribution::AnalysisContext>(6),"name":"call","releases":[{"version":"1.2"}]});
        let map = scan(&request, value).unwrap();
        let expansion = map.omissions[0].expand.0.as_ref().unwrap();
        assert_eq!(expansion.tool, Tool::GetOperation);
        assert_eq!(
            expansion.arguments["operation"]["member"],
            serde_json::to_value(id::<catalog::CatalogMember>(5)).unwrap()
        );
        assert_eq!(
            expansion.arguments["page"]["evidence_demand"]["context"]["analysis"],
            serde_json::to_value(id::<attribution::AnalysisContext>(6)).unwrap()
        );
    }
    #[test]
    fn evidence_demand_expands_only_actual_primary_member() {
        let request=decode_request("search_evidence",r#"{"query":"call","families":[],"page":{"evidence_demand":{"facets":["defaults"],"maximum_followups":2}}}"#,&ResourceLimits::default()).unwrap();
        let release = ReleaseIdentity {
            input: id(2),
            release: id(3),
            distribution: Name::new("package").unwrap(),
            version: Name::new("1").unwrap(),
        };
        let mut scan = Scan {
            request: &request,
            map: PacketEvidenceMap {
                fields: vec![],
                omissions: vec![],
                ranked_continuation: Nullable(None),
                packing_policy: Name::new("test").unwrap(),
            },
            followups: 0,
            maximum_followups: 2,
            libraries: std::collections::BTreeMap::from([(
                release.release,
                vec![Name::new("actual-domain").unwrap()],
            )]),
        };
        let window = serde_json::json!({"window":id::<retrieval::SearchWindow>(4),"member":id::<catalog::CatalogMember>(5),"analysis":id::<attribution::AnalysisContext>(6),"purpose":0,"text":"call","source_maps":[]});
        scan.visit(
            &window,
            "/structuredContent/results/items/0/delivered_windows/0",
            &DeliveryBinding::default(),
            &Some(release),
        )
        .unwrap();
        let args = &scan.map.omissions[0].expand.0.as_ref().unwrap().arguments;
        assert_eq!(args["library"], "actual-domain");
        assert_eq!(args["page"]["evidence_demand"]["context"]["release"], "1");
        let mut context = window;
        context["purpose"] = serde_json::json!(1);
        assert!(
            scan.visit(&context, "/context", &DeliveryBinding::default(), &None)
                .is_err()
        );
    }
    #[test]
    fn field_interpretation_never_invents_signature() {
        let map=scan(&request(),serde_json::json!({"option":id::<catalog::CatalogOption>(8),"field":id::<normalized::entities::FieldEntity>(7),"analysis":id::<attribution::AnalysisContext>(6),"signature":null,"variant":null,"parameter":null,"subject_name":"limit","readable":"8"})).unwrap();
        let field = map
            .fields
            .iter()
            .find(|v| v.field.as_str().ends_with("/readable"))
            .unwrap();
        assert_eq!(field.binding.field.0, Some(id(7)));
        assert!(field.binding.signature.0.is_none());
        assert!(field.binding.variant.0.is_none());
        assert_eq!(
            field
                .dependencies
                .iter()
                .map(Name::as_str)
                .collect::<Vec<_>>(),
            vec![
                "/structuredContent/evidence/analysis",
                "/structuredContent/evidence/field"
            ]
        );
    }
}
#[cfg(test)]
mod packing_tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn ranked_envelope_packing_preserves_next_undelivered_offset() {
        let request = decode_request(
            "search_evidence",
            r#"{"query":"fragment","families":[],"page":{"size":3}}"#,
            &ResourceLimits::default(),
        )
        .unwrap();
        let snapshot = SnapshotHandle {
            publication: lctx_model::domain::ContentHash::of(b"fixture-publication"),
            view: lctx_model::domain::ContentHash::of(b"fixture-view"),
            service_generation: lctx_model::domain::ContentHash::of(b"fixture-service_generation"),
            definition_epoch: lctx_model::domain::ContentHash::of(b"fixture-definition_epoch"),
            semantic: ContentHash::of(b"s"),
            realization: ContentHash::of(b"r"),
            database: DatabaseIdentity {
                namespace: Name::new("control").unwrap(),
                database: Name::new("packing").unwrap(),
            },
        };
        let channels = ChannelState {
            lexical: true,
            vector: VectorChannel::Degraded {
                reason: Name::new("test").unwrap(),
            },
        };
        let binding = crate::pagination::binding(
            &request,
            &snapshot,
            &channels,
            "search_evidence",
            "results",
            None,
        )
        .unwrap();
        let cursor = Cursor {
            binding: binding.clone(),
            after: CursorPosition::Ranked {
                session: ContentHash::of(b"session"),
                result: ContentHash::of(b"result"),
                digest: ContentHash::of(b"digest"),
                offset: 3,
            },
        };
        let mut items = vec![];
        let mut rankings = vec![];
        for n in 1..=3 {
            let unit = id(n);
            let analysis = id(n);
            items.push(EvidenceHit {
                unit,
                release: ReleaseIdentity {
                    input: id(n),
                    release: id(n),
                    distribution: Name::new("control").unwrap(),
                    version: Name::new("1").unwrap(),
                },
                family: retrieval::Family::Source,
                title: Name::new("original").unwrap(),
                originals: vec![],
                associated_members: vec![],
                delivered_windows: vec![DeliveredWindow {
                    window: id(n),
                    part: id(n),
                    purpose: retrieval::PartPurpose::Primary,
                    member: Nullable(None),
                    analysis,
                    binding: Nullable(None),
                    subject: Nullable(None),
                    basis: Nullable(None),
                    qualification: Nullable(None),
                    text: Text::new("x".repeat(18000)).unwrap(),
                    source_maps: vec![DeliveredWindowMap {
                        start: 0,
                        end: 18000,
                        original: Nullable(Some(OriginalRange {
                            source: OriginalReference::Occurrence { occurrence: id(n) },
                            artifact: id(n),
                            start: 0,
                            end: 18000,
                            digest: ContentHash::of(&vec![b'x'; 18000]),
                            encoding: Name::new("raw_bytes").unwrap(),
                            release: id(n),
                            context: analysis,
                        })),
                        availability: Availability::Available {},
                    }],
                }],
                interpretation: InterpretationClosure {
                    contexts: vec![],
                    defaults: vec![],
                    qualifications: vec![],
                    availability: Availability::Unavailable {
                        reason: Name::new("opaque").unwrap(),
                    },
                },
            });
            rankings.push(ranking::RankedHit {
                target: ranking::Target::Unit { unit },
                context: analysis,
                score: 1.0 / f64::from(n),
                promoted: false,
                witnesses: vec![],
            });
        }
        let mut response = Response::SearchEvidence(SearchEvidenceResponse {
            snapshot,
            delivery: Optional::default(),
            domains: vec![],
            results: SectionPage {
                availability: Availability::Available {},
                items,
                continuation: Optional::supplied(cursor.encode().unwrap()),
                omitted: 0,
                truncated: false,
            },
            channels,
            extent: SelectionExtent::Ranked { returned: 3 },
            ranking: rankings,
        });
        finalize(&request, &mut response).unwrap();
        assert!(
            response.mcp_result_len().unwrap()
                <= ResourceLimits::default().response_bytes(false) as usize - 1024
        );
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(256 * 1024).unwrap();
        let limit = ResourceLimits::default().response_bytes(false) as usize;
        let encoded = response.encode_json(&budget, limit).unwrap();
        assert_eq!(encoded.as_str(), response.to_json().unwrap());
        let raw: Value = serde_json::from_str(encoded.as_str()).unwrap();
        let envelope = response.encode_mcp_result(&budget, limit).unwrap();
        let public: Value = serde_json::from_str(envelope.as_str()).unwrap();
        assert_eq!(public["structuredContent"], raw);
        assert_eq!(envelope.as_str().len(), response.mcp_result_len().unwrap());
        assert_eq!(raw["results"]["items"][0]["delivered_windows"][0]["source_maps"][0]["original"]["end"], 18000);
        assert!(raw["delivery"]["fields"].as_array().unwrap().iter().any(|field|
            field["original"]["end"] == 18000 && field["role"] == "reference"));
        let Response::SearchEvidence(r) = response else {
            panic!("route")
        };
        assert_eq!(r.results.items.len(), 1);
        assert_eq!(r.ranking.len(), 1);
        assert_eq!(r.results.omitted, 2);
        assert!(r.results.truncated);
        let next = Cursor::decode(r.results.continuation.0.as_ref().unwrap(), &binding).unwrap();
        assert!(
            matches!(next.after,CursorPosition::Ranked{session,result,digest,offset:1} if session==ContentHash::of(b"session")&&result==ContentHash::of(b"result")&&digest==ContentHash::of(b"digest"))
        );
        assert!(
            r.delivery
                .0
                .unwrap()
                .fields
                .iter()
                .all(|f| !f.field.as_str().contains("/items/1/"))
        );
    }
}
