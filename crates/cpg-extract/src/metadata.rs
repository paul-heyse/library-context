//! Pure interpretation of verified distribution bytes. No environment expansion or resolution.
use cpg_schema::evidence::{CheckStatus, DeploymentDetail};
use std::collections::BTreeMap;
use std::str::FromStr;

fn observation(field: &str, original: &str) -> DeploymentDetail {
    DeploymentDetail {
        distribution: None,
        version: None,
        field: field.into(),
        original: original.into(),
        name: None,
        extras: vec![],
        marker: None,
        constraint: None,
        interpretation: CheckStatus::Passed,
        diagnostic: None,
        environment_digest: None,
        lock_digest: None,
        task: None,
        referenced_path: None,
    }
}
fn failed(field: &str, original: &str, error: impl ToString) -> DeploymentDetail {
    let mut out = observation(field, original);
    out.interpretation = CheckStatus::Failed;
    out.diagnostic = Some(error.to_string());
    out
}
pub fn requirement(text: &str) -> Result<pep508_rs::Requirement<url::Url>, String> {
    pep508_rs::Requirement::<url::Url>::from_str(text).map_err(|e| e.to_string())
}
pub fn metadata(bytes: &[u8]) -> Vec<DeploymentDetail> {
    let headers = match mailparse::parse_headers(bytes) {
        Ok((headers, _)) => headers,
        Err(e) => return vec![failed("metadata", "", e)],
    };
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for header in headers {
        let value = match std::str::from_utf8(header.get_value_raw()) {
            Ok(value) => value,
            Err(e) => return vec![failed("metadata", "", e)],
        };
        let key = String::from_utf8_lossy(header.get_key_raw()).to_ascii_lowercase();
        // Core metadata is UTF-8, with RFC-style continuation whitespace; no MIME word decoding.
        let value = value
            .replace("\r\n", "\n")
            .split('\n')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(" ");
        fields.entry(key).or_default().push(value);
    }
    let unique = |key: &str| {
        fields
            .get(key)
            .filter(|v| v.len() == 1)
            .map(|v| v[0].clone())
    };
    let distribution = unique("name");
    let version = unique("version");
    let mut out = Vec::new();
    for key in ["metadata-version", "name", "version", "requires-python"] {
        let values = fields.get(key);
        if values.is_none() && key != "requires-python" || values.is_some_and(|v| v.len() != 1) {
            out.push(failed(
                key,
                &values.map(|v| v.join("\n")).unwrap_or_default(),
                "missing or duplicate singleton metadata field",
            ));
        }
    }
    for (key, values) in fields {
        if !matches!(
            key.as_str(),
            "metadata-version"
                | "name"
                | "version"
                | "requires-python"
                | "requires-dist"
                | "provides-extra"
        ) {
            continue;
        }
        let duplicate =
            values.len() > 1 && !matches!(key.as_str(), "requires-dist" | "provides-extra");
        for value in values {
            let mut item = observation(&key, &value);
            if duplicate {
                item = failed(&key, &value, "duplicate singleton field");
            } else if key == "requires-dist" {
                match requirement(&value) {
                    Ok(req) => {
                        item.name = Some(req.name.to_string());
                        item.extras = req.extras.iter().map(ToString::to_string).collect();
                        item.marker = req.marker.contents().map(|m| m.to_string());
                        item.constraint = req.version_or_url.map(|v| v.to_string());
                    }
                    Err(e) => item = failed(&key, &value, e),
                }
            } else if key == "requires-python" {
                match pep508_rs::pep440_rs::VersionSpecifiers::from_str(&value) {
                    Ok(spec) => item.constraint = Some(spec.to_string()),
                    Err(e) => item = failed(&key, &value, e),
                }
            } else if key == "version" {
                if let Err(e) = pep508_rs::pep440_rs::Version::from_str(&value) {
                    item = failed(&key, &value, e);
                }
            } else if key == "name" {
                if let Err(e) = pep508_rs::PackageName::from_str(&value) {
                    item = failed(&key, &value, e);
                }
            } else if key == "metadata-version" {
                if !matches!(
                    value.as_str(),
                    "1.0" | "1.1" | "1.2" | "2.0" | "2.1" | "2.2" | "2.3" | "2.4" | "2.5"
                ) {
                    item = failed(&key, &value, "unsupported metadata version");
                }
            } else if key == "provides-extra" {
                match pep508_rs::ExtraName::from_str(&value) {
                    Ok(extra) => item.name = Some(extra.to_string()),
                    Err(e) => item = failed(&key, &value, e),
                }
            }
            item.distribution = distribution.clone();
            item.version = version.clone();
            out.push(item);
        }
    }
    out
}
pub fn entry_points(bytes: &[u8]) -> Vec<DeploymentDetail> {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => return vec![failed("entry-point", "", e)],
    };
    let options = ini::ParseOption {
        enabled_quote: false,
        enabled_escape: false,
        enabled_indented_mutiline_value: true,
        enabled_preserve_key_leading_whitespace: false,
    };
    let parsed = match ini::Ini::load_from_str_opt(text, options) {
        Ok(v) => v,
        Err(e) => return vec![failed("entry-point", text, e)],
    };
    let mut out = Vec::new();
    let mut seen = BTreeMap::<(String, String), usize>::new();
    for (section, properties) in &parsed {
        for (name, target) in properties.iter() {
            let group = section.unwrap_or_default();
            let field = format!("entry-point:{group}");
            let mut item = observation(&field, target);
            item.name = Some(name.into());
            let callable = target.split('[').next().unwrap_or_default().trim();
            let valid = !group.is_empty()
                && !name.is_empty()
                && callable.split(':').count() <= 2
                && callable.split([':', '.']).all(|s| {
                    let mut c = s.chars();
                    c.next().is_some_and(|c| c == '_' || c.is_alphabetic())
                        && c.all(|c| c == '_' || c.is_alphanumeric())
                });
            let extras = target.split_once('[').map(|(_, tail)| {
                tail.strip_suffix(']').map(|inside| {
                    inside
                        .split(',')
                        .map(|x| pep508_rs::ExtraName::from_str(x.trim()))
                        .collect::<Result<Vec<_>, _>>()
                })
            });
            let valid_extras = match extras {
                None => !target.contains(']'),
                Some(Some(Ok(extras))) => {
                    item.extras = extras.into_iter().map(|e| e.to_string()).collect();
                    true
                }
                _ => false,
            };
            if !valid || !valid_extras {
                item.interpretation = CheckStatus::Failed;
                item.diagnostic = Some("invalid entry-point group/name/object reference".into());
            }
            if let Some(previous) = seen.insert((group.into(), name.into()), out.len()) {
                let old: &mut DeploymentDetail = &mut out[previous];
                old.interpretation = CheckStatus::Failed;
                old.diagnostic = Some("duplicate entry point".into());
                item.interpretation = CheckStatus::Failed;
                item.diagnostic = old.diagnostic.clone();
            }
            out.push(item);
        }
    }
    out
}

