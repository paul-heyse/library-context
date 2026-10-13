//! Fresh revealing schema/permission cases, separate from native schema authority.
use anyhow::{Result, bail};
use std::path::Path;

pub const TOKEN_MARKER: &str = "LCTX_WORKER_ISSUED_TOKEN";

pub fn write(root: &Path) -> Result<()> {
    if !crate::read_projection(root)?
        .modules
        .iter()
        .any(|module| module == "completion")
    {
        bail!("sample suite requires the completion responsibility and its dependencies");
    }
    let tests = root.join("tests");
    if tests.exists() {
        bail!("sample suite requires an unpopulated tests directory");
    }
    std::fs::create_dir_all(tests.join("suites"))?;
    std::fs::write(tests.join("config.toml"), CONFIG)?;
    std::fs::write(tests.join("suites/native.toml"), SUITE)?;
    Ok(())
}

const CONFIG: &str = r#"[actors.root]
kind = "root"

[actors.token]
kind = "token"
token = "LCTX_WORKER_ISSUED_TOKEN"

[actors.record]
kind = "record"
access = "lifecycle_probe"

[actors.record.signin_params]
code = "current"

[[fixtures]]
actor = "root"
sql = "DEFINE TABLE lifecycle_probe_actor SCHEMAFULL PERMISSIONS NONE; DEFINE FIELD code ON lifecycle_probe_actor TYPE string; CREATE lifecycle_probe_actor:one SET code='current'; DEFINE ACCESS lifecycle_probe ON DATABASE TYPE RECORD SIGNIN (SELECT * FROM ONLY lifecycle_probe_actor:one WHERE code=$code); fn::native_touch(lifecycle_probe_actor:one);"
"#;

const SUITE: &str = r#"name = "native schema and permissions"
tags = ["lifecycle", "schema"]

[[cases]]
name = "required compiler physical fields"
kind = "schema_metadata"
actor = "root"
table = "compiler_membership"
contains = ["contribution", "semantic_key", "node", "content", "contribution_rows", "contribution_nodes"]

[[cases]]
name = "nested native function body executes"
kind = "schema_behavior"
actor = "root"
action_sql = "RETURN fn::native_touch(lifecycle_probe_actor:one);"
verify_sql = "RETURN (SELECT VALUE revision FROM ONLY type::record('native_predicate',[lifecycle_probe_actor:one]));"
[[cases.assertions]]
path = ""
equals = 2

[[cases]]
name = "native field type refuses invalid writes"
kind = "sql_expect"
actor = "root"
sql = "CREATE native_predicate:invalid SET revision='invalid';"
allow = false

[[cases]]
name = "native function argument type remains checked"
kind = "sql_expect"
actor = "root"
sql = "RETURN fn::native_touch(123);"
allow = false

[[cases]]
name = "record actor cannot read native control rows"
kind = "sql_expect"
actor = "record"
sql = "RETURN array::len(SELECT * FROM native_predicate);"
[[cases.assertions]]
path = ""
equals = 0

[[cases]]
name = "token actor authenticates through remote server"
kind = "sql_expect"
actor = "token"
sql = "RETURN array::len(SELECT * FROM native_predicate);"
[[cases.assertions]]
path = ""
equals = 1
"#;
