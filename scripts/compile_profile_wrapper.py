#!/usr/bin/env -S uv run --no-project --offline --no-python-downloads --python 3.14.7 python
"""Executable Cargo RUSTC_WRAPPER entry point (pinned Python 3.14.7)."""

from compile_profile_capture import wrapper_main

if __name__ == "__main__":
    raise SystemExit(wrapper_main())
