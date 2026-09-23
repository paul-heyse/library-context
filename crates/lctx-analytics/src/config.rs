//! The pre-registered analytics config (DESIGN §1.4, §9; ADR-0004 amendment):
//! `libraries/<name>/analytics.toml`. It declares the subsystem, the seeds and every analysis
//! parameter. Its digest is the `lctx-compiler` run's config digest, so it is part of
//! `content_digest`. It is written before any brief exists, and frozen before the first gold
//! scoring.

use std::path::Path;

use cpg_schema::findings::recipe;
use cpg_schema::id::Digest;
use serde::{Deserialize, Serialize};

use crate::AnalyticsError;

/// The config file's version this compiler reads.
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsConfig {
    pub version: u32,
    pub subsystem: Subsystem,
    pub seeds: Seeds,
    pub pass_a: PassA,
    pub briefs: Briefs,
}

/// The analyzed subsystem (§1.4): the compiler learns it only from here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subsystem {
    /// Module names in scope: a module is in the subsystem when its name is a prefix or starts
    /// with a prefix followed by `.`.
    pub module_prefixes: Vec<String>,
    /// The public roots every seed's access path starts from.
    pub public_roots: Vec<String>,
}

/// The seeds that get briefs (increment 1: one hand-registered seed plus distractors).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seeds {
    pub primary: Vec<String>,
    pub distractors: Vec<String>,
}

/// Pass A's budgets (§9.1): starting budgets, not measured optima.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassA {
    pub max_depth: u32,
    pub max_vertices: u32,
    pub max_edges: u32,
    pub max_witnesses: u32,
}

/// Brief selection and serving (§10, §6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Briefs {
    /// How many briefs a compile may publish: the seeds, until seed selection (2.6) chooses
    /// among candidates within it. More seeds than the budget are refused.
    pub budget: u32,
}

impl AnalyticsConfig {
    pub fn load(path: &Path) -> Result<Self, AnalyticsError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| AnalyticsError::Config(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, AnalyticsError> {
        let config: Self =
            toml::from_str(text).map_err(|e| AnalyticsError::Config(e.to_string()))?;
        config.check()?;
        Ok(config)
    }

    fn check(&self) -> Result<(), AnalyticsError> {
        let bad = |m: String| Err(AnalyticsError::Config(m));
        if self.version != VERSION {
            return bad(format!(
                "version {} (this compiler reads {VERSION})",
                self.version
            ));
        }
        if self.subsystem.module_prefixes.is_empty() || self.subsystem.public_roots.is_empty() {
            return bad("the subsystem needs module prefixes and public roots".to_owned());
        }
        let seeds = self.seeds();
        if seeds.is_empty() {
            return bad("no seeds".to_owned());
        }
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != seeds.len() {
            return bad("a seed is listed twice".to_owned());
        }
        let identifier = |s: &str| {
            let mut chars = s.chars();
            chars
                .next()
                .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
                && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        };
        let dotted = |s: &str| s.split('.').all(identifier);
        let names = seeds
            .iter()
            .chain(&self.subsystem.module_prefixes)
            .chain(&self.subsystem.public_roots);
        for name in names {
            if !dotted(name) {
                return bad(format!("{name:?} is not a dotted Python name"));
            }
        }
        for seed in &seeds {
            let under_root = self
                .subsystem
                .public_roots
                .iter()
                .any(|r| seed == r || seed.starts_with(&format!("{r}.")));
            if !under_root {
                return bad(format!("seed {seed} is under no public root"));
            }
        }
        if seeds.len() > self.briefs.budget as usize {
            return bad(format!(
                "{} seeds exceed the brief budget of {}",
                seeds.len(),
                self.briefs.budget
            ));
        }
        if self.pass_a.max_witnesses == 0 || self.pass_a.max_depth == 0 {
            return bad("pass_a needs a depth and a witness budget of at least 1".to_owned());
        }
        Ok(())
    }

    /// Every seed, primary first, in declared order.
    pub fn seeds(&self) -> Vec<String> {
        self.seeds
            .primary
            .iter()
            .chain(&self.seeds.distractors)
            .cloned()
            .collect()
    }

    /// The config as canonical JSON: fields in declaration order, no whitespace.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).expect("a config always serializes")
    }

    /// The `lctx-compiler` run's config digest.
    pub fn digest(&self) -> Digest {
        recipe::config_digest(&self.canonical_json())
    }

    /// Whether a module name is in the subsystem.
    pub fn in_subsystem(&self, module: &str) -> bool {
        self.subsystem
            .module_prefixes
            .iter()
            .any(|p| module == p || module.starts_with(&format!("{p}.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.Server.resource"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 4
"#;

    #[test]
    fn a_config_parses_checks_and_digests_its_canonical_form() {
        let c = AnalyticsConfig::parse(SAMPLE).unwrap();
        assert_eq!(c.seeds(), ["pkg.Server.tool", "pkg.Server.resource"]);
        assert!(c.in_subsystem("pkg.server"));
        assert!(c.in_subsystem("pkg.server.tools"));
        assert!(!c.in_subsystem("pkg.serverless"));
        // Whitespace and key order in the file do not change the digest; a value does.
        let spaced = SAMPLE.replace("max_depth = 2", "max_depth   =   2");
        assert_eq!(
            c.digest(),
            AnalyticsConfig::parse(&spaced).unwrap().digest()
        );
        let deeper = SAMPLE.replace("max_depth = 2", "max_depth = 3");
        assert_ne!(
            c.digest(),
            AnalyticsConfig::parse(&deeper).unwrap().digest()
        );
    }

    #[test]
    fn a_config_refuses_unknown_keys_bad_seeds_and_versions() {
        let unknown = SAMPLE.replace("budget = 4", "budget = 4\ncolour = 1");
        assert!(AnalyticsConfig::parse(&unknown).is_err());
        let rootless = SAMPLE.replace("pkg.Server.resource", "other.thing");
        assert!(AnalyticsConfig::parse(&rootless).is_err());
        let twice = SAMPLE.replace("pkg.Server.resource", "pkg.Server.tool");
        assert!(AnalyticsConfig::parse(&twice).is_err());
        let quoted = SAMPLE.replace("pkg.Server.resource", "pkg.Server.x'y");
        assert!(AnalyticsConfig::parse(&quoted).is_err());
        let v2 = SAMPLE.replace("version = 1", "version = 2");
        assert!(AnalyticsConfig::parse(&v2).is_err());
        // Increment-1 deep review F3: the budget binds; more seeds than it are refused.
        let tight = SAMPLE.replace("budget = 4", "budget = 1");
        let err = AnalyticsConfig::parse(&tight).unwrap_err().to_string();
        assert!(err.contains("exceed the brief budget"), "{err}");
        let gone = SAMPLE.replace("budget = 4", "budget = 4\nserve_unreviewed = true");
        assert!(
            AnalyticsConfig::parse(&gone).is_err(),
            "a removed key is unknown"
        );
    }
}
