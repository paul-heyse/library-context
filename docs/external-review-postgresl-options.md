Given the kind of Rust systems work you’ve been doing, I would favor a SQL-first PostgreSQL stack rather than an ORM-first stack. The Rust/Postgres ecosystem is quite strong now, and there are three particularly good architectural choices.
Best Rust PostgreSQL libraries
Library	Best use	My view
SQLx 0.9	General-purpose async application access	Best default
tokio-postgres 0.7	Lowest-level async/native PostgreSQL access	Best foundation / maximum control
Cornucopia 1.0	PostgreSQL-specific, SQL-first, generated typed interfaces	Especially attractive for your style of system
deadpool-postgres	Pooling for tokio-postgres	Preferred companion to tokio-postgres
Diesel 2.3 + diesel-async	Strong Rust query DSL / ORM	Best compile-time ORM/query builder
SeaORM 2.0	Conventional application/domain ORM	Best higher-level async ORM
SeaQuery 1.0	Programmatically constructing dynamic SQL ASTs	Very useful specialized tool
postgres-types	Custom Postgres ↔ Rust type mappings	Important with raw tokio-postgres
pgrx 0.19	Writing PostgreSQL extensions in Rust	Essential if code runs inside PostgreSQL
pgvector	Rust mappings for PostgreSQL vector	Useful if vectors belong in PostgreSQL


SQLx 0.9 is probably the safest general recommendation. It has a pure-Rust PostgreSQL driver, async I/O, compile-time checked SQL macros, pooling, migrations, streaming, statement caching, LISTEN/NOTIFY, TLS, and support for native PostgreSQL data types without making you adopt an ORM model.

Yes. For the kind of Rust systems architecture you are building, I would standardize on a **`tokio-postgres` + Cornucopia stack** rather than accumulating SQLx, Diesel, and SeaORM alongside it. The latter are mostly alternative abstraction layers, not complementary dependencies.

My recommended combined Rust/PostgreSQL stack would be:

1. **`tokio-postgres` — the core PostgreSQL driver**

This should be the foundation.

```toml
tokio-postgres = "0.7"
```

The current release is 0.7.18. It is a native async PostgreSQL client with pipelining and exposes PostgreSQL relatively directly rather than imposing an ORM/query abstraction. :chatgpt-content-reference{index="0"}

This gives you the important primitives directly:

- connections
- transactions
- prepared statements
- parameter binding
- streaming result sets
- `COPY`
- `LISTEN` / `NOTIFY`
- PostgreSQL types
- authentication
- PostgreSQL protocol support

For your purposes, this is desirable because PostgreSQL remains an explicit architectural component rather than disappearing underneath an ORM.

---

2. **`postgres-types` — explicit Rust ↔ PostgreSQL type integration**

```toml
postgres-types = { version = "0.2", features = ["derive"] }
```

Current release is 0.2.14. :chatgpt-content-reference{index="1"}

Technically, `tokio-postgres` already depends on this, so you don't need a direct dependency merely to execute normal queries.

I would nevertheless add it explicitly because you are likely to define PostgreSQL-native domain types.

It lets you derive:

```rust
#[derive(ToSql, FromSql)]
enum Phase {
    Vapor,
    Liquid,
    Solid,
}
```

for PostgreSQL constructs such as:

```sql
CREATE TYPE phase AS ENUM (
    'Vapor',
    'Liquid',
    'Solid'
);
```

This is particularly useful if you're treating PostgreSQL as a strongly typed data substrate rather than just a generic row store.

It supports/customizes mappings for things such as:

- enums
- domains
- composite types
- arrays
- custom `ToSql`
- custom `FromSql`

So I consider this part of your base stack. :chatgpt-content-reference{index="2"}

---

3. **`deadpool-postgres` — connection pooling**

```toml
deadpool-postgres = "0.14"
```

Current release is 0.14.2. It is specifically a pooling layer around `tokio-postgres` and additionally provides prepared-statement caching around clients and transactions. :chatgpt-content-reference{index="3"}

That gives you:

```text
application tasks
      ↓
Deadpool
      ↓
tokio-postgres connections
      ↓
PostgreSQL
```

This is preferable to repeatedly creating connections or building your own pool lifecycle.

I'd use Deadpool rather than introduce another generalized pool like `bb8` unless another part of your architecture already standardized on BB8.

So:

```text
tokio-postgres
postgres-types
deadpool-postgres
```

form one coherent family.

---

4. **`cornucopia` — typed code generation from actual PostgreSQL SQL**

This is the piece I think is especially useful for what you're doing.

Install its generator:

```bash
cargo install cornucopia
```

Current release is 1.0.1. Cornucopia takes PostgreSQL queries, validates them against PostgreSQL, and generates strongly typed Rust interfaces. It explicitly supports PostgreSQL-native enums, arrays, custom types, synchronous/asynchronous operation, and connection pools. :chatgpt-content-reference{index="4"}

