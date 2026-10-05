#!/bin/bash
# usage: q.sh <db> <file-or-stdin>
curl -s -X POST -H 'Accept: application/json' -H 'Surreal-NS: a3' -H "Surreal-DB: $1" --data-binary @- http://127.0.0.1:18765/sql | python3 -c '
import json,sys
for i,r in enumerate(json.load(sys.stdin)):
    print(i, r.get("status"), json.dumps(r.get("result"))[:600])'
