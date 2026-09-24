#!/bin/bash
# usage: variant.sh "<toml dependency lines>"
P=/tmp/claude-1000/-home-paul-library-context/32eabd30-fa2f-4969-9f50-f6f3537cdcb7/scratchpad/graph-probe/lockcheck
cp $P/Cargo.lock.orig $P/Cargo.lock
cat > $P/crates/zz-probe/Cargo.toml <<TOML
[package]
name = "zz-probe"
version = "0.1.0"
edition = "2024"
publish = false
[dependencies]
petgraph.workspace = true
$1
TOML
cd $P && cargo tree -p zz-probe --depth 1 -e normal 2>&1 | tail -n +1 | head -40
python3 $P/../lockdiff.py $P/Cargo.lock.orig $P/Cargo.lock
