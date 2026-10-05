use lctx_model::domain::{self as d, *, graph::*};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Default)]
struct Lookup {entities:BTreeMap<EntityId,EntityKind>,assertions:BTreeSet<AssertionId>,sources:BTreeMap<EntityId,u64>,subtypes:BTreeMap<EntityId,i16>}
impl Lookup {fn add(&mut self,entity:&Entity){self.entities.insert(entity.id(),entity.kind());if let Some(tag)=entity.subtype(){self.subtypes.insert(entity.id(),tag);}if let Entity::Source(row)=entity{self.sources.insert(entity.id(),row.byte_len.try_into().unwrap());}}}
impl GraphLookup for Lookup {
 fn entity_kind(&self,id:EntityId)->Result<Option<EntityKind>,ModelError>{Ok(self.entities.get(&id).copied())}
 fn entity_subtype(&self,id:EntityId)->Result<Option<i16>,ModelError>{Ok(self.subtypes.get(&id).copied())}
 fn assertion_exists(&self,id:AssertionId)->Result<bool,ModelError>{Ok(self.assertions.contains(&id))}
 fn source_length(&self,id:EntityId)->Result<Option<u64>,ModelError>{Ok(self.sources.get(&id).copied())}
}
fn id<T>(n:u8)->Id<T>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
fn hash(n:u8)->ContentHash {ContentHash([n;32])}
fn inline()->Qualification {Qualification::Inline(InlineQualification {context:EntityId(hash(1)),scope:EntityId(hash(2)),condition:EntityId(hash(3)),modality:d::attribution::Modality::Definite,approximation:d::assertion::Approximation::Exact,assumptions:vec![]})}
fn claim(participants:Vec<Participant>)->Assertion {Assertion {source:None,kind:AssertionKind::ParameterBinding,participants,qualification:inline(),run:None,evidence:vec![],value:AssertionValue::None,derivation:None}}
fn qualified_lookup()->Lookup {let mut lookup=Lookup::default();lookup.entities.extend([(EntityId(hash(1)),EntityKind::Context),(EntityId(hash(2)),EntityKind::Scope),(EntityId(hash(3)),EntityKind::Condition)]);lookup}

