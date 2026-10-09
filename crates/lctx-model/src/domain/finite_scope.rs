//! Indexed finite lowering of the actual scope program. Rich rows stay borrowed.
use super::{
    charged::{ChargedSet, StateCharge},
    resources::ResourceBudget,
    scope_program::*,
    source::Occurrence,
    *,
};
use std::collections::BTreeMap;

pub struct FiniteScopeRow<'a> {
    id: [u8; 16],
    references: Vec<SemanticReference>,
    occurrence: Option<&'a Occurrence>,
    nulls: Vec<&'static str>,
    values: Vec<(&'static str, ScopeValue)>,
}
impl<'a> FiniteScopeRow<'a> {
    /// Caller reserves compact metadata before constructing a selected input adapter.
    pub fn of<R: Record>(row: &R) -> Self {
        Self {
            id: *row.id().bytes(),
            references: row.physical_references(),
            occurrence: None,
            nulls: vec![],
            values: vec![],
        }
    }
    pub fn with_occurrence(mut self, row: &'a Occurrence) -> Self {
        self.occurrence = Some(row);
        self
    }
    pub fn with_null(mut self, field: &'static str) -> Self {
        self.nulls.push(field);
        self
    }
    pub fn with_code(mut self, field: &'static str, code: i16) -> Self {
        self.values.push((field, ScopeValue::Code(code)));
        self
    }
    pub fn with_boolean(mut self, field: &'static str, value: bool) -> Self {
        self.values.push((field, ScopeValue::Boolean(value)));
        self
    }
    pub fn with_integer(mut self, field: &'static str, value: i64) -> Self {
        self.values.push((field, ScopeValue::Integer(value)));
        self
    }
    pub fn with_text(mut self, field: &'static str, text: String) -> Self {
        self.values.push((field, ScopeValue::Text(text)));
        self
    }
    fn value(&self, field: &str) -> Option<ScopeValue> {
        if field == "id" {
            return Some(ScopeValue::Nominal(self.id));
        }
        if let Some(row) = self.occurrence {
            if field == "start" {
                return Some(ScopeValue::Integer(row.start));
            }
            if field == "end" {
                return Some(ScopeValue::Integer(row.end));
            }
        }
        if let Some((_, value)) = self.values.iter().find(|(name, _)| *name == field) {
            return Some(value.clone());
        }
        let mut refs = self.references.iter().filter(|r| r.field == field);
        let r = refs.next()?;
        if refs.next().is_some() {
            return None;
        }
        Some(ScopeValue::Nominal(r.key))
    }
}
type Nominal = (usize, [u8; 16]);
type AssignmentCallback<'a> = dyn FnMut(&[Option<usize>]) -> Result<(), ModelError> + 'a;
type NearestWitness = (usize, [u8; 16], Vec<Option<usize>>);
struct FiniteDiscovery {
    targets: ChargedSet<Nominal>,
    _charge: StateCharge,
}
pub struct FiniteSelection {
    partitions: Vec<Vec<Nominal>>,
    outcomes: Vec<ScopeRootOutcome>,
    _charge: Box<dyn resources::Reservation>,
}
impl FiniteSelection {
    pub fn partitions(&self) -> &[Vec<Nominal>] {
        &self.partitions
    }
    pub fn outcomes(&self) -> &[ScopeRootOutcome] {
        &self.outcomes
    }
    pub fn partition(&self, index: usize) -> Result<&[Nominal], ModelError> {
        self.partitions
            .get(index)
            .map(Vec::as_slice)
            .ok_or(ModelError::Schema("finite scope partition absent"))
    }
}
type Index = BTreeMap<(&'static str, ScopeValue), Vec<usize>>;
pub struct FiniteScope<'a> {
    program: ScopeProgram,
    rows: Vec<Vec<FiniteScopeRow<'a>>>,
    indices: Vec<Index>,
    _program_charge: Box<dyn resources::Reservation>,
    _charge: Box<dyn resources::Reservation>,
}
impl<'a> FiniteScope<'a> {
    pub fn new(
        program: ScopeProgram,
        rows: Vec<Vec<FiniteScopeRow<'a>>>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        program.validate(model)?;
        let program_charge = budget.reserve("finite-scope-program", program.allowance())?;
        if rows.len() != program.inputs.len() {
            return Err(ModelError::Schema("finite scope input inventory"));
        }
        // Scalar predicates need an explicit finite value or explicit NULL. A list is never
        // represented as SQL NULL simply because it has zero or several nominal references.
        let check_column = |port: usize, column: ScopeColumn| -> Result<(), ModelError> {
            if column.field == "id" {
                return Ok(());
            }
            let input = program
                .ports
                .get(port)
                .ok_or(ModelError::Schema("finite scope port absent"))?
                .input;
            let relation = model
                .relation(program.inputs[input].name())
                .ok_or(ModelError::Schema("finite scope relation absent"))?;
            let field = relation
                .fields()
                .iter()
                .find(|field| field.name() == column.field)
                .ok_or(ModelError::Schema("finite scope column absent"))?;
            if field.list() {
                return Err(ModelError::Schema(
                    "finite scalar predicate on list unsupported",
                ));
            }
            if field.target().is_none()
                && rows[input].iter().any(|row| {
                    row.value(column.field).is_none() && !row.nulls.contains(&column.field)
                })
            {
                return Err(ModelError::Schema(
                    "finite scalar column requires explicit value or null",
                ));
            }
            Ok(())
        };
        for rule in &program.rules {
            if let ScopeRule::Pairs {
                rows: ports,
                predicates,
                source_key,
                target_key,
                ..
            }
            | ScopeRule::FirstPairs {
                rows: ports,
                predicates,
                source_key,
                target_key,
                ..
            }
            | ScopeRule::OptionalPairs {
                rows: ports,
                predicates,
                source_key,
                target_key,
                ..
            }
            | ScopeRule::NearestPairs {
                rows: ports,
                predicates,
                source_key,
                target_key,
                ..
            } = rule
            {
                check_column(ports[source_key.row], *source_key)?;
                check_column(ports[target_key.row], *target_key)?;
                let occurrence_adapter = |row: usize| -> Result<(), ModelError> {
                    let input = program.ports[ports[row]].input;
                    if rows[input].iter().any(|row| {
                        row.occurrence
                            .is_none_or(|occurrence| *occurrence.id().bytes() != row.id)
                    }) {
                        return Err(ModelError::Schema(
                            "finite occurrence adapter absent or mismatched",
                        ));
                    }
                    Ok(())
                };
                if let ScopeRule::NearestPairs {
                    ancestor,
                    event_key,
                    ..
                } = rule
                {
                    occurrence_adapter(*ancestor)?;
                    check_column(ports[event_key.row], *event_key)?;
                }
                let mut columns = Vec::new();
                let post = if let ScopeRule::NearestPairs { post, .. } = rule {
                    post.as_slice()
                } else {
                    &[]
                };
                for predicate in predicates.iter().chain(post) {
                    match predicate {
                        ScopePredicate::CanonicalOccurrence { row } => occurrence_adapter(*row)?,
                        ScopePredicate::Equal(a, b) => columns.extend([*a, *b]),
                        ScopePredicate::Boolean(c, _)
                        | ScopePredicate::Integer(c, _)
                        | ScopePredicate::NotCode(c, _)
                        | ScopePredicate::CodeNotIn(c, _)
                        | ScopePredicate::Code(c, _)
                        | ScopePredicate::CodeIn(c, _)
                        | ScopePredicate::Text(c, _)
                        | ScopePredicate::TextSuffix(c, _)
                        | ScopePredicate::TextSuffixIn(c, _)
                        | ScopePredicate::Parameter(c, _)
                        | ScopePredicate::ParameterIn(c, _)
                        | ScopePredicate::IsNull(c, _) => columns.push(*c),
                        ScopePredicate::EqualCoalesce {
                            column,
                            primary,
                            fallback,
                        } => columns.extend([*column, *primary, *fallback]),
                        ScopePredicate::SpanContains {
                            outer_start,
                            outer_end,
                            inner_start,
                            inner_end,
                        } => columns.extend([*outer_start, *outer_end, *inner_start, *inner_end]),
                        ScopePredicate::QualifiedName {
                            wanted,
                            module,
                            leaf,
                            ..
                        } => columns.extend([*wanted, *module, *leaf]),
                        ScopePredicate::BodyContains { parent, child }
                        | ScopePredicate::PathPrefix { parent, child } => {
                            occurrence_adapter(*parent)?;
                            occurrence_adapter(*child)?;
                        }
                    }
                }
                if let ScopeRule::OptionalPairs { optional, .. } = rule {
                    for join in optional {
                        for (a, b) in &join.keys {
                            columns.extend([*a, *b]);
                        }
                    }
                }
                for c in columns {
                    check_column(ports[c.row], c)?;
                }
            }
        }
        let bytes = rows
            .iter()
            .flatten()
            .map(|r| {
                512 + if r.occurrence.is_some() { 1024 } else { 0 }
                    + r.nulls.len() * 64
                    + r.references.len() * 384
                    + r.values
                        .iter()
                        .map(|(_, v)| match v {
                            ScopeValue::Text(t) => t.len() * 3 + 512,
                            _ => 512,
                        })
                        .sum::<usize>()
            })
            .sum::<usize>();
        let charge = budget.reserve("finite-scope-index", bytes.saturating_add(4096))?;
        let mut indices = Vec::with_capacity(rows.len());
        for table in &rows {
            let mut index: Index = BTreeMap::new();
            for (i, row) in table.iter().enumerate() {
                index
                    .entry(("id", ScopeValue::Nominal(row.id)))
                    .or_default()
                    .push(i);
                for reference in &row.references {
                    index
                        .entry((reference.field, ScopeValue::Nominal(reference.key)))
                        .or_default()
                        .push(i);
                }
                for (field, _) in &row.values {
                    if let Some(value) = row.value(field) {
                        index.entry((*field, value)).or_default().push(i);
                    }
                }
                if row.occurrence.is_some() {
                    for field in ["start", "end"] {
                        if let Some(value) = row.value(field) {
                            index.entry((field, value)).or_default().push(i);
                        }
                    }
                }
            }
            for selected in index.values_mut() {
                selected.sort_unstable();
                selected.dedup();
            }
            indices.push(index);
        }
        Ok(Self {
            program,
            rows,
            indices,
            _program_charge: program_charge,
            _charge: charge,
        })
    }
    fn input(&self, port: usize) -> Result<usize, ModelError> {
        self.program
            .ports
            .get(port)
            .map(|p| p.input)
            .ok_or(ModelError::Schema("finite scope port absent"))
    }
    fn index_rows(
        &self,
        port: usize,
        field: &'static str,
        value: &ScopeValue,
    ) -> Result<&[usize], ModelError> {
        let input = self.input(port)?;
        Ok(self.indices[input]
            .get(&(field, value.clone()))
            .map_or(&[], Vec::as_slice))
    }
    fn column(
        &self,
        ports: &[usize],
        assigned: &[Option<usize>],
        column: ScopeColumn,
    ) -> Option<ScopeValue> {
        let row = assigned.get(column.row).copied().flatten()?;
        let input = self.program.ports.get(*ports.get(column.row)?)?.input;
        self.rows.get(input)?.get(row)?.value(column.field)
    }
    fn predicate(
        &self,
        ports: &[usize],
        assigned: &[Option<usize>],
        predicate: &ScopePredicate,
        parameters: &ScopeParameters,
    ) -> Option<bool> {
        Some(match predicate {
            ScopePredicate::CanonicalOccurrence { row } => {
                let input = self.input(*ports.get(*row)?).ok()?;
                let candidate = self.rows[input]
                    .get(assigned.get(*row).copied().flatten()?)?
                    .occurrence?;
                self.rows[input]
                    .iter()
                    .filter_map(|row| row.occurrence)
                    .filter(|other| {
                        other.source == candidate.source
                            && other.structural_path == candidate.structural_path
                    })
                    .map(|other| *other.id().bytes())
                    .max()
                    == Some(*candidate.id().bytes())
            }
            ScopePredicate::Boolean(c, v) => {
                self.column(ports, assigned, *c) == Some(ScopeValue::Boolean(*v))
            }
            ScopePredicate::Integer(c, v) => {
                self.column(ports, assigned, *c) == Some(ScopeValue::Integer(*v))
            }
            ScopePredicate::NotCode(c, v) => {
                matches!(self.column(ports,assigned,*c),Some(ScopeValue::Code(code)) if code!=*v)
            }
            ScopePredicate::CodeNotIn(_, []) => true,
            ScopePredicate::CodeNotIn(c, vs) => {
                matches!(self.column(ports,assigned,*c),Some(ScopeValue::Code(code)) if !vs.contains(&code))
            }
            ScopePredicate::SpanContains {
                outer_start,
                outer_end,
                inner_start,
                inner_end,
            } => {
                let (
                    Some(ScopeValue::Integer(a)),
                    Some(ScopeValue::Integer(b)),
                    Some(ScopeValue::Integer(c)),
                    Some(ScopeValue::Integer(d)),
                ) = (
                    self.column(ports, assigned, *outer_start),
                    self.column(ports, assigned, *outer_end),
                    self.column(ports, assigned, *inner_start),
                    self.column(ports, assigned, *inner_end),
                )
                else {
                    return Some(false);
                };
                a <= c && b >= d
            }
            ScopePredicate::QualifiedName {
                wanted,
                module,
                leaf,
                leaf_match,
            } => {
                let Some(ScopeValue::Text(wanted)) = self.column(ports, assigned, *wanted) else {
                    return Some(false);
                };
                let module = match self.column(ports, assigned, *module) {
                    Some(ScopeValue::Text(value)) => value,
                    None => String::new(),
                    _ => return Some(false),
                };
                let leaf = match self.column(ports, assigned, *leaf) {
                    Some(ScopeValue::Text(value)) => value,
                    None => String::new(),
                    _ => return Some(false),
                };
                if *leaf_match {
                    wanted.starts_with(&format!("{module}."))
                        && wanted.ends_with(&format!(".{leaf}"))
                } else {
                    wanted == format!("{module}.{leaf}")
                }
            }
            ScopePredicate::Equal(a, b) => self
                .column(ports, assigned, *a)
                .zip(self.column(ports, assigned, *b))
                .is_some_and(|(a, b)| a == b),
            ScopePredicate::Code(c, v) => {
                self.column(ports, assigned, *c) == Some(ScopeValue::Code(*v))
            }
            ScopePredicate::CodeIn(c, vs) => {
                matches!(self.column(ports,assigned,*c),Some(ScopeValue::Code(code)) if vs.contains(&code))
            }
            ScopePredicate::Text(c, v) => {
                self.column(ports, assigned, *c) == Some(ScopeValue::Text((*v).into()))
            }
            ScopePredicate::TextSuffix(c, v) => {
                matches!(self.column(ports,assigned,*c),Some(ScopeValue::Text(text)) if text.ends_with(v))
            }
            ScopePredicate::TextSuffixIn(c, vs) => {
                matches!(self.column(ports,assigned,*c),Some(ScopeValue::Text(text)) if vs.iter().any(|v|text.ends_with(v)))
            }
            ScopePredicate::Parameter(c, index) => {
                self.column(ports, assigned, *c).as_ref() == parameters.0.get(*index)
            }
            ScopePredicate::ParameterIn(c, index) => {
                matches!((self.column(ports,assigned,*c),parameters.0.get(*index)),(Some(ScopeValue::Nominal(key)),Some(ScopeValue::Nominals(keys))) if keys.contains(&key))
            }
            ScopePredicate::IsNull(c, yes) => self.column(ports, assigned, *c).is_none() == *yes,
            ScopePredicate::EqualCoalesce {
                column,
                primary,
                fallback,
            } => self
                .column(ports, assigned, *column)
                .zip(
                    self.column(ports, assigned, *primary)
                        .or_else(|| self.column(ports, assigned, *fallback)),
                )
                .is_some_and(|(a, b)| a == b),
            ScopePredicate::BodyContains { parent, child }
            | ScopePredicate::PathPrefix { parent, child } => {
                let a = self.rows[self.input(*ports.get(*parent)?).ok()?]
                    .get(assigned.get(*parent).copied().flatten()?)?
                    .occurrence?;
                let b = self.rows[self.input(*ports.get(*child)?).ok()?]
                    .get(assigned.get(*child).copied().flatten()?)?
                    .occurrence?;
                if matches!(predicate, ScopePredicate::PathPrefix { .. }) {
                    a.source == b.source && b.structural_path.starts_with(&a.structural_path)
                } else {
                    body_contains(a, b)
                }
            }
        })
    }
    fn ready(predicate: &ScopePredicate, assigned: &[Option<usize>]) -> bool {
        let bound = |row: usize| assigned.get(row).is_some_and(Option::is_some);
        match predicate {
            ScopePredicate::CanonicalOccurrence { row } => bound(*row),
            ScopePredicate::Boolean(c, _)
            | ScopePredicate::Integer(c, _)
            | ScopePredicate::NotCode(c, _)
            | ScopePredicate::CodeNotIn(c, _) => bound(c.row),
            ScopePredicate::SpanContains {
                outer_start,
                outer_end,
                inner_start,
                inner_end,
            } => [outer_start, outer_end, inner_start, inner_end]
                .iter()
                .all(|c| bound(c.row)),
            ScopePredicate::QualifiedName {
                wanted,
                module,
                leaf,
                ..
            } => [wanted, module, leaf].iter().all(|c| bound(c.row)),
            ScopePredicate::Equal(a, b) => bound(a.row) && bound(b.row),
            ScopePredicate::Code(c, _)
            | ScopePredicate::Text(c, _)
            | ScopePredicate::TextSuffix(c, _)
            | ScopePredicate::Parameter(c, _)
            | ScopePredicate::ParameterIn(c, _)
            | ScopePredicate::CodeIn(c, _)
            | ScopePredicate::TextSuffixIn(c, _)
            | ScopePredicate::IsNull(c, _) => bound(c.row),
            ScopePredicate::EqualCoalesce {
                column,
                primary,
                fallback,
            } => bound(column.row) && bound(primary.row) && bound(fallback.row),
            ScopePredicate::BodyContains { parent, child }
            | ScopePredicate::PathPrefix { parent, child } => bound(*parent) && bound(*child),
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Finite join binds the same bounded program and one nominal demand"
    )]
    fn join(
        &self,
        ports: &[usize],
        optional: &[ScopeOptionalJoin],
        predicates: &[ScopePredicate],
        source: ScopeColumn,
        key: [u8; 16],
        parameters: &ScopeParameters,
        assigned: &mut [Option<usize>],
        emit: &mut AssignmentCallback<'_>,
    ) -> Result<(), ModelError> {
        if predicates.iter().any(|p| {
            Self::ready(p, assigned) && self.predicate(ports, assigned, p, parameters) != Some(true)
        }) {
            return Ok(());
        }
        if assigned.iter().all(Option::is_some) {
            if self.column(ports, assigned, source) != Some(ScopeValue::Nominal(key)) {
                return Ok(());
            }
            return emit(assigned);
        }
        // Optional joins are keyed against already bound rows. No match binds SQL NULL;
        // it is distinct from an unbound row and never invents a nominal vertex.
        for join in optional {
            if assigned[join.row].is_none()
                && join.keys.iter().all(|(_, b)| assigned[b.row].is_some())
            {
                let mut candidates: Option<&[usize]> = None;
                for (a, b) in &join.keys {
                    let selected = if let Some(value) = self.column(ports, assigned, *b) {
                        self.index_rows(ports[join.row], a.field, &value)?
                    } else {
                        &[]
                    };
                    if candidates.is_none_or(|old| selected.len() < old.len()) {
                        candidates = Some(selected);
                    }
                }
                let mut matched = false;
                for &i in candidates.unwrap_or(&[]) {
                    assigned[join.row] = Some(i);
                    if join.keys.iter().all(|(a, b)| {
                        self.column(ports, assigned, *a)
                            .zip(self.column(ports, assigned, *b))
                            .is_some_and(|(a, b)| a == b)
                    }) {
                        matched = true;
                        self.join(
                            ports, optional, predicates, source, key, parameters, assigned, emit,
                        )?;
                    }
                }
                if !matched {
                    assigned[join.row] = Some(usize::MAX);
                    self.join(
                        ports, optional, predicates, source, key, parameters, assigned, emit,
                    )?;
                }
                assigned[join.row] = None;
                return Ok(());
            }
        }
        // Use an available equality index before enumerating any unbound relation. This
        // makes a chain of nominal joins linear in actual matches rather than Cartesian.
        let mut best: Option<(usize, &[usize])> = None;
        for row in 0..ports.len() {
            if assigned[row].is_some() || optional.iter().any(|j| j.row == row) {
                continue;
            }
            let mut constraints = Vec::new();
            if source.row == row {
                constraints.push((source.field, ScopeValue::Nominal(key)));
            }
            for p in predicates {
                match p {
                    ScopePredicate::Equal(a, b) => {
                        for (a, b) in [(*a, *b), (*b, *a)] {
                            if a.row == row && assigned[b.row].is_some() {
                                if let Some(value) = self.column(ports, assigned, b) {
                                    constraints.push((a.field, value));
                                } else {
                                    return Ok(());
                                }
                            }
                        }
                    }
                    ScopePredicate::Boolean(c, v) if c.row == row => {
                        constraints.push((c.field, ScopeValue::Boolean(*v)))
                    }
                    ScopePredicate::Integer(c, v) if c.row == row => {
                        constraints.push((c.field, ScopeValue::Integer(*v)))
                    }
                    ScopePredicate::Code(c, v) if c.row == row => {
                        constraints.push((c.field, ScopeValue::Code(*v)))
                    }
                    ScopePredicate::Text(c, v) if c.row == row => {
                        constraints.push((c.field, ScopeValue::Text((*v).into())))
                    }
                    ScopePredicate::Parameter(c, i) if c.row == row => constraints.push((
                        c.field,
                        parameters
                            .0
                            .get(*i)
                            .ok_or(ModelError::Schema("finite scope parameter absent"))?
                            .clone(),
                    )),
                    _ => {}
                }
            }
            for (field, value) in constraints {
                let candidates = self.index_rows(ports[row], field, &value)?;
                if best.is_none_or(|(_, b)| candidates.len() < b.len()) {
                    best = Some((row, candidates));
                }
            }
        }
        if let Some((row, candidates)) = best {
            for &i in candidates {
                assigned[row] = Some(i);
                self.join(
                    ports, optional, predicates, source, key, parameters, assigned, emit,
                )?;
            }
            assigned[row] = None;
        } else {
            let row = assigned
                .iter()
                .enumerate()
                .find(|(i, v)| v.is_none() && !optional.iter().any(|j| j.row == *i))
                .map(|(i, _)| i)
                .ok_or(ModelError::Schema("cyclic optional scope binding"))?;
            for i in 0..self.rows[self.input(ports[row])?].len() {
                assigned[row] = Some(i);
                self.join(
                    ports, optional, predicates, source, key, parameters, assigned, emit,
                )?;
            }
            assigned[row] = None;
        }
        Ok(())
    }
    fn discover(
        &self,
        key: Nominal,
        parameters: &ScopeParameters,
        budget: &ResourceBudget,
    ) -> Result<FiniteDiscovery, ModelError> {
        let mut charge = StateCharge::new(budget, "finite-scope-rule-scratch");
        let mut found = ChargedSet::default();
        for rule in &self.program.rules {
            match rule {
                ScopeRule::Reference {
                    source,
                    field,
                    target,
                    direction,
                    ..
                } => match direction {
                    ScopeDirection::Forward if *source == key.0 => {
                        for &row in self.index_rows(*source, "id", &ScopeValue::Nominal(key.1))? {
                            for r in &self.rows[self.input(*source)?][row].references {
                                if r.field == *field {
                                    found.insert(&mut charge, (*target, r.key))?;
                                }
                            }
                        }
                    }
                    ScopeDirection::OwnedReverse if *target == key.0 => {
                        for &row in self.index_rows(*source, field, &ScopeValue::Nominal(key.1))? {
                            found.insert(
                                &mut charge,
                                (*source, self.rows[self.input(*source)?][row].id),
                            )?;
                        }
                    }
                    _ => {}
                },
                ScopeRule::NearestPairs {
                    source,
                    target,
                    rows,
                    rank_rows,
                    predicates,
                    post,
                    source_key,
                    target_key,
                    event_key,
                    ancestor,
                } if *source == key.0 => {
                    let rank_ports = &rows[..*rank_rows];
                    charge.grow(rows.len() * 32)?;
                    let mut assigned = vec![None; *rank_rows];
                    let mut winners: BTreeMap<[u8; 16], NearestWitness> = BTreeMap::new();
                    let mut held = StateCharge::new(budget, "finite-nearest-witnesses");
                    self.join(
                        rank_ports,
                        &[],
                        predicates,
                        *source_key,
                        key.1,
                        parameters,
                        &mut assigned,
                        &mut |assigned| {
                            let Some(ScopeValue::Nominal(event)) =
                                self.column(rank_ports, assigned, *event_key)
                            else {
                                return Ok(());
                            };
                            let row = assigned[*ancestor]
                                .ok_or(ModelError::Schema("finite nearest ancestor assignment"))?;
                            let occurrence = self.rows[self.input(rows[*ancestor])?][row]
                                .occurrence
                                .ok_or(ModelError::Schema("finite nearest occurrence adapter"))?;
                            let rank = (occurrence.structural_path.len(), *occurrence.id().bytes());
                            if winners.get(&event).is_none_or(|old| rank > (old.0, old.1)) {
                                if !winners.contains_key(&event) {
                                    held.grow(256 + size_of_val(assigned))?;
                                }
                                winners.insert(event, (rank.0, rank.1, assigned.to_vec()));
                            }
                            Ok(())
                        },
                    )?;
                    let mut targets = ChargedSet::default();
                    let mut target_charge = StateCharge::new(budget, "finite-nearest-targets");
                    for (_, (_, _, prefix)) in winners {
                        let mut assigned = prefix;
                        assigned.resize(rows.len(), None);
                        self.join(
                            rows,
                            &[],
                            post,
                            *source_key,
                            key.1,
                            parameters,
                            &mut assigned,
                            &mut |assigned| {
                                if let Some(ScopeValue::Nominal(id)) =
                                    self.column(rows, assigned, *target_key)
                                {
                                    targets.insert(&mut target_charge, id)?;
                                }
                                Ok(())
                            },
                        )?;
                    }
                    for &id in targets.iter() {
                        found.insert(&mut charge, (*target, id))?;
                    }
                }
                ScopeRule::Pairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::FirstPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::OptionalPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                    ..
                } if *source == key.0 => {
                    charge.grow(rows.len() * 32)?;
                    let mut assigned = vec![None; rows.len()];
                    let mut targets = ChargedSet::default();
                    let mut target_charge = StateCharge::new(budget, "finite-scope-pair-targets");
                    let optional = if let ScopeRule::OptionalPairs { optional, .. } = rule {
                        optional.as_slice()
                    } else {
                        &[]
                    };
                    self.join(
                        rows,
                        optional,
                        predicates,
                        *source_key,
                        key.1,
                        parameters,
                        &mut assigned,
                        &mut |assigned| {
                            if let Some(ScopeValue::Nominal(id)) =
                                self.column(rows, assigned, *target_key)
                            {
                                targets.insert(&mut target_charge, id)?;
                            }
                            Ok(())
                        },
                    )?;
                    for &id in
                        targets
                            .iter()
                            .take(if matches!(rule, ScopeRule::FirstPairs { .. }) {
                                1
                            } else {
                                usize::MAX
                            })
                    {
                        found.insert(&mut charge, (*target, id))?;
                    }
                }
                _ => {}
            }
        }
        Ok(FiniteDiscovery {
            targets: found,
            _charge: charge,
        })
    }
    /// Parameters are fresh per execution. Immutable shape and finite equality indexes are shared.
    pub fn select(
        &self,
        roots: &[Nominal],
        parameters: &ScopeParameters,
        budget: &ResourceBudget,
    ) -> Result<FiniteSelection, ModelError> {
        for rule in &self.program.rules {
            if let ScopeRule::Pairs { predicates, .. }
            | ScopeRule::FirstPairs { predicates, .. }
            | ScopeRule::OptionalPairs { predicates, .. }
            | ScopeRule::NearestPairs { predicates, .. } = rule
            {
                let post = if let ScopeRule::NearestPairs { post, .. } = rule {
                    post.as_slice()
                } else {
                    &[]
                };
                for predicate in predicates.iter().chain(post) {
                    match predicate {
                        ScopePredicate::Parameter(_, index)
                            if parameters.0.get(*index).is_none() =>
                        {
                            return Err(ModelError::Schema("finite scope parameter absent"));
                        }
                        ScopePredicate::ParameterIn(_, index)
                            if !matches!(
                                parameters.0.get(*index),
                                Some(ScopeValue::Nominals(_))
                            ) =>
                        {
                            return Err(ModelError::Schema(
                                "finite scope nominal-set parameter absent",
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut charge = StateCharge::new(budget, "finite-scope-selection-state");
        let mut vertices = ChargedSet::default();
        let mut arcs = ChargedSet::default();
        let mut pending = Vec::new();
        charge.grow(roots.len() * 32)?;
        let mut outcomes = Vec::with_capacity(roots.len());
        for &root in roots {
            self.input(root.0)?;
            let outcome = if self.program.ports[root.0].virtual_owner {
                ScopeRootOutcome::Virtual
            } else if self
                .index_rows(root.0, "id", &ScopeValue::Nominal(root.1))?
                .is_empty()
            {
                ScopeRootOutcome::Absent
            } else {
                ScopeRootOutcome::Present
            };
            outcomes.push(outcome);
            if outcome != ScopeRootOutcome::Absent && vertices.insert(&mut charge, root)? {
                charge.grow(32)?;
                pending.push(root);
            }
        }
        while let Some(key) = pending.pop() {
            let targets = self.discover(key, parameters, budget)?;
            for &target in targets.targets.iter() {
                arcs.insert(&mut charge, (key, target))?;
                if vertices.insert(&mut charge, target)? {
                    charge.grow(32)?;
                    pending.push(target);
                }
            }
        }
        charge.grow(vertices.len() * 192 + arcs.len() * 64 + roots.len() * 64)?;
        let nodes = vertices.iter().copied().collect::<Vec<_>>();
        let indices = nodes
            .iter()
            .enumerate()
            .map(|(i, k)| (*k, i))
            .collect::<BTreeMap<_, _>>();
        let edges = arcs
            .iter()
            .map(|(a, b)| (indices[a], indices[b]))
            .collect::<Vec<_>>();
        let graph = CompactScope::new(&nodes, &edges, budget)?;
        let retained = budget.reserve(
            "finite-scope-result",
            roots
                .len()
                .saturating_mul(nodes.len())
                .saturating_mul(32)
                .saturating_add(roots.len() * 64),
        )?;
        let mut result = Vec::new();
        for (i, root) in roots.iter().enumerate() {
            if outcomes[i] == ScopeRootOutcome::Absent {
                result.push(vec![]);
            } else {
                let selected = graph.select(&[indices[root]], budget)?;
                result.push(selected.iter().map(|i| nodes[i]).collect());
            }
        }
        Ok(FiniteSelection {
            partitions: result,
            outcomes,
            _charge: retained,
        })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn absent_requested_owner_does_not_suppress_reached_nominal_expansion() {
        let package = input::Package {
            name: "absent".into(),
        };
        let a = input::Release {
            package: package.id(),
            version: "1".into(),
        };
        let b = input::Release {
            package: package.id(),
            version: "2".into(),
        };
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let program = ScopeProgram {
            inputs: vec![
                ValidationInput::of::<input::Package>(&["id"]),
                ValidationInput::of::<input::Release>(&["id"]),
            ],
            ports: vec![
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 1,
                    virtual_owner: false,
                },
            ],
            rules: vec![
                ScopeRule::Reference {
                    source: 1,
                    field: "package",
                    target: 0,
                    direction: ScopeDirection::Forward,
                    list: false,
                },
                ScopeRule::Reference {
                    source: 1,
                    field: "package",
                    target: 0,
                    direction: ScopeDirection::OwnedReverse,
                    list: false,
                },
            ],
        };
        let prepared = FiniteScope::new(
            program,
            vec![vec![], vec![FiniteScopeRow::of(&a), FiniteScopeRow::of(&b)]],
            &model,
            &budget,
        )
        .unwrap();
        let selected = prepared
            .select(
                &[(0, *package.id().bytes()), (1, *a.id().bytes())],
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        assert_eq!(
            selected.outcomes(),
            &[ScopeRootOutcome::Absent, ScopeRootOutcome::Present]
        );
        assert!(selected.partition(0).unwrap().is_empty());
        assert!(
            selected
                .partition(1)
                .unwrap()
                .contains(&(1, *b.id().bytes()))
        );
        let single = prepared
            .select(&[(1, *a.id().bytes())], &ScopeParameters(vec![]), &budget)
            .unwrap();
        assert_eq!(selected.partition(1).unwrap(), single.partition(0).unwrap());
        drop(single);
        drop(selected);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
}
#[cfg(test)]
mod scalar_and_charge_controls {
    use super::*;
    use crate::domain::source::{Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind};
    #[test]
    fn borrowed_occurrence_integer_equality_uses_derived_scalar_index_and_holds_discovery_charge() {
        let input = input::InputRevision {
            manifest: ContentHash::of(b"integer"),
        };
        let source = SourceArtifact::from_bytes(input.id(), "integer.py".into(), b"value").unwrap();
        let occurrence = Occurrence {
            source: source.id(),
            start: 1,
            end: 4,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let c = |field| ScopeColumn { row: 0, field };
        let program = ScopeProgram {
            inputs: vec![ValidationInput::of::<Occurrence>(&["id"])],
            ports: vec![ScopePort {
                input: 0,
                virtual_owner: false,
            }],
            rules: vec![ScopeRule::Pairs {
                source: 0,
                target: 0,
                rows: vec![0],
                predicates: vec![
                    ScopePredicate::Integer(c("start"), 1),
                    ScopePredicate::Integer(c("end"), 4),
                ],
                source_key: c("id"),
                target_key: c("id"),
            }],
        };
        let finite = FiniteScope::new(
            program,
            vec![vec![
                FiniteScopeRow::of(&occurrence).with_occurrence(&occurrence),
            ]],
            &owner,
            &budget,
        )
        .unwrap();
        let baseline = budget.reserved();
        let discovered = finite
            .discover(
                (0, *occurrence.id().bytes()),
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        assert_eq!(
            discovered.targets.iter().copied().collect::<Vec<_>>(),
            vec![(0, *occurrence.id().bytes())]
        );
        assert!(
            budget.reserved() > baseline,
            "discovery metadata stays charged while the consumer retains its keys"
        );
        drop(discovered);
        assert_eq!(budget.reserved(), baseline);
        drop(finite);
        assert_eq!(budget.reserved(), 0);
    }
}
#[cfg(test)]
mod sum_column_controls {
    use super::*;
    use crate::domain::types::{AnyFlavor, TypeTerm};
    #[test]
    fn scalar_sum_column_equality_selects_the_nominal_child_without_changing_semantic_role() {
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let _fixture = budget.reserve("sum-column-fixture", 8192).unwrap();
        let child = TypeTerm::None;
        let selected = TypeTerm::TypeOf { target: child.id() };
        let unrelated = TypeTerm::Any {
            flavor: AnyFlavor::Explicit,
        };
        assert_eq!(selected.references()[0].field, "target");
        assert_eq!(selected.physical_references()[0].field, "typeof_target");
        let id = |row| ScopeColumn { row, field: "id" };
        let program = ScopeProgram {
            inputs: vec![ValidationInput::of::<TypeTerm>(&["id"])],
            ports: vec![
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
            ],
            rules: vec![ScopeRule::Pairs {
                source: 0,
                target: 1,
                rows: vec![0, 1],
                predicates: vec![ScopePredicate::Equal(
                    ScopeColumn {
                        row: 0,
                        field: "typeof_target",
                    },
                    id(1),
                )],
                source_key: id(0),
                target_key: id(1),
            }],
        };
        let finite = FiniteScope::new(
            program,
            vec![vec![
                FiniteScopeRow::of(&child),
                FiniteScopeRow::of(&selected),
                FiniteScopeRow::of(&unrelated),
            ]],
            &owner,
            &budget,
        )
        .unwrap();
        let result = finite
            .select(
                &[(0, *selected.id().bytes())],
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        assert_eq!(
            result.partition(0).unwrap(),
            &[(0, *selected.id().bytes()), (1, *child.id().bytes())]
        );
        drop(result);
        drop(finite);
        drop(_fixture);
        assert_eq!(budget.reserved(), 0);
    }
}
#[cfg(test)]
mod occurrence_adapter_controls {
    use super::*;
    use crate::domain::source::{Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind};
    #[test]
    fn structural_predicates_and_nearest_refuse_missing_occurrence_adapters_upfront() {
        let input = input::InputRevision {
            manifest: ContentHash::of(b"missing-adapter"),
        };
        let source = SourceArtifact::from_bytes(input.id(), "adapter.py".into(), b"value").unwrap();
        let occurrence = Occurrence {
            source: source.id(),
            start: 0,
            end: 5,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let id = ScopeColumn {
            row: 0,
            field: "id",
        };
        for predicate in [
            ScopePredicate::CanonicalOccurrence { row: 0 },
            ScopePredicate::PathPrefix {
                parent: 0,
                child: 0,
            },
            ScopePredicate::BodyContains {
                parent: 0,
                child: 0,
            },
        ] {
            let program = ScopeProgram {
                inputs: vec![ValidationInput::of::<Occurrence>(&["id"])],
                ports: vec![ScopePort {
                    input: 0,
                    virtual_owner: false,
                }],
                rules: vec![ScopeRule::Pairs {
                    source: 0,
                    target: 0,
                    rows: vec![0],
                    predicates: vec![predicate],
                    source_key: id,
                    target_key: id,
                }],
            };
            assert!(
                FiniteScope::new(
                    program,
                    vec![vec![FiniteScopeRow::of(&occurrence)]],
                    &owner,
                    &budget
                )
                .is_err()
            );
            assert_eq!(budget.reserved(), 0);
        }
        let program = ScopeProgram {
            inputs: vec![ValidationInput::of::<Occurrence>(&["id"])],
            ports: vec![ScopePort {
                input: 0,
                virtual_owner: false,
            }],
            rules: vec![ScopeRule::NearestPairs {
                source: 0,
                target: 0,
                rows: vec![0],
                rank_rows: 1,
                predicates: vec![],
                post: vec![],
                source_key: id,
                target_key: id,
                event_key: id,
                ancestor: 0,
            }],
        };
        assert!(
            FiniteScope::new(
                program,
                vec![vec![FiniteScopeRow::of(&occurrence)]],
                &owner,
                &budget
            )
            .is_err()
        );
        assert_eq!(budget.reserved(), 0);
    }
}