So instead of creating this manually:

```rust
let rows = client
    .query(
        "SELECT id, name, molecular_weight
         FROM component
         WHERE id = $1",
        &[&id],
    )
    .await?;
```

you can define your canonical query:

```sql
--! component_by_id
SELECT
    id,
    name,
    molecular_weight
FROM component
WHERE id = :id;
```

and have Cornucopia generate the Rust API.

The architectural relationship becomes:

```text
PostgreSQL schema
       +
canonical SQL
       ↓
Cornucopia
       ↓
generated strongly typed Rust
       ↓
tokio-postgres
```

This is a particularly nice model for a sophisticated system because **SQL stays SQL**.

You're not translating relational concepts into an ORM-specific pseudo-language, while still getting compile-time Rust interfaces.

---

5. **`tokio-postgres-rustls` — TLS without OpenSSL**

For TLS connections:

```toml
tokio-postgres-rustls = {
    version = "0.14",
    features = ["ring", "native-certs"]
}
```

or use `aws-lc-rs` instead of `ring` if that is what your broader Rust crypto stack uses.

Current version is 0.14.0 and it supports Rustls 0.23 / Tokio-Rustls 0.26. :chatgpt-content-reference{index="5"}

I prefer this over:

```text
postgres-native-tls
postgres-openssl
```

unless you specifically require OpenSSL.

That keeps your application stack largely Rust-native:

```text
tokio-postgres
      ↓
tokio-postgres-rustls
      ↓
rustls
```

I would **choose one PostgreSQL TLS connector**, not install several.

---

6. **`refinery` — migrations**

```toml
refinery = {
    version = "0.9",
    features = [
        "tokio-postgres",
        "tokio-postgres-rustls"
    ]
}
```

Current release is 0.9.2 and it directly supports `tokio-postgres` as well as its Rustls integration. :chatgpt-content-reference{index="6"}

I like Refinery here because it has a fairly narrow responsibility:

```text
database migration state
        ↓
versioned SQL migration files
        ↓
PostgreSQL
```

rather than trying to become your database abstraction.

You can have migrations such as:

```text
migrations/

V001__initial_schema.sql
V002__component_types.sql
V003__thermodynamic_models.sql
V004__indexes.sql
V005__constraints.sql
```

and embed them in the executable if desired. Refinery supports migrations as SQL files and embedded Rust resources. :chatgpt-content-reference{index="7"}

This fits nicely with Cornucopia because:

```text
Refinery
    owns schema evolution

Cornucopia
    owns typed query generation

tokio-postgres
    owns database communication
```

Those are cleanly separated concerns.

---

7. **`sea-query` — optional, for genuinely dynamic SQL**

I would install this only if your application has substantial queries that **cannot sensibly be known ahead of time**.

```toml
sea-query = {
    version = "1.0",
    features = [
        "backend-postgres",
        "derive",
        "with-uuid",
        "with-json",
        "with-time",
        "postgres-array"
    ]
}
```

Current version is 1.0.2. It provides a safe Rust AST for constructing SQL and integrates with PostgreSQL. :chatgpt-content-reference{index="8"}

The dividing line I would use is:

```text
Known query
    → SQL + Cornucopia

Dynamically constructed query
    → SeaQuery
```

For example, suppose an analytical interface accepts arbitrary combinations of:

```text
components
phases
property packages
temperature ranges
pressure ranges
model families
source provenance
validation status
```

and needs to dynamically build predicates and joins.

That is where SeaQuery becomes useful.

I would **not** use SeaQuery to replace ordinary hand-written SQL. Cornucopia is cleaner for that.

---

8. **`pgvector` — if PostgreSQL will contain embeddings**

Given your use of embeddings elsewhere, this is worth including if this database has any semantic-search responsibility.

```toml
pgvector = {
    version = "0.4",
    features = ["postgres"]
}
```

Current version is 0.4.2 and it directly supports Rust-Postgres, SQLx, and Diesel. :chatgpt-content-reference{index="9"}

With the `postgres` feature it maps PostgreSQL vectors into Rust:

```rust
use pgvector::Vector;

let v = Vector::from(vec![0.23, -0.14, 0.81]);
```

and allows normal `tokio-postgres` parameter binding.

So your stack remains:

```text
Rust Vector
    ↓
pgvector
    ↓
postgres-types / tokio-postgres
    ↓
PostgreSQL vector
```

rather than building a separate serialization layer.

I would only include this where embeddings actually belong in PostgreSQL. It doesn't imply that PostgreSQL should replace LanceDB or a specialized vector system for every workload.

---

9. **`testcontainers` + `testcontainers-modules` — database integration testing**

As a development dependency, I would strongly recommend this:

