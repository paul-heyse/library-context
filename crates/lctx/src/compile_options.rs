//! Validate explicit upper-frontier configuration before acquisition, service requests or database access.
use cpg_core::embedding_service::{Embedder,FakeEmbedder};
use lctx_model::domain::analysis::settings::{AnalyticsConfiguration,Techniques};
use std::path::{Path,PathBuf};
#[derive(Debug,Clone,Copy,PartialEq,Eq,clap::ValueEnum)]
pub enum EmbeddingChoice {None,Fake,Vllm}
#[derive(Debug,Default)]
pub struct Options {
    pub techniques:Option<String>,
    pub embedder:Option<EmbeddingChoice>,
    pub embedding_endpoint:Option<String>,
    pub embedding_spec:Option<PathBuf>,
}
pub struct Upper {
    pub settings:AnalyticsConfiguration,
    pub embedder:Option<Box<dyn Embedder>>,
}
impl Options {
    pub fn prepare(&self,through:&str,library:&Path)->anyhow::Result<Option<Upper>> {
        if !matches!(through,"analysis"|"catalog") {
            if self.techniques.is_some()||self.embedder.is_some()||self.embedding_endpoint.is_some()||self.embedding_spec.is_some(){anyhow::bail!("techniques and embedding options require --through analysis or catalog");}
            return Ok(None);
        }
        let techniques=Techniques::parse(self.techniques.as_deref().unwrap_or("default"))?;
        let choice=self.embedder.unwrap_or(EmbeddingChoice::None);
        if choice!=EmbeddingChoice::Vllm&&(self.embedding_endpoint.is_some()||self.embedding_spec.is_some()){anyhow::bail!("embedding endpoint/spec require --embedder vllm");}
        let settings=AnalyticsConfiguration::parse(&std::fs::read_to_string(library.join("analytics.toml"))?,techniques)?;
        let embedder:Option<Box<dyn Embedder>>=match choice {
            EmbeddingChoice::None=>None,
            EmbeddingChoice::Fake=>Some(Box::new(FakeEmbedder::new())),
            EmbeddingChoice::Vllm=>{
                let endpoint=self.embedding_endpoint.as_deref().unwrap_or("http://127.0.0.1:8000");
                let url=url::Url::parse(endpoint)?;
                if !matches!(url.scheme(),"http"|"https")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.query().is_some()||url.fragment().is_some(){anyhow::bail!("embedding endpoint must be an HTTP(S) service URL without credentials, query or fragment");}
                let spec=match &self.embedding_spec {Some(path)=>serde_json::from_slice(&std::fs::read(path)?)?,None=>lctx_embed::qwen_spec()};
                spec.validate().map_err(anyhow::Error::msg)?;
                Some(Box::new(lctx_embed::VllmEmbedder::new(endpoint,spec)))
            }
        };
        Ok(Some(Upper {settings,embedder}))
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn lower_frontiers_refuse_flags_without_reading_a_library(){let absent=Path::new("/no/analysis/configuration");for through in ["facts","normalized"]{assert!(Options::default().prepare(through,absent).unwrap().is_none());for options in [Options{techniques:Some("default".into()),..Default::default()},Options{embedder:Some(EmbeddingChoice::None),..Default::default()}]{assert!(options.prepare(through,absent).err().unwrap().to_string().contains("require --through"));}}}
    #[test]fn contradictory_techniques_refuse_before_configuration_io(){let options=Options{techniques:Some("+fca,-fca".into()),..Default::default()};let error=options.prepare("analysis",Path::new("/no/analysis/configuration")).err().unwrap();assert!(error.to_string().contains("contradictory"));}
}
