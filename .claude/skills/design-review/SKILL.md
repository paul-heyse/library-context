---
name: design-review
description: Review a proposed design document or an active codebase against the Data Model–Based Design Charter (DM-01–DM-60, gates G1–G7), producing an evidence-grounded review document under docs/design_review/reviews/. Use when asked to review a design, assess a slice or increment before declaring it done, review an ADR that changes a §B decision, or answer "is this design any good".
argument-hint: <target> [focus] [depth]
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Task
---

# Design review — Data Model–Based Design Charter

Review a **proposed design** (document) or an **existing implementation** (code) against the
doctrine in `docs/design_review/design_principles/`, and produce one review document.

**How you conduct the analysis is yours to decide.** Where you start, what you read and in what
order, which tools you use, whether you delegate breadth — all of it is your call. What this
skill fixes is the other end: what the review has to establish, what a claim in it must be backed
by, and how the result is organized. Everything in
[`REVIEW_REFERENCE.md`](../../../docs/design_review/design_principles/REVIEW_REFERENCE.md)
beyond the principle index is a lens offered because it has proven useful, not a step you owe
anyone.

## The standard

| File (under `docs/design_review/design_principles/`) | Role |
|---|---|
| `DATA_MODEL_DESIGN_CHARTER.md` | Normative. DM-01–DM-60, gates G1–G7, dimensions, evidence vocabulary, §E–§G tests. |
| `AGENT_DESIGN_DIRECTIVE.md` | The governing objective and required approach, compressed. |
| `DESIGN_REVIEW_TEMPLATE.md` | The output shape (§1–§11). |
| `ADDENDUM.md` | **This repo's layer.** §B1–§B10 mapped to DM IDs and gates; the recurring review questions routed onto G1–G7; the three grading vocabularies kept distinct; the oracle tiers. |
| `REVIEW_REFERENCE.md` | DM index with MUST/SHOULD levels, lenses, finding calibration, and where the defect shapes land in this codebase. |

The charter is the standard; without it there is nothing to review against, so if it is missing,
stop and say so. `REVIEW_REFERENCE.md` §1 indexes the DM IDs with their requirement levels for
citation accuracy — it is a lookup table, not a substitute for the charter.

You are judging **meaning and authority**, not vocabulary, technology, or volume. Charter §G
exists because the most common way this review goes wrong is rating a design highly for fluently
using the doctrine's own words.

## Cadence

ADR-0001 sets when a review is owed, and it is deliberately not per change:

| Trigger | Depth |
|---|---|
| End of a slice that adds or changes a table family, adapter or projection | `compact` |
| An ADR that changes a §B decision | `standard`, on DESIGN.md plus the ADR |
| End of each increment (DESIGN §1.2) | `deep`, on the code plus DESIGN.md |

A fresh `design-reviewer` subagent ([definition](../../agents/design-reviewer.md)) usually runs
it, so the author of the change is not its reviewer. If two consecutive reviews find nothing,
that is ADR-0001's revisit trigger.

## Arguments

- **target** (required): a design document path (`docs/design/DESIGN.md`); a code scope
  (directory, crate, module, glob, file list); or both — a document plus the code claiming to
  implement it.
- **focus** (optional): principle groups `1`–`12`, specific DM IDs, or gate IDs to emphasize.
  Focus shifts where depth goes; it does not suppress a MUST-level defect found outside it.