```toml
[dev-dependencies]
testcontainers = "0.27"

testcontainers-modules = {
    version = "0.15",
    features = ["postgres"]
}
```

The PostgreSQL module starts a real PostgreSQL instance using the official PostgreSQL container image. :chatgpt-content-reference{index="10"}

That means your integration tests can do:

```text
cargo test
   ↓
start disposable PostgreSQL
   ↓
run migrations
   ↓
populate test data
   ↓
execute actual queries
   ↓
assert behavior
   ↓
destroy PostgreSQL instance
```

For a serious data system, this is much more valuable than mocking the database interface.

Especially with:

- custom PostgreSQL types
- triggers
- constraints
- generated columns
- extensions
- transaction semantics
- isolation behavior
- SQL functions

a mock database is usually a poor substitute for actual PostgreSQL.

---

10. **`pgrx` — for Rust code that actually runs inside PostgreSQL**

I would install the tooling:

```bash
cargo install cargo-pgrx
```

and use the library in a separate extension crate when needed:

```toml
pgrx = "0.19"
```

Current version is 0.19.2 and supports PostgreSQL 13–18 plus PostgreSQL 19 prereleases. :chatgpt-content-reference{index="11"}

This should **not normally be linked into your main application crate**.

Instead I'd structure it like:

```text
workspace/
│
├── crates/
│   ├── core/
│   ├── database/
│   ├── thermodynamics/
│   └── ...
│
└── postgres-extensions/
    └── some_extension/
        ├── Cargo.toml
        └── src/
```

`pgrx` lets you write things like:

- PostgreSQL functions
- aggregates
- operators
- custom PostgreSQL types
- triggers
- extension-defined SQL
- background workers
- shared-memory functionality
- server-side algorithms

in Rust.

That can eventually become quite interesting if you identify operations where repeatedly shipping large relational datasets across the process boundary is inefficient.

---

### The complete dependency architecture I would use

So I would treat the stack as:

```text
                        APPLICATION
                            │
              ┌─────────────┴────────────┐
              │                          │
         Cornucopia                 SeaQuery
        static SQL                dynamic SQL
              │                          │
              └─────────────┬────────────┘
                            │
                   deadpool-postgres
                            │
                    tokio-postgres
                            │
              ┌─────────────┼──────────────┐
              │             │              │
       postgres-types    pgvector    tokio-postgres-
                                      rustls
              │             │              │
              └─────────────┴──────────────┘
                            │
                       PostgreSQL
                            │
                        pgrx
                    server-side Rust
```

And beside the runtime path:

```text
Refinery
    → schema migrations

testcontainers
    → integration testing

Cornucopia CLI
    → SQL → Rust code generation

cargo-pgrx
    → extension development
```

### What I would actually put into the workspace initially

A reasonable starting configuration would therefore look approximately like:

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

tokio-postgres = {
    version = "0.7",
    features = [
        "with-serde_json-1",
        "with-uuid-1",
        "with-time-0_3",
        "array-impls"
    ]
}

postgres-types = {
    version = "0.2",
    features = [
        "derive",
        "with-serde_json-1",
        "with-uuid-1",
        "with-time-0_3",
        "array-impls"
    ]
}

deadpool-postgres = "0.14"

tokio-postgres-rustls = {
    version = "0.14",
    features = [
        "ring",
        "native-certs"
    ]
}

refinery = {
    version = "0.9",
    default-features = false,
    features = [
        "tokio-postgres",
        "tokio-postgres-rustls"
    ]
}

sea-query = {
    version = "1.0",
    default-features = false,
    features = [
        "backend-postgres",
        "derive",
        "with-json",
        "with-uuid",
        "with-time",
        "postgres-array"
    ]
}

pgvector = {
    version = "0.4",
    features = ["postgres"]
}

[dev-dependencies]
testcontainers = "0.27"

testcontainers-modules = {
    version = "0.15",
    features = ["postgres"]
}
```

And separately:

```bash
cargo install cornucopia
cargo install cargo-pgrx
```

I would **not** add `sqlx`, `diesel`, `diesel-async`, `sea-orm`, `bb8-postgres`, the synchronous `postgres` crate, `postgres-native-tls`, or `postgres-openssl` to that same base stack. They are mostly substitutes for things already covered above.

The resulting philosophy is quite clean:

**Cornucopia for statically known queries, SeaQuery only for truly dynamic queries, Refinery for schema evolution, Deadpool for lifecycle/pooling, `tokio-postgres` as the single database transport, `postgres-types` for the type system, Rustls for transport security, Testcontainers for real integration tests, and `pgrx` when computation genuinely belongs inside PostgreSQL.**

That is the combination I would standardize on rather than SQLx if the goal is a **PostgreSQL-native, explicit, strongly typed Rust architecture with minimal abstraction leakage**.