//! Maintenance-only typed source-format conversions. These helpers are not runtime
//! old-format readers and cannot earn completion, admission or ownership.
use lctx_model::{Domain,domain::{self as d,Id,ModelError,Record}};
use d::{normalized::coverage::{NormalizationEvidenceSet,NormalizationEvidenceMember},
    attribution::ProviderCoverage,derivation::RowRef};

/// Authentic source5 shape from ebcf423c. The source record's declared relation and
/// key fields are retained independently of the target sum's added discriminant.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="normalization_evidence_members")]
struct Source5NormalizationEvidenceMember{
    #[model(key)] premise:Id<NormalizationEvidenceSet>,
    #[model(key)] coverage:Id<ProviderCoverage>,
}

pub(crate) struct NormalizationMemberTranslation{
    pub original:RowRef,
    pub original_premise:Id<NormalizationEvidenceSet>,
    pub original_coverage:Id<ProviderCoverage>,
    pub target:NormalizationEvidenceMember,
}
pub(crate) struct NormalizationMemberTranslations{
    rows:Vec<NormalizationMemberTranslation>,
    _charge:Box<dyn d::resources::Reservation>,
}
impl NormalizationMemberTranslations{
    pub(crate) fn rows(&self)->&[NormalizationMemberTranslation]{&self.rows}
}

/// A source batch must have the exact authentic old typed columns and IDs. The
/// caller supplies explicitly translated dependency references from its checked
/// correspondence ledger; this function does not recursively replace hashes.
/// Every source row emits Scoped. Old NotRequested rows never become compact
/// Uniform evidence merely because their availability says NotRequested.
pub(crate) fn normalization_evidence_members(
    batch:&arrow_array::RecordBatch,
    mapped:impl Fn(Id<NormalizationEvidenceSet>,Id<ProviderCoverage>)
        ->Result<(Id<NormalizationEvidenceSet>,Id<ProviderCoverage>),ModelError>,
    budget:&d::resources::ResourceBudget,
)->Result<NormalizationMemberTranslations,ModelError>{
    let decoded=d::decode_allowance::<Source5NormalizationEvidenceMember>(batch)?;
    let retained=batch.num_rows().checked_mul(std::mem::size_of::<NormalizationMemberTranslation>())
        .and_then(|bytes|bytes.checked_add(4096)).ok_or(ModelError::Schema("source5 typed translation size overflow"))?;
    let mut charge=budget.reserve("source5 typed normalization member translation",
        decoded.checked_add(retained).ok_or(ModelError::Schema("source5 typed decode size overflow"))?)?;
    let rows=Source5NormalizationEvidenceMember::decode(batch)?;
    let rows=rows.into_iter().map(|row|{
        row.validate()?;
        let (premise,coverage)=mapped(row.premise,row.coverage)?;
        let target=NormalizationEvidenceMember::Scoped{premise,coverage};target.validate()?;
        Ok(NormalizationMemberTranslation{original:RowRef::of(row.id()),
            original_premise:row.premise,original_coverage:row.coverage,target})
    }).collect::<Result<Vec<_>,ModelError>>()?;
    charge.try_resize(retained)?;
    Ok(NormalizationMemberTranslations{rows,_charge:charge})
}

#[cfg(test)]
mod controls{
    use super::*;
    fn id<T>(byte:u8)->Id<T>{serde_json::from_value(serde_json::to_value([byte;16]).unwrap()).unwrap()}
    #[test]
    fn source5_scoped_evidence_gets_a_fresh_sum_identity_and_explicit_references(){
        let old=Source5NormalizationEvidenceMember{premise:id(1),coverage:id(2)};
        let original=old.id();let batch=Source5NormalizationEvidenceMember::encode(&[old]).unwrap();
        let budget=d::resources::ResourceBudget::fixed(1<<20).unwrap();
        let translated=normalization_evidence_members(&batch,|premise,coverage|{
            assert_eq!(premise,id(1));assert_eq!(coverage,id(2));Ok((id(3),id(4)))
        },&budget).unwrap();
        assert_eq!(translated.rows().len(),1);assert_eq!(translated.rows()[0].original,RowRef::of(original));
        assert_eq!(translated.rows()[0].original_premise,id(1));assert_eq!(translated.rows()[0].original_coverage,id(2));
        let NormalizationEvidenceMember::Scoped{premise,coverage}=&translated.rows()[0].target else{panic!("source5 never invents Uniform evidence")};
        assert_eq!(*premise,id(3));assert_eq!(*coverage,id(4));
        assert_ne!(original.bytes(),translated.rows()[0].target.id().bytes());
        assert!(budget.reserved()>0);
        assert!(normalization_evidence_members(&batch,|_,_|Err(ModelError::Conflict("missing exact dependency translation")),&budget).is_err());
        drop(translated);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn empty_source_and_target_columns_are_distinct_typed_formats(){
        let old=Source5NormalizationEvidenceMember::encode(&[]).unwrap();
        let budget=d::resources::ResourceBudget::fixed(1<<20).unwrap();
        assert!(normalization_evidence_members(&old,|_,_|panic!("empty calls no mapper"),&budget).unwrap().rows().is_empty());
        let target=NormalizationEvidenceMember::encode(&[]).unwrap();
        assert_ne!(old.schema(),target.schema());
        assert!(normalization_evidence_members(&target,|_,_|panic!("target is not old format"),&budget).is_err());
    }
}
