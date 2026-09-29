from __future__ import annotations

from pathlib import Path

import library_names as names


def make_index() -> names.Index:
    return names.Index(
        symbols={
            "lib::a::Type": "struct",
            "lib::a::run": "function",
            "lib::mods": "module",
            "lib::q::query": "macro",
            "lib::x::Dup": "struct",
            "lib::y::Dup": "struct",
        },
        aliases={"facade::Type": "lib::a::Type"},
        methods={("lib::a::Type", "go"), ("lib::a::Type", "run")},
        by_name={
            ("lib", "Type"): ["lib::a::Type"],
            ("lib", "run"): ["lib::a::run"],
            ("lib", "query"): ["lib::q::query"],
            ("lib", "Dup"): ["lib::x::Dup", "lib::y::Dup"],
        },
        crates={"lib", "facade"},
    )


def how(item: str) -> tuple[str | None, str]:
    r = names.resolve(item, make_index())
    return r.canonical, r.how


def test_exact_alias_and_unique_name_resolve_to_the_defining_path() -> None:
    assert how("lib::a::Type") == ("lib::a::Type", "canonical")
    assert how("facade::Type") == ("lib::a::Type", "alias")
    assert how("lib::Type") == ("lib::a::Type", "name")
    assert how("lib::query!") == ("lib::q::query!", "name")


def test_methods_resolve_through_their_type_and_never_as_free_functions() -> None:
    assert how("lib::Type::go") == ("lib::a::Type::go", "name")
    assert how("facade::Type::go") == ("lib::a::Type::go", "alias")
    assert how("lib::a::Type::go") == ("lib::a::Type::go", "method")
    # `run` is both a method of Type and a free function of the crate: the method reading wins.
    assert how("lib::a::Type::run") == ("lib::a::Type::run", "method")
    assert how("lib::Type::missing") == (None, "absent")


def test_ambiguous_absent_and_unindexed_items_are_left_alone() -> None:
    assert how("lib::Dup") == (None, "ambiguous")
    assert how("lib::Nothing") == (None, "absent")
    assert how("sqlx::query") == (None, "unindexed")


def test_normalize_items_dedupes_and_reports_replaced_spellings() -> None:
    items, replaced, _ = names.normalize_items(
        ["facade::Type", "lib::a::Type", "lib::Nothing"], make_index()
    )
    assert items == ["lib::a::Type", "lib::Nothing"] and replaced == ["facade::Type"]


def test_load_index_reads_symbols_aliases_and_methods_of_the_enabled_skills(tmp_path: Path) -> None:
    (tmp_path / ".config").mkdir()
    (tmp_path / ".config" / "library-skills.toml").write_text('enabled = ["s", "none"]\n')
    index_dir = tmp_path / ".claude" / "skills" / "s" / "content" / "index"
    index_dir.mkdir(parents=True)
    (index_dir / "symbols.tsv").write_text("lib::a::T\tstruct\tlib\nlib::m\tmodule\tlib\n")
    (index_dir / "aliases.tsv").write_text("lib::T\tlib::a::T\tstruct\n")
    (index_dir / "methods.tsv").write_text("lib::a::T\tgo\t-\tfn go()\n")
    index = names.load_index(tmp_path)
    assert index.symbols["lib::a::T"] == "struct" and index.aliases == {"lib::T": "lib::a::T"}
    assert index.methods == {("lib::a::T", "go")} and index.crates == {"lib"}
    assert index.by_name == {("lib", "T"): ["lib::a::T"]}


def test_an_item_the_compiler_resolved_exactly_is_not_rewritten_by_the_index() -> None:
    index = make_index()
    items, replaced, resolved = names.normalize_items(["facade::Type"], index)
    assert items == ["lib::a::Type"] and replaced == ["facade::Type"]
    items, replaced, resolved = names.normalize_items(["facade::Type"], index, {"facade::Type"})
    assert items == ["facade::Type"] and replaced == [] and resolved[0].how == "resolved"
