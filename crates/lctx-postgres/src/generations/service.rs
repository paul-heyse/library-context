//! One process owner composing canonical services; the transport owns no semantic dispatch.
use super::{
    CatalogService, Error, GenerationId, GenerationService, PreparedNative, RequestExecution,
    RetrievalService,
};
use crate::roles::RoleConfig;
use lctx_model::domain::{
    self,
    serving::{Request, Response},
};
use std::sync::Arc;
#[derive(Clone)]
pub struct ServingService {
    runtime: GenerationService,
    catalog: CatalogService,
    retrieval: RetrievalService,
    native: PreparedNative,
}
impl ServingService {
    pub async fn open(
        config: &RoleConfig,
        generation: Option<GenerationId>,
        vectors: bool,
    ) -> Result<Self, Error> {
        let model = Arc::new(domain::model()?);
        let runtime = GenerationService::admit(model, config, generation).await?;
        let prepared = async {
            let catalog = CatalogService::prepare(runtime.clone()).await?;
            let retrieval = RetrievalService::prepare(catalog.clone(), vectors).await?;
            let native = runtime.prepare_native().await?;
            Ok::<_, Error>((catalog, retrieval, native))
        }
        .await;
        match prepared {
            Ok((catalog, retrieval, native)) => Ok(Self {
                runtime,
                catalog,
                retrieval,
                native,
            }),
            Err(error) => {
                let _ = runtime.shutdown().await;
                Err(error)
            }
        }
    }
    pub fn runtime(&self) -> &GenerationService {
        &self.runtime
    }
    pub fn retrieval(&self) -> &RetrievalService {
        &self.retrieval
    }
    pub async fn dispatch(
        &self,
        e: &RequestExecution,
        request: Request,
    ) -> Result<Response, Error> {
        let response = match request {
            Request::FindOperations(r) => Response::FindOperations(self.catalog.find(e, &r).await?),
            Request::GetOperation(r) => {
                Response::GetOperation(self.catalog.operation(e, &r).await?)
            }
            Request::BrowseLibrary(r) => Response::BrowseLibrary(self.catalog.browse(e, &r).await?),
            Request::GetEvidence(r) => Response::GetEvidence(self.runtime.evidence(e, &r).await?),
            Request::CompareOperations(r) => {
                Response::CompareOperations(self.catalog.compare(e, &r).await?)
            }
            Request::GetCapability(r) => {
                Response::GetCapability(self.runtime.capability(e, &r).await?)
            }
            Request::InspectValuePaths(r) => {
                Response::InspectValuePaths(self.native.inspect(e, r).await?)
            }
            _ => return Err(Error::Contract),
        };
        super::catalog_service::retain(e, &response)?;
        e.confirm().await?;
        Ok(response)
    }
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.runtime.shutdown().await
    }
}
