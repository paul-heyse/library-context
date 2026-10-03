//! Scoped canonical packet reads check the attempted relation before querying or iterating.
use super::{Error, GenerationLease};
use lctx_model::domain::{
    serving::mappings::{PacketBinding, PacketOutput},
    *,
};
pub(super) struct PacketLease<'a> {
    pub(super) lease: &'a mut GenerationLease,
    binding: &'static PacketBinding,
}
impl<'a> PacketLease<'a> {
    pub(super) fn new<P: PacketOutput>(lease: &'a mut GenerationLease) -> Self {
        Self {
            lease,
            binding: P::binding(),
        }
    }
    pub(super) fn check_relation(&self, name: &str) -> Result<(), Error> {
        if !self.binding.permits_relation(name) {
            return Err(Error::Contract);
        }
        Ok(())
    }
    fn attempt<R: Record>(&self) -> Result<(), Error> {
        if !self.binding.permits::<R>() {
            return Err(Error::Contract);
        }
        Ok(())
    }
    pub(super) async fn read_ids<R: Record>(&mut self, ids: &[Id<R>]) -> Result<Batch<R>, Error> {
        self.attempt::<R>()?;
        self.lease.read_ids(ids).await
    }
    pub(super) async fn read_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
    ) -> Result<Batch<R>, Error> {
        self.attempt::<R>()?;
        self.lease.read_for(field, ids).await
    }
    pub(super) async fn visit_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
        order: &[&str],
        consume: impl FnMut(Batch<R>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.attempt::<R>()?;
        self.lease.visit_for(field, ids, order, consume).await
    }
}

#[cfg(all(test, feature = "testing"))]
#[allow(dead_code, reason = "Shared native fixture is broader than the packet control")]
#[path = "../../../lctx-model/tests/fixtures/types.rs"]
mod assumption_fixture;

