#!/usr/bin/env bash
# VERSION + index trap on a versioned SurrealKV store, with SURREAL_SURREALKV_VERSIONED_INDEX
# off (control) and on. The probed representation does not rely on VERSION; this records whether
# the knob makes indexed time-travel reads correct. Each run uses its own capped, removed container.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
IMG=surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20
OUT="$HERE/raw/version-index.txt"
: > "$OUT"
for knob in false true; do
  name=w1-version-$knob
  docker rm -f "$name" >/dev/null 2>&1 || true
  docker run -d --rm --name "$name" --user root --memory 2g --memory-swap 2g -p 127.0.0.1:18801:8000 \
    -e SURREAL_SURREALKV_VERSIONED_INDEX=$knob "$IMG" start --unauthenticated --log warn \
    "surrealkv:///tmp/kv?versioned=true" >/dev/null
  until curl -sf 127.0.0.1:18801/health >/dev/null; do sleep 0.5; done
  q() { curl -s -X POST -H 'Accept: application/json' -H 'Surreal-NS: v' -H 'Surreal-DB: v' --data-binary @- 127.0.0.1:18801/sql; }
  echo 'DEFINE NAMESPACE v; DEFINE DATABASE v; DEFINE TABLE item SCHEMAFULL; DEFINE FIELD k ON item TYPE string; DEFINE INDEX ik ON item FIELDS k;
        CREATE item:a SET k = "a"; CREATE item:c SET k = "c";' | q >/dev/null
  sleep 1.2; T1=$(date -u +%Y-%m-%dT%H:%M:%S.%NZ); sleep 1.2
  echo 'DELETE item:c; CREATE item:d SET k = "d";' | q >/dev/null
  res=$(echo "SELECT VALUE id FROM item WHERE k = 'c' VERSION d'$T1';
              SELECT VALUE id FROM item WITH NOINDEX WHERE k = 'c' VERSION d'$T1';
              SELECT VALUE id FROM item WHERE k = 'd' VERSION d'$T1';
              SELECT VALUE id FROM item WHERE k = 'c';" | q | python3 -c 'import json,sys; print(json.dumps([r["result"] for r in json.load(sys.stdin)]))')
  echo "SURREAL_SURREALKV_VERSIONED_INDEX=$knob [index@T1 k=c, noindex@T1 k=c, index@T1 k=d (created later), index now k=c] => $res" | tee -a "$OUT"
  docker stop "$name" >/dev/null
done
