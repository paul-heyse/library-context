Yes. The thing you were remembering is **Cornucopia**, and it is more directly code-generation-driven than most of the other PostgreSQL options we discussed.

Your attached review explicitly recommended the architecture:

> PostgreSQL schema + canonical SQL → Cornucopia → generated strongly typed Rust → `tokio-postgres`

and characterized Cornucopia as the layer for statically known SQL. :chatgpt-content-reference{index="0"} The same document ultimately recommends **Cornucopia for static queries, SeaQuery for dynamic SQL, and `postgres-types` for the Rust/PostgreSQL type system**. :chatgpt-content-reference{index="1"}

There is an important recent detail: **Cornucopia 1.0 absorbed the Clorinde fork**, including its rewritten code generator and accumulated improvements. Current Cornucopia therefore generates an actual Rust crate containing type-safe interfaces from SQL that it validates against PostgreSQL. :chatgpt-content-reference{index="2"}

For the architecture you are building, I would think of the Rust typed-data ecosystem in several layers.

### 1. Cornucopia: SQL → generated Rust types and interfaces

This is the most direct answer to what you remembered.

You write something conceptually like:

```sql
--! component_by_id
SELECT
    id,
    name,
    molecular_weight
FROM component
WHERE id = :id;
```

Cornucopia validates that query against PostgreSQL and generates the corresponding Rust query interface and return structures.

The key advantage is that **SQL remains the source of truth**. You aren't reproducing SQL semantics inside an ORM DSL.

Current Cornucopia explicitly supports PostgreSQL custom types, enums, arrays, custom Rust mappings, async/sync clients and pools. :chatgpt-content-reference{index="3"}

For your system, I still think this is the best database-access model:

```text
PostgreSQL schema
        │
        ├── canonical SQL queries
        │
        ▼
   Cornucopia
        │
        ▼
generated Rust crate
        │
        ▼
 tokio-postgres
```

That is actual **ahead-of-time source generation**, rather than just macros doing checks during compilation.

---

### 2. `postgres-types`: PostgreSQL types ↔ real Rust types

This is almost as important as Cornucopia for what you are trying to achieve.

It allows things such as:

```rust
#[derive(ToSql, FromSql)]
#[postgres(transparent)]
struct ComponentId(Uuid);
```

and mapping PostgreSQL:

```sql
CREATE TYPE phase AS ENUM ('vapor', 'liquid', 'solid');
```

to:

```rust
#[derive(ToSql, FromSql)]
enum Phase {
    Vapor,
    Liquid,
    Solid,
}
```

It supports PostgreSQL enums, domains, composites, transparent newtypes and custom conversions. :chatgpt-content-reference{index="4"}

That means you shouldn't reduce everything in your database API to:

```text
UUID
TEXT
DOUBLE PRECISION
INTEGER
```

and then reconstruct the semantics elsewhere.

You can have:

```text
ComponentId
StreamId
PropertyPackageId
EquationId
Phase
ModelFamily
ParameterSource
ValidationStatus
```

as genuinely distinct Rust/Postgres concepts.

That is especially valuable in the simulator.

---

## Other database code-generation options

There are three alternatives worth understanding, although I would **not** stack all of them with Cornucopia.

### Diesel: database schema → Rust type-level schema

Diesel's `print-schema` connects to the database and generates Rust `table!` definitions describing tables, columns and SQL types. It can also generate definitions for previously unknown database SQL types. :chatgpt-content-reference{index="5"}

This creates a type-level relational model that Diesel's query DSL uses to reject many invalid queries at compile time.

Conceptually:

```text
PostgreSQL
    ↓ introspection
diesel print-schema
    ↓
schema.rs
    ↓
typed Rust relational DSL
```

This is genuine generation, but it commits you much more heavily to Diesel's relational DSL.

**I would choose Diesel instead of Cornucopia, not alongside it**, unless a particular subsystem has unusual requirements.

### SeaORM: database schema → generated Rust entities

SeaORM's CLI can inspect PostgreSQL and generate Rust entity/model files for every table, including relationships and Serde derives. :chatgpt-content-reference{index="6"}

Conceptually:

```text
PostgreSQL schema
       ↓
sea-orm-cli generate entity
       ↓
Rust Entity / Model / ActiveModel
```

Excellent for conventional application/domain CRUD.

For your simulator and scientific data substrate, I think Cornucopia is a better fit because it keeps PostgreSQL semantics much more explicit.

### SQLx: compile-time SQL checking, but not really persistent code generation

SQLx sits somewhere between these models.

Its `query!` family sends the SQL to a development database at compile time so PostgreSQL itself validates the SQL and reports result-column types. :chatgpt-content-reference{index="7"} `query_as` then maps results onto your own Rust structures. :chatgpt-content-reference{index="8"}