#[test]
fn intrinsic_entities_keep_nominal_semantic_keys_and_conflicting_payloads_refuse(){
 let input=d::input::InputRevision::from_entries(vec![]).unwrap();
 let capture=Entity::from(input.clone());
 assert_eq!(capture.id(),EntityId::of(input.id()));
 let original=d::source::SourceArtifact::from_bytes(input.id(),"source.py".into(),b"abc").unwrap();
 let source=Entity::from(original.clone());
 let mut changed=original;changed.byte_len=4;
 let changed=Entity::from(changed);
 assert_eq!(source.id(),changed.id());assert_ne!(source.content(),changed.content());
 let mut family=FamilyHasher::new(GraphFamily::Entities);
 assert!(family.push(source.id().0,source.content()).unwrap());
 assert!(!family.push(source.id().0,source.content()).unwrap());
 assert!(family.push(changed.id().0,changed.content()).is_err());
 // The empty capture is an independent entity, even with no incident assertion.
 let mut lookup=Lookup::default();lookup.add(&capture);admit_entity(&capture,&lookup).unwrap();
}
#[test]
fn occurrence_and_evidence_ranges_resolve_to_exact_captured_bytes(){
 let input=d::input::InputRevision::from_entries(vec![]).unwrap();
 let source=d::source::SourceArtifact::from_bytes(input.id(),"a.py".into(),b"abc").unwrap();
 let mut lookup=Lookup::default();lookup.add(&Entity::from(input));lookup.add(&Entity::from(source.clone()));
 let occurrence=d::source::Occurrence {source:source.id(),start:1,end:4,syntax_kind:d::source::SyntaxKind::ExprName,role:d::source::OccurrenceRole::Syntax,structural_path:vec![0]};
 assert!(admit_entity(&Entity::from(occurrence),&lookup).is_err());
 let evidence=d::assertion::Evidence::SourceSpan{source:source.id(),start:0,end:4};
 assert!(admit_entity(&Entity::from(evidence),&lookup).is_err());
}
#[test]
fn native_supports_are_parallel_addressable_assertions_and_references_are_nominal(){
 let assertion=d::source::SyntaxObservation {qualification:id(1),occurrence:id(2),spelling:"x".into()};
 let support=d::source::SyntaxSupport {assertion:assertion.id(),run:id(3),surface:id(4),evidence:id(5),origin:d::attribution::Origin::AnalyzerAssertion,mode:d::attribution::ExtractionMode::NativeTraversal,fidelity:d::attribution::Fidelity::NativeStructural};
 let mut second=support.clone();second.run=id(6);
 let first=Assertion::from_record(support.clone()).unwrap();let second=Assertion::from_record(second).unwrap();
 assert_ne!(first.id(),second.id());assert_eq!(first.id(),AssertionId::of(support.id()));
 first.validate().unwrap();
 let refs=first.references().unwrap();assert!(refs.contains(&(Target::Assertion(AssertionId::of(assertion.id())),None)));
 assert!(refs.contains(&(Target::Entity(EntityId::of(support.run)),Some(EntityKind::Run))));
 let row=Assertion::from_record(assertion.clone()).unwrap();assert_eq!(row.id(),AssertionId::of(assertion.id()));
 assert!(row.references().unwrap().contains(&(Target::Entity(EntityId::of(assertion.qualification)),Some(EntityKind::Qualification))));
 let mut wrong=row;wrong.source=Some(SemanticKey::of(support.id()));assert!(wrong.validate().is_err());
}
#[test]
fn nary_roles_and_ordered_premises_survive_and_missing_internal_targets_refuse(){
 let mut lookup=qualified_lookup();let caller=EntityId(hash(4));let argument=EntityId(hash(5));let parameter=EntityId(hash(6));
 lookup.entities.extend([(caller,EntityKind::Occurrence),(argument,EntityKind::Occurrence),(parameter,EntityKind::Parameter)]);
 let mut assertion=claim(vec![Participant{role:ParticipantRole::Caller,field:None,position:None,target:Target::Entity(caller)},Participant{role:ParticipantRole::Argument,field:None,position:None,target:Target::Entity(argument)},Participant{role:ParticipantRole::Parameter,field:None,position:None,target:Target::Entity(parameter)}]);
 admit_assertion(&assertion,&lookup).unwrap();let original=assertion.id();assertion.participants.swap(1,2);assert_ne!(original,assertion.id());
 assertion.participants[0].target=Target::Entity(EntityId(hash(7)));assert!(admit_assertion(&assertion,&lookup).is_err());
 let a=AssertionId(hash(8));let b=AssertionId(hash(9));lookup.assertions.extend([a,b]);
 let mut derived=claim(vec![Participant{role:ParticipantRole::Subject,field:None,position:None,target:Target::Entity(caller)}]);
 derived.derivation=Some(Derivation{rule:"compose".into(),revision:1,conclusion:None,premises:vec![Target::Assertion(a),Target::Assertion(b)],assumptions:vec![],outcome:OutcomeKind::Complete});
 admit_assertion(&derived,&lookup).unwrap();let first=derived.id();derived.derivation.as_mut().unwrap().premises.reverse();assert_ne!(first,derived.id());
 assert!(admit_derivations([(a,&[Target::Assertion(b)][..]),(b,&[Target::Assertion(a)][..])]).is_err());
 admit_derivations([(a,&[Target::Assertion(b)][..])]).unwrap();
}
#[test]
fn external_uncertainty_is_explicit_but_cannot_fill_internal_roles(){
 let mut lookup=qualified_lookup();let provider=EntityId(hash(4));lookup.entities.insert(provider,EntityKind::Provider);
 let target=Target::External{provider,context:EntityId(hash(1)),name:"other.module".into(),reason:d::normalized::entities::EntityReason::ProviderExternal};
 let mut assertion=claim(vec![Participant{role:ParticipantRole::Callee,field:None,position:None,target}]);admit_assertion(&assertion,&lookup).unwrap();
 assertion.participants[0].role=ParticipantRole::Declaration;assert!(admit_assertion(&assertion,&lookup).is_err());
}
#[test]
fn manifest_distinguishes_exact_empty_partial_and_missing_obligations(){
 let key=OutcomeKey{producer:"native".into(),scope:EntityId(hash(1)),domain:hash(2)};
 let mut manifest=Manifest{format_version:ARTIFACT_FORMAT_VERSION,frontier:d::admission::Frontier::Facts,profile:d::stages::Profile::Catalog,captures:vec![EntityId(hash(3))],semantic_contract:hash(4),producers:vec![ProducerImplementation{producer:"native".into(),implementation:hash(5),configuration:hash(6)}],settings:hash(7),families:vec![],required_outcomes:vec![key.clone()],outcomes:vec![],originals:vec![],projections:vec![],embeddings:vec![]};
 assert!(manifest.validate().is_err());manifest.outcomes.push(Outcome{key,status:OutcomeKind::Complete,observed:Some(0),detail:None});manifest.validate().unwrap();
 let complete_empty=manifest.content();manifest.outcomes[0].status=OutcomeKind::Partial;assert_ne!(complete_empty,manifest.content());manifest.validate().unwrap();
 manifest.outcomes[0].status=OutcomeKind::NotRequested;manifest.outcomes[0].observed=Some(1);assert!(manifest.validate().is_err());
 manifest.format_version=0;assert!(manifest.validate().is_err());
}
#[test]
fn family_content_is_independent_of_transport_chunks_and_preserves_payload(){
 let rows=[(hash(1),hash(2)),(hash(3),hash(4)),(hash(5),hash(6))];
 let mut first=FamilyHasher::new(GraphFamily::Assertions);for (key,value) in rows {first.push(key,value).unwrap();}
 let mut second=FamilyHasher::new(GraphFamily::Assertions);for chunk in rows.chunks(2){for (key,value) in chunk {second.push(*key,*value).unwrap();second.push(*key,*value).unwrap();}}
 assert_eq!(first.finish(),second.finish());
}

