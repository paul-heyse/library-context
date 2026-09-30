from __future__ import annotations

from collections.abc import Sequence

import library_unitgraph as ug


def unit(pkg_id: str, features: list[str], deps: Sequence[int] = ()) -> dict:
    return {
        "pkg_id": pkg_id,
        "features": features,
        "dependencies": [{"index": i, "extern_crate_name": "x"} for i in deps],
    }


def test_package_ids_of_each_kind_parse_to_name_and_version() -> None:
    assert ug.parse_package("registry+https://x/crates.io-index#sqlx@0.9.0") == ("sqlx", "0.9.0")
    assert ug.parse_package("git+https://h/o/pyrefly?rev=ab#pyrefly@1.3.1") == ("pyrefly", "1.3.1")
    assert ug.parse_package("path+file:///r/crates/lctx#0.1.0") == ("lctx", "0.1.0")
    assert ug.parse_package("path+file:///r/python/lctx_semantics#lctx-semantics@0.1.0") == (
        "lctx-semantics",
        "0.1.0",
    )


def test_summary_unions_features_and_counts_only_workspace_edges_without_the_hack_crate() -> None:
    graph = {
        "units": [
            unit("registry+r#lib@1.0.0", ["a"]),  # 0: lib as a library unit
            unit("registry+r#lib@1.0.0", ["a", "b"]),  # 1: lib as a build-script unit
            unit("path+file:///r/crates/app#0.1.0", [], [0]),  # 2: workspace crate linking lib
            unit("path+file:///r/crates/lctx-workspace-hack#0.1.0", [], [1]),  # 3: the stub
            unit("path+file:///r/third_party/vendored#0.1.0", ["v"], [0]),  # 4: not a member
            unit("registry+r#other@2.0.0", []),  # 5: no features, nobody links it
        ]
    }
    hack = "lctx-workspace-hack"
    summary = ug.summarize(graph, {"app", hack})
    assert summary[("lib", "1.0.0")] == ug.Summary(("a", "b"), ("app",))
    assert summary[("other", "2.0.0")] == ug.Summary((), ())
    assert ("vendored", "0.1.0") in summary  # a path dependency that is not a member is external
    assert ("app", "0.1.0") not in summary and (hack, "0.1.0") not in summary


def test_a_missing_cargo_or_a_timeout_is_a_reason_not_an_exception(tmp_path, monkeypatch) -> None:
    import subprocess

    def timeout(*a, **k):
        raise subprocess.TimeoutExpired("cargo", 1)

    monkeypatch.setattr(subprocess, "run", timeout)
    graph, why = ug.load(tmp_path, timeout=1)
    assert graph is None and "did not finish within 1 s" in why
    monkeypatch.setattr(
        subprocess, "run", lambda *a, **k: (_ for _ in ()).throw(FileNotFoundError())
    )
    assert ug.load(tmp_path) == (None, "cargo not found")
