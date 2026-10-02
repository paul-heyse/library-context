#!/usr/bin/env python3
"""Back up the application database, or verify a backup in a disposable PG18 instance."""

import argparse
import json
import os
import subprocess
from pathlib import Path
from urllib.parse import parse_qs, unquote, urlsplit

# The retained services of the service baseline. Generation schemas are rebuilt from pinned
# inputs (ADR-0078), so their rows are not fingerprinted here.
TABLES = (
    "public._sqlx_migrations",
    "lctx_cache.specs",
    "lctx_cache.embedding_values",
    "lctx_ops.attempts",
    "lctx_ops.events",
)


def connection_env(url: str) -> dict[str, str]:
    parts = urlsplit(url)
    env = {k: v for k, v in os.environ.items() if not k.startswith("PG")}
    env.update(
        PGHOST=parts.hostname or "localhost",
        PGPORT=str(parts.port or 5432),
        PGUSER=unquote(parts.username or ""),
        PGPASSWORD=unquote(parts.password or ""),
        PGDATABASE=unquote(parts.path.lstrip("/")),
        PGCONNECT_TIMEOUT="10",
        PGTZ="UTC",
    )
    for key, values in parse_qs(parts.query).items():
        if key in {"sslmode", "sslrootcert", "sslcert", "sslkey"}:
            env["PG" + key.upper()] = values[-1]
        else:
            raise SystemExit("unsupported libpq backup URL option; use a dedicated configuration")
    return env


def protected(path: Path) -> None:
    if path.is_symlink() or not path.is_file() or path.stat().st_mode & 0o077:
        raise SystemExit(f"protected file requires mode 600: {path}")


def fingerprints(query, tables=TABLES) -> dict[str, dict[str, object]]:
    # Identifier list is fixed by the schema owner. Hash rows before aggregation, so large bytea
    # payloads do not accumulate in string_agg's state. These are recovery checks, not semantic IDs.
    return {
        table: json.loads(
            query(
                "SELECT json_build_object('rows',count(*),'digest',"
                "md5(coalesce(string_agg(h,'' ORDER BY h),''))) FROM "
                f"(SELECT md5(to_jsonb(t)::text) AS h FROM {table} t) q;"
            )
        )
        for table in tables
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["backup", "restore-drill"])
    parser.add_argument("archive", type=Path)
    parser.add_argument(
        "--config",
        type=Path,
        default=Path(
            os.environ.get(
                "LCTX_DATABASE_CONFIG", str(Path.home() / ".config/library-context/postgres.json")
            )
        ),
    )
    args = parser.parse_args()
    try:
        if args.command == "backup":
            from postgres_recovery import backup

            backup(args.config, args.archive)
        else:
            receipt = args.archive.with_suffix(args.archive.suffix + ".json")
            protected(receipt)
            version = json.loads(receipt.read_text()).get("format")
            if version == 4:
                from postgres_recovery import restore

                restore(args.archive)
            else:
                raise RuntimeError(
                    "unsupported backup format; rebuild the current design from pinned inputs"
                )
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        # Driver/utility error text can contain credentials, identifiers or row payloads.
        reason = str(error) if isinstance(error, RuntimeError) else type(error).__name__
        raise SystemExit(
            f"failed: PostgreSQL {args.command} ({reason}); archive retained for inspection"
        ) from None


if __name__ == "__main__":
    main()
