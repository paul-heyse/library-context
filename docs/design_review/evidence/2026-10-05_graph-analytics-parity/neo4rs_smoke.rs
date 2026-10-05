//! neo4rs 0.9.0-rc.10 smoke test against neo4j:2026.09.0-community-trixie with GDS 2026.09.0.
//! Copied to build/review-probes/graph-analytics-parity/neo4rs-smoke/src/main.rs by run_all.sh.
//! Checks: Bolt negotiation/connect, a parameterised UNWIND write, a read with typed row access,
//! and a GDS projection + pageRank.stream + drop. Prints one line per step: PASS/FAIL <step> <detail>.
use neo4rs::{query, ConfigBuilder, Graph};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let uri = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:17687".into());
    let pw = std::env::var("NEO4J_PROBE_PASSWORD").expect("NEO4J_PROBE_PASSWORD");
    let config = ConfigBuilder::default()
        .uri(uri)
        .user("neo4j")
        .password(pw)
        .build()
        .expect("config");
    let graph = match Graph::connect(config) {
        Ok(g) => g,
        Err(e) => {
            println!("FAIL connect {e:?}");
            return;
        }
    };
    let steps: Vec<(&str, neo4rs::Query)> = vec![
        (
            "components",
            query("CALL dbms.components() YIELD name, versions, edition RETURN name + ' ' + versions[0] + ' ' + edition AS s"),
        ),
        ("cleanup", query("MATCH (n:Smoke) DETACH DELETE n RETURN count(*) AS s")),
        (
            "unwind_write",
            query(
                "UNWIND $rows AS r MERGE (a:Smoke {sid: r.s}) MERGE (b:Smoke {sid: r.t}) \
                 CREATE (a)-[:CALLS {w: r.w}]->(b) RETURN count(*) AS s",
            )
            .param(
                "rows",
                vec![
                    std::collections::HashMap::from([("s", "a".to_string()), ("t", "b".to_string()), ("w", "1".to_string())]),
                    std::collections::HashMap::from([("s", "b".to_string()), ("t", "c".to_string()), ("w", "1".to_string())]),
                    std::collections::HashMap::from([("s", "c".to_string()), ("t", "a".to_string()), ("w", "1".to_string())]),
                    std::collections::HashMap::from([("s", "c".to_string()), ("t", "d".to_string()), ("w", "1".to_string())]),
                ],
            ),
        ),
        ("drop_old", query("CALL gds.graph.drop('smoke', false) YIELD graphName RETURN count(*) AS s")),
        (
            "gds_project",
            query("CALL gds.graph.project('smoke', 'Smoke', 'CALLS') YIELD nodeCount, relationshipCount RETURN toString(nodeCount) + '/' + toString(relationshipCount) AS s"),
        ),
        (
            "gds_pagerank",
            query(
                "CALL gds.pageRank.stream('smoke', {concurrency: 1}) YIELD nodeId, score \
                 WITH gds.util.asNode(nodeId).sid AS sid, score ORDER BY sid \
                 RETURN reduce(acc = '', x IN collect(sid + '=' + toString(round(score, 4))) | acc + x + ' ') AS s",
            ),
        ),
        ("gds_drop", query("CALL gds.graph.drop('smoke') YIELD graphName RETURN graphName AS s")),
        ("cleanup_end", query("MATCH (n:Smoke) DETACH DELETE n RETURN count(*) AS s")),
    ];
    for (name, q) in steps {
        match graph.execute(q).await {
            Ok(mut rows) => {
                let mut vals = Vec::new();
                loop {
                    match rows.next().await {
                        Ok(Some(row)) => vals.push(
                            row.get::<String>("s")
                                .or_else(|_| row.get::<i64>("s").map(|v| v.to_string()))
                                .unwrap_or_else(|e| format!("<get error {e:?}>")),
                        ),
                        Ok(None) => break,
                        Err(e) => {
                            vals.push(format!("<stream error {e:?}>"));
                            break;
                        }
                    }
                }
                let bad = vals.iter().any(|v| v.starts_with('<'));
                println!("{} {name} {:?}", if bad { "FAIL" } else { "PASS" }, vals);
            }
            Err(e) => println!("FAIL {name} {e:?}"),
        }
    }
}
