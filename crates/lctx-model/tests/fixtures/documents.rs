use std::collections::BTreeMap;
use arrow_array::RecordBatch;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, conditions::*, documents::*, input::*, source::*};

pub struct Fixture {
    pub model: ValidatedModel, pub batches: BTreeMap<&'static str,RecordBatch>,
    pub component: DocumentComponentObservation, pub component_support: DocumentComponentSupport,
    pub foreign: Evidence,
}
impl Fixture {
    pub fn new() -> Self {
        let model = model().unwrap();
        let bytes = include_bytes!("../../../../fixtures/python/semantic_documents/guide.mdx");
        let text = std::str::from_utf8(bytes).unwrap();
        let input = InputRevision::from_entries(vec![
            ManifestEntry { path: "guide.mdx".into(),content: ContentHash::of(bytes),byte_len: bytes.len() as i64 },
            ManifestEntry { path: "other.txt".into(),content: ContentHash::of(b"other"),byte_len: 5 },
        ]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(),"guide.mdx".into(),bytes).unwrap();
        let other = SourceArtifact::from_bytes(input.id(),"other.txt".into(),b"other").unwrap();
        let origin = InputOrigin::Tree { label: "document contract".into() };
        let acquisition = InputAcquisition { input: input.id(),origin: origin.id() };
        let context = AnalysisContext { python_version: "3.14.7".into(),python_platform: "linux".into(),search_path: vec![],site_package_path: vec![],
            config_digest: ContentHash::of(b"docs"),environment_digest: input.manifest,lock_digest: None };
        let provider = Provider { tool: "document-fixture".into(),revision: "1".into(),build_digest: ContentHash::of(b"fixture") };
        let (run,families) = ProviderRun::new(provider.id(),context.id(),input.id(),context.config_digest,[FactFamily::Docs]).unwrap();
        let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Docs,name: "document".into() };
        let scope = CoverageScope::Artifact { artifact: source.id() };
        let (condition,conditions) = Diagram::always().records();
        let qualification = AssertionQualification { context: context.id(),scope: scope.id(),condition: condition.id(),modality: Modality::Definite,approximation: Approximation::Exact };
        let q = qualification.id();
        let mut evidence = Vec::new();
        let mut span = |start: usize,end: usize| {
            let row = Evidence::SourceSpan { source: source.id(),start: start as i64,end: end as i64 };
            let id = EvidenceSourceSpanId::of(&row).unwrap(); evidence.push(row); id
        };
        let nested = text.find("## Nested").unwrap();
        let root_span = span(0,bytes.len());
        let first_span = span(0,nested);
        let second_span = span(nested,bytes.len());
        let card_start = text.find("<Card").unwrap(); let card_end = text.find("</Card>").unwrap()+7;
        let card_span = span(card_start,card_end);
        let inner = span(text[card_start..].find('>').unwrap()+card_start+1,card_end-7);
        let note_start = text.find("<Note>").unwrap(); let note_end = text.find("</Note>").unwrap()+7;
        let note_span = span(note_start,note_end);
        let code_start = text.find("```python").unwrap(); let code_end = text[code_start+3..].find("```").unwrap()+code_start+6;
        let code_span = span(code_start,code_end);
        let link_start = text.find("[API]").unwrap(); let link_end = text[link_start..].find(')').unwrap()+link_start+1;
        let link_span = span(link_start,link_end);
        let mention_start = text.find("Widget").unwrap(); let mention_span = span(mention_start,mention_start+6);
        let p0 = DocumentNode::Passage { span: first_span,ordinal: 0 };
        let p1 = DocumentNode::Passage { span: second_span,ordinal: 1 };
        let p0id = DocumentNodePassageId::of(&p0).unwrap(); let p1id = DocumentNodePassageId::of(&p1).unwrap();
        let card = DocumentNode::Component { span: card_span,ordinal: 0 };
        let note = DocumentNode::Component { span: note_span,ordinal: 1 };
        let code = DocumentNode::CodeBlock { span: code_span,ordinal: 0 };
        let link = DocumentNode::Link { span: link_span,ordinal: 0 };
        let mention = DocumentNode::Mention { span: mention_span };
        let document = DocumentObservation { qualification: q,source: source.id(),title: Some("Guide".into()),parsed: true };
        let passage0 = PassageObservation { qualification: q,passage: p0id,level: 1,heading: Some("Guide".into()),heading_path: vec![],text: text[..nested].into() };
        let passage1 = PassageObservation { qualification: q,passage: p1id,level: 2,heading: Some("Nested".into()),heading_path: vec!["Guide".into()],text: text[nested..].into() };
        let component = DocumentComponentObservation { qualification: q,component: DocumentNodeComponentId::of(&card).unwrap(),passage: p0id,parent: None,depth: 0,
            name: Some("Card".into()),form: ComponentForm::Flow,inner: Some(inner),lead: None };
        let child = DocumentComponentObservation { qualification: q,component: DocumentNodeComponentId::of(&note).unwrap(),passage: p1id,parent: Some(component.component),depth: 1,
            name: Some("Note".into()),form: ComponentForm::Flow,inner: None,lead: None };
        let code_text = "print(\"hello\")";
        let block = CodeBlockObservation { qualification: q,block: DocumentNodeCodeBlockId::of(&code).unwrap(),passage: p1id,language: Some("python".into()),
            meta: Some("title=\"example\"".into()),code: code_text.into(),content: ContentHash::of(code_text.as_bytes()),module_path: None };
        let link_observation = DocumentLinkObservation { qualification: q,link: DocumentNodeLinkId::of(&link).unwrap(),passage: p1id,url: "https://example.test/api".into(),title: None,text: "API".into() };
        let mention_observation = DocumentMentionObservation { qualification: q,mention: DocumentNodeMentionId::of(&mention).unwrap(),passage: p1id,class: MentionClass::Exact,
            source: MentionSource::InlineCode,form: "Widget".into(),access_path: Some("pkg.Widget".into()),qualified_name: Some("pkg.Widget".into()) };
        macro_rules! support { ($ty:ident,$row:expr,$e:expr) => { $ty { assertion: $row.id(),run: run.id(),surface: surface.id(),evidence: $e.id(),
            origin: Origin::SourceObservation,mode: ExtractionMode::Recognizer,fidelity: Fidelity::NativeStructural } }; }
        let component_support = support!(DocumentComponentSupport,component,card_span);
        let child_support = support!(DocumentComponentSupport,child,note_span);
        let values = vec![DocumentAttributeValue::Bare { name: "enabled".into() },DocumentAttributeValue::Literal { name: "label".into(),value: "hello".into() },
            DocumentAttributeValue::Expression { name: "count".into(),source: "size".into() },DocumentAttributeValue::Spread { source: "props".into() }];
        let attributes: Vec<_> = values.iter().enumerate().map(|(ordinal,value)| DocumentAttributeObservation { qualification: q,component: component.component,ordinal: ordinal as i64,value: value.id() }).collect();
        let attribute_supports: Vec<_> = attributes.iter().map(|a| support!(DocumentAttributeSupport,a,card_span)).collect();
        let foreign = Evidence::SourceSpan { source: other.id(),start: 0,end: 5 }; evidence.push(foreign.clone());
        let mut f = Self { model,batches: BTreeMap::new(),component,component_support,foreign };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(f.put(vec![$row.clone()]);)+ }; }
        one!(input,origin,acquisition,context,provider,run,surface,scope,condition,qualification,document.clone(),support!(DocumentSupport,document,root_span),
            block.clone(),support!(CodeBlockSupport,block,code_span),link_observation.clone(),support!(DocumentLinkSupport,link_observation,link_span),
            mention_observation.clone(),support!(DocumentMentionSupport,mention_observation,mention_span));
        f.put(families); f.put(conditions); f.put(evidence);
        f.put(vec![source.clone(),other.clone()]);
        f.put(ArtifactChunk::split(&source,bytes).unwrap().chain(ArtifactChunk::split(&other,b"other").unwrap()).collect());
        f.put(vec![p0,p1,card,note,code,link,mention]);
        f.put(vec![passage0.clone(),passage1.clone()]);
        f.put(vec![support!(PassageSupport,passage0,first_span),support!(PassageSupport,passage1,second_span)]);
        f.put(vec![f.component.clone(),child]); f.put(vec![f.component_support.clone(),child_support]);
        f.put(values); f.put(attributes); f.put(attribute_supports);
        f
    }
    pub fn put<R: Record>(&mut self, rows: Vec<R>) { self.batches.insert(R::NAME,Batch::new(&self.model,rows).unwrap().arrow().clone()); }
    pub fn rows<R: Record>(&self) -> Vec<R> { self.batches.get(R::NAME).map(|b| R::decode(b).unwrap()).unwrap_or_default() }
    pub fn check(&self, invariant: &Invariant) -> Result<(),ModelError> {
        let mut check = (invariant.create)();
        for input in &invariant.inputs { if let Some(batch) = self.batches.get(input.name()) { check.visit(input.name(),batch)?; } }
        check.finish()
    }
    pub fn foreign_inner(&mut self) {
        self.component.inner = Some(EvidenceSourceSpanId::of(&self.foreign).unwrap()); self.component_support.assertion = self.component.id();
        let mut rows = self.rows::<DocumentComponentObservation>(); rows.retain(|r| r.component != self.component.component); rows.push(self.component.clone()); self.put(rows);
        let mut supports = self.rows::<DocumentComponentSupport>(); supports.retain(|s| s.evidence != self.component_support.evidence); supports.push(self.component_support.clone()); self.put(supports);
    }
}
