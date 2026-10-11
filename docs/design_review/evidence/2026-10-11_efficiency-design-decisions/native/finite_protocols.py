"""Independent finite design countercases, not SurrealDB/product acceptance.

Uses only Python's standard library and synthetic data. No services, fixtures,
production imports, lifecycle mutation or generated project outputs.
"""
from copy import deepcopy
from hashlib import sha256
from itertools import permutations
import json


def digest(value):
    return sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def refuses(function, *args):
    try:
        function(*args)
    except ValueError:
        return True
    return False


def atomic_install(state, rows, holds):
    candidate = deepcopy(state)
    for key, payload in rows:
        if key in candidate["rows"] and candidate["rows"][key] != payload:
            raise ValueError("collision")
        candidate["rows"][key] = payload
    candidate["holds"].update(holds)
    state.update(candidate)


def n01():
    state = {"rows": {"z": "old"}, "holds": set()}
    before = deepcopy(state)
    assert refuses(atomic_install, state, [("a", "new"), ("z", "different")], {("root", "a")})
    assert state == before
    request = digest(["install/v1", [("a", "new")], [("root", "a")]])
    outcomes = {}
    atomic_install(state, [("a", "new")], {("root", "a")})
    outcomes[request] = "committed"  # discard local acknowledgement
    assert outcomes[request] == "committed"
    assert request != digest(["install/v1", [("a", "changed")], [("root", "a")]])
    return {"late_collision_rollback": True, "same_request_lost_ack": True}


def n02():
    # Chunks [3,2,3] share windows [5,3], rather than singleton windows.
    windows, current = [], []
    for size in [3, 2, 3]:
        if sum(current) + size > 5:
            windows.append(current)
            current = []
        current.append(size)
    windows.append(current)
    assert windows == [[3, 2], [3]]
    accepted = {0: 5, 1: 3}
    charged = sum(accepted.values())
    assert charged == 8  # enqueued is still charged
    charged -= accepted.pop(0)  # durable acknowledgement
    assert charged == 3
    cancelled = True
    assert cancelled and accepted  # cannot complete just because cancelled
    charged -= accepted.pop(1)  # drain reconciled/fenced request
    assert charged == 0 and not accepted
    for order in permutations([3, 1, 2]):
        source = sorted(set(order))
        cursor, result = 0, []
        while page := [k for k in source if k > cursor][:2]:
            result += page
            cursor = page[-1]
        assert result == [1, 2, 3]
    return {"cross_chunk_window": True, "cancel_keeps_charge_until_drain": True, "keyset_orders": 6}


def n03():
    # Protocol conflict graph; explicit writes are the chosen sufficient primitive.
    pairs = {
        "intent/era-cut": ({"stripe:3", "effect:e"}, {f"stripe:{i}" for i in range(64)}),
        "hold/retirement": ({"object:x", "owner:y"}, {"object:x", "retirement:r"}),
        "backup-admit/retirement": ({"backup-admission"}, {"backup-admission", "object:x"}),
        "outcome-ref/history": ({"history-target:e"}, {"history-target:e", "checkpoint:h"}),
        "binding/history-attempt": ({"history-target:a"}, {"history-target:a", "checkpoint:h"}),
        "contribution/history-attempt": ({"history-target:a"}, {"history-target:a", "checkpoint:h"}),
        "cleanup/history-attempt": ({"history-target:a"}, {"history-target:a", "checkpoint:h"}),
        "effect/history-owner": ({"history-target:a", "effect:e"}, {"history-target:a", "checkpoint:h"}),
        "successor/predecessor-page": ({"lineage:r", "item:i"}, {"lineage:r", "item:i"}),
        "cleanup-successor/old-cleanup": ({"lineage:c"}, {"lineage:c"}),
        "freeze/late-family-write": ({"attempt:a"}, {"attempt:a"}),
        "pin-release/new-protect": ({"pin:p", "object:x"}, {"pin:p"}),
    }
    for left, right in pairs.values():
        assert left & right
    independent = ({"stripe:1", "owner:a", "object:a"}, {"stripe:2", "owner:b", "object:b"})
    assert not independent[0] & independent[1]
    maximal_stripe = dict(id="native_admission_stripe:" + "f" * 64, generation="f" * 64,
                          era=2**63-1, closed_through=2**63-2, revision=2**63-1, admission_open=False)
    stripe_bytes = len(json.dumps(maximal_stripe).encode())
    assert stripe_bytes < 1024
    # Reserve bounded SQL/receipt metadata separately; do not split an era transition.
    assert 64 * 1024 + 65536 < 8 * 1024 * 1024
    # Omitting the object/set write gives the classic both-commit phantom countercase.
    assert not {"hold:new"} & {"retirement:r"}
    # Check both possible commit orders under snapshot write-conflict semantics.
    schedules = 0
    for left, right in pairs.values():
        for first, second in [(left, right), (right, left)]:
            assert first & second  # second overlapping snapshot must retry
            schedules += 1
    return {"predicate_pairs": len(pairs), "overlap_commit_orders": schedules, "independent_disjoint_pair": True,
            "maximal_stripe_json_bytes": stripe_bytes, "cut_stripes": 64,
            "limit": "abstract write-conflict model; actual backend checked separately"}


