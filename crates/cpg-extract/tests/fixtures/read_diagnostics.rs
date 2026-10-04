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