/// Selected JSON launch/configuration artifacts. Field syntax is an observation, never an
/// assertion that this dialect is accepted by the current CLI or sufficient to launch it.
pub fn configuration(path: &str, bytes: &[u8]) -> Vec<DeploymentDetail> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => return vec![failed("configuration", "", error)],
    };
    if !path.ends_with(".json") {
        let mut row = observation("configuration", path);
        row.interpretation = CheckStatus::NotRun;
        row.diagnostic = Some("Configuration dialect has not been interpreted.".into());
        return vec![row];
    }
    let json: serde_json::Value = match serde_json::from_str(text) {
        Ok(json) => json,
        Err(error) => return vec![failed("configuration", path, error)],
    };
    let mut out = Vec::new();
    for pointer in [
        "/source/path",
        "/entrypoint",
        "/source",
        "/transport",
        "/environment/dependencies",
        "/dependencies",
        "/environment/python",
        "/python",
        "/args",
        "/env",
    ] {
        let Some(value) = json.pointer(pointer) else {
            continue;
        };
        let mut row = observation(&format!("configuration:{pointer}"), &value.to_string());
        row.diagnostic=Some("Original JSON field; effective CLI interpretation and prerequisite sufficiency have not been checked.".into());
        if matches!(pointer, "/source/path" | "/entrypoint" | "/source")
            && let Some(target) = value.as_str()
        {
            let target = target.split(':').next().unwrap_or_default();
            if target.ends_with(".py") {
                let mut segments: Vec<_> = path.split('/').collect();
                segments.pop();
                let mut valid = true;
                for segment in target.split('/') {
                    match segment {
                        "" => {
                            valid = false;
                            break;
                        }
                        "." => {}
                        ".." => {
                            if segments.pop().is_none() {
                                valid = false;
                                break;
                            }
                        }
                        other => segments.push(other),
                    }
                }
                if valid {
                    row.referenced_path = Some(segments.join("/"));
                }
            }
        }
        out.push(row);
    }
    if out.is_empty() {
        let mut row = observation("configuration", path);
        row.diagnostic = Some("Valid JSON; no supported launch fields were found.".into());
        out.push(row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_requirements_preserve_original_and_extras() {
        let rows=metadata(b"Metadata-Version: 2.5\nName: pkg\nVersion: 1\nRequires-Dist: dep[server]>=1; extra == 'x'\n\nbody");
        let row = rows.iter().find(|r| r.field == "requires-dist").unwrap();
        assert_eq!(row.extras, ["server"]);
        assert_eq!(row.name.as_deref(), Some("dep"));
        assert!(row.marker.as_ref().unwrap().contains("extra"));
    }
    #[test]
    fn duplicate_entry_points_and_invalid_requirements_are_local() {
        let rows =
            entry_points(b"[console_scripts]\nRun = pkg:main\nRun = other:main\nrun = pkg:other\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].interpretation, CheckStatus::Failed);
        assert_eq!(rows[2].interpretation, CheckStatus::Passed);
        assert!(
            metadata(b"Name: pkg\nVersion: 1\nMetadata-Version: 2.5\nRequires-Dist: !!\n")
                .iter()
                .any(|r| r.field == "requires-dist" && r.interpretation == CheckStatus::Failed)
        );
    }
    #[test]
    fn configuration_preserves_fields_and_bounds_relative_source() {
        let rows=configuration("examples/fastmcp.json",br#"{"source":{"path":"server.py:mcp"},"transport":"stdio","dependencies":["x[y]>=1"],"env":{"TOKEN":"${TOKEN}"}}"#);
        assert!(
            rows.iter()
                .any(|r| r.referenced_path.as_deref() == Some("examples/server.py"))
        );
        assert!(rows.iter().any(|r| r.original.contains("${TOKEN}")));
        assert!(rows.iter().all(|r| r.diagnostic.is_some()));
        assert!(
            configuration("launch.json", br#"{"source":"../../outside.py"}"#)
                .iter()
                .all(|r| r.referenced_path.is_none())
        );
        assert_eq!(
            configuration("launch.json", b"{")[0].interpretation,
            CheckStatus::Failed
        );
        assert_eq!(
            configuration("launch.toml", b"[server]")[0].interpretation,
            CheckStatus::NotRun
        );
    }
    #[test]
    fn requirement_url_does_not_expand_operator_environment() {
        let req = requirement("pkg @ https://example.invalid/${HOME}/pkg.whl").unwrap();
        assert!(req.to_string().contains("HOME"));
    }
}