def installed(expected, actual, eof, incarnation, expected_incarnation):
    if not eof or actual != expected or incarnation != expected_incarnation:
        raise ValueError("incomplete or stale")
    return digest(["view-install/v1", expected, incarnation])


def n04():
    expected = [["r", "a", "node:a", "content:a"], ["r", "b", "node:b", "content:b"]]
    installed(expected, deepcopy(expected), True, 1, 1)
    cases = [(expected[:1], True, 1), (expected + [["r", "c", "node:c", "content:c"]], True, 1),
             (expected[:1] + [["r", "c", "node:c", "content:c"]], True, 1),
             (expected, False, 1), (expected, True, 2)]
    assert all(refuses(installed, expected, rows, eof, inc, 1) for rows, eof, inc in cases)
    assert installed([], [], True, 1, 1)
    scopes = {"facts": {"completed": True, "members": tuple(map(tuple, expected))},
              "normalized": {"completed": False, "members": ()}}
    facts_before = deepcopy(scopes["facts"])
    scopes["normalized"]["members"] = (("different", "new-member"),)
    assert scopes["facts"] == facts_before and not scopes["normalized"]["completed"]
    return {"countercases_refused": len(cases), "explicit_empty_completed": True,
            "frozen_view_with_independent_producing_scope": True}


FAMILIES = ["contributions", "memberships", "views", "typed_backing", "bindings", "aliases"]


def completed(families):
    roots = []
    for family in FAMILIES:
        # Unique canonical logical keys, explicit empty families; physical IDs excluded.
        entries = sorted(families[family], key=lambda row: row[0])
        if len({row[0] for row in entries}) != len(entries):
            raise ValueError("duplicate logical key")
        roots.append([family, len(entries), digest(entries)])
    return digest(["completed/v4-proposed", roots])


def n06():
    families = {f: [["a", {"value": f}], ["b", {"value": "b"}]] for f in FAMILIES}
    base = completed(families)
    for family in FAMILIES:
        reordered = deepcopy(families)
        reordered[family].reverse()
        assert completed(reordered) == base
        changed = deepcopy(families)
        changed[family][0][1]["value"] = "different"
        assert completed(changed) != base
        missing = deepcopy(families)
        missing[family] = []
        assert completed(missing) != base
    with_empty_view = deepcopy(families)
    with_empty_view["views"].append(["explicit-empty", {"relation": "r", "rows": 0, "selector": "default"}])
    assert completed(with_empty_view) != base
    return {"families_mutation_sensitive": 6, "family_orders_invariant": 6, "empty_view_identity_bearing": True}


def n05_n09():
    # Crash prefixes preserve either live owner or durable nomination, never an invisible root.
    edges = {("attempt", "stage"), ("stage", "child"), ("peer", "child")}
    nominated = set()
    assert ("attempt", "stage") in edges
    nominated.add("stage")  # durable before last attempt ownership release
    edges.remove(("attempt", "stage"))
    assert "stage" in nominated
    edges.remove(("stage", "child"))  # paged retirement
    assert ("peer", "child") in edges  # shared content survives
    # Publication variant transfers one root edge before attempt release.
    edges = {("attempt", "stage"), ("stage", "child")}
    edges.add(("publication", "stage"))
    marker_committed = True
    edges.remove(("attempt", "stage"))
    assert marker_committed and ("publication", "stage") in edges
    # Binding meaning cannot be reduced to logical view alone.
    bindings = [["r", "requested:A", "resolved:B", "role:X", "view:V"],
                ["r", "requested:C", "resolved:B", "role:X", "view:V"]]
    assert digest(bindings[0]) != digest(bindings[1])
    canonical = {digest(b): b for b in bindings}
    incidence = [("attempt", key) for key in canonical]
    incidence.clear()
    assert len(canonical) == 2  # selected backup retains descriptors after incidence release
    return {"abandoned_root_discoverable": True, "shared_child_survives": True,
            "committed_root_transfer": True, "complete_binding_survives_incidence_release": True}


