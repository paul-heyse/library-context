# Design review template

**Version 2.0 · 2026-09-24** · Core layer: repository- and domain-agnostic.
Standard: [design principles](design-principles.md) (`DP-nn`, gates `G1`–`G8`). Domain
profiles add content to the numbered slots below; the repository binding says where reviews
live and which profiles apply.

## Part 1 — The review contract

### Tier

| Tier | Use for | Slots |
|---|---|---|
| **Change review** | A bounded change, packet or PR within an accepted design | 1, 6, 7, 8, 12 (others only where a finding needs them) |
| **Design review** | Architecture decisions, new mechanisms or boundary contracts, plan-level or target designs | 1–12, each scoped to the subject |

Drop an irrelevant slot with a one-line reason tied to scope rather than inventing
requirements. Slots 6, 7 and 12 carry the review's weight and stay tabular in both tiers.

### Purpose

| Purpose | Judged against | Where a repository authority blocks a better design |
|---|---|---|
| **Conformance** | The current principles and the repository's current authorities | Follow the authority; note the conflict and whether it affects the decision |
| **Target** | The best available design for the functional target (principles §1) | The blocking text is itself a finding: state the target, the text to change and its route in slot 11 |

State the purpose in slot 1. The binding may set a default.

### What a claim can rest on

- **Document subject:** you have claims, not behaviour. You can establish whether the design is
  specifiable (two competent implementers would build the same semantics) and decidable (each
  invariant names where it is enforced and what is observable when violated). Evidence labels top
  out at *Proposed*, or *Interface-checked* where you inspected the named interface. A
  performance claim is a hypothesis unless it cites a measurement with conditions.
- **Code subject:** the executing path is the evidence; names, comments and documents are claims
  about it. Cite `file:line` and the expression. *Tested* and *Measured* name the test or
  benchmark and its conditions.
- **Both:** also report where document and code diverge, and which is authoritative. A design
  document maintained independently of its code is itself a second authority (DP-01).
- Anything a helper or subagent reports is a lead until you have read it yourself.

### Principle verdicts

For each applicable principle: **Satisfied** (the mechanism is identified, with where it is
enforced and what it rejects), **Violated** (a concrete situation loses, contests or fails to
enforce the required meaning), or **Unresolved** (the design neither establishes nor precludes
it). Unresolved is not a pass.

### Finding standard

Each finding is one row with these fields; the middle three make it a finding rather than a
preference. Reason from the evidence you have; do not demand probes or tests as proof of
diligence where the reasoning is sound.

| Field | Adequate when |
|---|---|
| ID | `F01`, `F02`, … stable within the review, so plans and follow-ups can cite `review#F03` |
| Finding | One defect, stated as a claim that could turn out wrong |
| Principles · gate | Only IDs the argument uses |
| Evidence or gap | Code: `file:line` and the expression. Document: section, quoted claim, and what is absent there |
| Consequence | Inputs or state → wrong, ambiguous or unrecoverable outcome. If this cannot be written, there is no finding |
| Correction | A direction with a rough surface area — not a patch |
| Verification | Optional: how one would tell the correction landed, where that is not obvious |

Group instances by structural cause: one cause with several instances is one finding.

### Severity order

1. **Correctness and authority** — failed gates, MUST gaps on in-scope behaviour.
2. **Library leverage and extension locality** — bespoke generic code, duplicated meaning (G8, §E).
3. **Measured cost.**

A finding that moves none of these is an observation; label it as one or cut it.
Over-construction is a finding too: bespoke machinery without a current requirement (DP-16).
A library capability is never over-construction merely for lacking a consumer.

### Decision rules

| Situation | Decision |
|---|---|
| No MUST gap and no failed or unresolved gate on in-scope behaviour | Accept |
| Only SHOULD deviations, each with an exception record | Accept scoped, with documented deviations |
| MUST gap or failed gate on behaviour the design claims | Revise — or Accept with that behaviour removed from the supported scope |
| Unresolved gate on in-scope behaviour | Not Accept; name the decision the author must make |
| G8 is the only failed gate | Revise |
| Competing authority, silent semantic loss, or an unbacked capability claim at the core | Reject or Revise, whatever else is strong |

Low code volume, elegance and performance never offset lost meaning, competing authority,
hidden effects, invalid reuse or unsupported behaviour.

### Profile additions

A profile adds rows, columns, journeys, gates or subsections **within** the slots below, each
marked with the profile's prefix. It never adds or removes a slot, so every review keeps the same
shape regardless of profile.

## Part 2 — The template

Replace bracketed prompts with evidence.

### 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | [Document path, code scope, or both] |
| Standard | [Core version; profiles; binding] |
| Tier · purpose | [Change / design] · [conformance / target] |
| Reviewer · date | [Accountable person or agent] · [date] |
| Decision | [From slot 12] |

