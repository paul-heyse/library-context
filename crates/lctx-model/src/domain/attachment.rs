//! One indexed source/span/kind/role attachment rule. Ambiguity never picks a convenient ID.
use std::mem::size_of;
use super::resources::{ResourceBudget,Reservation};
use super::{Id, ModelError, Record};
use super::source::{Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind};

#[derive(Debug, Clone)]
pub struct AttachmentQuery {
    pub source: Id<SourceArtifact>, pub start: i64, pub end: i64,
    pub syntax_kind: SyntaxKind, pub role: OccurrenceRole,
    pub structural_path: Option<Vec<i32>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attachment {
    Exact(Id<Occurrence>), Innermost(Id<Occurrence>),
    Ambiguous(Vec<Id<Occurrence>>), Unmatched, BudgetExceeded,
}
#[derive(Debug, Clone, Copy)]
pub struct AttachmentBudget { pub visited_nodes: usize, pub alternatives: usize }
impl Default for AttachmentBudget {
    fn default() -> Self { Self { visited_nodes: 1_000_000, alternatives: 256 } }
}
type GroupKey = (Id<SourceArtifact>,i16,i16);
#[derive(Debug)]
struct Entry { key: GroupKey, id: Id<Occurrence>, start: i64, end: i64, path: Box<[i32]> }
struct Group<'a> { entries: &'a [Entry], max_end: &'a [i64] }
fn build_index(entries: &[Entry], max_end: &mut [i64], low: usize, high: usize) -> i64 {
    if low == high { return -1; }
    let middle = low + (high-low)/2;
    let maximum = build_index(entries,max_end,low,middle).max(build_index(entries,max_end,middle+1,high)).max(entries[middle].end);
    max_end[middle] = maximum; maximum
}
impl Group<'_> {
    fn search(&self, low: usize, high: usize, query: &AttachmentQuery, found: &mut Search) {
        if low == high || found.exhausted { return; }
        let middle = low + (high - low) / 2;
        if found.visited == found.budget.visited_nodes { found.exhausted = true; return; }
        found.visited += 1;
        if self.max_end[middle] < query.end || self.entries[low].start > query.start { return; }
        self.search(low, middle, query, found);
        if found.exhausted { return; }
        let entry = &self.entries[middle];
        if entry.start <= query.start && entry.end >= query.end
            && query.structural_path.as_ref().is_none_or(|path| path.as_slice() == entry.path.as_ref()) {
            let width = entry.end - entry.start;
            if found.width.is_none_or(|best| width < best) {
                found.width = Some(width); found.ids.clear();
            }
            if found.width == Some(width) {
                if found.ids.len() == found.budget.alternatives { found.exhausted = true; return; }
                found.ids.push(entry.id);
            }
        }
        if entry.start <= query.start { self.search(middle + 1, high, query, found); }
    }
}
struct Search { width: Option<i64>, ids: Vec<Id<Occurrence>>, visited: usize, budget: AttachmentBudget, exhausted: bool }
/// Owns all index buffers and their reservation. Caller-owned occurrence batches are separately
/// charged. Sorting is in place, and each structural path has exact requested heap capacity.
#[derive(Debug)]
pub struct OccurrenceIndex {
    entries: Vec<Entry>, max_end: Vec<i64>, resources: ResourceBudget, _reservation: Box<dyn Reservation>,
}
/// An ambiguity may retain an alternative buffer after the index or query is dropped. Its
/// reservation therefore follows this result, not the index. Copying it requires a caller charge.
#[derive(Debug)]
pub struct AttachmentResult { value: Attachment, _reservation: Box<dyn Reservation> }
impl AttachmentResult { pub fn value(&self) -> &Attachment { &self.value } }
fn allocation(bytes: Option<usize>) -> Result<usize,ModelError> {
    bytes.ok_or_else(|| ModelError::Invalid("attachment allocation size overflow".into()))
}
impl OccurrenceIndex {
    pub fn new(occurrences: &[Occurrence], resources: ResourceBudget) -> Result<Self, ModelError> {
        let mut bytes = allocation(occurrences.len().checked_mul(size_of::<Entry>()+size_of::<i64>()))?;
        for occurrence in occurrences {
            occurrence.validate()?;
            bytes = allocation(occurrence.structural_path.len().checked_mul(size_of::<i32>()).and_then(|path| bytes.checked_add(path)))?;
        }
        let reservation = resources.reserve("occurrence index",bytes)?;
        let scratch = resources.reserve("occurrence identity check",allocation(occurrences.len().checked_mul(size_of::<Id<Occurrence>>()))?)?;
        let mut ids = Vec::with_capacity(occurrences.len());
        ids.extend(occurrences.iter().map(Record::id)); ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) { return Err(ModelError::Conflict(Occurrence::NAME)); }
        drop(ids); drop(scratch);
        let mut entries = Vec::with_capacity(occurrences.len());
        entries.extend(occurrences.iter().map(|o| Entry { key: (o.source,o.syntax_kind as i16,o.role as i16),id: o.id(),start: o.start,end: o.end,path: o.structural_path.clone().into_boxed_slice() }));
        entries.sort_unstable_by_key(|e| (e.key,e.start,e.end,e.id));
        let mut max_end = vec![0;entries.len()];
        let mut start = 0;
        while start < entries.len() {
            let end = start+entries[start..].partition_point(|e| e.key == entries[start].key);
            build_index(&entries[start..end],&mut max_end[start..end],0,end-start); start = end;
        }
        Ok(Self { entries,max_end,resources,_reservation: reservation })
    }
    pub fn attach(&self, query: &AttachmentQuery, budget: AttachmentBudget) -> Result<AttachmentResult, ModelError> {
        if query.start < 0 || query.end < query.start || query.structural_path.as_ref().is_some_and(|path| path.iter().any(|i| *i < 0)) {
            return Err(ModelError::Invalid("invalid attachment coordinates".into()));
        }
        let key = (query.source,query.syntax_kind as i16,query.role as i16);
        let start = self.entries.partition_point(|e| e.key < key);
        let end = self.entries.partition_point(|e| e.key <= key);
        let group = Group { entries: &self.entries[start..end],max_end: &self.max_end[start..end] };
        let capacity = (end-start).min(budget.alternatives);
        let mut reservation = self.resources.reserve("attachment alternatives",allocation(capacity.checked_mul(size_of::<Id<Occurrence>>()))?)?;
        let mut found = Search { width: None, ids: Vec::with_capacity(capacity), visited: 0, budget, exhausted: false };
        group.search(0, group.entries.len(), query, &mut found);
        found.ids.sort_unstable();
        let value = if found.exhausted { Attachment::BudgetExceeded } else { match found.ids.as_slice() {
            [] => Attachment::Unmatched,
            [id] if found.width == Some(query.end - query.start) => Attachment::Exact(*id),
            [id] => Attachment::Innermost(*id),
            _ => return Ok(AttachmentResult { value: Attachment::Ambiguous(found.ids),_reservation: reservation }),
        } };
        drop(found); reservation.try_resize(0)?;
        Ok(AttachmentResult { value,_reservation: reservation })
    }
}
