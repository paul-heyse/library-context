// DORMANT P5: actual native/Python serving and singleton hydration expectations.
// P4 resolved-builtin/read controls moved to cpg-extract/tests/retired_read_expectations.rs.
// Supplied P5 generations replace removed Delta compilation/import mechanics.
#[path = "serving/native_projection.rs"]
mod native_projection;
use std::path::Path;

/// Future typed P5 fixture includes the singleton and deliberately unserved dynamic instance.
pub const SINGLETON_SOURCE: &str = r#"
import builtins
from builtins import getattr as read_attribute

class Settings:
    """Read named configuration fields."""
    def __init__(self):
        self.bare_only = 1
        self.qualified_only = 2
        self.aliased_only = 3
        self.direct_only = 4
        self.unread_only = 5
    def bare(self):
        return getattr(self, "bare_only")
    def qualified(self):
        return builtins.getattr(self, "qualified_only")
    def aliased(self):
        return read_attribute(self, "aliased_only")
    def direct(self):
        return self.direct_only
    def shadowed(self):
        getattr = lambda obj, name: 0
        return getattr(self, "unread_only")

settings = Settings()

class DynamicSettings:
    def __init__(self):
        self.computed_name = 1
    def computed(self, suffix):
        return getattr(self, "computed_" + suffix)
    def formatted(self, suffix):
        return getattr(self, "{}".format(suffix))
    def joined(self, parts):
        return getattr(self, "".join(parts))

_dynamic_settings = DynamicSettings()
"#;

pub fn equivalent_getattr_spellings_and_computed_names_keep_no_read_sound(generation_dir: &Path) {
    native_projection::run_native_script(generation_dir, r#"
import json
import sys
from pathlib import Path
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
generation = load(Path(sys.argv[1]), None)
claims = {r["place_key"]: r for r in generation.tables["place_claims"].to_pylist()}
assert any(k.endswith("qualified_only]") and not v["holds"] for k, v in claims.items()), claims
assert any(k.endswith("aliased_only]") and not v["holds"] for k, v in claims.items()), claims
assert any(k.endswith("unread_only]") and v["holds"] for k, v in claims.items()), claims
with served_bundle(Path(sys.argv[1])) as pg:
    operation = pg.operation(pg.load(), generation.snapshot_id, "probe.Settings")
    singleton = pg.operation(pg.load(), generation.snapshot_id, "probe.settings")
    assert singleton.operation_id == operation.operation_id
    assert singleton.access_path == "probe.settings" and singleton.resolution == "singleton_class"
    class_fields = json.loads(pg.call(pg.pinned.get_operation, generation.snapshot_id, operation.member_id, True, json.dumps({"kind":"section", "section":"fields"})))
    assert class_fields["state"] == "unavailable" and not class_fields["items"]
    singleton_fields = [r.record for r in pg.section(generation.snapshot_id, singleton.member_id, "fields")]
    assert singleton.catalog.constructors == operation.catalog.constructors
    by_member = pg.operation(pg.load(), generation.snapshot_id, singleton.member_id)
    assert by_member.operation_id == singleton.operation_id
    dynamic = pg.operation(pg.load(), generation.snapshot_id, "probe.DynamicSettings")
    assert dynamic.catalog.constructors and dynamic.catalog.signatures
fields = {field.name: field.never_read for field in singleton_fields}
assert fields["qualified_only"].startswith("unknown (not refuted)"), fields
assert fields["aliased_only"].startswith("unknown (not refuted)"), fields
assert fields["unread_only"].startswith("refuted_under_model"), fields
"#);
}