So:

```text
SQLx
    = macro-expanded compile-time query typing

Cornucopia
    = explicit generated Rust query crate
```

I prefer Cornucopia for your architecture because generated code becomes an inspectable artifact rather than being hidden primarily behind procedural macros.

---

# More interesting: strengthen the type system above PostgreSQL

This is where I think there are several additional libraries you should seriously consider. They solve different problems from Cornucopia and therefore **are complementary rather than competing**.

## `nutype`: semantic scalar types with enforced invariants

This is one I would investigate seriously.

`nutype` generates proper Rust newtypes with validation and sanitization and can make it impossible to construct an invalid instance through its normal API. Validation is also applied during Serde deserialization. :chatgpt-content-reference{index="9"}

Instead of:

```rust
type MoleFraction = f64;
type Efficiency = f64;
type ComponentName = String;
```

you can represent:

```rust
MoleFraction(f64)       // finite, 0 <= x <= 1
Efficiency(f64)         // finite, perhaps bounds
ComponentName(String)   // nonempty/canonicalized
```

Then this becomes impossible:

```rust
let x: MoleFraction = MoleFraction::new(1.7);
```

This represents a fundamentally stronger architecture:

```text
primitive
   ↓
semantic newtype
   ↓
validated domain object
   ↓
database / serialization / computation
```

For database-facing types, I would either test `nutype`'s third-party derive support carefully or use ordinary Rust newtypes with `postgres-types` derives when direct PostgreSQL interoperability is paramount.

---

## `uom`: extremely relevant to your process simulator

This may actually be the most valuable addition outside the PostgreSQL stack.

`uom` provides **compile-time dimensional analysis with zero-cost typed quantities**. :chatgpt-content-reference{index="10"}

So Rust understands that:

```rust
Pressure
Temperature
MolarFlowRate
MassFlowRate
SpecificEnthalpy
Density
```

are not interchangeable `f64`s.

For example, its type system permits:

```rust
length / time
```

to produce velocity but rejects:

```rust
length + time
```

at compile time. :chatgpt-content-reference{index="11"}

For a scientific simulator I would very seriously consider a distinction like:

```text
storage representation:
    f64

scientific/domain representation:
    Pressure<f64>
    Temperature<f64>
    MolarEnergy<f64>
    ...

solver representation:
    normalized/scaled raw scalar
```

You don't necessarily want `uom` types penetrating the numerical solver kernels, but at **model-definition, persistence and compilation boundaries** they can eliminate entire classes of errors.

---

## `garde`: structural validation

`nutype` is excellent for local/scalar invariants.

`garde` is better for validating whole structures:

```rust
#[derive(Validate)]
struct PengRobinsonConfig {
    ...
}
```

It generates validation logic using `#[derive(Validate)]`. :chatgpt-content-reference{index="12"}

I'd use the dividing line:

```text
nutype
    → this individual value cannot be invalid

garde
    → this assembled object/configuration cannot be invalid
```

For example:

```text
MoleFraction
    → nutype

FlashSpecification
    → garde

PropertyPackageConfiguration
    → garde

BinaryInteractionParameterSet
    → garde
```

---

# Schema generation in both directions

There are two libraries that fit exceptionally well if you want your data architecture to become more self-describing.

## `schemars`: Rust → JSON Schema

Given:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
struct PropertyPackageDefinition {
    ...
}
```

Schemars generates a JSON Schema describing the serialized structure. :chatgpt-content-reference{index="13"}

This gives you:

```text
Rust canonical type
       ↓
   Schemars
       ↓
JSON Schema
       ↓
configs / external APIs /
validation / tooling / UI /
LLM structured output
```

For the kind of highly structured simulator metadata you are building, this is extremely attractive.

It means the same Rust representation can programmatically describe itself to:

- config systems
- Python bindings
- UI generators
- import/export tooling
- AI agents
- validation systems
- external APIs

without separately maintaining another schema.

## `typify`: JSON Schema → Rust

Typify goes in the opposite direction.

It currently supports CLI generation, macros, `build.rs` generation and persistent generated source files from JSON Schema. :chatgpt-content-reference{index="14"}

So:

```text
external authoritative JSON Schema
            ↓
          typify
            ↓
        Rust types
```

This becomes valuable for standards or external interfaces where **you do not control the canonical schema**.

I wouldn't use both directions indiscriminately. I'd establish an ownership rule:

```text
We own concept
    → Rust type is authoritative
    → schemars exports schema

External standard owns concept
    → external JSON Schema authoritative
    → typify imports Rust representation