#[cfg(all(test, feature = "testing"))]
mod tests {
    use super::*;
    use crate::{
        generations::{GenerationReader, GenerationStore},
        testing::{DisposableDatabase, Harness},
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn conditional_claim_basis_resolves_actual_native_premise_through_postgres() {
        use lctx_model::domain::{assumptions::*,assertion::*,attribution::*,conditions::*,artifact::*,input::*,lexical::*,source::*,types::*,calls::*};
        let mut f = super::assumption_fixture::Fixture::new(false);
        let observation = f.base.rows::<TypeObservation>()[0].clone();
        let support = f.base.rows::<TypeSupport>().into_iter().find(|s| s.assertion == observation.id()).unwrap();
        let definition = Assumption::TypeConformance {observation:observation.id(),support:support.id()};
        let symbol=f.base.rows::<ProviderSymbol>().into_iter().find(|s|s.kind==SymbolKind::Class).unwrap();
        let lower_q=f.base.rows::<AssertionQualification>()[0].clone();let context=f.base.rows::<AnalysisContext>()[0].clone();let input=f.base.rows::<InputRevision>()[0].id();
        let class=symbols::ClassTraitObservation{qualification:lower_q.id(),symbol:symbol.id(),synthesized:false,dataclass:false,named_tuple:false,typed_dict:false};
        let (class_run,class_families)=ProviderRun::new(symbol.provider,context.id(),input,context.config_digest,[FactFamily::Signatures]).unwrap();
        let class_surface=ProviderSurface{provider:symbol.provider,family:FactFamily::Signatures,name:"native class".into()};
        let class_evidence=Evidence::Invocation{run:class_run.id()};
        let class_support=symbols::ClassTraitSupport{assertion:class.id(),run:class_run.id(),surface:class_surface.id(),evidence:class_evidence.id(),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural};
        let symbol_observation=symbols::SymbolObservation{qualification:lower_q.id(),symbol:symbol.id(),parent:None};
        let symbol_support=symbols::SymbolSupport{assertion:symbol_observation.id(),run:class_run.id(),surface:class_surface.id(),evidence:class_evidence.id(),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural};
        let catalog=models::Catalog::committed().unwrap();
        let universe=AssumptionUniverse{context:context.id(),input,environment:context.environment_digest,model_definition:catalog.declaration().content};
        let pinned=assumptions_universe::AssumptionUniverseSupport::new(&universe,catalog.declaration(),catalog.models()[0].declaration()).unwrap();
        let closed=Assumption::NoExtraOverrides{class:class.id(),support:class_support.id(),universe:universe.id()};
        macro_rules! append {($ty:ty,$rows:expr)=>{let mut rows=f.base.rows::<$ty>();rows.extend($rows);f.base.put(rows);};}
        append!(ProviderRun,vec![class_run.clone()]);append!(RunFamily,class_families);append!(ProviderSurface,vec![class_surface]);append!(Evidence,vec![class_evidence]);
        f.base.put(vec![symbol_observation]);f.base.put(vec![symbol_support]);
        f.base.put(vec![class.clone()]);f.base.put(vec![class_support.clone()]);f.base.put(vec![universe.clone()]);f.base.put(vec![pinned.clone()]);
        let basis = AssumptionSet::new([definition.id(),closed.id()]).unwrap();
        let q = AssertionQualification {assumptions:basis.set.id(),..f.base.rows::<AssertionQualification>()[0].clone()};
        let mut qualifications = f.base.rows::<AssertionQualification>(); qualifications.push(q.clone());f.base.put(qualifications);
        f.base.put(vec![AssumptionSet::empty(),basis.set.clone()]);f.base.put(basis.members.clone());f.base.put(vec![definition.clone(),closed.clone()]);
        let db = DisposableDatabase::start().await;db.migrate().await;
        let mut relations=facts_relations();relations.extend(models::records::relations());relations.push(Relation::of::<assumptions_universe::AssumptionUniverseSupport>());
        let model = Arc::new(ValidatedModel::validate(relations).unwrap());
        let store = GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
        let budget = resources::ResourceBudget::fixed(256 << 20).unwrap();
        let mut declarations=models::records::CatalogRecords::new(&budget);declarations.insert_borrowed(&catalog).unwrap();
        f.base.put(declarations.catalogs.iter().cloned().collect());f.base.put(declarations.targets.iter().cloned().collect());f.base.put(declarations.models.iter().cloned().collect());f.base.put(declarations.protocols.iter().cloned().collect());
        let mut attempt = Harness::begin(&store,db.writer.clone(),stages::Profile::Catalog,budget.clone()).await.unwrap();
        macro_rules! copy {($($ty:ty),*)=>{$(let rows=f.base.rows::<$ty>();if !rows.is_empty(){attempt.copy(&Batch::new(&model,rows,&budget).unwrap(),&budget).await.unwrap();})*};}
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,CoverageScope,Condition,ConditionNode,AssertionQualification,AssumptionSet,AssumptionSetMember,Assumption,SourceArtifact,ArtifactChunk,Occurrence,Evidence,BindingEvent,LexicalTarget,ReferenceObservation,ReferenceSupport,BindingObservation,BindingSupport,LexicalResolution,LexicalResolutionSupport,LexicalScope,LexicalScopeObservation,LexicalScopeSupport,ProviderCoverage,ProviderModule,ProviderSymbol,TypeVariable,TypeObservation,TypeSupport,TypeVariableRestriction,TypeRestrictionSupport,TypePresentation,TypePresentationSupport,value::Literal,TypeTerm,TypeSequence,TypeSequenceMember,symbols::ClassTraitObservation,symbols::ClassTraitSupport,symbols::SymbolObservation,symbols::SymbolSupport,AssumptionUniverse,assumptions_universe::AssumptionUniverseSupport,models::ModelCatalog,models::AuthoredTarget,models::AuthoredModel,models::AuthoredContextProtocol);
        attempt.seal().await.unwrap();attempt.validate(&budget).await.unwrap();attempt.publish().await.unwrap();
        let dir=tempfile::tempdir().unwrap();db.write_configs(dir.path()).unwrap();
        let role=crate::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
        let reader=GenerationReader::connect(model,&role).await.unwrap();
        let guard=reader.guard(attempt.generation(),budget).await.unwrap();
        {
            let mut locked=guard.state.lease.lock().await;let lease=locked.as_mut().unwrap();
            let mut scoped=PacketLease::new::<serving::NativeAssessmentPacket>(lease);
            let packet=scoped.claim_basis(&q).await.unwrap();
            assert_eq!(packet.set,basis.set.id());assert_eq!(packet.members_digest,basis.set.members);
            assert_eq!(packet.definitions.len(),2);
            let serving::ClaimAssumptionPacket::TypeConformance{assumption,observation:actual,subject,role,declared,term,support:resolved}= packet.definitions.iter().find(|p|p.assumption()==definition.id()).unwrap() else {panic!("expected type conformance")};
            assert_eq!(*assumption,definition.id());assert_eq!(*actual,observation.id());assert_eq!((*subject,*role,*declared,*term),(observation.subject,observation.role,observation.declared,observation.term));
            assert_eq!(resolved.run,support.run);assert_eq!(resolved.support.row,*support.id().bytes());
            let context=f.base.rows::<AnalysisContext>()[0].clone();assert_eq!(resolved.environment,context.environment_digest);
            let original=f.base.rows::<Evidence>().into_iter().find(|r|r.id()==support.evidence).unwrap();
            let Evidence::Occurrence{occurrence}=original else {panic!("expected source premise")};
            let original=f.base.rows::<Occurrence>().into_iter().find(|r|r.id()==occurrence).unwrap();
            let source=f.base.rows::<SourceArtifact>().into_iter().find(|r|r.id()==original.source).unwrap();
            assert!(matches!(&resolved.evidence,serving::AssumptionEvidencePacket::Source{artifact,path,content,start,end} if (*artifact,path.as_str(),*content,*start,*end)==(source.id(),source.path.as_str(),source.content,original.start,original.end)));
            let serving::ClaimAssumptionPacket::NoExtraOverrides{class:actual_class,symbol:actual_symbol,support:actual_support,universe:actual_universe,..}=packet.definitions.iter().find(|p|p.assumption()==closed.id()).unwrap() else {panic!("expected pinned override premise")};
            assert_eq!((*actual_class,*actual_symbol),(class.id(),symbol.id()));assert_eq!(actual_support.run,class_run.id());
            assert!(matches!(actual_support.evidence,serving::AssumptionEvidencePacket::Invocation{run} if run==class_run.id()));
            assert_eq!((actual_universe.universe,actual_universe.model_definition,actual_universe.support,actual_universe.catalog,actual_universe.model),(universe.id(),catalog.declaration().content,pinned.id(),catalog.declaration().id(),catalog.models()[0].declaration().id()));
            assert_eq!(actual_universe.source.as_str(),catalog.declaration().source);assert_eq!(actual_universe.environment,context.environment_digest);
            let empty_q=f.base.rows::<AssertionQualification>().into_iter().find(|q| q.assumptions==AssumptionSet::empty_id()).unwrap();
            let empty=scoped.claim_basis(&empty_q).await.unwrap();assert_eq!(empty.set,AssumptionSet::empty_id());assert!(empty.definitions.is_empty());
            let missing=AssertionQualification{assumptions:AssumptionSet::new([serde_json::from_value(serde_json::json!(vec![99;16])).unwrap()]).unwrap().set.id(),..q};
            assert!(matches!(scoped.claim_basis(&missing).await,Err(Error::Contract)));
        }
        guard.check().await.unwrap();guard.release().await.unwrap();reader.close().await;
    }
    #[tokio::test]
    async fn empty_attempted_undeclared_read_refuses_before_canonical_query() {
        let db = DisposableDatabase::start().await;
        db.migrate().await;
        let model = Arc::new(model().unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
        let mut attempt = Harness::begin_empty_conformance(
            &store,
            db.writer.clone(),
            stages::Profile::Catalog,
            budget.clone(),
            vec![],
        )
        .await
        .unwrap();
        attempt.seal().await.unwrap();
        attempt.validate(&budget).await.unwrap();
        attempt.publish().await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        db.write_configs(dir.path()).unwrap();
        let role =
            crate::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
        let reader = GenerationReader::connect(model, &role).await.unwrap();
        let guard = reader.guard(attempt.generation(), budget).await.unwrap();
        {
            let mut locked = guard.state.lease.lock().await;
            let lease = locked.as_mut().unwrap();
            let mut scoped = PacketLease::new::<serving::OperationCore>(lease);
            assert!(
                matches!(
                    scoped.read_ids::<retrieval::Fragment>(&[]).await,
                    Err(Error::Contract)
                ),
                "an empty ID set still constitutes an undeclared attempted read"
            );
            assert!(
                scoped
                    .read_ids::<input::Package>(&[])
                    .await
                    .unwrap()
                    .rows()
                    .is_empty()
            );
        }
        guard.check().await.unwrap();
        guard.release().await.unwrap();
        reader.close().await;
    }
}

fn required<R: Record + Clone>(batch: &Batch<R>, id: Id<R>) -> Result<R, Error> {
    batch.rows().iter().find(|row| row.id() == id).cloned().ok_or(Error::Contract)
}
impl PacketLease<'_> {
    /// Resolve the exact required premise definition and its real supporting source/model rows.
    /// Missing rows refuse the packet; they cannot be represented by an empty basis.
    pub(super) async fn claim_basis(&mut self, q: &assertion::AssertionQualification) -> Result<serving::ClaimBasisPacket, Error> {
        use assumptions::*;
        use serving::*;
        let set = required(&self.read_ids::<AssumptionSet>(&[q.assumptions]).await?, q.assumptions)?;
        if set.count < 0 || set.count as usize > MAX_ASSUMPTIONS { return Err(Error::Contract); }
        let members = self.read_for::<AssumptionSetMember,AssumptionSet>("set", &[set.id()]).await?;
        if members.rows().len() != set.count as usize { return Err(Error::Contract); }
        let basis = AssumptionSet::new(members.rows().iter().map(|row| row.assumption))?;
        if basis.set != set || basis.members.len() != members.rows().len() { return Err(Error::Contract); }
        let ids = basis.members.iter().map(|m|m.assumption).collect::<Vec<_>>();
        let definitions = self.read_ids::<Assumption>(&ids).await?;
        let _construction = self.lease.budget.reserve("packet-claim-assumptions", ids.len().saturating_mul(512 * 1024))?;
        let mut packets = Vec::new();
        for id in ids {
            let definition = required(&definitions, id)?;
            packets.push(match definition {
                Assumption::TypeConformance {observation,support} => {
                    let row = required(&self.read_ids::<types::TypeObservation>(&[observation]).await?, observation)?;
                    let actual = required(&self.read_ids::<types::TypeSupport>(&[support]).await?, support)?;
                    if actual.assertion != observation { return Err(Error::Contract); }
                    let native = self.assumption_support(&actual, row.qualification, q.context).await?;
                    ClaimAssumptionPacket::TypeConformance {assumption:id,observation,subject:row.subject,role:row.role,declared:row.declared,term:row.term,support:native}
                }
                Assumption::NoExtraOverrides {class,support,universe} => {
                    let row = required(&self.read_ids::<symbols::ClassTraitObservation>(&[class]).await?, class)?;
                    let actual = required(&self.read_ids::<symbols::ClassTraitSupport>(&[support]).await?, support)?;
                    if actual.assertion != class { return Err(Error::Contract); }
                    let native = self.assumption_support(&actual,row.qualification,q.context).await?;
                    let u = required(&self.read_ids::<AssumptionUniverse>(&[universe]).await?, universe)?;
                    let support_rows = self.read_for::<assumptions_universe::AssumptionUniverseSupport,AssumptionUniverse>("universe", &[universe]).await?;
                    let pinned = support_rows.rows().first().ok_or(Error::Contract)?;
                    let catalog = required(&self.read_ids::<models::ModelCatalog>(&[pinned.catalog]).await?, pinned.catalog)?;
                    let model = required(&self.read_ids::<models::AuthoredModel>(&[pinned.model]).await?, pinned.model)?;
                    let _parse = self.lease.budget.reserve("packet-universe-parse",catalog.source.len().saturating_mul(32).saturating_add(65536))?;
                    assumptions_universe::AssumptionUniverseSupport::new(&u,&catalog,&model)?;
                    if (u.context,u.input,u.environment) != (native.context,native.input,native.environment) { return Err(Error::Contract); }
                    let universe = AssumptionUniversePacket {universe,context:u.context,input:u.input,environment:u.environment,model_definition:u.model_definition,support:pinned.id(),catalog:catalog.id(),model:model.id(),source_name:Name::new(catalog.source_name).map_err(|e|Error::Codec(e.to_string()))?,format:catalog.format,source:Text::new(catalog.source).map_err(|e|Error::Codec(e.to_string()))?};
                    ClaimAssumptionPacket::NoExtraOverrides {assumption:id,class,symbol:row.symbol,synthesized:row.synthesized,dataclass:row.dataclass,named_tuple:row.named_tuple,typed_dict:row.typed_dict,support:native,universe}
                }
            });
        }
        Ok(ClaimBasisPacket::from_canonical(&basis, packets)?)
    }
    async fn assumption_support<S: assertion::Support>(&mut self, support:&S, qualification:Id<assertion::AssertionQualification>, expected:Id<attribution::AnalysisContext>) -> Result<serving::AssumptionNativeSupportPacket,Error> {
        use serving::*;
        let a = support.attribution().ok_or(Error::Contract)?;
        let q = required(&self.read_ids::<assertion::AssertionQualification>(&[qualification]).await?,qualification)?;
        if q.context != expected || q.assumptions != assumptions::AssumptionSet::empty_id() || q.condition != conditions::Diagram::always().id() || q.modality != attribution::Modality::Definite || q.approximation != assertion::Approximation::Exact { return Err(Error::Contract); }
        let run = required(&self.read_ids::<attribution::ProviderRun>(&[a.run]).await?,a.run)?;
        let context = required(&self.read_ids::<attribution::AnalysisContext>(&[run.context]).await?,run.context)?;
        let provider = required(&self.read_ids::<attribution::Provider>(&[run.provider]).await?,run.provider)?;
        let surface = required(&self.read_ids::<assertion::ProviderSurface>(&[a.surface]).await?,a.surface)?;
        if run.context != expected || surface.provider != provider.id() || a.fidelity != attribution::Fidelity::NativeStructural { return Err(Error::Contract); }
        let evidence = required(&self.read_ids::<assertion::Evidence>(&[a.evidence]).await?,a.evidence)?;
        let evidence = match evidence {
            assertion::Evidence::Invocation {run:r} => { if r != run.id() { return Err(Error::Contract); } AssumptionEvidencePacket::Invocation {run:r} },
            assertion::Evidence::Occurrence {occurrence} => {
                let row = required(&self.read_ids::<source::Occurrence>(&[occurrence]).await?,occurrence)?;
                self.assumption_source(row.source,row.start,row.end,run.input).await?
            }
            assertion::Evidence::SourceSpan {source,start,end} => self.assumption_source(source,start,end,run.input).await?,
        };
        Ok(AssumptionNativeSupportPacket {support:ProofReference::from_canonical(derivation::RowRef::of(support.id())),run:run.id(),input:run.input,context:run.context,environment:context.environment_digest,provider:Name::new(provider.tool).map_err(|e|Error::Codec(e.to_string()))?,provider_revision:Name::new(provider.revision).map_err(|e|Error::Codec(e.to_string()))?,provider_build:provider.build_digest,surface:Name::new(surface.name).map_err(|e|Error::Codec(e.to_string()))?,evidence,fidelity:a.fidelity})
    }
    async fn assumption_source(&mut self, id:Id<source::SourceArtifact>,start:i64,end:i64,input:Id<input::InputRevision>) -> Result<serving::AssumptionEvidencePacket,Error> {
        let row = required(&self.read_ids::<source::SourceArtifact>(&[id]).await?,id)?;
        if row.input != input || start < 0 || end < start || end > row.byte_len { return Err(Error::Contract); }
        Ok(serving::AssumptionEvidencePacket::Source {artifact:id,path:serving::Name::new(row.path).map_err(|e|Error::Codec(e.to_string()))?,content:row.content,start,end})
    }
}
