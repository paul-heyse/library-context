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
print(json.dumps({"python": sys.version, "cases": rows}, indent=2))
