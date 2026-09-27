"""Independent runtime controls, never imported by the analyzer or run from fixtures."""

import json
import sys

assert sys.version_info[:3] == (3, 14, 7), sys.version
rows = []
for keyword_only in (False, True):
    separator = ", *, " if keyword_only else ", "
    for removed in (False, True):
        for supplied in (False, True):
            namespace = {}
            exec(f"def identity(value{separator}unused=True):\n    return value\n", namespace)
            identity = namespace["identity"]
            if removed:
                if keyword_only:
                    identity.__kwdefaults__ = None
                else:
                    identity.__defaults__ = None
            marker = object()
            try:
                actual = identity(marker, unused=False) if supplied else identity(marker)
            except TypeError:
                assert removed and not supplied
                outcome = "TypeError"
            else:
                assert actual is marker and (not removed or supplied)
                outcome = "identity"
            rows.append({"keyword_only": keyword_only, "default_removed": removed,
                         "argument_supplied": supplied, "outcome": outcome})
fresh = []
for keyword_only in (False, True):
    separator = ", *, " if keyword_only else ", "
    for default in (False, True):
        for supplied in (None, False, True):
            namespace = {}
            suffix = "" if supplied is None else f", enabled={supplied!r}"
            program = (
                "def caller(marker):\n"
                f"    def choose(value{separator}enabled={default!r}):\n"
                "        if enabled:\n            return value\n        return None\n"
                f"    return choose(marker{suffix})\n"
            )
            exec(program, namespace)
            marker = object()
            result = namespace["caller"](marker)
            selected = default if supplied is None else supplied
            assert result is (marker if selected else None)
            fresh.append({"keyword_only": keyword_only, "default": default,
                          "supplied": supplied, "outcome": "identity" if selected else "none"})

events = []


def make_default():
    events.append("definition")
    return True


def factory():
    def identity(value, unused=make_default()):
        events.append("body")
        return value

    return identity


identity = factory()
assert events == ["definition"]
marker = object()
assert identity(marker) is marker and identity(marker) is marker
assert events == ["definition", "body", "body"]
for expression, raises in (("missing_default()", True), ("True or missing_default()", False)):
    namespace = {}
    exec(f"def caller(value):\n    def inner(item, unused={expression}):\n        return item\n    return inner(value)\n", namespace)
    try:
        actual = namespace["caller"](marker)
    except NameError:
        assert raises
    else:
        assert not raises and actual is marker
print(json.dumps({"python": sys.version, "mutation_cases": rows, "fresh_cases": fresh,
                  "definition_events": events, "unused_default_raise_and_skip": "passed"}, indent=2))
