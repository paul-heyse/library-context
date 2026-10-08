#!/usr/bin/env python3
"""Executable Cargo RUSTC_WRAPPER entry point (system Python 3.12 compatible)."""

from compile_profile_capture import wrapper_main

if __name__ == "__main__":
    raise SystemExit(wrapper_main())
