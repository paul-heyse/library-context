"""Native graph serving is pending the selected publisher and serving stages."""

from __future__ import annotations

import sys


def main(argv: list[str] | None = None) -> None:
    print("lctx-mcp: native graph serving is not implemented", file=sys.stderr)
    raise SystemExit(3)


if __name__ == "__main__":
    main()
