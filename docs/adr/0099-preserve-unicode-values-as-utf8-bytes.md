---
id: ADR-0099
title: Preserve Unicode values as UTF-8 bytes
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A new domain field admits Unicode text with embedded NUL
---

## Context

The full pinned catalog pilot exposed a Python string literal containing NUL. Rust strings and
Arrow UTF-8 preserve it, but PostgreSQL rejects it in `text` (SQLSTATE 22021). Python mapping
keys and native expanded keyword slots can contain the same value. [DESIGN §15](../design/sections/semantic-model.md)
owns their typed meaning and mechanical physical lowering.

## Options

1. Declare a nominal Unicode value with lossless UTF-8 binary lowering.
2. Escape text for PostgreSQL and unescape readers. This makes physical text comparisons depend
on an extra encoding contract and introduces a second interpretation at the store boundary.
3. Drop NUL or downgrade supported literals. This changes or discards supported evidence.
4. Lower every text column to binary. This unnecessarily changes identifier and SQL text contracts.

## Decision

Choose option 1. `Utf8Text` owns a private valid Rust string and serializes its UTF-8 bytes.
Arrow Binary and PostgreSQL bytea follow from its declared scalar. Decoding rejects invalid UTF-8.
Literal strings, TypedDict keys, record field names and native callable slot names use this value.
Its semantic key encoding remains text, preserving the distinction between Unicode strings and
byte literals and preserving existing identities. Ordinary identifier/display columns retain their
existing declarations. No escaping, legacy reader or parallel store is introduced.

## Consequences

This is a schema migration and requires a rebuilt store. NUL, empty, non-ASCII and ordinary values
must round-trip through shared Arrow decoding and real PostgreSQL. Native fixtures and the full
facts pilots qualify the producer path; downstream P3–P5 remain unavailable.
