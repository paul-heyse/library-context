# Generating comprehensive library overviews

Use this brief when asking ChatGPT or another research-capable author to generate a library
capability overview. Supply the library and desired research scope in the request. The instructions
are reusable across libraries; adapt the organization to the subject rather than treating the
suggested content as a fixed template or assessment checklist.

## Purpose and audience

Research and write a comprehensive, repository-agnostic overview of the requested library for
highly capable programming agents and engineers. Help the reader form a coherent picture of what
the library can do, recognize valuable capabilities they might otherwise overlook, and identify
what deserves deeper investigation for a problem they are solving.

Prioritize library-specific understanding: functional families, advanced mechanisms, extension
points, important distinctions and interactions. Explain enough to reveal architectural options
and integration implications, while keeping implementation detail available through precise
sources. Assume engineering competence; spend explanation on what is distinctive or easily
misunderstood about this library.

Comprehensiveness means covering the major capability territory, including significant advanced
or less obvious functionality. Depth should follow explanatory value. Choose the document's length
and organization accordingly, keeping the opening orientation easy to skim and avoiding repeated
summaries, generic tutorials or a bottom-up inventory of every API member.

## Keep the overview independent of repositories

Describe the library through general problem shapes and documented capabilities. Do not tie the
document to a particular repository, organization, current architecture, directory layout, local
dependency pins, active plan or implementation backlog. Do not claim what a repository installs,
uses, needs or should migrate to. Do not carry application assumptions from earlier conversations
or supplied examples into the reference.

Use generic scenarios to make capabilities concrete. Explain conditions under which a mechanism
may be valuable and the distinctions that could make it unsuitable. Keep system-specific adoption
decisions and priorities with the later design work that has current evidence about that system.
The overview should remain useful as those systems change.

User-specified technical emphasis can direct additional depth, while preserving the broad capability
map. Express that emphasis in terms of problems, workloads or features rather than repository
recommendations. Explain related packages and ecosystem integrations through their actual boundaries:
which capability belongs to this library, a companion package, an underlying engine or another service.

## Research scope and evidence

Research current primary sources rather than relying on remembered functionality. Use Context7
where available as the first documentation lookup, then read the relevant official documentation,
release notes, tagged upstream source or tests as needed. When Context7 is unavailable, research
primary sources directly. Use issue discussions for specific unresolved behavior with their
status and limits visible; announcements alone may not establish a usable API.

Follow the requested research scope and state the research date near the beginning. Separate
released behavior from development-branch, experimental, deprecated or planned functionality.

Explain consequential optional extras, build options, platform requirements or dependencies where they change availability. Describe these conditions without asserting a
reader's installed environment.

Connect substantial capability claims to sources that support them. Distinguish documented
guarantees, observations from source, reported limitations and your own architectural inferences
in ordinary language. Explain important uncertainty where the sources do not settle it. An empty
search does not establish the absence of a feature. Do not claim execution, testing or measurement
unless it actually occurred and its conditions can be stated.

Qualitative explanations of performance mechanisms are useful. Relate expected benefits to what
work, movement, preparation or coordination changes and under which conditions. Quantitative
speed, memory, throughput or capacity claims need an attributable measurement and its relevant
context. Preserve the scope of upstream benchmarks rather than transferring their results to
an unspecified application.

## Build a coherent capability picture

Begin with an explanation of the library's conceptual model, its main capability families and
the roles it can play in a system. Clarify important boundaries between its own responsibilities
and those of its callers or adjacent libraries. A compact capability map can connect problems,
features and the sections that develop them.

Organize the deeper discussion around coherent capabilities and problem families. Module names
and API identifiers help readers investigate; they need not determine the document's structure.
Give significant advanced features enough context to explain why someone would investigate them.
An overview dominated by familiar basic usage will miss much of its purpose.

For a capability, develop whichever of these connections help the reader:

- The problem or class of operation it addresses.
- The mechanism it provides, including useful API or feature names as search handles.
- The architectural options it opens or custom machinery it may make unnecessary.
- How it composes with other capabilities and which guarantees or opportunities survive that composition.
- Conditions, limits and tradeoffs that could change its suitability.
- Direct sources for deeper investigation.

These are explanatory considerations, not required fields for every feature. Several capabilities
may be best explained together. Use a small example when it makes a consequential distinction
clearer than prose; extensive implementation walkthroughs can remain in the linked documentation.

Consider both the individual feature and the larger operation it participates in. A combination
of native capabilities may enable a simpler design than isolated substitutions. Conversely,
interoperation may introduce conversions, lifecycle obligations or semantic gaps that affect the
benefit. Make the important interactions understandable without designing an application around them.

## Make limits and choices useful

Explain semantic distinctions that materially affect use. Depending on the library, these might
include information loss, identity, null or missing values, ordering, precision, approximation,
validation, errors, side effects, ownership, concurrency, resource lifetime or completeness.
Select the distinctions that matter instead of requiring every topic to have a section.

Show where superficially similar mechanisms support different needs, and why one might be chosen
over another. Discuss integration burden and extension behavior as well as runtime efficiency.
Performance is one source of value alongside correctness, expressiveness, composition, maintainability,
testability and reduced custom machinery.

Keep adoption guidance conditional and supported. Availability does not establish suitability,
and an advanced mechanism is not automatically preferable. Distinguish established constraints
from context-dependent judgment. Mention material limits near the capabilities they qualify;
a compact cross-cutting discussion can address interactions that would otherwise be repeated.

## Make the overview easy to use

Give the overview a clear library title, research date, skimmable orientation, coherent
capability discussions, and discoverable sources. Use headings, prose,
compact tables and occasional diagrams or examples where they improve understanding. Adapt the
shape to the library; no fixed number of sections, features or words is required.

Place direct, descriptive links near the claims and capabilities they support. Prefer
topic-specific primary pages over documentation homepages, with actual URLs available for later
research. A short resource map can help navigate substantial documentation without duplicating
every link in a second bibliography.

Keep code illustrations concise and readable, with preserved line breaks, and distinguish
illustrative sketches from executed examples. Use clean source links without tracking parameters
or source-card fragments.

Before delivering, reconsider whether the document gives a reader a broad mental model, useful
awareness of advanced capabilities, meaningful conditions for investigating them, and reliable
routes into deeper detail. Correct unsupported claims and accidental repository-specific
assumptions. Preserve genuine uncertainty rather than filling gaps with
plausible-sounding functionality. This is an editorial judgment, not a required scoring matrix,
probe campaign or new verification artifact.

## Example request

> Generate a comprehensive library overview using these authoring instructions.
>
> Library: [name and official project URL].
>
> Research scope: [requested capabilities or release scope, if any].
>
> Optional technical emphasis: [general capabilities or problem families deserving additional depth].
>
> Keep the overview repository-agnostic. Choose the organization and depth that best explain
> this library's capabilities and important interactions.
