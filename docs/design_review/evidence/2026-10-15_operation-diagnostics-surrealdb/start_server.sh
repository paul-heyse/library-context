#!/usr/bin/env bash
# Private, memory-engine SurrealDB probe server. Credentials come from a file
# outside the repository so they never appear in argv or run records.
# Usage: start_server.sh BINARY PORT PASSFILE OTLP_BASE_PORT QUERY_TIMEOUT
set -euo pipefail
binary=$1 port=$2 passfile=$3 otlp=$4 qtimeout=$5
export SURREAL_USER=root
SURREAL_PASS=$(cat "$passfile")
export SURREAL_PASS
export SURREAL_TELEMETRY_PROVIDER=otlp
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT="http://127.0.0.1:$((otlp + 1))"
export OTEL_EXPORTER_OTLP_METRICS_ENDPOINT="http://127.0.0.1:$((otlp + 2))"
export OTEL_EXPORTER_OTLP_LOGS_ENDPOINT="http://127.0.0.1:$((otlp + 3))"
export OTEL_BSP_SCHEDULE_DELAY=500
export OTEL_METRIC_EXPORT_INTERVAL=3000
export SURREAL_SLOW_QUERY_METRIC_THRESHOLD_MS=150
exec "$binary" start --no-banner --log=warn --bind "127.0.0.1:$port" \
  --query-timeout "$qtimeout" --transaction-timeout 10s \
  --slow-log-threshold 150ms --slow-log-param-deny secret \
  memory
