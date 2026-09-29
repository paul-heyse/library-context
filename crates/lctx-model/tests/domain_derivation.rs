use lctx_model::{Domain,domain::{*,derivation::*}};

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "proof_nodes",rule = "follows",semantic_source = b"proof fixture v1")]
struct Node { #[model(key)] name: String, #[model(premise)] next: Option<Id<Node>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "proof_steps",rule = "step",conclusion = result,semantic_source = b"proof fixture v1")]
struct Step { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Id<Node> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "optional_conclusion",rule = "step",conclusion = result,semantic_source = b"proof fixture v1")]
struct OptionalConclusion { #[model(key)] name: String,result: Option<Id<Node>>,#[model(premise)] previous: Id<Node> }

fn node(name: &str) -> Node { Node { name: name.into(),next: None } }
fn validate(model: &ValidatedModel,nodes: Vec<Node>,steps: Vec<Step>) -> Result<(),ModelError> {
    let invariant = model.invariants().iter().find(|i| i.name == "derivation_acyclic").unwrap();
    let mut check = (invariant.create)();
    check.visit(Node::NAME,Batch::new(model,nodes)?.arrow())?;
    check.visit(Step::NAME,Batch::new(model,steps)?.arrow())?;
    check.finish()
}
#[test]
fn targets_are_nominal_and_global_cycle_check_spans_proof_relations() {
    let model = ValidatedModel::validate(vec![Relation::of::<Node>(),Relation::of::<Step>()]).unwrap();
    let rule = Step::derivation().unwrap(); assert_eq!(rule.conclusion.unwrap().target().1,Node::NAME);
    assert_eq!(rule.premises[0].target().1,Node::NAME); assert_eq!(rule.premises[0].name(),"previous");
    let a = node("a"); let b = node("b"); let c = node("c");
    let a_to_b = Node { next: Some(b.id()),..a.clone() };
    let step = Step { name: "s1".into(),result: b.id(),previous: c.id() };
    validate(&model,vec![a_to_b.clone(),b.clone(),c.clone()],vec![step.clone()]).unwrap();
    // A -> B comes from Node; B -> A comes from Step. Checking each source separately is unsound.
    assert!(validate(&model,vec![a_to_b,b.clone(),c.clone()],vec![Step { previous: a.id(),..step }]).is_err());
    assert!(validate(&model,vec![Node { next: Some(a.id()),..a.clone() }],vec![]).is_err());
    // Alternatives and repeated edges are admissible without erasing their individual proof rows.
    let step = Step { name: "s1".into(),result: a.id(),previous: b.id() };
    let alternative = Step { name: "s2".into(),..step.clone() };
    assert_ne!(step.id(),alternative.id());
    validate(&model,vec![a,b,c],vec![step,alternative]).unwrap();
    assert!(ValidatedModel::validate(vec![Relation::of::<Node>(),Relation::of::<OptionalConclusion>()]).is_err());
}
#[test]
fn resource_work_refusal_does_not_certify_an_empty_or_truncated_proof() {
    let a = node("a"); let b = node("b");
    let proof = Step { name: "s".into(),result: a.id(),previous: b.id() }.proof().unwrap();
    assert!(acyclic(&[proof.clone()],1).is_err()); acyclic(&[proof],3).unwrap();
    let empty = a.proof().unwrap(); assert!(empty.premises.is_empty());
    assert!(acyclic(&[empty],0).is_err());
}

#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "self_steps",rule = "self_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct SelfStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<SelfStep>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "left_steps",rule = "left_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct LeftStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<RightStep>> }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "right_steps",rule = "right_step",conclusion = result,semantic_source = b"proof fixture v1")]
struct RightStep { #[model(key)] name: String,result: Id<Node>,#[model(premise)] previous: Option<Id<LeftStep>> }
#[test]
fn proof_source_identity_prevents_self_and_mutual_explicit_step_cycles() {
    let model = ValidatedModel::validate(vec![Relation::of::<Node>(),Relation::of::<SelfStep>(),Relation::of::<LeftStep>(),Relation::of::<RightStep>()]).unwrap();
    let a = node("a"); let b = node("b");
    let own = SelfStep { name: "s".into(),result: a.id(),previous: None };
    let left = LeftStep { name: "l".into(),result: a.id(),previous: None };
    let right = RightStep { name: "r".into(),result: b.id(),previous: None };
    for (self_cycle,mutual_cycle) in [(false,false),(true,false),(false,true)] {
        let own = SelfStep { previous: self_cycle.then_some(own.id()),..own.clone() };
        let left = LeftStep { previous: Some(right.id()),..left.clone() };
        let right = RightStep { previous: mutual_cycle.then_some(left.id()),..right.clone() };
        let invariant = model.invariants().iter().find(|i| i.name == "derivation_acyclic").unwrap();
        let mut check = (invariant.create)();
        check.visit(Node::NAME,Batch::new(&model,vec![a.clone(),b.clone()]).unwrap().arrow()).unwrap();
        check.visit(SelfStep::NAME,Batch::new(&model,vec![own]).unwrap().arrow()).unwrap();
        check.visit(LeftStep::NAME,Batch::new(&model,vec![left]).unwrap().arrow()).unwrap();
        check.visit(RightStep::NAME,Batch::new(&model,vec![right]).unwrap().arrow()).unwrap();
        assert_eq!(check.finish().is_err(),self_cycle || mutual_cycle);
    }
}
