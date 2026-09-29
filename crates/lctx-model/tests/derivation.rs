//! The derivation index and projection declarations (DESIGN §15.9–§15.10).

use lctx_model::calls::CallPolicy;
use lctx_model::ddl::DdlConfig;
use lctx_model::derivation::{self, DerivationSource, Premise};
use lctx_model::id::Id;
use lctx_model::projection::{Direction, ParallelArcs, ProjectionDecl, Unresolved};
use lctx_model::vocab::EntityKind;
use lctx_model::{model, relation};

relation! {
    /// Conclusions.
    Transfers, TransfersRow = "transfers" {
        layer: L2, family: "sample", stage: "compose",
        fidelity: Derived, polarity: May, coverage: "generation",
        key: [transfer_id],
    }
    row { transfer_id: Id }
}

relation! {
    /// Steps: one composition of a caller transfer, a binding and a callee transfer.
    CompositionSteps, CompositionStepsRow = "composition_steps" {
        layer: L2, family: "sample", stage: "compose",
        fidelity: Derived, polarity: May, coverage: "generation",
        key: [step_id],
    }
    row {
        step_id: Id,
        conclusion_id: Id [ref transfers.transfer_id],
        caller_transfer_id: Id [ref transfers.transfer_id],
        binding_id: Option<Id>,
        callee_transfer_id: Id [ref transfers.transfer_id],
    }
}

model! { Transfers, CompositionSteps }

const SOURCES: &[DerivationSource] = &[DerivationSource {
    relation: "composition_steps",
    id_column: "step_id",
    conclusion_relation: "transfers",
    conclusion_column: "conclusion_id",
    rule: "compose_call",
    premises: &[
        Premise { column: "caller_transfer_id", role: "caller_transfer", relation: "transfers" },
        Premise { column: "binding_id", role: "binding", relation: "transfers" },
        Premise { column: "callee_transfer_id", role: "callee_transfer", relation: "transfers" },
    ],
}];

const CFG: DdlConfig = DdlConfig { schema: "lctx", reader: "lctx_serving" };

#[test]
fn sources_validate_against_the_model() {
    derivation::validate(SOURCES, DECLS).unwrap();
    const BAD: &[DerivationSource] = &[DerivationSource {
        relation: "composition_steps",
        id_column: "missing",
        conclusion_relation: "nowhere",
        conclusion_column: "conclusion_id",
        rule: "r",
        premises: &[],
    }];
    let errors = derivation::validate(BAD, DECLS).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
}

#[test]
fn the_index_views_are_generated() {
    insta::assert_snapshot!(format!(
        "{};\n\n{};\n\n{}",
        derivation::derivations_view(SOURCES, &CFG),
        derivation::premises_view(SOURCES, &CFG),
        derivation::derivations_view(&[], &CFG),
    ));
}

#[test]
fn premise_graphs_must_be_acyclic() {
    let (a, b, c) = (Id([1; 16]), Id([2; 16]), Id([3; 16]));
    assert!(derivation::acyclic(&[(a, b), (b, c), (a, c)]));
    assert!(!derivation::acyclic(&[(a, b), (b, c), (c, a)]));
    assert!(derivation::acyclic(&[]));
}

#[test]
fn a_projection_digest_follows_its_declaration() {
    const INVOCATION: ProjectionDecl = ProjectionDecl {
        name: "invocation",
        universe: &[EntityKind::Function, EntityKind::Class, EntityKind::Module],
        arcs: CallPolicy::Invocation,
        direction: Direction::Forward,
        parallel: ParallelArcs::Keep,
        unresolved: Unresolved::Node,
    };
    let dataflow = ProjectionDecl { arcs: CallPolicy::Dataflow, ..INVOCATION };
    let reversed = ProjectionDecl { direction: Direction::Reverse, ..INVOCATION };
    assert_eq!(INVOCATION.digest(), INVOCATION.digest());
    assert_ne!(INVOCATION.digest(), dataflow.digest());
    assert_ne!(INVOCATION.digest(), reversed.digest());
}
