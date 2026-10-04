//! Independent finite-context consequences, including iceberg support and partial-result limits.
use bit_set::BitSet;
use lctx_analytics::native_concepts::{Context,Implication,Lattice};
use lctx_model::domain::resources::ResourceBudget;
use odis::{FormalContext,algorithms::{CanonicalBasis,NextClosure,Titanic},traits::{ConceptEnumerator,ImplicationEngine,IcebergConceptEnumerator}};
use std::collections::{BTreeMap,BTreeSet};

type Basis=Vec<(BTreeSet<u16>,BTreeSet<u16>)>;
type Concepts=BTreeSet<(Vec<u32>,Vec<u16>)>;
#[derive(Clone,Debug)]
struct Matrix { objects:Vec<u32>,attributes:Vec<u16>,incidence:Vec<(u32,u16)> }
impl Matrix {
    fn masks(objects:usize,attributes:usize,rows:&[usize])->Self {
        assert!(objects<=12 && attributes<=8);assert_eq!(objects,rows.len());
        let objects=(0..objects).map(|i|1000+i as u32*7).collect::<Vec<_>>();
        let attributes=(0..attributes).map(|i|200+i as u16*11).collect::<Vec<_>>();
        let incidence=objects.iter().zip(rows).flat_map(|(g,row)|attributes.iter().enumerate().filter(move |(i,_)|row & (1<<i)!=0).map(move |(_,m)|(*g,*m))).collect();
        Self {objects,attributes,incidence}
    }
    // Direct finite incidence derivation: no production or oracle closure helper is called.
    fn extent(&self,intent:&BTreeSet<u16>)->Vec<u32> { self.objects.iter().copied().filter(|g|intent.iter().all(|m|self.incidence.contains(&(*g,*m)))).collect() }
    fn closure(&self,intent:&BTreeSet<u16>)->BTreeSet<u16> { let extent=self.extent(intent);self.attributes.iter().copied().filter(|m|extent.iter().all(|g|self.incidence.contains(&(*g,*m)))).collect() }
    fn subsets(&self)->impl Iterator<Item=BTreeSet<u16>>+'_ { (0..1usize<<self.attributes.len()).map(|mask|self.attributes.iter().enumerate().filter(|(i,_)|mask & (1<<i)!=0).map(|(_,m)|*m).collect()) }
    fn concepts(&self,support:usize)->Concepts { self.subsets().filter(|s|self.closure(s)==*s).filter_map(|s| {let mut extent=self.extent(&s);extent.sort();(extent.len()>=support).then(||(extent,s.into_iter().collect()))}).collect() }
    fn oracle(&self)->(FormalContext<usize>,Vec<u32>,Vec<u16>) {
        let mut objects=self.objects.clone();objects.sort();let mut attributes=self.attributes.clone();attributes.sort();
        let mut context=FormalContext::<usize>::new();
        for index in 0..attributes.len() { context.add_attribute(index,&BitSet::new()); }
        for (index,object) in objects.iter().enumerate() {
            let row=attributes.iter().enumerate().filter(|(_,m)|self.incidence.contains(&(*object,**m))).map(|(i,_)|i).collect::<BitSet>();context.add_object(index,&row);
        }
        (context,objects,attributes)
    }
}
fn production_basis(implications:&[Implication<u16>])->Basis { implications.iter().map(|i|(i.premise.iter().copied().collect(),i.conclusion.iter().copied().collect())).collect() }
fn basis_closure(start:&BTreeSet<u16>,basis:&Basis)->BTreeSet<u16> {
    let mut result=start.clone();loop {let previous=result.clone();for (premise,conclusion) in basis {if premise.is_subset(&result) {result.extend(conclusion);}}if result==previous {return result;}}
}
fn consequences_match(matrix:&Matrix,basis:&Basis,support:usize)->bool {matrix.subsets().filter(|s|matrix.extent(s).len()>=support).all(|s|basis_closure(&s,basis)==matrix.closure(&s))}
fn sound(matrix:&Matrix,basis:&Basis)->bool {basis.iter().all(|(p,c)|c.is_subset(&matrix.closure(p)))}
fn verify_retained(matrix:&Matrix,lattice:&Lattice<u32,u16>) {
    let basis=production_basis(lattice.implications());assert!(sound(matrix,&basis));
    for implication in lattice.implications() {assert_eq!(implication.support,matrix.extent(&implication.premise.iter().copied().collect()).len());}
    for concept in lattice.concepts() {let intent=concept.intent.iter().copied().collect();assert_eq!(matrix.closure(&intent),intent);let mut extent=matrix.extent(&intent);extent.sort();assert_eq!(extent,concept.extent);}
}
fn cases()->Vec<Matrix> {
    let mut cases=vec![
        Matrix::masks(0,0,&[]),Matrix::masks(0,3,&[]),Matrix::masks(3,0,&[0,0,0]),Matrix::masks(1,1,&[1]),Matrix::masks(1,1,&[0]),
        Matrix::masks(3,3,&[7,7,7]),Matrix::masks(3,3,&[0,0,0]),Matrix::masks(4,3,&[3,3,6,6]),Matrix::masks(2,2,&[3,2]),Matrix::masks(3,3,&[3,5,7]),
    ];
    for seed in 1..=12u64 { let mut state=seed;let rows=(0..12).map(|_|{let mut row=0;for i in 0..8 {state^=state<<13;state^=state>>7;state^=state<<17;if state%3==0 {row|=1<<i;}}row}).collect::<Vec<_>>();cases.push(Matrix::masks(12,8,&rows)); }
    assert_eq!(cases.len(),22);cases
}
#[test]
fn complete_and_iceberg_consequences_match_independent_algorithms_and_incidence() {
    for (case,matrix) in cases().into_iter().enumerate() {
        let (oracle,objects,attributes)=matrix.oracle();
        let basis=CanonicalBasis.compute_basis(&oracle).into_iter().map(|(p,c)| { let premise=p.iter().map(|i|attributes[i]).collect::<BTreeSet<_>>();let conclusion=c.iter().map(|i|attributes[i]).filter(|m|!premise.contains(m)).collect();(premise,conclusion) }).collect::<Basis>();
        assert!(sound(&matrix,&basis),"oracle soundness case {case}");assert!(consequences_match(&matrix,&basis,0),"oracle completeness case {case}");
        let full=NextClosure.enumerate_concepts(&oracle).map(|(extent,intent)|(extent.iter().map(|i|objects[i]).collect(),intent.iter().map(|i|attributes[i]).collect())).collect::<Concepts>();assert_eq!(full,matrix.concepts(0),"independent concepts case {case}");
        let budget=ResourceBudget::fixed(1<<24).unwrap();
        {
            let context=Context::new(&matrix.objects,&matrix.attributes,&matrix.incidence,&budget).unwrap();
            let mut shuffled=matrix.clone();shuffled.objects.reverse();shuffled.attributes.rotate_left(matrix.attributes.len()/2);shuffled.incidence.reverse();
            let reordered=Context::new(&shuffled.objects,&shuffled.attributes,&shuffled.incidence,&budget).unwrap();
            let object_names=matrix.objects.iter().copied().zip(matrix.objects.iter().rev().copied()).collect::<BTreeMap<_,_>>();let attribute_names=matrix.attributes.iter().copied().zip(matrix.attributes.iter().rev().copied()).collect::<BTreeMap<_,_>>();
            let inverse_objects=object_names.iter().map(|(a,b)|(*b,*a)).collect::<BTreeMap<_,_>>();let inverse_attributes=attribute_names.iter().map(|(a,b)|(*b,*a)).collect::<BTreeMap<_,_>>();
            let renamed_incidence=matrix.incidence.iter().map(|(g,m)|(object_names[g],attribute_names[m])).collect::<Vec<_>>();let renamed=Context::new(&matrix.objects,&matrix.attributes,&renamed_incidence,&budget).unwrap();
            for support in BTreeSet::from([0,1,2,matrix.objects.len(),matrix.objects.len()+1]) {
                let result=context.analyse(support,100_000).unwrap();assert!(!result.budget_reached(),"case {case} support {support}");verify_retained(&matrix,&result);
                assert!(consequences_match(&matrix,&production_basis(result.implications()),support),"production consequences case {case} support {support}");
                let expected=full.iter().filter(|(g,_)|g.len()>=support).cloned().collect::<Concepts>();
                assert_eq!(result.concepts().iter().map(|c|(c.extent.clone(),c.intent.clone())).collect::<Concepts>(),expected,"production iceberg case {case} support {support}");
                let iceberg=Titanic.enumerate(&oracle,support as u32);let independent=iceberg.poset.nodes.iter().map(|(g,m)|(g.iter().map(|i|objects[i]).collect(),m.iter().map(|i|attributes[i]).collect())).collect::<Concepts>();if matrix.objects.is_empty() {
                    // odis 2026.9.1 Titanic explicitly declines the zero-object lattice.
                    // NextClosure and direct incidence above still verify its unique concept.
                    assert!(independent.is_empty());
                } else { assert_eq!(independent,expected,"independent iceberg case {case} support {support}"); }
                let reordered=reordered.analyse(support,100_000).unwrap();assert_eq!(result.concepts(),reordered.concepts());assert_eq!(result.implications(),reordered.implications());
                let renamed=renamed.analyse(support,100_000).unwrap();
                let translated=renamed.concepts().iter().map(|c|{let mut g=c.extent.iter().map(|g|inverse_objects[g]).collect::<Vec<_>>();g.sort();let mut m=c.intent.iter().map(|m|inverse_attributes[m]).collect::<Vec<_>>();m.sort();(g,m)}).collect::<Concepts>();assert_eq!(translated,expected);
                let translated=renamed.implications().iter().map(|i|(i.premise.iter().map(|m|inverse_attributes[m]).collect(),i.conclusion.iter().map(|m|inverse_attributes[m]).collect())).collect::<Basis>();assert!(consequences_match(&matrix,&translated,support));
            }
        }
        assert_eq!(budget.reserved(),0);
    }
}
#[test]
fn revealing_asymmetry_and_mutations_are_detected_by_consequences() {
    let matrix=Matrix::masks(2,2,&[3,2]);let budget=ResourceBudget::fixed(1<<20).unwrap();
    let context=Context::new(&matrix.objects,&matrix.attributes,&matrix.incidence,&budget).unwrap();let result=context.analyse(0,1000).unwrap();let basis=production_basis(result.implications());
    let a=matrix.attributes[0];let b=matrix.attributes[1];assert!(basis_closure(&BTreeSet::from([a]),&basis).contains(&b));assert!(!basis_closure(&BTreeSet::from([b]),&basis).contains(&a));
    assert!(basis.iter().any(|(p,c)|p.is_empty() && c.contains(&b)),"common attribute requires empty premise");
    let mut deleted=basis.clone();deleted.remove(0);assert!(!consequences_match(&matrix,&deleted,0));
    let mut inserted=basis.clone();inserted.push((BTreeSet::from([b]),BTreeSet::from([a])));assert!(!sound(&matrix,&inserted));assert!(!consequences_match(&matrix,&inserted,0));
    let mut changed=matrix.clone();changed.incidence.push((changed.objects[1],a));assert_ne!(matrix.closure(&BTreeSet::new()),changed.closure(&BTreeSet::new()));assert!(!consequences_match(&changed,&basis,0));
    let altered=Context::new(&changed.objects,&changed.attributes,&changed.incidence,&budget).unwrap().analyse(0,1000).unwrap();assert!(consequences_match(&changed,&production_basis(altered.implications()),0));
}
#[test]
fn partial_results_keep_soundness_caps_and_reservation_lifetimes() {
    let matrix=Matrix::masks(4,4,&[3,5,9,15]);let budget=ResourceBudget::fixed(1<<20).unwrap();
    let context=Context::new(&matrix.objects,&matrix.attributes,&matrix.incidence,&budget).unwrap();let context_bytes=budget.reserved();assert!(context_bytes>0);
    for cap in [0,1,2,3] {let result=context.analyse(0,cap).unwrap();assert!(result.budget_reached());assert_eq!(result.examined(),cap);verify_retained(&matrix,&result);assert!(budget.reserved()>context_bytes);drop(result);assert_eq!(budget.reserved(),context_bytes);}
    let unsupported=context.analyse(matrix.objects.len()+1,0).unwrap();assert!(!unsupported.budget_reached());assert_eq!(unsupported.examined(),0);assert!(unsupported.concepts().is_empty());drop(unsupported);
    let retained=context.analyse(0,1000).unwrap();assert!(!retained.budget_reached());drop(context);assert!(budget.reserved()>0);drop(retained);assert_eq!(budget.reserved(),0);
    let denied=ResourceBudget::fixed(context_bytes).unwrap();let context=Context::new(&matrix.objects,&matrix.attributes,&matrix.incidence,&denied).unwrap();assert!(context.analyse(0,1000).is_err());assert_eq!(denied.reserved(),context_bytes);drop(context);assert_eq!(denied.reserved(),0);
    let tiny=ResourceBudget::fixed(1).unwrap();assert!(Context::new(&matrix.objects,&matrix.attributes,&matrix.incidence,&tiny).is_err());assert_eq!(tiny.reserved(),0);
}

#[test]
fn titanic_zero_object_omission_is_explicit_and_not_a_finite_context_answer() {
    for attributes in [0,3] {
        let matrix=Matrix::masks(0,attributes,&[]);let (oracle,_,_)=matrix.oracle();
        assert!(Titanic.enumerate(&oracle,0).poset.nodes.is_empty());
        assert_eq!(NextClosure.enumerate_concepts(&oracle).count(),1);assert_eq!(matrix.concepts(0).len(),1);
    }
}