```

That avoids schema synchronization hell.

---

# Typed identity/indexing is another major opportunity

For your simulator compiler in particular, I would add one of these patterns rather than passing `usize` everywhere.

## `typed-index-collections`

It provides:

```rust
TiVec<ComponentIdx, Component>
TiVec<StreamIdx, Stream>
TiVec<EquationIdx, Equation>
TiVec<VariableIdx, Variable>
```

instead of:

```rust
Vec<Component>
Vec<Stream>
Vec<Equation>
Vec<Variable>
```

with every one indexed by an undifferentiated `usize`.

The compiler will reject accidentally indexing the component array with a stream index. :chatgpt-content-reference{index="15"}

This is **extremely relevant to your equation compiler**.

You could have:

```rust
struct ComponentIdx(usize);
struct PhaseIdx(usize);
struct VariableIdx(usize);
struct ConstraintIdx(usize);
struct UnitOpIdx(usize);
```

and therefore make:

```rust
variables[component_idx]
```

a compile error.

For dense numerical structures, I'd favor this over hash-map-like identity systems.

## `slotmap`

When entities need dynamic creation/deletion with stable handles, `slotmap` is complementary.

It can generate distinct key types:

```rust
new_key_type! {
    struct StreamKey;
    struct UnitKey;
    struct PortKey;
}
```

and prevents using a key from one entity collection on another. :chatgpt-content-reference{index="16"}

I'd use:

```text
typed-index-collections
    → dense compiled model / numerical arrays

slotmap
    → mutable model authoring / registries / long-lived object graph
```

That separation could work extremely well for your flowsheet compiler.

---

# A few smaller but useful type-system enhancers

**`bon`** generates type-safe builders where missing mandatory fields or setting a field twice is a compile-time error. :chatgpt-content-reference{index="17"} This could be useful for complicated model definitions with many optional/required parameters.

**`strum`** is useful for closed scientific/model enums. It generates iteration, string parsing, discriminants, variant arrays and related functionality from Rust enums. :chatgpt-content-reference{index="18"}

**`enum-map`** lets you efficiently index arrays using enum variants. :chatgpt-content-reference{index="19"} This is potentially elegant for things such as:

```rust
EnumMap<Phase, PhaseState>
EnumMap<Property, PropertyValue>
```

when the domain really is closed.

**`serde_with`** complements Serde when you want standardized serialization representations without filling your actual domain model with conversion boilerplate. Its current release is 3.24.0. :chatgpt-content-reference{index="20"}

**`derive_more`** removes boilerplate around semantic newtypes by deriving `From`, `Into`, `Display`, etc. :chatgpt-content-reference{index="21"} I see this as convenience rather than architectural infrastructure.

---

# Schema-first IPC is a separate category

If portions of the simulator eventually communicate across processes or languages, **Protobuf + `prost-build`** gives another form of genuine code generation:

```text
.proto
  ↓
prost-build
  ↓
generated Rust structs
```

`prost-build` is specifically designed as build-time `.proto` → Rust generation. :chatgpt-content-reference{index="22"}

I wouldn't use Protobuf for your internal canonical domain model, but it can be excellent for versioned RPC/service boundaries.

---

## What I would actually add to your architecture

If I take your process-simulator architecture rather than treating this as a generic Rust application, my preferred stack becomes:

```text
DATABASE / RELATIONAL CONTRACT
────────────────────────────────────────
PostgreSQL
postgres-types
Cornucopia
tokio-postgres
SeaQuery                     dynamic queries only


DOMAIN TYPE SYSTEM
────────────────────────────────────────
Rust structs/enums           primary domain representation
nutype / explicit newtypes   semantic primitive types
uom                          physical dimensions / units
garde                        cross-field/object validation
strum                        closed enumerated domains


IDENTITY / COMPILED MODEL
────────────────────────────────────────
typed-index-collections      dense solver/compiler IDs
slotmap                      mutable model entity handles
enum-map                     small closed-domain arrays


SCHEMA / INTERCHANGE
────────────────────────────────────────
serde                        serialization
serde_with                   boundary representations
schemars                     Rust → JSON Schema
typify                       JSON Schema → Rust, when externally owned
prost-build                  service/RPC schema → Rust


CONSTRUCTION
────────────────────────────────────────
bon                          compile-time-safe builders
```

I would **not** add Diesel, SeaORM or SQLx to that stack. They are mostly alternatives to the Cornucopia/Postgres-access layer, which is exactly the conclusion in the attached review as well. :chatgpt-content-reference{index="23"}

The especially interesting additions for your project, beyond what we discussed previously, are **`uom`, `nutype`, `schemars`, `typed-index-collections`, and `garde`**. Together with Cornucopia and `postgres-types`, those let you push type safety much further than merely "typed database rows": into physical dimensions, semantic IDs, valid value ranges, structural invariants, compiled-model indexing, and machine-readable schemas. That is the direction I would take if the objective is to make illegal simulator states difficult or impossible to represent.