#[test]
fn selected_semantic_inventory_has_nominally_closed_reference_types(){
 let mut absent=BTreeSet::new();
 macro_rules! check {($($variant:ident:$record:ty,)*)=>{$(for field in Relation::of::<$record>().fields(){if let Some((_,target))=field.target(){let reference=SemanticReference{field:field.name(),target,key:[0;16],subtype:field.subtype()};if reference_target(&reference).is_err(){absent.insert(format!("{}::{} -> {target}",<$record>::NAME,field.name()));}}})*};}
 lctx_model::graph_entity_records!(check);
 lctx_model::graph_assertion_records!(check);
 assert!(absent.is_empty(),"unselected semantic references: {absent:#?}");
}

#[test]
fn graph_admission_preserves_nominal_sum_arm_obligations(){
 let node=d::documents::DocumentNode::Passage{span:serde_json::from_value(serde_json::json!(vec![1;16])).unwrap(),ordinal:0};
 let node_entity=Entity::from(node.clone());
 let row=d::documents::PassageObservation{qualification:id(2),passage:d::documents::DocumentNodePassageId::of(&node).unwrap(),level:0,heading:None,heading_path:vec![],text:"body".into()};
 let assertion=Assertion::from_record(row).unwrap();
 let mut lookup=Lookup::default();lookup.add(&node_entity);lookup.entities.insert(EntityId::of(id::<d::assertion::AssertionQualification>(2)),EntityKind::Qualification);
 assert!(assertion.reference_requirements().unwrap().iter().any(|reference|reference.subtype==Some(0)));
 admit_assertion(&assertion,&lookup).unwrap();
 // A structurally valid membership entry with the same nominal family but a wrong arm fails.
 lookup.subtypes.insert(node_entity.id(),1);
 assert!(admit_assertion(&assertion,&lookup).is_err());
}
