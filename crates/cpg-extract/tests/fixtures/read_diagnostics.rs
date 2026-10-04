//! Failure-only raw premise diagnostics; these never supply admission or expected answers.
use crate::fixture::NativeFixture;
use lctx_model::domain::{execution::read_channels::ReadRecords, *};

fn sample<R: Record + std::fmt::Debug>(f: &NativeFixture) {
    let rows = f.rows::<R>();
    eprintln!("read failure {} count={} sample={:?}", R::NAME, rows.len(), rows.iter().take(24).collect::<Vec<_>>());
}

pub fn dump(
    f: &NativeFixture,
    data: &execution::source_call_records::SourceCallData,
    records: &ReadRecords,
) {
    sample::<attribution::ProviderCoverage>(f);
    sample::<syntax::SubjectBoundary>(f);
    eprintln!("read failure fields={:?}", records.fields.assessments.iter().take(24).collect::<Vec<_>>());
    eprintln!("read failure locations={:?}", records.fields.locations.iter().take(24).collect::<Vec<_>>());
    eprintln!("read failure dynamic={:?}", records.dynamic.iter().take(24).collect::<Vec<_>>());
    let spelling = |site| {
        let occurrence = data.flow.occurrences.get(site).unwrap();
        String::from_utf8_lossy(&f.source_bytes(occurrence.source)[occurrence.start as usize..occurrence.end as usize]).into_owned()
    };
    for call in data.evaluation.call_syntax.iter().filter(|c| {
        matches!(spelling(c.callee).as_str(), "getattr" | "hasattr")
    }).take(24) {
        let resolutions = data.evaluation.lexical_resolutions.iter().filter(|r| r.read == call.callee).collect::<Vec<_>>();
        eprintln!("read failure call={} row={call:?}; resolutions={resolutions:?}", spelling(call.site));
        for resolution in resolutions.iter().take(8) {
            let support = data.evaluation.lexical_resolution_supports.iter().filter(|s| s.assertion == resolution.id()).take(8).collect::<Vec<_>>();
            eprintln!("read failure resolution qualification={:?}; supports={support:?}; target={:?}", data.flow.qualifications.get(resolution.qualification), data.evaluation.lexical_targets.get(resolution.target));
        }
    }
    for read in records.reads.iter().filter(|r| {
        data.flow.places.get(r.place).is_some_and(|p| matches!(data.flow.roots.get(p.root), Some(value::PlaceRoot::Global { .. })))
    }).take(24) {
        let observation = data.flow.use_observations.get(read.observation).unwrap();
        let use_ = data.flow.uses.get(observation.use_).unwrap();
        eprintln!("read failure global={} row={read:?}; lexical={:?}", spelling(use_.occurrence), data.evaluation.lexical_resolutions.iter().filter(|r| r.read == use_.occurrence).take(8).collect::<Vec<_>>());
    }
    sample::<lexical::BindingObservation>(f);
    sample::<lexical::BindingSupport>(f);
    sample::<ruff::RuffBindingObservation>(f);
    sample::<ruff::RuffBindingSupport>(f);
}

/// Exact initializer path diagnostics, including source geometry and per-use conditions.
pub fn dump_global_initializer(
    f: &NativeFixture,
    data: &execution::source_call_records::SourceCallData,
    records: &ReadRecords,
) {
    let spelling = |site| {
        let occurrence = data.flow.occurrences.get(site).unwrap();
        String::from_utf8_lossy(&f.source_bytes(occurrence.source)[occurrence.start as usize..occurrence.end as usize]).into_owned()
    };
    for row in records.dynamic.iter().filter(|d| spelling(d.site) == "getattr(fixed, name)").take(8) {
        eprintln!("initializer dynamic={row:?}; receiver={:?}", row.receiver.and_then(|r| data.flow.occurrences.get(r)));
    }
    for event in data.evaluation.binding_events.iter().filter(|e| e.name == "fixed" || e.name == "ChoiceA").take(16) {
        eprintln!("initializer event={event:?}; occurrence={:?}", data.flow.occurrences.get(event.site));
        for binding in data.evaluation.ruff_bindings.iter().filter(|b| b.event == event.id()).take(8) {
            eprintln!("initializer binding={binding:?}; scope={:?}; qualification={:?}", binding.scope.and_then(|s| data.flow.lexical_scopes.get(s)), data.flow.qualifications.get(binding.qualification));
            for support in data.evaluation.ruff_binding_supports.iter().filter(|s| s.assertion == binding.id()).take(8) {
                eprintln!("initializer binding support={support:?}");
            }
        }
    }
    for use_ in data.flow.uses.iter().filter(|u| spelling(u.occurrence) == "fixed").take(16) {
        eprintln!("initializer use={use_:?}; occurrence={:?}", data.flow.occurrences.get(use_.occurrence));
        for observed in data.flow.use_observations.iter().filter(|o| o.use_ == use_.id()).take(8) {
            eprintln!("initializer use observation={observed:?}; scope={:?}; qualification={:?}; supports={:?}",
                data.flow.lexical_scopes.get(observed.scope), data.flow.qualifications.get(observed.qualification),
                data.flow.use_supports.iter().filter(|s| s.assertion == observed.id()).take(8).collect::<Vec<_>>());
        }
    }
    for call in data.evaluation.call_syntax.iter().filter(|c| spelling(c.site) == "ChoiceA()" || spelling(c.site) == "getattr(fixed, name)").take(16) {
        eprintln!("initializer call={call:?}; source={}; qualification={:?}; supports={:?}", spelling(call.site), data.flow.qualifications.get(call.qualification), data.evaluation.call_syntax_supports.iter().filter(|s| s.assertion == call.id()).take(8).collect::<Vec<_>>());
        for read in [call.callee].into_iter().chain(data.evaluation.call_arguments.iter().filter(|a| a.call == call.id() && a.ordinal == 0).map(|a| a.value)) {
            for resolution in data.evaluation.lexical_resolutions.iter().filter(|r| r.read == read).take(8) {
                eprintln!("initializer lexical={resolution:?}; target={:?}; qualification={:?}; supports={:?}",
                    data.evaluation.lexical_targets.get(resolution.target), data.flow.qualifications.get(resolution.qualification),
                    data.evaluation.lexical_resolution_supports.iter().filter(|s| s.assertion == resolution.id()).take(8).collect::<Vec<_>>());
            }
        }
    }
    for placement in data.evaluation.placements.iter().filter(|p| {
        matches!(spelling(p.occurrence).as_str(), "fixed" | "ChoiceA()")
            && p.parent.is_some_and(|parent| spelling(parent) == "fixed = ChoiceA()")
    }).take(16) {
        eprintln!("initializer placement={placement:?}; qualification={:?}; supports={:?}",
            data.flow.qualifications.get(placement.qualification), data.flow.placement_supports.iter().filter(|s| s.assertion == placement.id()).take(8).collect::<Vec<_>>());
    }
}
