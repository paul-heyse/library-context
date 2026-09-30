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
        aliases={
            "facade::Type": ["lib::a::Type"],
            "lib::Both": ["lib::a::Trait", "lib_macros::Both"],
        },
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
    (index_dir / "aliases.tsv").write_text(
        "lib::T\tlib::a::T\tstruct\nlib::Both\tlib::a::Trait\ttrait\n"
        "lib::Both\tlib_macros::Both\tmacro\nlib::Both\tlib_macros::Both\tmacro\n"
    )
    (index_dir / "methods.tsv").write_text("lib::a::T\tgo\t-\tfn go()\n")
    index = names.load_index(tmp_path)
    assert index.symbols["lib::a::T"] == "struct"
    assert index.aliases == {
        "lib::T": ["lib::a::T"],
        "lib::Both": ["lib::a::Trait", "lib_macros::Both"],
    }
    assert index.methods == {("lib::a::T", "go")} and index.crates == {"lib"}
    assert index.by_name == {("lib", "T"): ["lib::a::T"]}


def test_an_item_the_compiler_resolved_exactly_is_not_rewritten_by_the_index() -> None:
    index = make_index()
    items, replaced, resolved = names.normalize_items(["facade::Type"], index)
    assert items == ["lib::a::Type"] and replaced == ["facade::Type"]
    items, replaced, resolved = names.normalize_items(["facade::Type"], index, {"facade::Type"})
    assert items == ["facade::Type"] and replaced == [] and resolved[0].how == "resolved"


def test_a_path_naming_two_items_is_ambiguous_with_both_candidates_and_never_guessed() -> None:
    r = names.resolve("lib::Both", make_index())
    assert (r.canonical, r.how, r.candidates) == (
        None,
        "ambiguous",
        ("lib::a::Trait", "lib_macros::Both"),
    )
    items, replaced, _ = names.normalize_items(["lib::Both"], make_index())
    assert items == ["lib::Both"] and replaced == []


def test_a_kept_item_records_how_the_index_places_the_compiler_path() -> None:
    index = make_index()
    kept = {"lib::a::Type", "lib::Type", "lib::Elsewhere"}
    _, _, resolved = names.normalize_items(
        ["lib::a::Type", "lib::Type", "lib::Elsewhere"], index, kept
    )
    assert [(r.how, r.index_how) for r in resolved] == [
        ("resolved", "canonical"),
        ("resolved", "name"),  # the index would say lib::a::Type; the compiler path stands
        ("resolved", "absent"),
    ]
    assert resolved[1].candidates == ("lib::a::Type",)


def test_a_macro_is_only_kept_when_the_compiler_resolved_a_macro_of_that_path() -> None:
    index = make_index()
    items = ["lib::q::query!", "lib::mods!"]
    _, _, as_module = names.normalize_items(items, index, {"lib::q::query", "lib::mods"})
    assert [r.how for r in as_module] != [
        "resolved",
        "resolved",
    ]  # module hits do not keep a `!` item
    _, _, as_macro = names.normalize_items(items, index, {"lib::q::query!", "lib::mods!"})
    assert [r.how for r in as_macro] == ["resolved", "resolved"]
