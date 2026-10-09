//! Mechanical relational lowering of model-owned scope intent.
use crate::consumed_rows::{ClosureTable, NominalClosure, identifier};
use lctx_model::domain::{ModelError, scope_program::*};

/// Validate and own one exact model program before physical lowering. The temporary metadata
/// copy is charged while the factory's construction guard remains live.
pub(crate) fn compile(
    program: &ScopeProgram,
    model: &lctx_model::domain::ValidatedModel,
    budget: &lctx_model::domain::resources::ResourceBudget,
    programs: Option<&std::sync::Mutex<ScopeInterner>>,
) -> Result<std::sync::Arc<CompiledScopeProgram>, ModelError> {
    let _copy = budget.reserve("scope-program-copy", program.allowance())?;
    if let Some(programs) = programs {
        programs
            .lock()
            .map_err(|_| ModelError::Conflict("scope interner poisoned"))?
            .intern(program.clone(), model)
    } else {
        ScopeInterner::new(budget)?.intern(program.clone(), model)
    }
}
pub(crate) fn lower_compiled(
    program: std::sync::Arc<CompiledScopeProgram>,
    inputs: &[ClosureTable],
    parameters: &ScopeParameters,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<NominalClosure, ModelError> {
    let charge = budget.reserve(
        "scope-physical-lowering",
        lowering_allowance(program.program(), inputs, parameters),
    )?;
    let mut plan = lower_with(program.program(), inputs, parameters)?;
    plan.retain_lowering(charge);
    plan.retain_program(program, budget)?;
    Ok(plan)
}

/// Reserve before formatting the physical program. Runtime aliases and values are deliberately
/// absent from canonical program identity, but their repeated escaped SQL representations must
/// still be charged. The envelope covers intermediate strings, duplicate map keys, table copies,
/// and the retained plan; query-only callers keep this reservation with their returned SQL.
pub(crate) fn lowering_allowance(
    program: &ScopeProgram,
    inputs: &[ClosureTable],
    parameters: &ScopeParameters,
) -> usize {
    // Canonical framing is not SQL storage. Account for the actual physical rule shape:
    // scalar references retain small fallback queries, whereas joins retain their expressions,
    // ranking projections and repeated escaped runtime bindings.
    let mut bytes = 8192usize;
    for port in &program.ports {
        if let Some(table) = inputs.get(port.input) {
            bytes = bytes
                .saturating_add(table.alias.len().saturating_mul(8))
                .saturating_add(512);
        }
    }
    let alias_len = |port: usize| {
        program
            .ports
            .get(port)
            .and_then(|p| inputs.get(p.input))
            .map_or(0, |t| t.alias.len().saturating_mul(2))
    };
    let column_len = |column: &ScopeColumn| column.field.len().saturating_mul(2).saturating_add(32);
    let value_len = |value: &ScopeValue| match value {
        ScopeValue::Nominal(_) => 36,
        ScopeValue::Nominals(keys) => keys.len().saturating_mul(40).saturating_add(2),
        ScopeValue::Text(text) => text.len().saturating_mul(2).saturating_add(2),
        ScopeValue::Code(_) | ScopeValue::Integer(_) => 24,
        ScopeValue::Boolean(_) => 5,
    };
    for rule in &program.rules {
        let (rows, predicates, post) = match rule {
            ScopeRule::Reference {
                source,
                target,
                field,
                ..
            } => {
                bytes = bytes
                    .saturating_add(1024)
                    .saturating_add(field.len().saturating_mul(16))
                    .saturating_add(
                        alias_len(*source)
                            .saturating_add(alias_len(*target))
                            .saturating_mul(8),
                    );
                continue;
            }
            ScopeRule::Pairs {
                rows, predicates, ..
            }
            | ScopeRule::FirstPairs {
                rows, predicates, ..
            }
            | ScopeRule::OptionalPairs {
                rows, predicates, ..
            } => (rows, predicates, &[][..]),
            ScopeRule::NearestPairs {
                rows,
                predicates,
                post,
                ..
            } => (rows, predicates, post.as_slice()),
        };
        bytes = bytes
            .saturating_add(2048)
            .saturating_add(rows.len().saturating_mul(512))
            .saturating_add(
                rows.iter()
                    .fold(0usize, |sum, port| sum.saturating_add(alias_len(*port)))
                    .saturating_mul(8),
            );
        let (source_key, target_key) = match rule {
            ScopeRule::Pairs {
                source_key,
                target_key,
                ..
            }
            | ScopeRule::FirstPairs {
                source_key,
                target_key,
                ..
            }
            | ScopeRule::OptionalPairs {
                source_key,
                target_key,
                ..
            }
            | ScopeRule::NearestPairs {
                source_key,
                target_key,
                ..
            } => (source_key, target_key),
            _ => unreachable!(),
        };
        bytes = bytes.saturating_add(
            column_len(source_key)
                .saturating_add(column_len(target_key))
                .saturating_mul(8),
        );
        if let ScopeRule::OptionalPairs { optional, .. } = rule {
            for join in optional {
                bytes = bytes.saturating_add(512);
                for (a, b) in &join.keys {
                    bytes = bytes.saturating_add(
                        column_len(a)
                            .saturating_add(column_len(b))
                            .saturating_mul(8),
                    );
                }
            }
        }
        if let ScopeRule::NearestPairs { event_key, .. } = rule {
            bytes = bytes.saturating_add(column_len(event_key).saturating_mul(8));
        }
        for predicate in predicates.iter().chain(post) {
            let columns = match predicate {
                ScopePredicate::Equal(a, b) => column_len(a).saturating_add(column_len(b)),
                ScopePredicate::EqualCoalesce {
                    column,
                    primary,
                    fallback,
                } => column_len(column)
                    .saturating_add(column_len(primary))
                    .saturating_add(column_len(fallback)),
                ScopePredicate::SpanContains {
                    outer_start,
                    outer_end,
                    inner_start,
                    inner_end,
                } => [outer_start, outer_end, inner_start, inner_end]
                    .into_iter()
                    .fold(0usize, |sum, c| sum.saturating_add(column_len(c))),
                ScopePredicate::QualifiedName {
                    wanted,
                    module,
                    leaf,
                    ..
                } => column_len(wanted)
                    .saturating_add(column_len(module))
                    .saturating_add(column_len(leaf)),
                ScopePredicate::CanonicalOccurrence { row } => rows
                    .get(*row)
                    .map_or(0, |port| alias_len(*port).saturating_mul(4)),
                ScopePredicate::BodyContains { .. } | ScopePredicate::PathPrefix { .. } => 512,
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
                | ScopePredicate::IsNull(c, _) => column_len(c),
            };
            bytes = bytes
                .saturating_add(1024)
                .saturating_add(columns.saturating_mul(8));
            let literals = match predicate {
                ScopePredicate::Parameter(_, index) | ScopePredicate::ParameterIn(_, index) => {
                    parameters.0.get(*index).map_or(0, value_len)
                }
                ScopePredicate::Text(_, text) | ScopePredicate::TextSuffix(_, text) => {
                    text.len().saturating_mul(2)
                }
                ScopePredicate::TextSuffixIn(_, texts) => texts.iter().fold(0usize, |sum, text| {
                    sum.saturating_add(text.len().saturating_mul(2))
                        .saturating_add(columns)
                        .saturating_add(32)
                }),
                ScopePredicate::CodeIn(_, codes) | ScopePredicate::CodeNotIn(_, codes) => {
                    codes.len().saturating_mul(24)
                }
                _ => 0,
            };
            bytes = bytes.saturating_add(literals.saturating_mul(16));
        }
    }
    bytes
}

#[cfg(test)]
pub(crate) fn lower(
    program: &ScopeProgram,
    inputs: &[ClosureTable],
) -> Result<NominalClosure, ModelError> {
    lower_with(program, inputs, &ScopeParameters(vec![]))
}
pub(crate) fn select_pair_queries(
    program: &ScopeProgram,
    inputs: &[ClosureTable],
    parameters: &ScopeParameters,
) -> Result<Vec<(usize, usize, String)>, ModelError> {
    if program.rules.iter().any(|r| {
        !matches!(
            r,
            ScopeRule::Pairs { .. }
                | ScopeRule::FirstPairs { .. }
                | ScopeRule::OptionalPairs { .. }
                | ScopeRule::NearestPairs { .. }
        )
    }) {
        return Err(ModelError::Schema("root selection requires pair program"));
    }
    Ok(lower_with(program, inputs, parameters)?.into_pair_queries())
}
fn literal(value: &ScopeValue) -> String {
    match value {
        ScopeValue::Nominal(key) => format!("X'{}'", hex::encode(key)),
        ScopeValue::Nominals(keys) => format!(
            "({})",
            keys.iter()
                .map(|key| format!("X'{}'", hex::encode(key)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScopeValue::Code(code) => code.to_string(),
        ScopeValue::Integer(value) => value.to_string(),
        ScopeValue::Boolean(value) => {
            if *value {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        ScopeValue::Text(text) => format!("'{}'", text.replace('\'', "''")),
    }
}
fn predicate_sql(
    p: &ScopePredicate,
    col: &dyn Fn(&ScopeColumn) -> String,
    row_table: &dyn Fn(usize) -> Result<String, ModelError>,
    parameters: &ScopeParameters,
) -> Result<String, ModelError> {
    Ok(match p {
        ScopePredicate::CanonicalOccurrence { row } => format!(
            "{}=(SELECT MAX(canonical_occurrence.id) FROM {} canonical_occurrence WHERE canonical_occurrence.source={} AND canonical_occurrence.structural_path={})",
            col(&ScopeColumn {
                row: *row,
                field: "id"
            }),
            row_table(*row)?,
            col(&ScopeColumn {
                row: *row,
                field: "source"
            }),
            col(&ScopeColumn {
                row: *row,
                field: "structural_path"
            })
        ),
        ScopePredicate::Boolean(c, v) => {
            format!("{}={}", col(c), if *v { "TRUE" } else { "FALSE" })
        }
        ScopePredicate::Integer(c, v) => format!("{}={v}", col(c)),
        ScopePredicate::NotCode(c, v) => format!("{}<>{v}", col(c)),
        ScopePredicate::CodeNotIn(_, []) => "TRUE".into(),
        ScopePredicate::CodeNotIn(c, vs) => format!(
            "{} NOT IN ({})",
            col(c),
            vs.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScopePredicate::SpanContains {
            outer_start,
            outer_end,
            inner_start,
            inner_end,
        } => format!(
            "{}<={} AND {}>={}",
            col(outer_start),
            col(inner_start),
            col(outer_end),
            col(inner_end)
        ),
        ScopePredicate::PathPrefix { parent, child } => format!(
            "{}={} AND array_slice({},1,CAST(array_length({}) AS BIGINT))={}",
            col(&ScopeColumn {
                row: *child,
                field: "source"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "source"
            }),
            col(&ScopeColumn {
                row: *child,
                field: "structural_path"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "structural_path"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "structural_path"
            })
        ),
        ScopePredicate::QualifiedName {
            wanted,
            module,
            leaf,
            leaf_match,
        } => {
            if *leaf_match {
                format!(
                    "starts_with({},concat({},'.')) AND ends_with({},concat('.',{}))",
                    col(wanted),
                    col(module),
                    col(wanted),
                    col(leaf)
                )
            } else {
                format!("{}=concat({},'.',{})", col(wanted), col(module), col(leaf))
            }
        }
        ScopePredicate::Equal(a, b) => format!("{}={}", col(a), col(b)),
        ScopePredicate::Code(c, code) => format!("{}={code}", col(c)),
        ScopePredicate::CodeIn(_, []) => "FALSE".into(),
        ScopePredicate::CodeIn(c, codes) => format!(
            "{} IN ({})",
            col(c),
            codes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScopePredicate::Text(c, text) => {
            format!("{}={}", col(c), literal(&ScopeValue::Text((*text).into())))
        }
        ScopePredicate::TextSuffix(c, text) => format!(
            "ends_with({},{})",
            col(c),
            literal(&ScopeValue::Text((*text).into()))
        ),
        ScopePredicate::TextSuffixIn(_, []) => "FALSE".into(),
        ScopePredicate::TextSuffixIn(c, texts) => format!(
            "({})",
            texts
                .iter()
                .map(|text| format!(
                    "ends_with({},{})",
                    col(c),
                    literal(&ScopeValue::Text((*text).into()))
                ))
                .collect::<Vec<_>>()
                .join(" OR ")
        ),
        ScopePredicate::Parameter(c, index) => format!(
            "{}={}",
            col(c),
            literal(
                parameters
                    .0
                    .get(*index)
                    .ok_or(ModelError::Schema("scope parameter absent"))?
            )
        ),
        ScopePredicate::ParameterIn(c, index) => match parameters.0.get(*index) {
            Some(ScopeValue::Nominals(keys)) if keys.is_empty() => "FALSE".into(),
            Some(value @ ScopeValue::Nominals(_)) => format!("{} IN {}", col(c), literal(value)),
            _ => return Err(ModelError::Schema("scope nominal-set parameter absent")),
        },
        ScopePredicate::IsNull(c, yes) => {
            format!("{} IS {}NULL", col(c), if *yes { "" } else { "NOT " })
        }
        ScopePredicate::EqualCoalesce {
            column,
            primary,
            fallback,
        } => format!(
            "{}=COALESCE({},{})",
            col(column),
            col(primary),
            col(fallback)
        ),
        ScopePredicate::BodyContains { parent, child } => format!(
            "{}={} AND {}>={} AND {}<={} AND array_slice({},1,CAST(array_length({}) AS BIGINT))={}",
            col(&ScopeColumn {
                row: *child,
                field: "source"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "source"
            }),
            col(&ScopeColumn {
                row: *child,
                field: "start"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "start"
            }),
            col(&ScopeColumn {
                row: *child,
                field: "end"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "end"
            }),
            col(&ScopeColumn {
                row: *child,
                field: "structural_path"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "structural_path"
            }),
            col(&ScopeColumn {
                row: *parent,
                field: "structural_path"
            })
        ),
    })
}
pub(crate) fn lower_with(
    program: &ScopeProgram,
    inputs: &[ClosureTable],
    parameters: &ScopeParameters,
) -> Result<NominalClosure, ModelError> {
    let tables = program
        .ports
        .iter()
        .map(|port| {
            inputs
                .get(port.input)
                .cloned()
                .ok_or(ModelError::Schema("scope physical input absent"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let table = |port: usize| -> Result<String, ModelError> {
        Ok(identifier(
            &tables
                .get(port)
                .ok_or(ModelError::Schema("scope physical port absent"))?
                .alias,
        ))
    };
    let mut plan = NominalClosure::new(tables.clone())?;
    for rule in &program.rules {
        match rule {
            ScopeRule::Reference {
                source,
                field,
                target,
                direction,
                list,
            } => {
                if *list {
                    let projected = format!(
                        "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                        identifier(field),
                        table(*source)?
                    );
                    let sql = match direction {
                        ScopeDirection::Forward => projected,
                        ScopeDirection::OwnedReverse => format!(
                            "SELECT target_id AS source_id,source_id AS target_id FROM ({projected}) refs"
                        ),
                    };
                    let (a, b) = match direction {
                        ScopeDirection::Forward => (*source, *target),
                        ScopeDirection::OwnedReverse => (*target, *source),
                    };
                    plan.pairs(a, b, sql)?;
                } else {
                    match direction {
                        ScopeDirection::Forward => plan.follow(*source, field, *target)?,
                        ScopeDirection::OwnedReverse => {
                            plan.own_reverse(*source, field, *target)?
                        }
                    }
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
            }
            | ScopeRule::NearestPairs {
                source,
                target,
                rows,
                predicates,
                source_key,
                target_key,
                ..
            } => {
                let col = |c: &ScopeColumn| format!("r{}.{}", c.row, identifier(c.field));
                let row_table = |row: usize| table(rows[row]);
                let mut from = String::new();
                let rank_rows = if let ScopeRule::NearestPairs { rank_rows, .. } = rule {
                    *rank_rows
                } else {
                    rows.len()
                };
                for (i, port) in rows[..rank_rows].iter().enumerate() {
                    let optional = if let ScopeRule::OptionalPairs { optional, .. } = rule {
                        optional.iter().find(|j| j.row == i)
                    } else {
                        None
                    };
                    if let Some(join) = optional {
                        from.push_str(&format!(
                            " LEFT JOIN {} r{i} ON {}",
                            table(*port)?,
                            join.keys
                                .iter()
                                .map(|(a, b)| format!("{}={}", col(a), col(b)))
                                .collect::<Vec<_>>()
                                .join(" AND ")
                        ));
                    } else {
                        from.push_str(&format!(
                            "{}{} r{i}",
                            if i == 0 { "" } else { " CROSS JOIN " },
                            table(*port)?
                        ));
                    }
                }
                let conditions = predicates
                    .iter()
                    .map(|p| predicate_sql(p, &col, &row_table, parameters))
                    .collect::<Result<Vec<_>, ModelError>>()?
                    .join(" AND ");
                let filter = if conditions.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {conditions}")
                };
                let sql = if let ScopeRule::NearestPairs {
                    event_key,
                    ancestor,
                    post,
                    ..
                } = rule
                {
                    let mut needed = post
                        .iter()
                        .flat_map(ScopePredicate::columns)
                        .chain([*source_key, *target_key])
                        .filter(|c| c.row < rank_rows)
                        .collect::<Vec<_>>();
                    needed.sort_unstable();
                    needed.dedup();
                    let projected = needed
                        .iter()
                        .enumerate()
                        .map(|(i, c)| format!("{} AS v{i}", col(c)))
                        .collect::<Vec<_>>()
                        .join(",");
                    let inner = format!(
                        "SELECT {projected},row_number() OVER (PARTITION BY {},{} ORDER BY array_length(r{ancestor}.structural_path) DESC,r{ancestor}.id DESC) AS nearest FROM {from}{filter}",
                        col(source_key),
                        col(event_key)
                    );
                    let outer_col = |c: &ScopeColumn| {
                        if c.row < rank_rows {
                            format!(
                                "ranked.v{}",
                                needed.binary_search(c).expect("projected nearest field")
                            )
                        } else {
                            col(c)
                        }
                    };
                    let suffix = rows[rank_rows..]
                        .iter()
                        .enumerate()
                        .map(|(n, port)| {
                            Ok(format!(" CROSS JOIN {} r{}", table(*port)?, n + rank_rows))
                        })
                        .collect::<Result<Vec<_>, ModelError>>()?
                        .join("");
                    let conditions = post
                        .iter()
                        .map(|p| predicate_sql(p, &outer_col, &row_table, parameters))
                        .collect::<Result<Vec<_>, ModelError>>()?
                        .join(" AND ");
                    format!(
                        "SELECT {} AS source_id,{} AS target_id FROM ({inner}) ranked{suffix} WHERE nearest=1{}",
                        outer_col(source_key),
                        outer_col(target_key),
                        if conditions.is_empty() {
                            String::new()
                        } else {
                            format!(" AND {conditions}")
                        }
                    )
                } else {
                    format!(
                        "SELECT {} AS source_id,{} AS target_id FROM {from}{filter}",
                        col(source_key),
                        col(target_key)
                    )
                };
                let sql = if matches!(rule, ScopeRule::FirstPairs { .. }) {
                    format!(
                        "SELECT source_id,MIN(target_id) AS target_id FROM ({sql}) candidates GROUP BY source_id"
                    )
                } else {
                    sql
                };
                plan.pairs(*source, *target, sql)?;
            }
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::consumed_rows::{PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        Record, Relation, ValidationInput,
        finite_scope::{FiniteScope, FiniteScopeRow},
        input, model,
        resources::ResourceBudget,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn finite_and_relational_programs_preserve_absence_and_reached_reverse_members() {
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
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
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
        let finite = FiniteScope::new(
            program.clone(),
            vec![vec![], vec![FiniteScopeRow::of(&a), FiniteScopeRow::of(&b)]],
            &model,
            &budget,
        )
        .unwrap();
        let expected = finite
            .select(
                &[(0, *package.id().bytes()), (1, *a.id().bytes())],
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        let session = SessionContext::new();
        for (name, batch) in [
            ("packages", input::Package::encode(&[]).unwrap()),
            ("releases", input::Release::encode(&[a.clone(), b]).unwrap()),
        ] {
            session
                .register_table(
                    name,
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<input::Package>(),
                alias: "packages".into(),
            },
            ClosureTable {
                relation: Relation::of::<input::Release>(),
                alias: "releases".into(),
            },
        ];
        let prepared = lower(&program, &tables)
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let actual = prepared
            .batch(
                &[
                    PreparedRoot {
                        table: 0,
                        key: *package.id().bytes(),
                        kind: PreparedRootKind::Physical,
                    },
                    PreparedRoot {
                        table: 1,
                        key: *a.id().bytes(),
                        kind: PreparedRootKind::Physical,
                    },
                ],
                &budget,
            )
            .await
            .unwrap();
        assert_eq!(actual.outcomes(), expected.outcomes());
        for partition in 0..2 {
            let mut keys = Vec::new();
            for table in 0..2 {
                keys.extend(
                    actual
                        .keys(partition, table)
                        .unwrap()
                        .map(|key| (table, key)),
                );
            }
            assert_eq!(keys, expected.partition(partition).unwrap());
        }
    }
}

#[cfg(test)]
mod compiled_owner_controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        Record, Relation, ValidationInput, input, model, resources::ResourceBudget,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn compiled_program_owner_survives_native_preparation_and_merged_plans() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let program = ScopeProgram {
            inputs: vec![ValidationInput::of::<input::Package>(&["id"])],
            ports: vec![ScopePort {
                input: 0,
                virtual_owner: false,
            }],
            rules: vec![],
        };
        let owner = compile(&program, &model, &budget, None).unwrap();
        let weak = Arc::downgrade(&owner);
        let tables = vec![ClosureTable {
            relation: Relation::of::<input::Package>(),
            alias: "packages".into(),
        }];
        let mut plan =
            lower_compiled(owner.clone(), &tables, &ScopeParameters(vec![]), &budget).unwrap();
        let second =
            lower_compiled(owner.clone(), &tables, &ScopeParameters(vec![]), &budget).unwrap();
        plan.extend(second).unwrap();
        drop(owner);
        assert!(weak.upgrade().is_some());
        let session = SessionContext::new();
        let batch = input::Package::encode(&[]).unwrap();
        session
            .register_table(
                "packages",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        drop(plan);
        assert!(weak.upgrade().is_some());
        drop(edges);
        assert!(weak.upgrade().is_none());
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn runtime_text_lowering_refuses_before_allocation_and_releases_plan_charge() {
        let model = model().unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let program = ScopeProgram {
            inputs: vec![ValidationInput::of::<input::Package>(&["id"])],
            ports: vec![ScopePort {
                input: 0,
                virtual_owner: false,
            }],
            rules: vec![ScopeRule::Pairs {
                source: 0,
                target: 0,
                rows: vec![0],
                predicates: vec![ScopePredicate::Parameter(
                    ScopeColumn {
                        row: 0,
                        field: "name",
                    },
                    0,
                )],
                source_key: ScopeColumn {
                    row: 0,
                    field: "id",
                },
                target_key: ScopeColumn {
                    row: 0,
                    field: "id",
                },
            }],
        };
        let owner = compile(&program, &model, &budget, None).unwrap();
        let baseline = budget.reserved();
        let tables = vec![ClosureTable {
            relation: Relation::of::<input::Package>(),
            alias: "packages".into(),
        }];
        let large = ScopeParameters(vec![ScopeValue::Text("'".repeat(128 << 10))]);
        assert!(lower_compiled(owner.clone(), &tables, &large, &budget).is_err());
        assert_eq!(budget.reserved(), baseline);
        let small = ScopeParameters(vec![ScopeValue::Text("kept".into())]);
        let plan = lower_compiled(owner.clone(), &tables, &small, &budget).unwrap();
        assert!(budget.reserved() > baseline);
        drop(plan);
        assert_eq!(budget.reserved(), baseline);
        drop(owner);
        assert_eq!(budget.reserved(), 0);
    }
}
#[cfg(test)]
mod nearest_controls {
    use super::*;
    use crate::consumed_rows::{PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        Record, Relation, ValidationInput,
        finite_scope::{FiniteScope, FiniteScopeRow},
        input, model,
        resources::ResourceBudget,
        source::*,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn nearest_ranks_before_post_filter_and_uses_greatest_nominal_tie() {
        let input = input::InputRevision {
            manifest: lctx_model::domain::ContentHash::of(b"nearest"),
        };
        let source = SourceArtifact::from_bytes(
            input.id(),
            "nearest.py".into(),
            b"                                 ",
        )
        .unwrap();
        let outer = Occurrence {
            source: source.id(),
            start: 0,
            end: 30,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let site = Occurrence {
            start: 10,
            end: 20,
            syntax_kind: SyntaxKind::ExprName,
            structural_path: vec![0, 1, 0],
            ..outer.clone()
        };
        let inner = Occurrence {
            start: 12,
            end: 15,
            syntax_kind: SyntaxKind::StmtAssign,
            structural_path: vec![0, 1],
            ..outer.clone()
        };
        let tied = Occurrence {
            role: OccurrenceRole::Declaration,
            ..inner.clone()
        };
        let rows = vec![outer.clone(), site.clone(), inner.clone(), tied.clone()];
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let column = |row, field| ScopeColumn { row, field };
        let mut program = ScopeProgram {
            inputs: vec![
                ValidationInput::of::<Occurrence>(&["id"]),
                ValidationInput::of::<Occurrence>(&["id"]),
            ],
            ports: vec![
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 1,
                    virtual_owner: true,
                },
            ],
            rules: vec![ScopeRule::NearestPairs {
                source: 1,
                target: 0,
                rows: vec![1, 0, 0],
                rank_rows: 3,
                predicates: vec![
                    ScopePredicate::Equal(column(1, "id"), column(0, "id")),
                    ScopePredicate::PathPrefix {
                        parent: 2,
                        child: 1,
                    },
                    ScopePredicate::CodeIn(
                        column(2, "syntax_kind"),
                        &[
                            SyntaxKind::StmtFunctionDef as i16,
                            SyntaxKind::StmtAssign as i16,
                        ],
                    ),
                ],
                post: vec![ScopePredicate::SpanContains {
                    outer_start: column(2, "start"),
                    outer_end: column(2, "end"),
                    inner_start: column(0, "start"),
                    inner_end: column(0, "end"),
                }],
                source_key: column(0, "id"),
                target_key: column(2, "id"),
                event_key: column(1, "id"),
                ancestor: 2,
            }],
        };
        let session = SessionContext::new();
        let batch = Occurrence::encode(&rows).unwrap();
        session
            .register_table(
                "nearest_rows",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<Occurrence>(),
                alias: "nearest_rows".into()
            };
            2
        ];
        for post_filter in [true, false] {
            if !post_filter && let ScopeRule::NearestPairs { post, .. } = &mut program.rules[0] {
                post.clear();
            }
            let adapter = || {
                rows.iter()
                    .map(|row| {
                        FiniteScopeRow::of(row)
                            .with_occurrence(row)
                            .with_code("syntax_kind", row.syntax_kind as i16)
                    })
                    .collect()
            };
            let finite =
                FiniteScope::new(program.clone(), vec![adapter(), adapter()], &owner, &budget)
                    .unwrap();
            let expected = finite
                .select(
                    &[(1, *site.id().bytes())],
                    &ScopeParameters(vec![]),
                    &budget,
                )
                .unwrap();
            let edges = lower(&program, &tables)
                .unwrap()
                .prepare(&session, &budget)
                .await
                .unwrap();
            let actual = edges
                .batch(
                    &[PreparedRoot {
                        table: 1,
                        key: *site.id().bytes(),
                        kind: PreparedRootKind::Virtual,
                    }],
                    &budget,
                )
                .await
                .unwrap();
            let keys = actual.keys(0, 0).unwrap().collect::<Vec<_>>();
            let finite_keys = expected
                .partition(0)
                .unwrap()
                .iter()
                .filter(|(port, _)| *port == 0)
                .map(|(_, key)| *key)
                .collect::<Vec<_>>();
            assert_eq!(keys, finite_keys);
            if post_filter {
                assert!(
                    keys.is_empty(),
                    "outer eligible ancestor cannot bypass a failed nearest post filter"
                );
            } else {
                assert_eq!(
                    keys,
                    vec![std::cmp::max(*inner.id().bytes(), *tied.id().bytes())]
                );
            }
        }
    }
}
#[cfg(test)]
mod captured_name_controls {
    use super::*;
    use crate::consumed_rows::{PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        ContentHash, Record, Relation, ValidationInput,
        finite_scope::{FiniteScope, FiniteScopeRow},
        input::{CorpusLibrary, InputRevision},
        model,
        resources::ResourceBudget,
        source::{Module, SourceArtifact},
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn qualified_names_keep_captured_library_alternatives_and_exclude_unlinked_inputs() {
        let own = InputRevision {
            manifest: ContentHash::of(b"corpus"),
        };
        let linked = InputRevision {
            manifest: ContentHash::of(b"linked"),
        };
        let outside = InputRevision {
            manifest: ContentHash::of(b"outside"),
        };
        let root = SourceArtifact::from_bytes(own.id(), "pkg.Thing".into(), b"root").unwrap();
        let artifacts = [own.id(), linked.id(), outside.id()]
            .map(|input| SourceArtifact::from_bytes(input, "Thing".into(), b"candidate").unwrap());
        let modules = artifacts
            .iter()
            .map(|source| Module {
                source: source.id(),
                qualified_name: "pkg".into(),
            })
            .collect::<Vec<_>>();
        let library = CorpusLibrary {
            corpus: own.id(),
            library: linked.id(),
        };
        let inputs = vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<Module>(&["id"]),
            ValidationInput::of::<CorpusLibrary>(&["id"]),
            ValidationInput::of::<SourceArtifact>(&["id"]),
        ];
        let col = |row, field| ScopeColumn { row, field };
        let common = vec![
            ScopePredicate::Equal(col(2, "id"), col(1, "source")),
            ScopePredicate::QualifiedName {
                wanted: col(0, "path"),
                module: col(1, "qualified_name"),
                leaf: col(2, "path"),
                leaf_match: false,
            },
        ];
        let mut own_predicate = common.clone();
        own_predicate.push(ScopePredicate::Equal(col(2, "input"), col(0, "input")));
        let mut linked_predicate = common;
        linked_predicate.extend([
            ScopePredicate::Equal(col(3, "corpus"), col(0, "input")),
            ScopePredicate::Equal(col(2, "input"), col(3, "library")),
        ]);
        let program = ScopeProgram {
            inputs,
            ports: vec![
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 1,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 2,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 3,
                    virtual_owner: true,
                },
            ],
            rules: vec![
                ScopeRule::Pairs {
                    source: 3,
                    target: 1,
                    rows: vec![3, 1, 0],
                    predicates: own_predicate,
                    source_key: col(0, "id"),
                    target_key: col(1, "id"),
                },
                ScopeRule::Pairs {
                    source: 3,
                    target: 1,
                    rows: vec![3, 1, 0, 2],
                    predicates: linked_predicate,
                    source_key: col(0, "id"),
                    target_key: col(1, "id"),
                },
            ],
        };
        let mut all_artifacts = artifacts.to_vec();
        all_artifacts.push(root.clone());
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let _adapters = budget.reserve("captured-name-fixture", 8192).unwrap();
        let artifact_rows = || {
            all_artifacts
                .iter()
                .map(|r| FiniteScopeRow::of(r).with_text("path", r.path.clone()))
                .collect()
        };
        let finite = FiniteScope::new(
            program.clone(),
            vec![
                artifact_rows(),
                modules
                    .iter()
                    .map(|r| {
                        FiniteScopeRow::of(r).with_text("qualified_name", r.qualified_name.clone())
                    })
                    .collect(),
                vec![FiniteScopeRow::of(&library)],
                artifact_rows(),
            ],
            &owner,
            &budget,
        )
        .unwrap();
        let expected = finite
            .select(
                &[(3, *root.id().bytes())],
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        let session = SessionContext::new();
        for (name, batch) in [
            (
                "name_artifacts",
                SourceArtifact::encode(&all_artifacts).unwrap(),
            ),
            ("name_modules", Module::encode(&modules).unwrap()),
            ("name_libraries", CorpusLibrary::encode(&[library]).unwrap()),
        ] {
            session
                .register_table(
                    name,
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<SourceArtifact>(),
                alias: "name_artifacts".into(),
            },
            ClosureTable {
                relation: Relation::of::<Module>(),
                alias: "name_modules".into(),
            },
            ClosureTable {
                relation: Relation::of::<CorpusLibrary>(),
                alias: "name_libraries".into(),
            },
            ClosureTable {
                relation: Relation::of::<SourceArtifact>(),
                alias: "name_artifacts".into(),
            },
        ];
        let prepared = lower(&program, &tables)
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let selected = prepared
            .batch(
                &[PreparedRoot {
                    table: 3,
                    key: *root.id().bytes(),
                    kind: PreparedRootKind::Virtual,
                }],
                &budget,
            )
            .await
            .unwrap();
        let keys = selected.keys(0, 1).unwrap().collect::<Vec<_>>();
        let expected_keys = expected
            .partition(0)
            .unwrap()
            .iter()
            .filter(|(port, _)| *port == 1)
            .map(|(_, key)| *key)
            .collect::<Vec<_>>();
        assert_eq!(keys, expected_keys);
        let mut accepted = vec![*modules[0].id().bytes(), *modules[1].id().bytes()];
        accepted.sort_unstable();
        assert_eq!(keys, accepted);
        assert!(!keys.contains(modules[2].id().bytes()));
    }
}
#[cfg(test)]
mod canonical_ancestor_controls {
    use super::*;
    use crate::consumed_rows::{PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        Record, Relation, ValidationInput,
        finite_scope::{FiniteScope, FiniteScopeRow},
        input, model,
        resources::ResourceBudget,
        source::*,
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn canonical_ineligible_path_winner_is_chosen_before_nearest_eligibility() {
        let input = input::InputRevision {
            manifest: lctx_model::domain::ContentHash::of(b"canonical"),
        };
        let source = SourceArtifact::from_bytes(
            input.id(),
            "canonical.py".into(),
            b"                                 ",
        )
        .unwrap();
        let outer = Occurrence {
            source: source.id(),
            start: 0,
            end: 30,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let site = Occurrence {
            start: 10,
            end: 20,
            syntax_kind: SyntaxKind::ExprName,
            structural_path: vec![0, 1, 0],
            ..outer.clone()
        };
        let eligible = Occurrence {
            start: 12,
            end: 15,
            syntax_kind: SyntaxKind::StmtAssign,
            structural_path: vec![0, 1],
            ..outer.clone()
        };
        let ineligible = (0..64)
            .map(|start| Occurrence {
                start,
                end: start + 30,
                syntax_kind: SyntaxKind::ExprName,
                ..eligible.clone()
            })
            .find(|candidate| candidate.id() > eligible.id())
            .expect("fixture has a greater canonical ineligible identity");
        let rows = vec![outer.clone(), site.clone(), eligible, ineligible];
        let column = |row, field| ScopeColumn { row, field };
        let program = ScopeProgram {
            inputs: vec![
                ValidationInput::of::<Occurrence>(&["id"]),
                ValidationInput::of::<Occurrence>(&["id"]),
            ],
            ports: vec![
                ScopePort {
                    input: 0,
                    virtual_owner: false,
                },
                ScopePort {
                    input: 1,
                    virtual_owner: true,
                },
            ],
            rules: vec![ScopeRule::NearestPairs {
                source: 1,
                target: 0,
                rows: vec![1, 0, 0],
                rank_rows: 3,
                predicates: vec![
                    ScopePredicate::Equal(column(1, "id"), column(0, "id")),
                    ScopePredicate::PathPrefix {
                        parent: 2,
                        child: 1,
                    },
                    ScopePredicate::CanonicalOccurrence { row: 2 },
                    ScopePredicate::CodeIn(
                        column(2, "syntax_kind"),
                        &[
                            SyntaxKind::StmtFunctionDef as i16,
                            SyntaxKind::StmtAssign as i16,
                        ],
                    ),
                ],
                post: vec![],
                source_key: column(0, "id"),
                target_key: column(2, "id"),
                event_key: column(1, "id"),
                ancestor: 2,
            }],
        };
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let _fixtures = budget.reserve("canonical-ancestor-fixtures", 8192).unwrap();
        let adapter = || {
            rows.iter()
                .map(|r| {
                    FiniteScopeRow::of(r)
                        .with_occurrence(r)
                        .with_code("syntax_kind", r.syntax_kind as i16)
                })
                .collect()
        };
        let finite =
            FiniteScope::new(program.clone(), vec![adapter(), adapter()], &owner, &budget).unwrap();
        let expected = finite
            .select(
                &[(1, *site.id().bytes())],
                &ScopeParameters(vec![]),
                &budget,
            )
            .unwrap();
        assert_eq!(
            expected
                .partition(0)
                .unwrap()
                .iter()
                .filter(|(port, _)| *port == 0)
                .map(|(_, key)| *key)
                .collect::<Vec<_>>(),
            vec![*outer.id().bytes()]
        );
        let session = SessionContext::new();
        let batch = Occurrence::encode(&rows).unwrap();
        session
            .register_table(
                "canonical_ancestors",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<Occurrence>(),
                alias: "canonical_ancestors".into()
            };
            2
        ];
        let edges = lower(&program, &tables)
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let actual = edges
            .batch(
                &[PreparedRoot {
                    table: 1,
                    key: *site.id().bytes(),
                    kind: PreparedRootKind::Virtual,
                }],
                &budget,
            )
            .await
            .unwrap();
        assert_eq!(
            actual.keys(0, 0).unwrap().collect::<Vec<_>>(),
            vec![*outer.id().bytes()]
        );
    }
}

#[cfg(test)]
mod sum_reference_controls {
    use super::*;
    use crate::consumed_rows::{PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{
        Record, Relation, ValidationInput,
        finite_scope::{FiniteScope, FiniteScopeRow},
        model,
        normalized::{normalization_scope_program, relation_normalization::RelationKernel},
        resources::ResourceBudget,
        types::{AnyFlavor, TypeTerm},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn sum_variant_child_matches_native_and_finite_normalization_scope() {
        let owner = model().unwrap();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let fixture_charge = budget.reserve("sum-reference-fixture", 8192).unwrap();
        let child = TypeTerm::None;
        let selected = TypeTerm::TypeOf { target: child.id() };
        let unrelated = TypeTerm::Any {
            flavor: AnyFlavor::Explicit,
        };
        let records = vec![child.clone(), selected.clone(), unrelated.clone()];
        let declared = normalization_scope_program::relation(
            vec![ValidationInput::of::<TypeTerm>(&["id"])],
            &[Relation::of::<TypeTerm>()],
            RelationKernel::Type,
            &budget,
        )
        .unwrap();
        let root = declared.entity_roots()[0].1;
        assert_eq!(root, 1);
        assert_eq!(declared.program().inputs.len(), 2);
        assert!(declared.program().ports[root].virtual_owner);
        let rows = declared
            .program()
            .inputs
            .iter()
            .map(|input| {
                assert_eq!(input.type_id(), std::any::TypeId::of::<TypeTerm>());
                records.iter().map(FiniteScopeRow::of).collect()
            })
            .collect();
        let finite = FiniteScope::new(declared.program().clone(), rows, &owner, &budget).unwrap();
        let parameters = ScopeParameters(vec![]);
        let expected = finite
            .select(&[(root, *selected.id().bytes())], &parameters, &budget)
            .unwrap();

        // Both declarations bind the same immutable relation. The second is a private root
        // namespace, rather than an absent copy of the physical TypeTerm inventory.
        let session = SessionContext::new();
        let batch = TypeTerm::encode(&records).unwrap();
        session
            .register_table(
                "sum_terms",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<TypeTerm>(),
                alias: "sum_terms".into()
            };
            2
        ];
        let plan = lower_compiled(
            compile(declared.program(), &owner, &budget, None).unwrap(),
            &tables,
            &parameters,
            &budget,
        )
        .unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        drop(plan);
        let actual = edges
            .batch(
                &[PreparedRoot {
                    table: root,
                    key: *selected.id().bytes(),
                    kind: PreparedRootKind::Virtual,
                }],
                &budget,
            )
            .await
            .unwrap();

        let mut physical = vec![*child.id().bytes(), *selected.id().bytes()];
        physical.sort_unstable();
        let known = physical
            .iter()
            .map(|key| (0, *key))
            .chain(std::iter::once((root, *selected.id().bytes())))
            .collect::<Vec<_>>();
        assert_eq!(expected.partition(0).unwrap(), known.as_slice());
        assert_eq!(actual.keys(0, 0).unwrap().collect::<Vec<_>>(), physical);
        assert_eq!(
            actual.keys(0, root).unwrap().collect::<Vec<_>>(),
            vec![*selected.id().bytes()]
        );
        assert!(
            !actual
                .keys(0, 0)
                .unwrap()
                .any(|key| key == *unrelated.id().bytes())
        );
        assert_eq!(actual.outcomes(), expected.outcomes());
        drop(actual);
        drop(edges);
        drop(expected);
        drop(finite);
        drop(declared);
        drop(session);
        drop(known);
        drop(physical);
        drop(tables);
        drop(records);
        drop(parameters);
        drop(fixture_charge);
        assert_eq!(budget.reserved(), 0);
    }
}
