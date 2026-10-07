"""Smoke cases for shparse (library-context shapes). Run: python -I t_shparse.py"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from shparse import summarize  # noqa: E402

CASES = [
    "just verify-model -E 'test(ids)'",
    "just verify-serving --command mcp -- -k session 2>&1 | tail -40",
    "uv run --no-sync python scripts/verify.py store -E 'test(native)'",
    "uv run python scripts/native_controls.py compiler -E 'binary(fixture_corpus)'",
    "python3 scripts/surrealdb_fixture.py -- cargo nextest run --release -p lctx-surrealdb",
    "python3 scripts/build_environment.py -- cargo test -p cpg-core --lib",
    "eval \"$(python3 scripts/build_environment.py --shell)\" && cargo nextest run -p lctx-model",
    "UV_NO_SYNC=1 uv run pytest python/lctx_mcp/tests -q",
    "uv run --no-project --offline python scripts/adr.py index",
    "uv run --frozen python scripts/library_catalog_mcp.py",
    "./target/release/lctx compile fastmcp --artifact-only --output /tmp/x",
    "docker ps -a --filter name=lctx",
    "cargo nextest run --release -p cpg-extract --test python_reference_oracle",
    "cd /home/paul/library-context && just ready",
    "uv sync --locked --inexact",
    "git add docs/plans/x.md && git commit -m 'x'",
]
for c in CASES:
    s = summarize(c)
    print(repr(c[:72]))
    print("   ", s["cls"], "|", s["sub"], "|", s["norm"][:90], "| envs", s["envs"], "feats", s["feats"],
          "| fam", s["pinfo"].get("family"), "bnd", s["pinfo"].get("boundary"), "flt", s["pinfo"].get("filtered"))
