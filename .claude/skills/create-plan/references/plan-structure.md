# Suggested plan structure and examples

Use this outline to make the plan useful to an implementing agent. Adapt it to the subject and
the repository's conventions; it is not a required set of headings or a completion checklist.
Technical design may need several subject-specific documents, while a small plan can combine
most of the outline into a few paragraphs and a work table. Choose content for the understanding
it gives the reader; the same explanation need not appear under multiple headings.

Identify the reference review or parent plan, inspected baseline and date, and the plan's
evidence status near the beginning. Explain which document owns architecture, sequencing or
finding disposition where those responsibilities are split. An approved design remains distinct
from implemented or verified behavior.

| Part | What it establishes |
|---|---|
| Purpose, scope and basis | Intended outcome, completion boundary, reference material, governing architecture and credible changes the target should accommodate |
| Current baseline and affected foundations | What exists, relevant evidence limits, reusable components, and conclusions from the focused dependency assessment |
| Target design and contracts | Responsibilities, semantic owners, interactions, preserved guarantees, intentional behavior changes and support limits |
| Choices and alternatives | Material decisions, library fit, benefit mechanisms, ownership costs, assumptions and conditions for reconsideration |
| Execution sequence | Coherent work packages, specific prerequisites, affected owners, available capabilities and completion evidence |
| Migration and adoption | Consumer changes, preservation and retirement, intermediate states and operational transitions where relevant |
| Verification and acceptance | Revealing scenarios, local and assembled acceptance, applicable review points, and measurements with their conditions and completion role |
| Finding disposition and current state | Traceability to source findings, open decisions, deferred work and the next executable step |

## One document or a coordinated series

In one document, an overview can establish the combined target before subject-specific design
and execution sections. For a series, use a coordinator to hold that overview and the relationships
among supporting plans. Give each supporting plan enough context to develop its own design and
packages without repeating the whole series.

A possible division of responsibility is:

| Coordinator | Supporting plan |
|---|---|
| Combined outcome, scope and reading route | Local context and concrete target |
| Shared decisions and cross-plan contract ownership | Owned decisions and consumed contracts |
| Dependency structure and one useful execution route | Packages with exact prerequisites and implementation visions |
| Overall completion and finding-disposition ownership | Local progress and evidence, linked to broader acceptance |

This is a useful division, not a new hierarchy of architectural authority. Existing architecture
and decision records retain their roles. Integrated qualification may deserve its own supporting
document when its cross-cutting scope needs substantial explanation; smaller plans can retain it
in the main document. Neither choice fixes the number or order of implementation phases.

## Package summaries and dependency explanations

A compact package table can expose what changes, its prerequisites and its completion boundary.
Use stable identifiers when other documents refer to packages, with links to their explanations.
Add execution responsibility or status where useful; assign each live status fact one owner.

| Package | Required input | Delivered capability and completion boundary |
|---|---|---|
| Request interpretation | Settled request meaning and existing context lookup | One implemented interpretation operation used by the affected entry points, with focused success and refusal controls |
| Execution migration | The working validated-request product | Execution consumes resolved identities without repeating interpretation; replaced consumer paths are retired |
| Durable consumer migration | Validated-request contract and the supported storage transition | Persistence retains the same meaning and relevant failures; affected readers and writers migrate |

The last two packages may be logically independent once their consumed contracts are available.
Shared file changes can still require coordination. Conversely, agreeing the request shape may
allow consumer design to begin before it permits working integration. Explain these differences
when a simple ordered list would hide them.

For an apparent cycle, identify the actual products exchanged. A reporting area may need an
execution result while execution needs an early diagnostic contract owned by that reporting
area. Implementing the diagnostic contract first can unblock execution without waiting for all
reporting work. State the prerequisite's required behavior and remaining reporting scope.

Explain why an order is useful when several routes are valid. Establishing a widely consumed
contract or resolving a premise that could change later work may deserve priority. Describe
the capability available after a milestone, and whether that milestone is a real barrier or
simply a convenient grouping. A dependency diagram helps when branches and joins carry more
information than row order.

## Implementation visions

Develop the package beyond its table entry where the target needs explanation. For example:

> The operation owner resolves a submitted request against its selected context and returns a
> validated request carrying the resolved identities. Execution and persistence consume this
> product. Resolution failures retain the submitted location and the failed condition, even
> when no execution result exists. Existing entry points move to this operation and remove their
> independent interpretation rules. Valid requests retain their meaning; ambiguous references
> now receive an explicit rejection.

This paragraph supplies ownership, dataflow, failure behavior, consumer migration and the intended
behavioral difference. It leaves the local representation and function layout to the implementer.
Add the context-sensitive details the actual operation needs; no fixed paragraph pattern is required.

A revealing example can sharpen acceptance:

> Two entry points resolving the same request in the same context produce equivalent admitted
> meaning. A reference valid only in another context fails before execution and retains its source
> location. A storage round trip preserves those resolved identities without reapplying current
> defaults.

Name the complete affected consumer scope separately from representative examples. A few controls
can demonstrate the mechanism while migration still has to cover all relevant entry points.
Where a round trip could preserve the same bug in both directions, choose an independently specified
expectation for the meaning at risk. Select checks proportional to the actual uncertainty.

## Reasoning that helps implementation adapt

A small amount of rationale can preserve the reason for an architectural boundary:

> Adding another delivery channel supplies its channel-specific transport and capability
> declaration. Existing scheduling and retry decisions continue through the shared operation
> contract. A channel with genuinely different delivery guarantees would require revisiting
> that contract, rather than adding exceptions in unrelated consumers.

Distinguish this expected extension experience from a promise to implement every future channel.
It explains the variation the design accommodates and the condition that would change the design.

Similarly, connect benefit claims to their conditions and costs:

> Preparing an index once removes repeated parsing and dependency resolution for requests over
> stable source revisions. It introduces retained memory and invalidation responsibilities. If
> inputs change on every request, reassess the preparation lifecycle before extending the cache.
> Evaluate the benefit with representative revision reuse and end-to-end resource measurements.

Keep such reasoning with the decision it explains. An unresolved premise should identify the
work it constrains and the evidence that would settle it. Avoid a separate register when a short
local explanation suffices.

## Transitions, evidence and current state

For consequential transitions, explain the intermediate system states and the path to adoption.
Identify any required consumer coordination, data migration, activation or derived-artifact
rebuild. Describe useful pause points or required coordinated cutovers, including recovery where
interruption matters. Apply the project's preservation and retirement policy; these considerations
do not imply universal requirements for compatibility paths or deployment stages.

Separate future acceptance requirements from actual evidence. Identify local checks, assembled
journeys and measurements by what they establish and when they are needed. Follow repository
policy for execution timing and evidence reporting. Optional or deferred measurements should have
a clear purpose and trigger, without obscuring required completion evidence.

As execution proceeds, maintain concise state, decisions and next steps at their declared owner.
If a package consumes an early slice of another plan, record which contract and consumers are
complete, link the evidence owner, and state the enclosing package's remaining work. Local
completion does not automatically close a finding shared with other packages or establish full
system qualification. Update affected links and dependency descriptions when the design changes,
and move enduring meaning to its established owner before retiring the plan under local policy.
