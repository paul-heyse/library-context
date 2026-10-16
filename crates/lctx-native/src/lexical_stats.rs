//! Immutable exact-view lexical statistics. FULLTEXT is nomination, never corpus authority.
use crate::surrealdb::types::{Object, RecordId, RecordIdKey, SurrealValue, Value};
use crate::{
    Loader, NativeReader,
    derived_search::selected_occurrences,
    ordered_rows::{PreparedRows, SortedRows},
    prepared::PreparedQuery,
};
use lctx_model::domain::{
    ContentHash, KeySink, ModelError,
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::collections::BTreeMap;
const TABLES: [&str; 5] = [
    "lexical_document",
    "lexical_member",
    "lexical_term",
    "lexical_corpus",
    "lexical_posting",
];
const FAMILIES: [&str; 4] = [
    "search_api_options",
    "search_documentation_deployment",
    "search_scenario",
    "search_source",
];
pub fn analyzer_identity() -> ContentHash {
    ContentHash::of(b"lctx-discovery/v2:class,camel;lowercase")
}
fn analyzer_definition_matches(actual: &str) -> Result<(), ModelError> {
    use crate::surrealdb::types::ToSql;
    use surrealdb_sql::{Expr, TopLevelExpr, statements::DefineStatement};
    // The generated native declaration is the sole policy owner. Normalization
    // uses the same parser/printer as installation, rather than another SQL string.
    let expected = surrealdb_syn::parse(&crate::materialization::native_definitions())
        .map_err(ModelError::codec)?
        .expressions
        .into_iter()
        .find(|expression| {
            matches!(expression,
            TopLevelExpr::Expr(Expr::Define(definition)) if matches!(definition.as_ref(),
                DefineStatement::Analyzer(analyzer) if analyzer.name.to_sql() == "lctx_discovery"))
        })
        .ok_or(ModelError::Schema("compiled discovery analyzer definition"))?;
    let actual = surrealdb_syn::parse(actual)
        .map_err(ModelError::codec)?
        .expressions;
    let [actual] = actual.as_slice() else {
        return Err(ModelError::Schema("actual discovery analyzer definition"));
    };
    if expected.to_sql() != actual.to_sql() {
        return Err(ModelError::Conflict(
            "prepared lexical actual analyzer changed",
        ));
    }
    Ok(())
}
/// Retained native publication can be called independently of the publisher's
/// full definition check. Its mutable executable analyzer premise is checked here.
pub(crate) async fn check_analyzer(loader: &Loader) -> Result<(), ModelError> {
    let reader = loader.reader();
    reader.ensure_scope_live()?;
    // One database-level metadata response; no field/index/table traversal.
    let info: Object = reader
        .query_native("INFO FOR DB", surrealdb::types::Variables::new())
        .await?;
    let Some(Value::Object(analyzers)) = info.get("analyzers") else {
        return Err(ModelError::Schema("actual analyzer inventory"));
    };
    let Some(Value::String(actual)) = analyzers.get("lctx_discovery") else {
        return Err(ModelError::Conflict(
            "prepared lexical actual analyzer missing",
        ));
    };
    analyzer_definition_matches(actual.as_str())?;
    reader.ensure_scope_live()
}
pub fn scope_identity<Context>(reader: &NativeReader<Context>) -> Result<ContentHash, ModelError> {
    let vars = reader.view_bindings();
    let Some(Value::Array(views)) = vars.get("lctx_views") else {
        return Err(ModelError::Conflict(
            "lexical statistics require exact view scope",
        ));
    };
    let mut ids = views
        .iter()
        .map(|v| match v {
            Value::RecordId(id) if id.table.as_str() == "compiler_view" => match &id.key {
                RecordIdKey::String(key) => Ok(key.clone()),
                _ => Err(ModelError::Schema("lexical view identity")),
            },
            _ => Err(ModelError::Schema("lexical view identity")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    ids.dedup();
    let mut key = KeySink::new("native-lexical-scope/v1;distinct-query-terms;view-family-bm25");
    key.part(b"derivation", &reader.derivation()?.0);
    key.part(b"analyzer", &analyzer_identity().0);
    for id in ids {
        key.part(b"view", id.as_bytes());
    }
    Ok(key.finish())
}
pub fn corpus_id(scope: ContentHash, family: i16) -> RecordId {
    RecordId::new("lexical_corpus", format!("{}:{family}", scope.hex()))
}
pub fn definitions() -> String {
    let mut sql = String::new();
    for (table, fields) in [
        ("lexical_document", vec![("length", "int")]),
        (
            "lexical_posting",
            vec![
                ("in", "record<lexical_document>"),
                ("term", "string"),
                ("tf", "int"),
            ],
        ),
        (
            "lexical_member",
            vec![
                ("scope", "string"),
                ("family", "int"),
                (
                    "source",
                    "record<search_api_options|search_documentation_deployment|search_scenario|search_source>",
                ),
                ("in", "record<lexical_document>"),
                ("out", "record<lexical_corpus>"),
            ],
        ),
        (
            "lexical_term",
            vec![
                ("scope", "string"),
                ("family", "int"),
                ("term", "string"),
                ("df", "int"),
            ],
        ),
        (
            "lexical_corpus",
            vec![
                ("scope", "string"),
                ("family", "int"),
                ("documents", "int"),
                ("total_length", "int"),
            ],
        ),
    ] {
        sql.push_str(&format!(
            "DEFINE TABLE {table} SCHEMAFULL;\nDEFINE FIELD derivation ON {table} TYPE string;\n"
        ));
        for (name, kind) in fields {
            sql.push_str(&format!("DEFINE FIELD {name} ON {table} TYPE {kind};\n"));
        }
    }
    sql.push_str("DEFINE INDEX document_term ON lexical_posting FIELDS in,term UNIQUE;\nDEFINE INDEX scoped_document ON lexical_member FIELDS scope,family,source UNIQUE;\nDEFINE INDEX scoped_term ON lexical_term FIELDS scope,family,term UNIQUE;\nDEFINE INDEX scoped_corpus ON lexical_corpus FIELDS scope,family UNIQUE;\n");
    sql
}
fn value<T: serde::Serialize>(v: T) -> Result<Value, ModelError> {
    crate::loader::json_value(serde_json::to_value(v).map_err(ModelError::codec)?)
}
fn row(id: RecordId) -> Object {
    let mut row = Object::new();
    row.insert("id", id);
    row
}
fn object(row: Value) -> Result<Object, ModelError> {
    if let Value::Object(row) = row {
        Ok(row)
    } else {
        Err(ModelError::Schema("lexical statistics row"))
    }
}
fn string(row: &Object, key: &str) -> Result<String, ModelError> {
    String::from_value(
        row.get(key)
            .cloned()
            .ok_or(ModelError::Schema("lexical statistics field"))?,
    )
    .map_err(ModelError::codec)
}

fn increment(n: u64, by: u64) -> Result<u64, ModelError> {
    n.checked_add(by)
        .ok_or(ModelError::Schema("lexical statistics overflow"))
}
pub(crate) struct Expected {
    rows: [PreparedRows; 5],
    scope: ContentHash,
    budget: ResourceBudget,
}
#[derive(Clone, Copy)]
enum DocumentProjection {
    AnalyzedSource,
    StoredDocument,
    Postings,
}
// A complete actual nomination stream feeds a charged identity spool, followed by
// bounded point batches. Duplicate occurrences never multiply corpus documents.
fn documents<Context>(
    reader: &NativeReader<Context>,
    mut nominees: crate::reader::NativeRows,
    projection: DocumentProjection,
    budget: &ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    let reader = reader.transport_reader();
    let budget = budget.clone();
    crate::reader::NativeRows::owned(budget.clone(), move |sender| async move {
        let mut ids = SortedRows::with_budget(&budget)?;
        // Posting identities hash document and term, so batches sorted by document
        // are not globally sorted by posting identity. Reconcile through one spillable
        // identity spool, never by retaining an unbounded native result vector.
        let mut postings = matches!(projection, DocumentProjection::Postings)
            .then(|| SortedRows::with_budget(&budget))
            .transpose()?;
        let result = async {
            while let Some(value) = nominees.next().await? {
                if sender.is_closed() {
                    break;
                }
                let value = object(value)?;
                let source = RecordId::from_value(
                    value
                        .get("in")
                        .cloned()
                        .ok_or(ModelError::Schema("lexical nominee source"))?,
                )
                .map_err(ModelError::codec)?;
                ids.push(Value::Object(row(source)))?;
            }
            Ok(())
        }
        .await;
        let mut terminal = Completion::default();
        terminal.step(
            "lexical nomination drainage",
            nominees.drain_transport().await,
        );
        complete(result, terminal)?;
        let mut ids = ids.finish()?;
        loop {
            if sender.is_closed() {
                break;
            }
            let mut batch = Vec::new();
            while batch.len() < 128 {
                let Some(value) = ids.next_row()? else {
                    break;
                };
                batch.push(
                    RecordId::from_value(
                        object(value)?
                            .get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("lexical source identity"))?,
                    )
                    .map_err(ModelError::codec)?,
                );
            }
            if batch.is_empty() {
                break;
            }
            let mut vars = surrealdb::types::Variables::new();
            vars.insert("documents", batch);
            let sql = match projection {
                DocumentProjection::AnalyzedSource => {
                    "SELECT id,text,search::analyze('lctx_discovery',text) AS tokens FROM $documents ORDER BY id"
                }
                DocumentProjection::StoredDocument => "SELECT * FROM $documents ORDER BY id",
                DocumentProjection::Postings => {
                    "SELECT * FROM lexical_posting WITH INDEX document_term WHERE in IN $documents ORDER BY id"
                }
            };
            let mut docs =
                reader.stream_prepared(PreparedQuery::new(vars, vec![], vec![sql.into()])?)?;
            let result = async {
                while let Some(value) = docs.next().await? {
                    if sender.is_closed() {
                        break;
                    }
                    if let Some(postings) = &mut postings {
                        postings.push(value)?;
                    } else if !sender.send(value).await? {
                        break;
                    }
                }
                Ok(())
            }
            .await;
            let mut terminal = Completion::default();
            terminal.step(
                "lexical source batch drainage",
                docs.drain_transport().await,
            );
            complete(result, terminal)?;
        }
        if let Some(postings) = postings {
            let mut postings = postings.finish()?;
            while let Some(value) = postings.next_row()? {
                if !sender.send(value).await? {
                    break;
                }
            }
        }
        Ok(())
    })
}
async fn prepare(loader: &Loader) -> Result<Expected, ModelError> {
    let reader = loader.reader();
    let scope = scope_identity(&reader)?;
    let namespace = reader.derivation()?;
    let budget = loader.read_budget()?;
    let mut expected = (0..5)
        .map(|_| SortedRows::with_budget(&budget))
        .collect::<Result<Vec<_>, _>>()?;
    for (family, _table) in FAMILIES.iter().enumerate() {
        let family = family as i16;
        let mut count = 0u64;
        let mut total = 0u64;
        let mut terms = SortedRows::with_budget(&budget)?;
        let mut vars = surrealdb::types::Variables::new();
        vars.insert("family", family);
        let nominees = selected_occurrences(
            &reader,
            "lex_occurs",
            "eligible=true AND family=$family",
            vars,
            &budget,
        )?;
        let mut docs = documents(
            &reader,
            nominees,
            DocumentProjection::AnalyzedSource,
            &budget,
        )?;
        let lowered: Result<(), ModelError> = async {
            while let Some(doc) = docs.next().await? {
                let charge = budget.reserve(
                    "lexical statistics document",
                    serde_json::to_vec(&doc)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(12),
                )?;
                let doc = object(doc)?;
                let id = RecordId::from_value(
                    doc.get("id")
                        .cloned()
                        .ok_or(ModelError::Schema("lexical document id"))?,
                )
                .map_err(ModelError::codec)?;
                let text = string(&doc, "text")?;
                let tokens = Vec::<String>::from_value(
                    doc.get("tokens")
                        .cloned()
                        .ok_or(ModelError::Schema("lexical analyzed tokens"))?,
                )
                .map_err(ModelError::codec)?;
                let mut frequencies = BTreeMap::<String, u64>::new();
                for token in &tokens {
                    let n = frequencies.entry(token.clone()).or_default();
                    *n = increment(*n, 1)?;
                }
                let mut key = KeySink::new("native-lexical-document/v2;postings");
                key.part(b"analyzer", &analyzer_identity().0);
                key.part(b"text", text.as_bytes());
                let token_id =
                    crate::derivation::address(namespace, "lexical_document", &key.finish().0);
                let mut token_row = row(token_id.clone());
                token_row.insert("derivation", namespace.hex());
                token_row.insert("length", tokens.len());
                expected[0].push(Value::Object(token_row))?;
                // One immutable posting per analyzed document and distinct term; neither
                // selected scope nor source occurrence multiplies these shared rows.
                for (term, tf) in &frequencies {
                    expected[4].push(posting(namespace, &token_id, term, *tf)?)?;
                }
                let mut member_key = KeySink::new("native-lexical-member/v1");
                member_key.part(b"scope", &scope.0);
                member_key.part(b"family", &family.to_le_bytes());
                member_key.part(
                    b"document",
                    &serde_json::to_vec(&id).map_err(ModelError::codec)?,
                );
                let mut member = row(crate::derivation::address(
                    namespace,
                    "lexical_member",
                    &member_key.finish().0,
                ));
                member.insert("derivation", namespace.hex());
                member.insert("scope", scope.hex());
                member.insert("family", family);
                member.insert("source", id.clone());
                member.insert("in", token_id);
                member.insert("out", corpus_id(scope, family));
                expected[1].push(Value::Object(member))?;
                for term in frequencies.keys() {
                    let mut occurrence = row(RecordId::new(
                        "lexical_sort",
                        format!(
                            "{}:{}",
                            hex::encode(term.as_bytes()),
                            hex::encode(serde_json::to_vec(&id).map_err(ModelError::codec)?)
                        ),
                    ));
                    occurrence.insert("term", term.clone());
                    terms.push(Value::Object(occurrence))?;
                }
                count = increment(count, 1)?;
                total = increment(total, tokens.len() as u64)?;
                drop(charge);
            }
            Ok(())
        }
        .await;
        let mut terminal = Completion::default();
        terminal.step(
            "lexical analyzed-document stream drainage",
            docs.drain_transport().await,
        );
        complete(lowered, terminal)?;
        let mut ordered = terms.finish()?;
        let mut current: Option<String> = None;
        let mut df = 0u64;
        while let Some(term) = ordered.next_row()? {
            let term = string(&object(term)?, "term")?;
            if current.as_ref().is_some_and(|old| old != &term) {
                emit_term(
                    &mut expected[2],
                    namespace,
                    scope,
                    family,
                    current.take().unwrap(),
                    df,
                )?;
                df = 0;
            }
            current = Some(term);
            df = increment(df, 1)?;
        }
        if let Some(term) = current {
            emit_term(&mut expected[2], namespace, scope, family, term, df)?;
        }
        let mut corpus = row(corpus_id(scope, family));
        corpus.insert("derivation", namespace.hex());
        corpus.insert("scope", scope.hex());
        corpus.insert("family", family);
        corpus.insert("documents", value(count)?);
        corpus.insert("total_length", value(total)?);
        expected[3].push(Value::Object(corpus))?;
    }
    Ok(Expected {
        rows: expected
            .into_iter()
            .map(|rows| rows.finish().map(|rows| rows.into_prepared()))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| ModelError::Schema("lexical prepared family count"))?,
        scope,
        budget,
    })
}
fn posting(
    namespace: ContentHash,
    document: &RecordId,
    term: &str,
    tf: u64,
) -> Result<Value, ModelError> {
    let mut key = KeySink::new("native-lexical-posting/v1");
    key.part(
        b"document",
        &serde_json::to_vec(document).map_err(ModelError::codec)?,
    );
    key.part(b"term", term.as_bytes());
    let mut result = row(crate::derivation::address(
        namespace,
        "lexical_posting",
        &key.finish().0,
    ));
    result.insert("derivation", namespace.hex());
    result.insert("in", document.clone());
    result.insert("term", term.to_owned());
    result.insert("tf", value(tf)?);
    Ok(Value::Object(result))
}
/// Candidate and term predicates are both compound-index dimensions. Shared by
/// scoring and actual EXPLAIN controls so a different test query cannot qualify it.
pub fn posting_selection_sql() -> &'static str {
    "SELECT in,term,tf FROM lexical_posting WITH INDEX document_term WHERE in IN $token_documents AND term IN $query_terms ORDER BY in,term"
}
/// Exact selected-corpus BM25 over bounded candidate documents. The caller binds
/// corpus_id, lexical_scope, family, query, and the distinct source document window.
/// Physical postings alter access only; analyzer and scoring policy are unchanged.
pub fn scoring_sql() -> String {
    format!(
        r#"RETURN {{
 LET $corpus=(SELECT * FROM ONLY $corpus_id);
 IF $corpus=NONE {{ THROW 'missing frozen lexical corpus'; }};
 LET $query_terms=array::distinct(search::analyze('lctx_discovery',$query));
 LET $df=object::from_entries((SELECT term,df FROM lexical_term WITH INDEX scoped_term WHERE scope=$lexical_scope AND family=$family AND term IN $query_terms).map(|$v|[$v.term,$v.df]));
 LET $members=SELECT source,in,in.length AS length FROM lexical_member WITH INDEX scoped_document WHERE scope=$lexical_scope AND family=$family AND source IN $documents;
 IF array::len($members)!=array::len($documents) {{ THROW 'missing frozen lexical document'; }};
 LET $token_documents=array::distinct($members.in);
 LET $lengths=object::from_entries($members.map(|$member| [<string>$member.in,$member.length]));
 LET $postings=({});
 // Matched terms are ordered as the previous BTreeMap-backed term arrays,
 // preserving floating-point accumulation order as well as scoring policy.
 // Aggregate the matched posting stream once. Source documents with equal
 // analyzed content reuse its score; no document rescans the whole posting set.
 LET $scores=object::from_entries((SELECT in,math::sum(math::max([0.0,math::ln(($corpus.documents-($df[term] ?? 0)+0.5)/(($df[term] ?? 0)+0.5))]) * tf * 2.5 / (tf+1.5*(0.25+0.75*$lengths[<string>in]/((<float>$corpus.total_length/$corpus.documents))))) AS score FROM $postings GROUP BY in).map(|$v|[<string>$v.in,$v.score]));
 LET $ranked=$members.map(|$member| {{ id:$member.source,score:$scores[<string>$member.in] }});
 RETURN SELECT id,score FROM $ranked WHERE score!=NONE;
}};"#,
        posting_selection_sql()
    )
}
fn emit_term(
    rows: &mut SortedRows,
    namespace: ContentHash,
    scope: ContentHash,
    family: i16,
    term: String,
    df: u64,
) -> Result<(), ModelError> {
    let mut key = KeySink::new("native-lexical-term/v1");
    key.part(b"scope", &scope.0);
    key.part(b"family", &family.to_le_bytes());
    key.part(b"term", term.as_bytes());
    let mut row = row(crate::derivation::address(
        namespace,
        "lexical_term",
        &key.finish().0,
    ));
    row.insert("derivation", namespace.hex());
    row.insert("scope", scope.hex());
    row.insert("family", family);
    row.insert("term", term);
    row.insert("df", value(df)?);
    rows.push(Value::Object(row))
}
async fn check(loader: &Loader, expected: &Expected) -> Result<(), ModelError> {
    let reader = loader.reader();
    for (index, table) in TABLES.iter().enumerate() {
        let mut vars = reader.view_bindings();
        vars.insert("scope", expected.scope.hex());
        // Charge and own the complete expected carrier before any native transport
        // is admitted, including nomination's independently owned document driver.
        let mut actual = crate::derived_search::retain_expected_until_drained(
            expected.rows.iter().cloned(),
            &expected.budget,
            || {
                if index == 0 || index == 4 {
                    let nominees = reader.stream_prepared(PreparedQuery::new(vars, vec![], vec!["SELECT in FROM lexical_member WITH INDEX scoped_document WHERE scope=$scope ORDER BY id".into()])?)?;
                    documents(
                        &reader,
                        nominees,
                        if index == 0 {
                            DocumentProjection::StoredDocument
                        } else {
                            DocumentProjection::Postings
                        },
                        &expected.budget,
                    )
                } else {
                    let scope_index = match *table {
                        "lexical_member" => "scoped_document",
                        "lexical_term" => "scoped_term",
                        "lexical_corpus" => "scoped_corpus",
                        _ => return Err(ModelError::Schema("lexical scope index")),
                    };
                    reader.stream_prepared(PreparedQuery::new(vars, vec![], vec![format!("SELECT * FROM {table} WITH INDEX {scope_index} WHERE scope=$scope ORDER BY id")])?)
                }
            },
        )?;
        let compared = async {
            let mut rows = expected.rows[index].cursor()?;
            loop {
                let a = actual.next().await?;
                let e = rows.next_row()?;
                if a != e {
                    return Err(ModelError::Conflict("frozen lexical statistics differ"));
                }
                if a.is_none() {
                    return Ok(());
                }
            }
        }
        .await;
        let mut terminal = Completion::default();
        terminal.step(
            "lexical actual-statistics stream drainage",
            actual.drain_transport().await,
        );
        complete(compared, terminal)?;
    }
    Ok(())
}
/// Construct once for a complete exact view; guarded immutable ingress rejects changed content.
pub async fn materialize(loader: &Loader) -> Result<(), ModelError> {
    materialize_prepared(loader).await.map(drop)
}
pub(crate) async fn materialize_prepared(loader: &Loader) -> Result<Expected, ModelError> {
    if loader.attempt_id().is_none() {
        return Err(ModelError::Conflict(
            "lexical construction requires an owned attempt",
        ));
    }
    check_analyzer(loader).await?;
    let expected = prepare(loader).await?;
    let mut charge = expected
        .budget
        .reserve("lexical statistics write window", 0)?;
    // Membership retention links require both immutable endpoints to exist.
    for index in [0usize, 4, 3, 2, 1] {
        let table = TABLES[index];
        let mut batch = Vec::new();
        let mut bytes = 0usize;
        let mut rows = expected.rows[index].cursor()?;
        while let Some(row) = rows.next_row()? {
            let size = serde_json::to_vec(&row)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(12);
            if !batch.is_empty()
                && (batch.len() == 128 || bytes.saturating_add(size) > 8 * 1024 * 1024)
            {
                loader
                    .insert(table, std::mem::take(&mut batch), false)
                    .await?;
                bytes = 0;
                charge.try_resize(0)?;
            }
            bytes = bytes
                .checked_add(size)
                .ok_or(ModelError::Schema("lexical statistics write bytes"))?;
            charge.try_resize(bytes)?;
            batch.push(row);
        }
        loader.insert(table, batch, false).await?;
        charge.try_resize(0)?;
    }
    check(loader, &expected).await?;
    check_analyzer(loader).await?;
    Ok(expected)
}
/// Independent native analysis and actual-content comparison, with no writes or marker shortcuts.
pub async fn reconcile(loader: &Loader) -> Result<(), ModelError> {
    reconcile_owned(loader, None).await
}
pub(crate) async fn reconcile_owned(
    loader: &Loader,
    inventory: Option<&mut SortedRows>,
) -> Result<(), ModelError> {
    let expected = prepare(loader).await?;
    reconcile_prepared(loader, &expected, inventory).await
}
pub(crate) async fn reconcile_prepared(
    loader: &Loader,
    expected: &Expected,
    inventory: Option<&mut SortedRows>,
) -> Result<(), ModelError> {
    if !expected.budget.shares_pool(&loader.read_budget()?)
        || expected.scope != scope_identity(&loader.reader())?
    {
        return Err(ModelError::Conflict("prepared lexical scope/pool changed"));
    }
    check_analyzer(loader).await?;
    check(loader, expected).await?;
    if let Some(inventory) = inventory {
        for prepared in &expected.rows {
            let mut rows = prepared.cursor()?;
            while let Some(row) = rows.next_row()? {
                crate::control::staging::remember_expected_child(inventory, &row)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postings_are_shared_by_document_and_term_and_preserve_exact_frequency() {
        let namespace = ContentHash::of(b"one realization");
        let document = RecordId::new("lexical_document", "one");
        let first = posting(namespace, &document, "needle", 3).unwrap();
        assert_eq!(first, posting(namespace, &document, "needle", 3).unwrap());
        let changed = posting(namespace, &document, "needle", 4).unwrap();
        assert_eq!(
            first.as_object().unwrap().get("id"),
            changed.as_object().unwrap().get("id")
        );
        assert_ne!(
            first, changed,
            "immutable equality detects changed term frequency"
        );
        assert_ne!(
            first.as_object().unwrap().get("id"),
            posting(namespace, &document, "other", 3)
                .unwrap()
                .as_object()
                .unwrap()
                .get("id")
        );
        assert_ne!(
            first.as_object().unwrap().get("id"),
            posting(
                namespace,
                &RecordId::new("lexical_document", "two"),
                "needle",
                3
            )
            .unwrap()
            .as_object()
            .unwrap()
            .get("id")
        );
        assert_eq!(
            first.as_object().unwrap().get("tf"),
            Some(&value(3u64).unwrap())
        );
    }
    #[test]
    fn repeated_postings_coalesce_but_changed_frequency_refuses_without_leaking_spill_owners() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let namespace = ContentHash::of(b"one realization");
        let document = RecordId::new("lexical_document", "one");
        {
            let mut rows = SortedRows::with_budget(&budget).unwrap();
            rows.push(posting(namespace, &document, "needle", 3).unwrap())
                .unwrap();
            rows.push(posting(namespace, &document, "needle", 3).unwrap())
                .unwrap();
            let mut rows = rows.finish().unwrap();
            assert!(rows.next_row().unwrap().is_some());
            assert!(rows.next_row().unwrap().is_none());
        }
        assert_eq!(budget.reserved(), 0);
        {
            let mut rows = SortedRows::with_budget(&budget).unwrap();
            rows.push(posting(namespace, &document, "needle", 3).unwrap())
                .unwrap();
            let changed = rows.push(posting(namespace, &document, "needle", 4).unwrap());
            assert!(changed.is_err() || rows.finish().is_err());
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn posting_and_selected_corpus_scoring_programs_parse_at_the_pin() {
        surrealdb_syn::parse(&definitions()).unwrap();
        surrealdb_syn::parse(posting_selection_sql()).unwrap();
        surrealdb_syn::parse(&scoring_sql()).unwrap();
        assert!(!definitions().contains("terms.*"));
        assert!(crate::derivation::FAMILIES.contains(&"lexical_posting"));
    }
    #[test]
    fn retained_analyzer_policy_uses_compiled_native_declaration() {
        let actual = "DEFINE ANALYZER lctx_discovery TOKENIZERS class, camel FILTERS lowercase;";
        analyzer_definition_matches(actual).unwrap();
        assert!(
            analyzer_definition_matches(
                "DEFINE ANALYZER lctx_discovery TOKENIZERS blank FILTERS lowercase;"
            )
            .is_err()
        );
        assert!(
            analyzer_definition_matches(
                "DEFINE ANALYZER another TOKENIZERS class,camel FILTERS lowercase;"
            )
            .is_err()
        );
    }
    #[test]
    fn scope_inventory_is_order_independent_and_binds_actual_views() {
        let client = std::sync::Arc::new(crate::surrealdb::Surreal::init());
        let a = ContentHash::of(b"a");
        let b = ContentHash::of(b"b");
        let realization = ContentHash::of(b"native realization");
        let unbound = NativeReader::for_views(client.clone(), vec![a, b]);
        assert!(
            scope_identity(&unbound).is_err(),
            "derived scope needs exact realization"
        );
        let first =
            NativeReader::for_views(client.clone(), vec![a, b, a]).with_derivation(realization);
        let reordered =
            NativeReader::for_views(client.clone(), vec![b, a]).with_derivation(realization);
        let different =
            NativeReader::for_views(client.clone(), vec![a]).with_derivation(realization);
        let changed = NativeReader::for_views(client, vec![a, b])
            .with_derivation(ContentHash::of(b"another realization"));
        assert_eq!(
            scope_identity(&first).unwrap(),
            scope_identity(&reordered).unwrap()
        );
        assert_ne!(
            scope_identity(&first).unwrap(),
            scope_identity(&different).unwrap()
        );
        assert_ne!(
            scope_identity(&first).unwrap(),
            scope_identity(&changed).unwrap()
        );
    }
}
