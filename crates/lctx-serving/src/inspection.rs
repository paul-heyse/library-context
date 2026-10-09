//! Exact-input kernel over the selected member's complete native semantic closure.
use crate::records::wire;
use lctx_model::domain::{
    resources::ResourceBudget, serving::mappings::PacketOutput, serving::*, *,
};
use lctx_surrealdb::{NativeReader, reader::target_id};
pub async fn get(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    r: &InspectValuePathsRequest,
    request: &Request,
    _limits: &ResourceLimits,
    b: &ResourceBudget,
) -> Result<InspectValuePathsResponse, ModelError> {
    let inputs = NativeAssessmentPacket::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect::<Vec<_>>();
    let mut fields = crate::scope::OWNED_FIELDS.to_vec();
    fields.extend([
        "owner", "formal", "entry", "path", "access", "key", "transfer", "route",
    ]);
    let batches = preparation.hydrate(
        reader,
        vec![target_id(graph::Target::Entity(graph::EntityId::of(
            r.member,
        )))],
        &inputs,
        &inputs,
        &fields,
    )
    .await?;
    let mut data = selection::classification::ClassificationData::new(b);
    let mut native = native_requests::NativeInventory::new(b);
    let mut rows = native_requests::PreparationRows::new(b);
    let native_inputs = native_requests::NativeInventory::inputs()
        .into_iter()
        .map(|input| input.name().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    for (name, batch) in &batches.batches {
        data.visit(name, batch)?;
        if native_inputs.contains(*name) {
            native.visit(name, batch)?;
        }
        rows.visit(name, batch)?;
    }
    let binding = if !rows.substitutions.is_empty() {
        let mut d = normalized::binding_normalization::BindingData::new(b);
        let mut o = normalized::binding_normalization::BindingOutput::new(b);
        for (name, batch) in &batches.batches {
            d.visit(name, batch)?;
            o.visit(name, batch)?;
        }
        Some((d, o))
    } else {
        None
    };
    let prepared = native_requests::PreparedNativeSemantics::prepare(
        native_requests::PreparationInputs {
            native,
            rows,
            binding,
        },
        &data,
        b,
    )?;
    let canonical=crate::records::Prepared::new(&batches,b);
    let claims = canonical.claims()?;
    let mut canonical = r.clone();
    let owner = prepared.domain().resolve(&mut canonical)?;
    let mut values = Vec::new();
    for input in &canonical.inputs {
        let exact = native_requests::ExactRequest {
            snapshot: reader.handle().clone(),
            owner,
            formal: input.formal,
            value: &input.value,
            assumptions: canonical.assumptions,
        };
        let count = prepared
            .path_count(owner, input.formal, r.analysis)
            .unwrap_or(0);
        for index in 0..count {
            let result = prepared.assess_path(&exact, r.analysis, index, b)?;
            let q = claims
                .qualifications
                .get(&result.qualification)
                .ok_or(ModelError::Schema("native path qualification"))?;
            let packet = NativeAssessmentPacket::from_canonical(&result, claims.basis(q.id())?)?;
            let mut sink = KeySink::new("native-path-page/v1");
            input.formal.encode(&mut sink);
            sink.part(b"path", result.path.bytes());
            values.push((sink.finish(), packet));
        }
    }
    let channels = ChannelState {
        lexical: false,
        vector: VectorChannel::Disabled {},
    };
    let availability = if values.is_empty() {
        Availability::Unavailable {
            reason: Name::new(native_requests::UNAVAILABLE_REASON).map_err(wire)?,
        }
    } else if prepared.coverage(r.analysis)
        == Some(attribution::CoverageStatus::CompleteUnderStatedModel)
    {
        Availability::Available {}
    } else {
        Availability::Partial {
            reason: Name::new("native_collection_partial").map_err(wire)?,
        }
    };
    Ok(InspectValuePathsResponse {
        delivery: Optional::default(),
        snapshot: reader.handle().clone(),
        member: r.member,
        paths: crate::pagination::page(
            values,
            request,
            reader.handle(),
            &channels,
            request.tool().name(),
            "paths",
            Some(r.member),
            availability,
        )
        .map_err(wire)?,
    })
}
