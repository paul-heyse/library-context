# FastMCP 4.0.5 behavior-channel prevalence, 2026-09-24

`uv run python prevalence.py <path-to-fastmcp-package-dir>` (stdlib `ast` only; never imports
fastmcp). `prevalence.out` is the recorded output behind the pivot plan's §2 baseline: field
assignments from `__init__`, pydantic classes, settings reads, ContextVars, registry writes,
exception conversion, dynamic access, the async share.
