# Python fixtures

Tiny Python packages that the analyzers read. They are **input data**: never imported, executed,
linted or type-checked. `just fixtures-check` only parses them, so a case can't silently become a
syntax-error case. Put intentional syntax errors under `_invalid/`.

One directory per case, named for what it pins down (e.g. `distinct_typevar_binders/`,
`non_ascii_offsets/`). Planned cases are in Initial_plan §7.2.
