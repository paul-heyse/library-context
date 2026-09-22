# Agent directive: data model–based design

Use `DATA_MODEL_DESIGN_CHARTER.md` as the normative reference for architecture, implementation, and refactoring. Refer to stable principle IDs in your findings. Apply the principles to the task’s actual scope; do not build an unnecessary platform to satisfy a checklist.

## Governing objective

Make domain meaning, relationships, constraints, capabilities, and important behavior explicit in a canonical, typed, versioned model. Derive routine construction, execution representations, adapters, and inspection views from shared contracts. Minimize independently maintained semantic decisions and coordinated procedural edits—not source lines or the amount of handwritten algorithmic code.

One unified logical substrate does not mean one universal table, one execution engine, or one memory layout. Specialized kernels and physical representations are appropriate when their contracts, dependencies, and mappings to the canonical model are explicit.

## Required approach

Before designing APIs or selecting libraries, identify the authoritative concepts and their semantic types, relationships, invariants, identities, and lifecycles. Separate definitions, bindings, policies, observations, mutable execution state, results, and derived artifacts.

Represent reusable structure and important selection policies as declarations when doing so improves correctness, composability, or change locality. Preserve high-level structure until a consumer needs expansion. Use explicit capability resolution, typed transformations, and inspectable plans. Do not hide important meaning in reflection, naming conventions, ad hoc scripts, implicit defaults, arbitrary callbacks, or serialization glue.

Keep inspection semantically non-mutating. Declare side effects, ambient inputs, nondeterminism, and failure behavior. Scope mutable runtime workspaces and publish coherent outcomes. Make interrupted execution, unsupported capabilities, invalid states, retries, and partial outputs distinguishable.

Trace transformations and results to their sources. Track every dependency needed for safe reuse, including policies, provider versions, and assumptions. Ensure adapters preserve meaning or explicitly declare a selected loss or approximation. Validate capability support end to end rather than assuming a library or datatype supplies missing semantics.

Use workload-appropriate execution mechanisms and data layouts. Hoist stable preparation out of repeated execution. Cross expensive boundaries in suitably coarse, typed units. Evaluate performance end to end; describe unmeasured benefits as hypotheses.

Generate repeated mechanical artifacts from schemas and operation contracts where useful. Keep specialized algorithms as ordinary code behind complete contracts. Compare major proposals with a simpler viable alternative and justify each additional architectural layer.

## Review output

Use `DESIGN_REVIEW_TEMPLATE.md` for substantial changes. For smaller changes, compress the same reasoning into a short assessment. Report the objective and scope, canonical model and authority, lifecycle and transformations, applicable principle findings, failure behavior, verification evidence, tradeoffs, and recommended changes.

For each important finding, cite the relevant principle IDs and concrete evidence or an explicitly stated gap. Do not claim that a design satisfies a principle merely because it uses a fashionable technology or vocabulary. Distinguish proposed, interface-checked, implemented, tested, measured, and formally established claims.

Check the charter’s acceptance gates before calculating any optional score. Do not allow lower code volume or better performance to compensate for lost meaning, competing authority, hidden effects, inconsistent revisions, invalid reuse, or unsupported behavior.

The practical extension test is: does a normal change add one authoritative declaration, any genuinely new implementation, and focused tests—or require the same meaning to be re-expressed across several subsystems? Prefer the former, while keeping the architecture proportionate to actual needs.