- **depth** (optional): `compact` (a short assessment in the directive's style), `standard`
  (default), `deep` (adds adversarial journeys, a constructed counter-design, verification detail).
- **slug** (optional): kebab-case filename descriptor; infer it if omitted.

## Output

```
docs/design_review/reviews/design_review_{slug}_{YYYY-MM-DD}.md
```

Follow `DESIGN_REVIEW_TEMPLATE.md` §1–§11, scoped to the change. Drop sections irrelevant to the
scope with a one-line note rather than filling them with invented requirements; charter §A and
the directive both say the review machinery should not cost more than the decision warrants.
`REVIEW_REFERENCE.md` §4 sketches how the sections tend to compress by mode and depth.

Two additions the template's prompts make explicit and that are easy to skip:

- **Method and coverage** (in §1). What you looked at, what you did not, what you could not
  verify and why. A reader has to be able to tell "no defect here" from "not inspected" —
  otherwise the review's silences read as assurance they haven't earned.
- **Applicability note** (in §7). Which principle groups bore on this scope and which did not,
  with the reason tied to scope. The charter permits "not applicable" only for a stated reason;
  unapplied is not the same as irrelevant.

Sections §6 (gates), §7 (findings) and §11 (decision) carry the review's weight and stay tabular
at every depth.

Close by reporting to the user: scope and coverage, gate results, the top few findings in
severity order, the decision, and the file path.

## What the review has to establish

1. **Gates G1–G7, settled independently.** Pass, fail, unresolved, or not-applicable — each on
   its own evidence. They are not averaged, not offset by a strong dimension score, and not
   softened because the design is otherwise impressive. `ADDENDUM.md` §2 routes this project's
   recurring review questions onto them; a review that settles the gates has answered them, and
   one that answers the questions without settling the gates has not.
2. **A verdict per applicable principle, from three options.** *Satisfied* — the mechanism is
   identified and you can say where it is enforced and what it rejects. *Violated* — you can
   describe a concrete situation in which the required meaning is lost, ambiguous, contested, or
   unenforced. *Unresolved* — the design neither establishes nor precludes the requirement; the
   decision hasn't been made. Unresolved is the honest verdict for most document-stage gaps, and
   the charter is explicit that it is not a pass. Resist the pull to upgrade it because the
   surrounding design is good, or to downgrade it to a violation for rhetorical weight.
3. **Findings that survive contact with the design**, each naming an oracle. See below.
4. **A decision that follows from the gates**, per the calibration table below.
5. **Claims labeled at the strength the evidence supports** — charter §D's vocabulary, applied
   honestly, and never confused with the two other vocabularies (`ADDENDUM.md` §3).

### Claim strength is bounded by what you actually have

This is the constraint most worth getting right, because it determines whether the review's
conclusions mean anything.

**Reviewing a document**, you have claims, not behavior. You can establish whether the design is
*specifiable* and *decidable* — whether two competent implementers reading only this would build
the same semantics, and whether each invariant names a place it is enforced and something
observable when it is violated. You cannot establish correctness. Evidence labels therefore top
out at **Proposed**, or **Interface-checked** where you actually inspected the interface or
library surface named. A performance claim in a design document is a hypothesis (DM-39) unless it
cites a measurement with its conditions; say that rather than letting it pass.

**Reviewing code**, you have behavior. Docstrings, names, type aliases and design notes are
claims *about* the code; the path that executes is the evidence. Cite `file:line` with the actual
expression rather than a paraphrase, and only cite what you have read yourself — that applies
equally to anything a subagent hands you, which is a lead until you've looked. **Tested** and
**Measured** require naming the test or benchmark and its conditions; "there are tests" is not
Tested, and a green `just check` you did not run in this session is not either — re-run it.

**Reviewing both together**, the additional question is where `DESIGN.md`'s semantics and the
implementation's diverge, and which of them is authoritative. `DESIGN.md` is authoritative by
construction here, so a divergence is either a bug in the code or a stale spine — say which.
`Initial_plan.md` is **research input**, not authority: DESIGN.md may depart from it, but
DESIGN.md or an ADR should say so, and an unrecorded departure is a finding.

## Every finding names an oracle

A finding without an oracle regresses. `ADDENDUM.md` §4 sets out the tiers, chosen by asking
what the cheapest reproducible oracle is:

| Tier | Where | When |
|---|---|---|
| **test** | `cargo nextest` / pytest, incl. insta schema snapshots and the shared DataFusion validators | the behavior or invariant can be exercised |
| **`ast-grep` rule** | a rule in `rules/` plus fixtures in `rule-tests/` (`just rules-test`) | it is a code shape |
| **`just` recipe** | `just deps`, `just adr lint`, `just lint-agents`, … | it needs the whole repo or its metadata |
| **prose** | `AGENTS.md` | only when there is no mechanical oracle |

§7's *Verification* column names one of those, or says explicitly that none exists. There is no
blocking-hook tier (ADR-0001). Adding a rule to `rules/` or a test is frictionless on purpose,
because that is where findings are supposed to land.

## Judgment calibration

**Severity ordering.** Rank findings by: correctness and authority defects (G1–G5 failures,
MUST-level gaps on in-scope behavior) → semantic duplication and extension difficulty (charter
§E) → measured performance and cost. Let that ordering drive §11's priority column. A finding
that moves none of the three is an observation; label it as one or cut it.

**Proportionality cuts both ways.** The directive is explicit that a design should not become an
unnecessary platform to satisfy a checklist. Under-specification is a finding; so is machinery —
a registry, compiler, generator, IR layer, service, plugin system — that has no demonstrated
consumer or costs more than the risk it addresses (DM-56, DM-57, DM-58). §B10's exclusion list is
this repository's standing version of that judgment. Recommending *removal* is a legitimate and
sometimes the strongest outcome. A specialized algorithm behind a complete contract is aligned;
charter §F is the placement test, and "make it declarative" is not the default answer. Touching
many files is not a defect — the same meaning independently re-expressed across subsystems is.

**Unattacked guarantees are asserted, not verified.** Trying to break a claimed guarantee is
usually what separates a review that helps from one that summarizes. The situations that tend to
reward the attempt here: two authorities disagreeing (a `cpg-schema` spec and an adapter's
columns); an unvalidated batch reaching Delta; an interruption leaving tables written but no
manifest, or a manifest naming a table version that failed validation; a rerun under a new
analyzer revision reusing an old run's IDs; an inner join silently dropping unmapped endpoints;
an adapter claiming a fact family it only partly extracts; Pyrefly inference relabelled as
runtime dataflow. Whichever you attempt, record the
guarantees you did not attack as asserted, and say so in the Method note.

**The simpler alternative is part of the output.** Template §8 wants a real third row, and
charter §C asks for the comparison. Constructing one takes work and is easy to skip; when the
simpler alternative wins, that is usually the review's headline.

## Finding quality

Each §7 row carries six fields. The middle three are what make it a finding rather than a
preference:

| Field | What makes it adequate |
|---|---|
| Finding | One substantive defect, stated as a claim that could turn out to be wrong. |
| Principle IDs | Only IDs the argument actually uses. If deleting a citation wouldn't change the reasoning, it was decoration. |
| Evidence or gap | Code: `file:line` and the expression. Document: the section, the quoted claim, and what is absent at that exact point. "Not addressed anywhere" is a legitimate gap when stated precisely. |
| Consequence | A concrete situation — inputs or state → wrong, ambiguous, or unrecoverable outcome. If this sentence can't be written, there is no finding yet. |
| Proposed correction | A direction with a rough surface area. Not a patch, not a signature. |
| Verification | The oracle, by tier — or an explicit statement that none exists (DM-60). |

Group by **cause**, not symptom: several instances of one structural cause are one finding citing
several instances, ranked by the cause's severity rather than the instance count.

`REVIEW_REFERENCE.md` §3 has worked adequate/inadequate pairs, including the over-construction
finding that tends to go unwritten.

## Decision calibration (§11)

| Situation | Decision |
|---|---|
| No MUST gap and no failed or unresolved gate on in-scope behavior | Accept |
| Only SHOULD deviations, each with a §10 exception record | Accept scoped design with documented deviations |
| MUST gap or failed gate on behavior the design claims to support | Revise — or Accept with the supported scope explicitly narrowed to exclude it |
| Unresolved gate on in-scope behavior | Not Accept. Record it as unresolved and name the decision the author has to make |
| Competing authority, silent semantic loss, or an unbacked capability claim at the core | Reject or Revise, whatever the rest looks like |

Low code volume, elegance and performance do not offset lost meaning, competing authority, hidden
effects, inconsistent revisions, invalid reuse, or unsupported behavior.

## After the review

A review is **evidence, never authority**. It does not change `DESIGN.md`; the ADR that responds
to a finding does. So:

- A finding that calls for a decision → `just adr new <slug>` (or `just adr supersede` for a
  pivot), citing the review in its Context. A record may land at `status: proposed` while the
  review is outstanding.
- A finding that names a missing oracle → add it: a test, or a rule in `rules/` plus fixtures in
  `rule-tests/`. Neither needs an ADR.
- A finding deliberately not acted on → a row in a **Deferred** table inside the review (§10/§11)
  with the trigger that would reopen it, not a paragraph that nobody revisits. There is no
  separate register.

## When the review isn't finished yet

Worth a pass before writing the file — each of these has a way of being true right up until you
check:

- A finding somewhere has no concrete consequence written out, and is really a preference.
- A finding's Verification column names no oracle and doesn't say that none exists.
- A citation is doing no work in the argument that carries it.
- Something cited hasn't actually been read at the grain it's cited at.
- A gate verdict drifted toward the overall impression of the design rather than its own
  evidence, or *unresolved* got rounded to *pass*.
- An evidence label outruns what was inspected, or a charter §D label got applied to a gate.
- No simpler alternative was constructed, so §8's third row is a placeholder.
- Over-construction went unexamined while under-specification got all the attention.
- Strengths are stated as praise rather than as what would break without them.
- The coverage note doesn't distinguish what was skipped from what was clean.
- The author would learn the design is imperfect but not *where it is unsafe*.

## Failure modes in the output

A review with any of these is worse than none, because it presents a checklist as assurance:

- All 60 principles enumerated regardless of scope.
- Dimension scores computed while gates went unevaluated, or a score offsetting a failed gate.
- Alignment asserted from vocabulary — *canonical*, *typed*, *contract*, *declarative*,
  *zero-copy*. Charter §G lists the hidden defect to check behind each attractive claim.
- Findings with no consequence, or with no oracle and no admission that there is none.
- Platform machinery recommended for a scope that hasn't demonstrated the need — §B10 names what
  is excluded outright.
- *Tested* or *Measured* claimed without naming the test or benchmark and its conditions.
- **A mocked provider treated as a pass**, or a remembered green `just check` treated as a
  substitute for running it.
- **An empty result treated as proof that a capability is absent.**
- **Gate results converted into a quality percentage.**
- File or line counts offered as evidence of semantic duplication.
- Unverified secondhand evidence, from a subagent or from the design's own prose.

## Edge cases

| Situation | Suggested handling |
|---|---|
| Charter missing or unreadable | Stop and report. |
| Template, directive or addendum missing | Proceed against the charter; note the substitution. |
| Target ambiguous — document or code unclear | Ask. What the review can establish depends on it. |
| Scope larger than the depth supports | Narrow to the semantically load-bearing part, state the selection rule, say what was excluded. |
| `DESIGN.md` describes a phase that does not exist yet | Document-stage claims only; a spine section marked *Proposed* is honest, not a defect. Judge whether it is *decidable*, not whether it is built. |
| `DESIGN.md` and the code disagree | A finding against the pair. `DESIGN.md` is authoritative, so say whether the code is wrong or the spine is stale. |
| `DESIGN.md` departs from `Initial_plan.md` | Fine if DESIGN.md or an ADR says so; otherwise a finding, corrected by a sentence or an ADR, never by editing the research input. |
| A claim can't be verified with what's available | Record it as asserted, name the check that would settle it, put it in §9. Don't guess a verdict. |
| Nothing wrong found | Say so, with the coverage statement and what was examined. Don't manufacture findings. If this happens twice running, that is ADR-0001's revisit trigger firing. |
| Scope is a mechanical refactor or pure performance change | Say which burdens of proof apply and compress the semantic sections accordingly. |