**Outcome sought:** [What becomes possible, safer, simpler or measurably better.]
**Baseline:** [The existing design and its relevant limits.]
**Supported scope and non-goals:** [Capabilities and guarantees actually claimed.]
**Method and coverage:** [What was examined, what was not, what could not be verified and why —
so silence cannot be mistaken for assurance.]

### 2. Authority and identity map

| Fact or concept | Semantic type and identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| [Definition / binding / policy / observation / execution state / result] | | | | | |

**Opaque behaviour:** [What sits outside the declarative model, and which contract governs it.]
**Identity behaviour:** [Rename, reorder, reserialize, regenerate, split or merge, as applicable.]

### 3. Contracts and invariants

| Invariant or contract | Enforcement point | Failure behaviour | Evidence |
|---|---|---|---|

**Absence and outcome states:** [Which of not supplied / not applicable / not computed / unknown /
invalid / partial / failed are distinguished, and where.]
**Equivalence promised:** [Byte / structural / semantic / approximate, with tolerances.]

### 4. Derivation and execution

One row per nontrivial stage. Every cell you had to invent is a decision the design has not made.

| Stage | Semantic output and equality | Mechanism (library / routine / own code) | Inputs and observed dependencies (incl. membership, absence) | Structural vs value inputs | Reuse boundary | Termination / exactness / determinism | Effects, ownership, publication | Expected size and cost |
|---|---|---|---|---|---|---|---|---|

**Relationship structures:** [Which graphs or projections exist, and what their edges mean.]
**Boundaries:** [Schema, identity, ownership, batching and loss at each language, process or
storage boundary.]

### 5. Journeys

Trace the journeys relevant to the scope; skip the rest with a reason.

- **Ordinary extension:** a new entity, rule, provider or model. Count new semantic decisions,
  genuinely new code, and places meaning is re-expressed (§E).
- **Meaningful change:** a changed definition, policy or assumption. Identity, invalidation,
  retained artifacts, and what the change looks like at the level of meaning.
- **Boundary round trip:** meaning through serialization, a language bridge or another backend.
- **Interruption or failure:** cancellation, crash, retry, unsupported capability, invalid input.
  Observable state, valid partial outputs, effects already performed, recovery.

### 6. Gates

| Gate | Verdict (pass / fail / unresolved / n.a.) | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | | | |
| G2 Semantic fidelity | | | |
| G3 Validity | | | |
| G4 Hidden behaviour | | | |
| G5 Consistency and recovery | | | |
| G6 Transformation and reuse | | | |
| G7 Truthful capability claims | | | |
| G8 Library leverage | | | |
| [Profile gates] | | | |

### 7. Findings

| ID | Finding | Principles · gate | Evidence or gap | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|

**Applicability:** [Which pillars and profile principles bore on this scope and which did not,
with the reason.]
**Strengths that carry weight:** [What would break without them — not praise.]

### 8. Library-leverage ledger

Generic capabilities in scope that are, or would be, bespoke where a library plausibly applies
(DP-13, §F). Use judgment about which matter; this is not an exhaustive inventory.

| Capability | Bespoke code (location) | Candidate libraries or built-ins | Fit and gaps | Recommendation |
|---|---|---|---|---|---|

### 9. Alternatives

| Alternative | Meaning duplicated / extension locality | Bespoke code carried | Correctness and operational risk | Cost / performance evidence | Selected or rejected, and why |
|---|---|---|---|---|---|
| Current baseline | | | | | |
| Proposed design | | | | | |
| Library-owned alternative | | | | | |
| Simplest viable alternative | | | | | |

The library-owned and simplest rows may coincide; say so. When a simpler alternative wins, it is
usually the headline.

### 10. Verification plan

| Claim or risk | Evidence label | Test / analysis / benchmark | Conditions and expected result | Current result or gap |
|---|---|---|---|---|

Include negative, round-trip, incremental-versus-clean, differential and interruption checks as
the scope requires.

### 11. Authority changes and exceptions

**Required authority changes** (target reviews): [Blocking text · target · route.]
**Exception records** (SHOULD deviations, principles §H): [IDs · scope · reason · alternatives ·
consequence · compensating controls · evidence · owner · revisit trigger.]

### 12. Decision

**Decision:** [Accept / Accept scoped / Revise / Reject] — **Reason:** [the strongest evidence
and remaining limits.]

| Priority | Change | Findings | Acceptance evidence |
|---|---|---|---|
| Correctness first | | | |
| Library leverage and locality next | | | |
| Measured cost where justified | | | |

**Final check:** claims match evidence; supported scope matches implemented guarantees; the next
ordinary extension has a clear path.