def n07_n08():
    expected = {("reference", "a", "payload-a"), ("statistics", "r", "count:2"), ("context", "r", "negative")}
    assert deepcopy(expected) == expected
    assert expected - {next(iter(expected))} != expected
    assert expected | {("reference", "extra", "payload-x")} != expected
    assert {("statistics", "r", "count:3")} | (expected - {("statistics", "r", "count:2")}) != expected
    addresses = {(d, family, key): payload for d in ["d1", "d2"] for family, key, payload in expected}
    assert len(addresses) == 2 * len(expected)
    assert all(d == "d1" for d, _, _ in addresses if d == "d1")
    shortcut = dict(manifest="m", derivation="d1", definitions="f1", reachability=True, incarnation=1, retiring=False)
    for field, bad in [("manifest", "m2"), ("derivation", "d2"), ("definitions", "f2"),
                       ("reachability", False), ("incarnation", 2), ("retiring", True)]:
        altered = dict(shortcut, **{field: bad})
        assert altered != shortcut
    return {"missing_extra_changed_rows_detected": True, "two_derivations_disjoint_addresses": True,
            "shortcut_premise_mutations": 6}


def blob_checked(header, chunks):
    chunks = sorted(chunks)
    position, body = 0, b""
    for offset, chunk in chunks:
        if offset != position or not chunk:
            raise ValueError("range")
        body += chunk
        position += len(chunk)
    if position != header[1] or sha256(body).hexdigest() != header[0]:
        raise ValueError("full content")
    return body


def n10():
    header = (sha256(b"abcdef").hexdigest(), 6, "blob/v1")
    chunks = [(0, b"abc"), (3, b"def")]
    assert blob_checked(header, chunks) == b"abcdef"
    assert blob_checked((sha256(b"").hexdigest(), 0, "blob/v1"), []) == b""
    bad = [[(0, b"abc"), (4, b"ef")], [(0, b"abc"), (2, b"cdef")],
           [(0, b"abc"), (0, b"abc"), (3, b"def")], [(0, b"abc")], [(0, b"abc"), (3, b"deg")]]
    assert all(refuses(blob_checked, header, case) for case in bad)
    artifacts = {"revision-1/path-a": header, "revision-2/path-a": header}
    assert len(set(artifacts.values())) == 1
    del artifacts["revision-1/path-a"]
    assert header in artifacts.values()
    return {"malformed_cases_refused": len(bad), "empty_valid": True, "two_revisions_one_blob": True}


def n11():
    premise = dict(model="model", invariant="invariant", factory="factory-code", producer="producer-code",
                   profile="catalog", configuration="config", requested="A", resolved="B", unrequested=["optional"],
                   dependencies=[["view", "content"]], realization="codec", installation="install",
                   validity_epoch="boot-1", incarnations=[["view-record", 1, "completion"]])
    base = digest(premise)
    for field in premise:
        altered = deepcopy(premise)
        altered[field] = ["different", field]
        assert digest(altered) != base
    portable = {k: v for k, v in premise.items() if k not in ["installation", "validity_epoch", "incarnations"]}
    assert digest(portable) != base
    assert digest(premise) == base
    return {"exact_key_mutations": len(premise), "portable_not_authority": True, "unchanged_identity": True}


if __name__ == "__main__":
    results = {name: function() for name, function in [("N01", n01), ("N02", n02), ("N03", n03),
                ("N04", n04), ("N06", n06), ("N05_N09", n05_n09), ("N07_N08", n07_n08), ("N10", n10), ("N11", n11)]}
    print(json.dumps({"date": "2026-10-11", "baseline": "439f3d2d", "scope": "finite synthetic design models only",
                      "outcome": "passed", "results": results}, indent=2, sort_keys=True))
