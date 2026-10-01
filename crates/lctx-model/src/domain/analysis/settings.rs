//! Authored structural scope and the resolved optional technique set. Parsing is pure; the
//! driver owns file access. Mandatory public catalog construction does not consume seed limits.
use crate::domain::*;
use crate::Domain;
use serde::{Deserialize, Serialize};
fn invalid(message: impl Into<String>) -> ModelError { ModelError::Invalid(message.into()) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all="kebab-case",deny_unknown_fields)]
pub struct Techniques {
    pub communities: bool, pub pagerank: bool, pub fca: bool, pub knn: bool,
    pub rca: bool, pub type_layer: bool, pub mention_layer: bool, pub knn_layer: bool,
}
impl Techniques {
    fn flags(&mut self)->[(&'static str,&mut bool);8] {[
        ("communities",&mut self.communities),("pagerank",&mut self.pagerank),
        ("fca",&mut self.fca),("knn",&mut self.knn),("rca",&mut self.rca),
        ("type-layer",&mut self.type_layer),("mention-layer",&mut self.mention_layer),
        ("knn-layer",&mut self.knn_layer),
    ]}
    pub fn validate(&self)->Result<(),ModelError> {
        if self.rca&&!self.fca {return Err(invalid("rca needs fca"));}
        if !self.communities&&(self.type_layer||self.mention_layer||self.knn_layer) {return Err(invalid("a community layer needs communities"));}
        Ok(())
    }
    pub fn parse(spec:&str)->Result<Self,ModelError> {
        let mut out=Self::default();
        if spec.trim().is_empty()||spec.trim()=="default" {return Ok(out);}
        let mut seen=std::collections::BTreeMap::new();
        for part in spec.split(',').map(str::trim) {
            let (enabled,name)=match part.split_at_checked(1) {Some(("+",name))=>(true,name),Some(("-",name))=>(false,name),_=>return Err(invalid(format!("{part}: expected +name or -name")))};
            if seen.insert(name,enabled).is_some_and(|old|old!=enabled) {return Err(invalid(format!("contradictory technique: {name}")));}
            let mut flags=out.flags();let (_,flag)=flags.iter_mut().find(|(key,_)|*key==name).ok_or_else(||invalid(format!("{name}: no such technique")))?;**flag=enabled;
        }
        out.validate()?;Ok(out)
    }
    pub fn label(self)->String {let mut copy=self;let names=copy.flags().into_iter().filter_map(|(n,on)|(*on).then_some(n)).collect::<Vec<_>>();if names.is_empty(){"none".into()}else{names.join(",")}}
}
/// Scalar fields lower directly to the declared relation; the full resolved technique set is
/// identity-bearing even when every optional method is off.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="analytics_configurations",validate=validate_configuration)]
pub struct AnalyticsConfiguration {
    #[model(key)] pub module_prefixes:Vec<String>,
    #[model(key)] pub public_roots:Vec<String>,
    #[model(key)] pub configured_seeds:Vec<String>,
    #[model(key)] pub depth:i64,
    #[model(key)] pub vertices:i64,
    #[model(key)] pub arcs:i64,
    #[model(key)] pub witnesses:i64,
    #[model(key)] pub brief_budget:i64,
    #[model(key)] pub communities:bool,
    #[model(key)] pub pagerank:bool,
    #[model(key)] pub fca:bool,
    #[model(key)] pub knn:bool,
    #[model(key)] pub rca:bool,
    #[model(key)] pub type_layer:bool,
    #[model(key)] pub mention_layer:bool,
    #[model(key)] pub knn_layer:bool,
}
fn dotted(name:&str)->bool {name.split('.').all(|part|{let mut chars=part.chars();chars.next().is_some_and(|c|c=='_'||c.is_ascii_alphabetic())&&chars.all(|c|c=='_'||c.is_ascii_alphanumeric())})}
fn validate_configuration(row:&AnalyticsConfiguration)->Result<(),ModelError>{
    if row.module_prefixes.is_empty()||row.public_roots.is_empty(){return Err(invalid("analytics requires module prefixes and public roots"));}
    if [row.depth,row.vertices,row.witnesses].iter().any(|v|*v<=0||*v>u32::MAX as i64)||row.arcs<0||row.arcs>u32::MAX as i64||row.brief_budget<0{return Err(invalid("invalid analytics bounds"));}
    if row.configured_seeds.is_empty(){return Err(invalid("no configured seeds"));}
    if row.configured_seeds.len()>row.brief_budget as usize{return Err(invalid("configured seeds exceed the brief budget"));}
    let mut seen=std::collections::BTreeSet::new();
    for name in &row.configured_seeds {if !seen.insert(name){return Err(invalid("duplicate configured seed"));}if !row.public_roots.iter().any(|root|name==root||name.starts_with(&format!("{root}."))){return Err(invalid("seed is outside public roots"));}}
    if row.module_prefixes.iter().chain(&row.public_roots).chain(&row.configured_seeds).any(|s|!dotted(s)){return Err(invalid("analytics path is not a dotted Python name"));}
    row.techniques().validate()
}
impl AnalyticsConfiguration {
    pub fn techniques(&self)->Techniques {Techniques {communities:self.communities,pagerank:self.pagerank,fca:self.fca,knn:self.knn,rca:self.rca,type_layer:self.type_layer,mention_layer:self.mention_layer,knn_layer:self.knn_layer}}
    pub fn in_subsystem(&self,module:&str)->bool {self.module_prefixes.iter().any(|p|module==p||module.starts_with(&format!("{p}.")))}
    pub fn bounds(&self)->Result<super::delegation::Bounds,ModelError>{self.validate()?;Ok(super::delegation::Bounds {depth:self.depth as u32,vertices:self.vertices as u32,arcs:self.arcs as u32,witnesses:self.witnesses as u32})}
    pub fn parse(text:&str,techniques:Techniques)->Result<Self,ModelError>{
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Document {version:u32,subsystem:Subsystem,seeds:Seeds,pass_a:PassA,briefs:Briefs}
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Subsystem {module_prefixes:Vec<String>,public_roots:Vec<String>}
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Seeds {primary:Vec<String>,distractors:Vec<String>}
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct PassA {max_depth:u32,max_vertices:u32,max_edges:u32,max_witnesses:u32}
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Briefs {budget:u32}
        let document:Document=toml::from_str(text).map_err(ModelError::codec)?;
        if document.version!=1{return Err(invalid("unsupported analytics configuration version"));}
        let row=Self {module_prefixes:document.subsystem.module_prefixes,public_roots:document.subsystem.public_roots,configured_seeds:document.seeds.primary.into_iter().chain(document.seeds.distractors).collect(),depth:document.pass_a.max_depth as i64,vertices:document.pass_a.max_vertices as i64,arcs:document.pass_a.max_edges as i64,witnesses:document.pass_a.max_witnesses as i64,brief_budget:document.briefs.budget as i64,communities:techniques.communities,pagerank:techniques.pagerank,fca:techniques.fca,knn:techniques.knn,rca:techniques.rca,type_layer:techniques.type_layer,mention_layer:techniques.mention_layer,knn_layer:techniques.knn_layer};row.validate()?;Ok(row)
    }
}
