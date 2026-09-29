//! One indexed source/span/kind/role attachment rule. Ambiguity never picks a convenient ID.
use std::collections::{BTreeMap, BTreeSet};
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
#[derive(Debug)]
struct Entry { id: Id<Occurrence>, start: i64, end: i64, path: Vec<i32> }
#[derive(Debug)]
struct Group { entries: Vec<Entry>, max_end: Vec<i64> }
impl Group {
    fn build(mut entries: Vec<Entry>) -> Self {
        entries.sort_by_key(|e| (e.start, e.end, e.id));
        let mut group = Self { max_end: vec![0; entries.len()], entries };
        group.index(0, group.entries.len());
        group
    }
    fn index(&mut self, low: usize, high: usize) -> i64 {
        if low == high { return -1; }
        let middle = low + (high - low) / 2;
        let maximum = self.index(low, middle).max(self.index(middle + 1, high)).max(self.entries[middle].end);
        self.max_end[middle] = maximum;
        maximum
    }
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
            && query.structural_path.as_ref().is_none_or(|path| path == &entry.path) {
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
#[derive(Debug)]
pub struct OccurrenceIndex { groups: BTreeMap<(Id<SourceArtifact>, i16, i16), Group> }
impl OccurrenceIndex {
    pub fn new(occurrences: &[Occurrence]) -> Result<Self, ModelError> {
        let mut groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for occurrence in occurrences {
            occurrence.validate()?;
            if !seen.insert(occurrence.id()) { return Err(ModelError::Conflict(Occurrence::NAME)); }
            groups.entry((occurrence.source, occurrence.syntax_kind as i16, occurrence.role as i16)).or_default()
                .push(Entry { id: occurrence.id(), start: occurrence.start, end: occurrence.end, path: occurrence.structural_path.clone() });
        }
        Ok(Self { groups: groups.into_iter().map(|(key, entries)| (key, Group::build(entries))).collect() })
    }
    pub fn attach(&self, query: &AttachmentQuery, budget: AttachmentBudget) -> Result<Attachment, ModelError> {
        if query.start < 0 || query.end < query.start || query.structural_path.as_ref().is_some_and(|path| path.iter().any(|i| *i < 0)) {
            return Err(ModelError::Invalid("invalid attachment coordinates".into()));
        }
        let Some(group) = self.groups.get(&(query.source, query.syntax_kind as i16, query.role as i16)) else { return Ok(Attachment::Unmatched); };
        let mut found = Search { width: None, ids: Vec::new(), visited: 0, budget, exhausted: false };
        group.search(0, group.entries.len(), query, &mut found);
        if found.exhausted { return Ok(Attachment::BudgetExceeded); }
        found.ids.sort();
        Ok(match found.ids.as_slice() {
            [] => Attachment::Unmatched,
            [id] if found.width == Some(query.end - query.start) => Attachment::Exact(*id),
            [id] => Attachment::Innermost(*id),
            _ => Attachment::Ambiguous(found.ids),
        })
    }
}
