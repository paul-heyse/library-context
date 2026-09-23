//! The release proposal of `lctx library init` (ADR-0013): the requested distribution plus every
//! installed distribution built from the same source repository. Only repository roots
//! (`host/owner/repo`) under the labels that name a source count; funding and sponsor links,
//! which unrelated projects by one author share, never do.

use std::collections::BTreeSet;

const SOURCE_LABELS: &[&str] = &[
    "source",
    "source code",
    "repository",
    "code",
    "homepage",
    "home",
    "github",
    "gitlab",
];

/// `github.com/owner/repo` for a repository URL; `None` for anything else (sponsors, orgs,
/// documentation hosts).
fn repository_root(url: &str) -> Option<String> {
    let url = url.trim().to_ascii_lowercase();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    let host = parts.next()?.trim_start_matches("www.");
    if host != "github.com" && host != "gitlab.com" {
        return None;
    }
    let owner = parts.next()?;
    let repo = parts.next()?.trim_end_matches(".git");
    if matches!(owner, "sponsors" | "orgs" | "users") || repo.is_empty() {
        return None;
    }
    Some(format!("{host}/{owner}/{repo}"))
}

/// The repository roots a distribution's METADATA names under source labels.
pub fn repository_roots(metadata: &str) -> BTreeSet<String> {
    metadata
        .lines()
        .take_while(|l| !l.is_empty())
        .filter_map(|l| {
            if let Some(v) = l.strip_prefix("Project-URL:") {
                let (label, url) = v.split_once(',')?;
                let label = label.trim().to_ascii_lowercase();
                SOURCE_LABELS.contains(&label.as_str()).then_some(url)
            } else {
                l.strip_prefix("Home-page:")
            }
        })
        .filter_map(repository_root)
        .collect()
}

/// The proposed release: `requested` plus every distribution sharing one of its repository
/// roots, sorted. The flag says whether `requested` named any repository at all.
pub fn propose(requested: &str, installed: &[(String, String)]) -> (Vec<String>, bool) {
    let own: BTreeSet<String> = installed
        .iter()
        .find(|(name, _)| name == requested)
        .map(|(_, metadata)| repository_roots(metadata))
        .unwrap_or_default();
    let mut release: Vec<String> = installed
        .iter()
        .filter(|(name, metadata)| {
            name == requested || (!own.is_empty() && !repository_roots(metadata).is_disjoint(&own))
        })
        .map(|(name, _)| name.clone())
        .collect();
    release.sort();
    release.dedup();
    if release.is_empty() {
        release.push(requested.to_owned());
    }
    (release, !own.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(urls: &[(&str, &str)]) -> String {
        let mut s = "Metadata-Version: 2.4\nName: x\n".to_owned();
        for (label, url) in urls {
            s.push_str(&format!("Project-URL: {label}, {url}\n"));
        }
        s.push_str("\nbody text with https://github.com/elsewhere/repo\n");
        s
    }

    #[test]
    fn the_fastmcp_distributions_share_one_repository() {
        let fastmcp = meta(&[
            ("Homepage", "https://gofastmcp.com"),
            ("Repository", "https://github.com/PrefectHQ/fastmcp"),
        ]);
        let installed = vec![
            ("fastmcp".to_owned(), fastmcp.clone()),
            ("fastmcp-slim".to_owned(), fastmcp.clone()),
            ("fastmcp-tasks".to_owned(), fastmcp),
            (
                "mcp".to_owned(),
                meta(&[(
                    "Repository",
                    "https://github.com/modelcontextprotocol/python-sdk",
                )]),
            ),
        ];
        assert_eq!(
            propose("fastmcp", &installed),
            (
                vec![
                    "fastmcp".to_owned(),
                    "fastmcp-slim".to_owned(),
                    "fastmcp-tasks".to_owned()
                ],
                true
            )
        );
    }

    #[test]
    fn a_shared_sponsor_link_is_not_a_shared_source() {
        // Slice review F5: jsonschema, referencing and friends share only the author's funding
        // link.
        let sponsor = ("Funding", "https://github.com/sponsors/Julian");
        let installed = vec![
            (
                "jsonschema".to_owned(),
                meta(&[
                    sponsor,
                    ("Source", "https://github.com/python-jsonschema/jsonschema"),
                ]),
            ),
            (
                "referencing".to_owned(),
                meta(&[
                    sponsor,
                    ("Source", "https://github.com/python-jsonschema/referencing"),
                ]),
            ),
        ];
        assert_eq!(
            propose("jsonschema", &installed).0,
            vec!["jsonschema".to_owned()]
        );
    }

    #[test]
    fn a_distribution_without_a_repository_is_proposed_alone() {
        let installed = vec![
            ("lonely".to_owned(), meta(&[])),
            (
                "other".to_owned(),
                meta(&[("Source", "https://github.com/a/b")]),
            ),
        ];
        assert_eq!(
            propose("lonely", &installed),
            (vec!["lonely".to_owned()], false)
        );
    }
}
