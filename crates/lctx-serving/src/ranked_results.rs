//! Session-owned immutable rankings. Continuations borrow charged rows without discovery.
use lctx_model::domain::{ContentHash, KeySink, charged::StateCharge, resources::ResourceBudget, serving::*};
use serde::Serialize;
use std::io::{self, Write};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant, SystemTime},
};

const MAX_ENTRIES: usize = RankedContinuationPolicy::MAXIMUM_ENTRIES as usize;
const MAX_BYTES: usize = RankedContinuationPolicy::MAXIMUM_RETAINED_BYTES as usize;
const LIFETIME: Duration = Duration::from_secs(RankedContinuationPolicy::EXPIRES_AFTER_SECONDS);
static SESSION: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RankedResults {
    session: ContentHash,
    state: Mutex<State>,
    sequence: AtomicU64,
    shared: ResourceBudget,
    lifetime: Duration,
}
#[derive(Default)]
struct State {
    entries: BTreeMap<ContentHash, Retained>,
    next: u64,
    bytes: usize,
}
struct Retained {
    entry: Arc<Entry>,
    used: u64,
}
struct Entry {
    binding: CursorBinding,
    digest: ContentHash,
    channels: ChannelState,
    rows: Vec<Vec<u8>>,
    template: Vec<u8>,
    created: Instant,
    charge: StateCharge,
}
struct Pending {
    result: ContentHash,
    entry: Entry,
}
/// Private construction and active borrowing live as long as the admitted service request.
/// Dropping a cancelled/refused request releases pending state without exposing a cursor.
pub(crate) struct RankedRequest<'a> {
    cache: &'a RankedResults,
    pending: Option<Pending>,
    borrowed: Option<Arc<Entry>>,
    scratch: StateCharge,
}
#[derive(serde::Deserialize)]
struct Row<T> {
    ranking: serde_json::Value,
    #[serde(rename = "key")]
    _key: ContentHash,
    item: T,
}
#[derive(Serialize)]
struct RowRef<'a, T> {
    ranking: &'a ranking::RankedHit,
    key: ContentHash,
    item: &'a T,
}
struct Limited<'a> {
    bytes: Vec<u8>,
    cap: usize,
    charge: &'a mut StateCharge,
}
impl Write for Limited<'_> {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let end = self.bytes.len().checked_add(value.len())
            .filter(|end| *end <= self.cap)
            .ok_or_else(|| io::Error::other("ranked result retention bytes"))?;
        if end > self.bytes.capacity() {
            let old = self.bytes.capacity();
            let capacity = end.max(old.saturating_add(old.clamp(64, 8192))).min(self.cap);
            self.charge.grow(capacity - old).map_err(io::Error::other)?;
            self.bytes.try_reserve_exact(capacity - self.bytes.len()).map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(value);
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
fn encode<T: Serialize>(value: &T, cap: usize, charge: &mut StateCharge) -> Result<Vec<u8>, WireError> {
    let mut writer = Limited { bytes: Vec::new(), cap, charge };
    serde_json::to_writer(&mut writer, value).map_err(|_| retention_refused())?;
    Ok(writer.bytes)
}
fn retention_refused() -> WireError {
    WireError::ResourceRefused("ranked result retention bytes".into())
}
fn unavailable() -> WireError {
    WireError::Continuation("ranked continuation unavailable in this session".into())
}
impl RankedResults {
    pub(crate) fn new(shared: &ResourceBudget) -> Self {
        let mut key = KeySink::new("native-ranked-session/v1");
        key.part(b"process", &std::process::id().to_le_bytes());
        key.part(b"time", &SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default().as_nanos().to_le_bytes());
        key.part(b"sequence", &SESSION.fetch_add(1, Ordering::Relaxed).to_le_bytes());
        Self {
            session: key.finish(), state: Mutex::new(State::default()),
            sequence: AtomicU64::new(0), shared: shared.clone(), lifetime: LIFETIME,
        }
    }
    pub(crate) fn request<'a>(&'a self, budget: &ResourceBudget) -> RankedRequest<'a> {
        RankedRequest {
            cache: self, pending: None, borrowed: None,
            scratch: StateCharge::new(budget, "native-ranked-request"),
        }
    }
    /// Called only after existing request admission has fenced and drained actual requests.
    pub(crate) fn clear(&self) {
        let mut state = self.state.lock().expect("ranked result retention");
        state.entries.clear();
        state.bytes = 0;
    }
    fn lookup(&self, result: ContentHash) -> Result<Arc<Entry>, WireError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        state.expire(self.lifetime);
        state.next += 1;
        let used = state.next;
        let retained = state.entries.get_mut(&result).ok_or_else(unavailable)?;
        retained.used = used;
        Ok(retained.entry.clone())
    }
}
impl State {
    fn remove(&mut self, id: ContentHash) {
        if let Some(retained) = self.entries.remove(&id) {
            self.bytes -= retained.entry.charge.reserved();
        }
    }
    fn expire(&mut self, lifetime: Duration) {
        // No intermediate collection; retain drops map ownership, never active borrowers.
        self.entries.retain(|_, retained| {
            if retained.entry.created.elapsed() >= lifetime {
                self.bytes -= retained.entry.charge.reserved();
                false
            } else { true }
        });
    }
    fn make_room(&mut self, bytes: usize) -> Result<(), WireError> {
        if bytes > MAX_BYTES { return Err(retention_refused()); }
        while self.bytes + bytes > MAX_BYTES || self.entries.len() >= MAX_ENTRIES {
            let id = self.entries.iter().min_by_key(|(_, e)| e.used)
                .map(|(id, _)| *id).ok_or_else(retention_refused)?;
            self.remove(id);
        }
        Ok(())
    }
}
/// Serialization supplies the same allocation allowance for DTO clones without a JSON buffer.
#[derive(Default)]
struct JsonShape {
    bytes: usize,
    tokens: usize,
    string: bool,
    escaped: bool,
    atom: bool,
}
impl JsonShape {
    fn allowance(&self, copies: usize) -> Result<usize, WireError> {
        self.tokens.checked_mul(2 * size_of::<serde_json::Value>() + 64)
            .and_then(|bytes| self.bytes.checked_mul(2).and_then(|strings| bytes.checked_add(strings)))
            .and_then(|bytes| bytes.checked_mul(copies)).ok_or_else(retention_refused)
    }
}
impl Write for JsonShape {
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        self.bytes = self.bytes.checked_add(raw.len()).ok_or_else(|| io::Error::other("ranked scratch size"))?;
        for &byte in raw {
            if self.string {
                if self.escaped { self.escaped = false; }
                else if byte == b'\\' { self.escaped = true; }
                else if byte == b'"' { self.string = false; }
                continue;
            }
            match byte {
                b'"' => { self.tokens += 1; self.string = true; self.atom = false; },
                b'[' | b'{' => { self.tokens += 1; self.atom = false; },
                b']' | b'}' | b',' | b':' | b' ' | b'\n' | b'\r' | b'\t' => { self.atom = false; },
                _ if !self.atom => { self.tokens += 1; self.atom = true; },
                _ => {},
            }
        }
        Ok(raw.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
impl RankedRequest<'_> {
    /// Admit JSON tree/DTO scratch before parsing. The allocation-free counter includes
    /// array growth and map-node/key overhead as well as string/serialized copies.
    /// This is a conservative allocation allowance, not a process-RSS measurement.
    fn admit_json(&mut self, raw: &[u8], copies: usize) -> Result<(), WireError> {
        let mut shape = JsonShape::default();
        shape.write_all(raw).map_err(|_| retention_refused())?;
        self.scratch.grow(shape.allowance(copies)?).map_err(|_| retention_refused())
    }
    /// Admit packet/ranking copies while they still borrow their source, before cloning.
    pub(crate) fn admit_copy<T: Serialize>(&mut self, value: &T) -> Result<(), WireError> {
        let mut shape = JsonShape::default();
        serde_json::to_writer(&mut shape, value)?;
        self.scratch.grow(shape.allowance(1)?).map_err(|_| retention_refused())
    }
    /// First-page discovery supplies its order once. Nothing is retained in the map yet.
    pub(crate) fn page<T: Serialize>(
        &mut self,
        values: Vec<(ranking::RankedHit, ContentHash, T)>,
        request: &Request,
        snapshot: &SnapshotHandle,
        channels: &ChannelState,
    ) -> Result<(SectionPage<T>, Vec<ranking::RankedHit>), WireError> {
        if request.page().cursor.0.is_some() || self.pending.is_some() { return Err(unavailable()); }
        let expected = crate::pagination::binding(request, snapshot, channels,
            request.tool().name(), "results", None)?;
        let count = values.len();
        let size = request.page().size as usize;
        let continuation = if count > 1 {
            let mut charge = StateCharge::new(&self.cache.shared, "native-ranked-retention");
            // Include immutable Arc and map/node ownership as well as row-vector storage.
            let base = (size_of::<Entry>() + 2 * size_of::<usize>() + size_of::<Retained>()
                + size_of::<ContentHash>() + 32).checked_add(count.checked_mul(size_of::<Vec<u8>>())
                .ok_or_else(retention_refused)?).ok_or_else(retention_refused)?;
            if base > MAX_BYTES { return Err(retention_refused()); }
            charge.grow(base).map_err(|_| retention_refused())?;
            // Metadata copies are charged before cloning; cursor encoding is request scratch.
            let metadata = channels_name_bytes(channels)
                + expected.group.as_str().len() + expected.section.as_str().len()
                + expected.snapshot.database.namespace.as_str().len()
                + expected.snapshot.database.database.as_str().len();
            charge.grow(metadata).map_err(|_| retention_refused())?;
            let mut rows = Vec::with_capacity(count);
            for (ranking, key, item) in &values {
                rows.push(encode(&RowRef { ranking, key: *key, item },
                    MAX_BYTES.saturating_sub(charge.reserved()), &mut charge)?);
            }
            let mut key = KeySink::new("native-ranked-result/v1");
            for row in &rows { key.part(b"row", row); }
            let digest = key.finish();
            let sequence = self.cache.sequence.fetch_add(1, Ordering::Relaxed) + 1;
            let mut key = KeySink::new("native-ranked-entry/v1");
            key.part(b"session", &self.cache.session.0);
            key.part(b"sequence", &sequence.to_le_bytes());
            key.part(b"digest", &digest.0);
            let result = key.finish();
            self.scratch.grow(size_of::<Cursor>().saturating_mul(8).saturating_add(metadata.saturating_mul(8)))
                .map_err(|_| retention_refused())?;
            let token = Cursor { binding: expected.clone(), after: CursorPosition::Ranked {
                session: self.cache.session, result, digest, offset: size.min(count) as u64,
            }}.encode()?;
            self.pending = Some(Pending { result, entry: Entry {
                binding: expected, digest, channels: channels.clone(), rows, template: Vec::new(),
                created: Instant::now(), charge,
            }});
            Optional(Some(token))
        } else { Optional::default() };
        let returned = size.min(count);
        self.scratch.grow(returned.saturating_mul(size_of::<T>() + size_of::<ranking::RankedHit>()))
            .map_err(|_| retention_refused())?;
        let mut items = Vec::with_capacity(returned);
        let mut ranking = Vec::with_capacity(returned);
        for (hit, _, item) in values.into_iter().take(returned) {
            items.push(item);
            ranking.push(hit);
        }
        let omitted = count.saturating_sub(size) as u64;
        Ok((SectionPage { availability: Availability::Available {}, items, continuation,
            omitted, truncated: omitted > 0 }, ranking))
    }
    /// Final packing determines whether a transient first-page cursor is needed.
    pub(crate) fn complete(&mut self, response: &mut Response) {
        let (omitted, continuation) = match response {
            Response::SearchOperations(r) => (r.results.omitted, &mut r.results.continuation),
            Response::SearchEvidence(r) => (r.results.omitted, &mut r.results.continuation),
            Response::SearchCapabilities(r) => (r.results.omitted, &mut r.results.continuation),
            _ => return,
        };
        if omitted == 0 {
            self.pending = None;
            *continuation = Optional::default();
        }
    }
    /// Prepare the complete metadata template privately. Publication is a separate final step,
    /// so a packing/template/deadline failure leaves no retained partial entry.
    pub(crate) fn attach(&mut self, encoded_response: &EncodedJson) -> Result<(), WireError> {
        if self.pending.is_none() { return Ok(()); }
        self.admit_json(encoded_response.as_str().as_bytes(), 1)?;
        let mut template: serde_json::Value = serde_json::from_str(encoded_response.as_str())?;
        let token = template.pointer("/results/continuation").and_then(|v| v.as_str())
            .ok_or_else(unavailable)?;
        let cursor: Cursor = serde_json::from_slice(&hex::decode(token).map_err(|_| unavailable())?)?;
        let pending = self.pending.as_mut().expect("pending checked");
        match cursor.after {
            CursorPosition::Ranked { session, result, digest, .. }
                if session == self.cache.session && result == pending.result
                    && digest == pending.entry.digest && cursor.binding == pending.entry.binding => {},
            _ => return Err(unavailable()),
        }
        template["results"]["items"] = serde_json::json!([]);
        template["ranking"] = serde_json::json!([]);
        pending.entry.template = encode(&template,
            MAX_BYTES.saturating_sub(pending.entry.charge.reserved()), &mut pending.entry.charge)?;
        Ok(())
    }
    pub(crate) fn publish(&mut self) -> Result<(), WireError> {
        let Some(pending) = self.pending.take() else { return Ok(()); };
        if pending.entry.template.is_empty() { return Err(unavailable()); }
        let bytes = pending.entry.charge.reserved();
        // Allocate immutable ownership before taking the map lock.
        let entry = Arc::new(pending.entry);
        let mut state = self.cache.state.lock().map_err(|_| unavailable())?;
        state.expire(self.cache.lifetime);
        state.make_room(bytes)?;
        state.next += 1;
        let used = state.next;
        state.entries.insert(pending.result, Retained { entry, used });
        state.bytes += bytes;
        Ok(())
    }
    /// The map lock covers lookup/LRU/expiry only; the Arc and its service reservation stay
    /// alive through decoding, final packing and encoding, even after map eviction.
    pub(crate) fn resume(
        &mut self,
        request: &Request,
        snapshot: &SnapshotHandle,
        vector: Option<&crate::service::QueryVector>,
        limits: &ResourceLimits,
    ) -> Result<Option<Response>, WireError> {
        let Some(token) = &request.page().cursor.0 else { return Ok(None); };
        self.admit_json(token.as_str().as_bytes(), 1)?;
        let cursor: Cursor = serde_json::from_slice(&hex::decode(token.as_str()).map_err(|_| unavailable())?)?;
        let CursorPosition::Ranked { session, result, digest, offset } = cursor.after else {
            if matches!(request, Request::SearchOperations(_) | Request::SearchEvidence(_)
                | Request::SearchCapabilities(_)) { return Err(unavailable()); }
            return Ok(None);
        };
        if session != self.cache.session { return Err(unavailable()); }
        let entry = self.cache.lookup(result)?;
        if let Some(v) = vector {
            let query = match request {
                Request::SearchOperations(r) => r.query.as_str(),
                Request::SearchEvidence(r) => r.query.as_str(),
                Request::SearchCapabilities(r) => r.query.as_str(),
                _ => return Err(unavailable()),
            };
            if v.recipe.validate().is_err() || v.input
                != lctx_model::domain::embedding::value::input_hash(&v.recipe.text(query)) {
                return Err(unavailable());
            }
            let supplied = VectorChannel::Available {
                spec: v.spec, query_vector: lctx_model::domain::embedding::value::value_digest(&v.vector),
                query_recipe: v.recipe.identity(), projection: v.projection,
            };
            if supplied != entry.channels.vector { return Err(unavailable()); }
        }
        let expected = crate::pagination::binding(request, snapshot, &entry.channels,
            request.tool().name(), "results", None)?;
        Cursor::decode(token, &expected)?;
        if entry.binding != expected || entry.digest != digest { return Err(unavailable()); }
        let start = usize::try_from(offset).map_err(|_| unavailable())?;
        if start >= entry.rows.len() { return Err(unavailable()); }
        let end = start.saturating_add(request.page().size as usize).min(entry.rows.len());
        self.admit_json(&entry.template, 1)?;
        for row in &entry.rows[start..end] { self.admit_json(row, 1)?; }
        let mut template: serde_json::Value = serde_json::from_slice(&entry.template)?;
        let mut items = Vec::with_capacity(end - start);
        let mut ranking = Vec::with_capacity(end - start);
        for row in &entry.rows[start..end] {
            let row: Row<serde_json::Value> = serde_json::from_slice(row)?;
            items.push(row.item);
            ranking.push(row.ranking);
        }
        template["results"]["items"] = serde_json::Value::Array(items);
        template["ranking"] = serde_json::Value::Array(ranking);
        template["results"]["omitted"] = serde_json::json!(entry.rows.len() - end);
        template["results"]["truncated"] = serde_json::json!(end < entry.rows.len());
        template["results"]["continuation"] = serde_json::json!(Cursor {
            binding: expected, after: CursorPosition::Ranked { session, result, digest, offset: end as u64 }
        }.encode()?.as_str());
        template["extent"] = serde_json::json!({"extent":"ranked","returned":end-start});
        // Serialization is incremental and charged to request scratch, never the retained pool.
        let raw = encode(&template, limits.response_bytes(request.page().expanded) as usize,
            &mut self.scratch)?;
        let raw = std::str::from_utf8(&raw).map_err(|_| unavailable())?;
        // decode_response simultaneously holds the DTO and its closed-shape JSON controls.
        self.admit_json(raw.as_bytes(), 3)?;
        let response = decode_response(request.tool().name(), raw, request.page().expanded, limits)?;
        self.borrowed = Some(entry);
        Ok(Some(response))
    }
}
fn channels_name_bytes(channels: &ChannelState) -> usize {
    match &channels.vector { VectorChannel::Degraded { reason } => reason.as_str().len(), _ => 0 }
}
#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{Id, attribution::AnalysisContext, retrieval};
    impl Default for RankedResults {
        fn default() -> Self {
            Self::new(&ResourceBudget::fixed(MAX_BYTES * 4).unwrap())
        }
    }
    fn budget() -> ResourceBudget { ResourceBudget::fixed(64 * 1024 * 1024).unwrap() }
    fn resume(cache: &RankedResults, request: &Request, handle: &SnapshotHandle,
        vector: Option<&crate::service::QueryVector>) -> Result<Option<Response>, WireError> {
        cache.request(&budget()).resume(request, handle, vector, &ResourceLimits::default())
    }
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn handle() -> SnapshotHandle {
        SnapshotHandle {
            publication: lctx_model::domain::ContentHash::of(b"fixture-publication"),
            view: lctx_model::domain::ContentHash::of(b"fixture-view"),
            service_generation: lctx_model::domain::ContentHash::of(b"fixture-service_generation"),
            definition_epoch: lctx_model::domain::ContentHash::of(b"fixture-definition_epoch"),
            semantic: ContentHash::of(b"semantic"),
            realization: ContentHash::of(b"physical"),
            database: DatabaseIdentity {
                namespace: Name::new("control").unwrap(),
                database: Name::new("snapshot").unwrap(),
            },
        }
    }
    fn request() -> Request {
        Request::SearchEvidence(SearchEvidenceRequest {
            library: Optional::default(),
            query: QueryText::new("original").unwrap(),
            families: vec![],
            page: PageRequest {
                size: 1,
                ..PageRequest::default()
            },
        })
    }
    fn channels() -> ChannelState {
        ChannelState {
            lexical: true,
            vector: VectorChannel::Degraded {
                reason: Name::new("unavailable").unwrap(),
            },
        }
    }
    fn response(cache: &RankedResults, request: &Request) -> Response {
        response_with_channels(cache, request, &channels())
    }
    fn response_with_channels(
        cache: &RankedResults,
        request: &Request,
        channels: &ChannelState,
    ) -> Response {
        let budget = budget();
        let mut pending = cache.request(&budget);
        let mut response = draft(&mut pending, request, channels, 0);
        pending.complete(&mut response);
        let encoded = response.encode_json(&budget, MAX_BYTES).unwrap();
        pending.attach(&encoded).unwrap();
        pending.publish().unwrap();
        response
    }
    fn values() -> Vec<(ranking::RankedHit, ContentHash, EvidenceHit)> {
        (1..=3)
            .map(|n| {
                let unit = id::<retrieval::Unit>(n);
                let ranking = ranking::RankedHit {
                    target: ranking::Target::Unit { unit },
                    context: id::<AnalysisContext>(n),
                    score: 1.0 / f64::from(n),
                    promoted: false,
                    witnesses: vec![],
                };
                (
                    ranking,
                    ContentHash::of(&[n]),
                    EvidenceHit {
                        unit,
                        release: ReleaseIdentity {
                            input: id(n),
                            release: id(n),
                            distribution: Name::new("control").unwrap(),
                            version: Name::new("1").unwrap(),
                        },
                        family: retrieval::Family::Source,
                        title: Name::new(format!("original{n}")).unwrap(),
                        originals: vec![],
                        associated_members: vec![],
                        delivered_windows: vec![],
                        interpretation: InterpretationClosure {
                            contexts: vec![],
                            defaults: vec![],
                            qualifications: vec![],
                            availability: Availability::NotRequested {},
                        },
                    },
                )
            })
            .collect()
    }
    fn draft(pending: &mut RankedRequest<'_>, request: &Request, channels: &ChannelState,
        payload: usize) -> Response {
        let mut values = values();
        for (_, _, item) in &mut values {
            if payload > 0 {
                item.delivered_windows.push(DeliveredWindow {
                    window: id(1), part: id(1), purpose: retrieval::PartPurpose::Primary,
                    member: Nullable(None), analysis: id(1), binding: Nullable(None),
                    subject: Nullable(None), basis: Nullable(None), qualification: Nullable(None),
                    text: Text::new("x".repeat(payload)).unwrap(), source_maps: vec![],
                });
            }
        }
        let (results, ranking) = pending.page(values, request, &handle(), channels).unwrap();
        Response::SearchEvidence(SearchEvidenceResponse {
            delivery: Optional::default(), snapshot: handle(), domains: vec![],
            extent: SelectionExtent::Ranked { returned: results.items.len() as u64 },
            results, channels: channels.clone(), ranking,
        })
    }
    fn next(request: &mut Request, response: &Response) {
        let Request::SearchEvidence(request) = request else {
            unreachable!()
        };
        let Response::SearchEvidence(response) = response else {
            unreachable!()
        };
        request.page.cursor = response.results.continuation.clone();
    }
    #[test]
    fn continuation_replays_frozen_rows_and_channels_and_checks_context() {
        let cache = RankedResults::default();
        let mut request = request();
        let first = response(&cache, &request);
        next(&mut request, &first);
        let second = resume(&cache, &request, &handle(), None).unwrap().unwrap();
        let Response::SearchEvidence(page) = &second else {
            unreachable!()
        };
        assert_eq!(page.results.items[0].title.as_str(), "original2");
        assert_eq!(page.channels, channels());
        assert_eq!(page.ranking[0].context, id(2));
        next(&mut request, &second);
        let mut third = resume(&cache, &request, &handle(), None).unwrap().unwrap();
        cache.request(&budget()).complete(&mut third);
        let Response::SearchEvidence(page) = &third else {
            unreachable!()
        };
        assert_eq!(page.results.items[0].title.as_str(), "original3");
        assert!(page.results.continuation.0.is_none());
        let mut foreign = handle();
        foreign.semantic = ContentHash::of(b"changed");
        assert!(resume(&cache, &request, &foreign, None).is_err());
        let Request::SearchEvidence(changed) = &mut request else {
            unreachable!()
        };
        changed.query = QueryText::new("other").unwrap();
        assert!(resume(&cache, &request, &handle(), None).is_err());
    }
    #[test]
    fn expiry_foreign_session_and_eviction_refuse_without_rediscovery() {
        let cache = RankedResults::default();
        let mut request = request();
        let first = response(&cache, &request);
        next(&mut request, &first);
        assert!(
            resume(&RankedResults::default(), &request, &handle(), None)
                .is_err()
        );
        for _ in 0..MAX_ENTRIES {
            response(&cache, &self::request());
        }
        assert!(resume(&cache, &request, &handle(), None).is_err());
        let expired = RankedResults {
            lifetime: Duration::ZERO,
            ..RankedResults::default()
        };
        let fresh = response(&expired, &self::request());
        let mut continuation = self::request();
        next(&mut continuation, &fresh);
        assert!(resume(&expired, &continuation, &handle(), None).is_err());
    }
    #[test]
    fn retained_writer_refuses_before_oversized_row_allocation() {
        let budget = budget();
        let mut charge = StateCharge::new(&budget, "test-retention");
        assert!(encode(&vec!["payload"; 100], 16, &mut charge).is_err());
        let mut state = State::default();
        assert!(state.make_room(MAX_BYTES + 1).is_err());
    }
    #[test]
    fn complete_first_page_releases_unused_retention_and_has_no_continuation() {
        let cache = RankedResults::default();
        let mut request = request();
        let Request::SearchEvidence(r) = &mut request else {
            unreachable!()
        };
        r.page.size = 3;
        let response = response(&cache, &request);
        let Response::SearchEvidence(r) = response else {
            unreachable!()
        };
        assert!(r.results.continuation.0.is_none());
        assert_eq!(r.results.items.len(), 3);
        assert!(cache.state.lock().unwrap().entries.is_empty());
    }
    #[test]
    fn continuation_rejects_changed_query_value_or_input_but_needs_no_new_inference() {
        use lctx_model::domain::embedding;
        let recipe = embedding::QueryRecipe {
            template: "{task_description}: {query}".into(),
            task: "retrieve".into(),
            max_tokens: 8192,
        };
        let mut vector = vec![0.0; 4096];
        vector[0] = 1.0;
        let mut query = crate::service::QueryVector {
            spec: ContentHash::of(b"encoder"),
            input: embedding::value::input_hash(&recipe.text("original")),
            vector,
            recipe,
            projection: id(1),
        };
        let channels = ChannelState {
            lexical: true,
            vector: VectorChannel::Available {
                spec: query.spec,
                query_vector: embedding::value::value_digest(&query.vector),
                query_recipe: query.recipe.identity(),
                projection: query.projection,
            },
        };
        let cache = RankedResults::default();
        let mut request = request();
        let first = response_with_channels(&cache, &request, &channels);
        next(&mut request, &first);
        assert!(resume(&cache, &request, &handle(), None).unwrap().is_some());
        assert!(
            resume(&cache, &request, &handle(), Some(&query))
                .unwrap()
                .is_some()
        );
        query.input = ContentHash::of(b"foreign input");
        assert!(resume(&cache, &request, &handle(), Some(&query)).is_err());
        query.input = embedding::value::input_hash(&query.recipe.text("original"));
        query.vector[0] = 0.0;
        query.vector[1] = 1.0;
        assert!(resume(&cache, &request, &handle(), Some(&query)).is_err());
    }
    #[test]
    fn selected_large_decode_and_final_envelope_admission_are_both_preserved() {
        let cache = RankedResults::default();
        let budget = budget();
        let mut request = request();
        let limits = ResourceLimits { default_response_bytes: 128 * 1024, ..Default::default() };
        let mut initial = cache.request(&budget);
        let mut first = draft(&mut initial, &request, &channels(), 40 * 1024);
        crate::delivery::finalize(&request, &mut first, &limits).unwrap();
        initial.complete(&mut first);
        let encoded = first.encode_json(&budget, limits.default_response_bytes as usize).unwrap();
        initial.attach(&encoded).unwrap();
        initial.publish().unwrap();
        drop(initial);
        next(&mut request, &first);
        let mut replay = cache.request(&budget);
        let mut second = replay.resume(&request, &handle(), None, &limits).unwrap().unwrap();
        assert!(second.to_json().unwrap().len() > 40 * 1024);
        crate::delivery::finalize(&request, &mut second, &limits).unwrap();
        let envelope = second.encode_mcp_result(&budget, limits.default_response_bytes as usize).unwrap();
        lctx_model::domain::serving::resources::admit_final_mcp_bytes(envelope.as_str().as_bytes(), false, &limits).unwrap();
        let smaller = ResourceLimits::default();
        assert!(matches!(cache.request(&budget).resume(&request, &handle(), None, &smaller),
            Err(WireError::ResourceRefused(_))));
        // Structured bytes fitting their cap never waive MCP framing/metadata admission.
        let structured = second.encode_json(&budget, limits.default_response_bytes as usize).unwrap();
        let exact = ResourceLimits {
            default_response_bytes: structured.as_str().len() as u64, ..limits.clone()
        };
        assert!(second.encode_mcp_result(&budget, exact.default_response_bytes as usize).is_err());
        assert!(lctx_model::domain::serving::resources::admit_final_mcp_bytes(envelope.as_str().as_bytes(), false, &exact).is_err());
        assert!(crate::delivery::finalize(&request, &mut second, &exact).is_err());
    }
    #[test]
    fn pending_is_private_and_cancelled_or_rejected_packing_releases_every_charge() {
        let shared = ResourceBudget::fixed(MAX_BYTES * 4).unwrap();
        let cache = RankedResults::new(&shared);
        let request_budget = ResourceBudget::scoped(&shared, MAX_BYTES * 2).unwrap();
        let request = request();
        {
            let mut pending = cache.request(&request_budget);
            let mut response = draft(&mut pending, &request, &channels(), 40 * 1024);
            assert!(shared.reserved() > 0);
            assert!(cache.state.lock().unwrap().entries.is_empty());
            assert!(crate::delivery::finalize(&request, &mut response, &ResourceLimits::default()).is_err());
            // This is the same owned-state release as dropping a cancelled service future.
        }
        assert_eq!(request_budget.reserved(), 0);
        assert_eq!(shared.reserved(), 0);
        {
            let mut pending = cache.request(&request_budget);
            let response = draft(&mut pending, &request, &channels(), 0);
            assert!(response.encode_json(&request_budget, 1).is_err());
        }
        assert!(cache.state.lock().unwrap().entries.is_empty());
        assert_eq!(shared.reserved(), 0);
        {
            let mut pending = cache.request(&request_budget);
            let response = draft(&mut pending, &request, &channels(), 0);
            let encoded = response.encode_json(&request_budget, MAX_BYTES).unwrap();
            pending.attach(&encoded).unwrap();
            assert!(cache.state.lock().unwrap().entries.is_empty());
            // A deadline/cancellation after template construction still publishes nothing.
        }
        assert_eq!(shared.reserved(), 0);
    }
    #[test]
    fn retained_rows_pay_service_pool_and_scratch_refusal_does_not_destroy_replay() {
        let cache = RankedResults::default();
        let mut request = request();
        let first = response(&cache, &request);
        assert!(cache.shared.reserved() > 0);
        next(&mut request, &first);
        let before = cache.shared.reserved();
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(cache.request(&tiny).resume(&request, &handle(), None, &ResourceLimits::default()).is_err());
        assert_eq!(tiny.reserved(), 0);
        assert_eq!(cache.shared.reserved(), before);
        assert!(resume(&cache, &request, &handle(), None).unwrap().is_some());
        cache.clear();
        assert_eq!(cache.shared.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        let refused = RankedResults::new(&tiny);
        assert!(refused.request(&budget()).page(values(), &self::request(), &handle(), &channels()).is_err());
        assert_eq!(tiny.reserved(), 0);
        assert!(refused.state.lock().unwrap().entries.is_empty());
    }
    #[test]
    fn replay_borrows_outside_map_lock_and_keeps_charge_after_eviction() {
        let cache = RankedResults::default();
        let mut request = request();
        let first = response(&cache, &request);
        next(&mut request, &first);
        let budget = budget();
        let mut active = cache.request(&budget);
        let mut page = active.resume(&request, &handle(), None, &ResourceLimits::default()).unwrap().unwrap();
        assert!(cache.state.try_lock().is_ok());
        let borrowed_bytes = active.borrowed.as_ref().unwrap().charge.reserved();
        for _ in 0..MAX_ENTRIES { response(&cache, &self::request()); }
        assert!(resume(&cache, &request, &handle(), None).is_err());
        cache.clear();
        assert_eq!(cache.shared.reserved(), borrowed_bytes);
        crate::delivery::finalize(&request, &mut page, &ResourceLimits::default()).unwrap();
        let encoded = page.encode_json(&budget, 32 * 1024).unwrap();
        assert!(encoded.as_str().contains("original2"));
        drop(encoded);
        drop(active);
        assert_eq!(cache.shared.reserved(), 0);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn close_drains_admitted_ranked_borrower_before_clearing_retention() {
        use crate::preparation::PreparedCache;
        use tokio::sync::Semaphore;
        let cache = Arc::new(RankedResults::default());
        let prepared = PreparedCache::new(handle(), &cache.shared, &ResourceLimits::default(),
            Arc::new(Semaphore::new(2)), Arc::new(Semaphore::new(2))).unwrap();
        let mut request = request();
        let first = response(&cache, &request);
        next(&mut request, &first);
        let budget = budget();
        let lease = prepared.admit_request().unwrap();
        let mut active = cache.request(&budget);
        active.resume(&request, &handle(), None, &ResourceLimits::default()).unwrap();
        let closing = {
            let prepared = prepared.clone();
            let cache = cache.clone();
            tokio::spawn(async move { prepared.close().await; cache.clear(); })
        };
        while prepared.admit_request().is_ok() { tokio::task::yield_now().await; }
        assert!(!closing.is_finished());
        assert!(!cache.state.lock().unwrap().entries.is_empty());
        drop(active);
        drop(lease);
        closing.await.unwrap();
        assert!(cache.state.lock().unwrap().entries.is_empty());
        assert_eq!(cache.shared.reserved(), 0);
    }

    #[test]
    fn incremental_retention_refusal_and_copy_admission_release_without_publication() {
        let shared = ResourceBudget::fixed(4096).unwrap();
        let cache = RankedResults::new(&shared);
        let request_budget = budget();
        let mut request = cache.request(&request_budget);
        let large = values().into_iter().map(|(ranking, key, item)| {
            (ranking, key, (item, "x".repeat(8000)))
        }).collect();
        assert!(matches!(request.page(large, &self::request(), &handle(), &channels()),
            Err(WireError::ResourceRefused(_))));
        assert_eq!(shared.reserved(), 0);
        assert!(cache.state.lock().unwrap().entries.is_empty());
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(cache.request(&tiny).admit_copy(&values()[0]).is_err());
        assert_eq!(tiny.reserved(), 0);
    }
    #[test]
    fn scratch_shape_counts_escaped_strings_across_serializer_write_boundaries() {
        let value = serde_json::json!({"quoted\\key": ["\\\"\nα", true, false, null, 17]});
        let raw = serde_json::to_vec(&value).unwrap();
        let mut whole = JsonShape::default();
        whole.write_all(&raw).unwrap();
        let mut chunks = JsonShape::default();
        serde_json::to_writer(&mut chunks, &value).unwrap();
        assert_eq!(whole.allowance(3).unwrap(), chunks.allowance(3).unwrap());
        assert!(whole.allowance(3).unwrap() > raw.len() * 3);
    }

}
