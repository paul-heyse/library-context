//! Current Catalog compaction through actual producer, compiler, publication and serving gates.
//! No encoder, selected operator handle, fake provider or serialized semantic grant is used.
#[path = "../../cpg-core/tests/fixtures/native.rs"]
mod native_fixture;
#[path = "fixtures/native_library.rs"]
mod native_library;
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact,
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    admission::Frontier,
    stages::Profile,
    uniform_outcome::{UniformKind, UniformNotRequested},
    *,
};
use lctx_native::{NativeReader, RecordSelection, RuntimeConfig};
use std::{collections::BTreeSet, sync::Arc};

fn require(condition: bool, message: &'static str) -> Result<(), ModelError> {
    if condition {
        Ok(())
    } else {
        Err(ModelError::Conflict(message))
    }
}

async fn retire_owned(
    config: &RuntimeConfig,
    handle: &serving::SnapshotHandle,
) -> Result<(), ModelError> {
    let mut progress = lctx_publisher::backup::retire(config, handle, true).await?;
    if progress.remaining.is_empty() {
        return Ok(());
    }
    let client = lctx_native::reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let result = async {
        while !progress.remaining.is_empty() {
            let next =
                lctx_native::control::resume_retirement(&client, progress.identity, 4096).await?;
            require(
                next != progress,
                "compact publication retirement made no progress",
            )?;
            progress = next;
        }
        Ok(())
    }
    .await;
    let mut terminal = completion::Completion::default();
    terminal.step(
        "compact retirement session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    completion::complete(result, terminal)
}

async fn native_call(
    service: &lctx_serving::NativeService,
    name: &str,
    request: serde_json::Value,
) -> Result<serde_json::Value, ModelError> {
    let response = service
        .execute(name, &request.to_string())
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    serde_json::from_str(&response).map_err(ModelError::codec)
}

async fn linked_native_queries(
    reader: &NativeReader,
    service: &lctx_serving::NativeService,
    library: &str,
) -> Result<(), ModelError> {
    let operation=native_call(service,"get_operation",serde_json::json!({"library":library,
        "operation":{"kind":"public_path","path":["api","choose"]},
        "sections":["briefs","contextual_typing","relationships","access_routes"],"page":{"size":8,"expanded":true}})).await?;
    require(
        operation["operation"]["resolution"] == "unique",
        "generated choose operation must resolve uniquely",
    )?;
    let core = &operation["operation"]["packet"]["core"];
    require(
        core["signatures"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "generated choose signatures missing from native operation",
    )?;
    let member: Id<catalog::CatalogMember> =
        serde_json::from_value(core["member"].clone()).map_err(ModelError::codec)?;
    require(
        operation["operation"]["packet"]["access_routes"]["items"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "generated choose access-route analysis missing from native operation",
    )?;
    let alias=native_call(service,"get_operation",serde_json::json!({"library":library,
        "operation":{"kind":"public_path","path":["api","alias"]},"sections":["access_routes"],"page":{"size":8,"expanded":true}})).await?;
    require(
        alias["operation"]["resolution"] == "unique",
        "generated alias operation must resolve uniquely",
    )?;
    let alias_member: Id<catalog::CatalogMember> =
        serde_json::from_value(alias["operation"]["packet"]["core"]["member"].clone())
            .map_err(ModelError::codec)?;
    let comparison=native_call(service,"compare_operations",serde_json::json!({"library":library,
        "operations":[{"kind":"member","member":member},{"kind":"public_path","path":["api","alias"]}],"page":{"size":8,"expanded":true}})).await?;
    let comparison: serving::CompareOperationsResponse =
        serde_json::from_value(comparison).map_err(ModelError::codec)?;
    require(
        comparison.operations.len() == 2,
        "native comparison omitted generated choose or alias",
    )?;
    for (entry, expected) in comparison.operations.iter().zip([member, alias_member]) {
        require(
            !entry.ambiguous
                && !entry.candidates.is_empty()
                && entry
                    .candidates
                    .iter()
                    .all(|candidate| candidate.member == expected),
            "native comparison did not resolve its generated operation",
        )?;
    }
    let operations = native_call(
        service,
        "search_operations",
        serde_json::json!({"library":library,"query":"choose","page":{"size":8,"expanded":true}}),
    )
    .await?;
    require(
        operations["channels"]["vector"]["status"] == "disabled",
        "native operation control must not request embeddings",
    )?;
    let member_json = serde_json::to_value(member).map_err(ModelError::codec)?;
    require(
        operations["results"]["items"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["member"] == member_json)),
        "native lexical search omitted its generated choose operation",
    )?;
    let evidence=native_call(service,"search_evidence",serde_json::json!({"library":library,"query":"choose","families":[],"page":{"size":8,"expanded":true}})).await?;
    require(
        evidence["channels"]["vector"]["status"] == "disabled",
        "native evidence control must not request embeddings",
    )?;
    let evidence: serving::SearchEvidenceResponse =
        serde_json::from_value(evidence).map_err(ModelError::codec)?;
    let hit = evidence
        .results
        .items
        .iter()
        .find(|hit| hit.associated_members.contains(&member) && !hit.originals.is_empty())
        .ok_or(ModelError::Conflict(
            "generated evidence has no native link to choose and its original source",
        ))?;
    let units = reader
        .records::<retrieval::Unit>(RecordSelection::Keys(vec![*hit.unit.bytes()]))
        .await?;
    let [unit] = units.as_slice() else {
        return Err(ModelError::Conflict(
            "search evidence did not name one actually generated native unit",
        ));
    };
    let subject = catalog::evidence::RootSubject::Member { member };
    let roots = reader
        .records::<catalog::evidence::EvidenceRoot>(RecordSelection::Scope {
            field: "subject".into(),
            values: vec![serde_json::to_value(subject.id()).map_err(ModelError::codec)?],
        })
        .await?;
    require(
        roots
            .iter()
            .any(|root| root.input == unit.input && root.context == unit.context),
        "native search evidence and generated member analysis have no exact input/context root",
    )?;
    let original = &hit.originals[0];
    require(
        original.context == unit.context,
        "native search evidence relabelled its generated analysis context",
    )?;
    let original_response = native_call(
        service,
        "get_evidence",
        serde_json::json!({"source":original.source,"page":{"size":8,"expanded":true}}),
    )
    .await?;
    let original_response: serving::GetEvidenceResponse =
        serde_json::from_value(original_response).map_err(ModelError::codec)?;
    let full = &original_response.evidence.original;
    require(
        full.source == original.source
            && full.artifact == original.artifact
            && full.digest == original.digest
            && full.encoding == original.encoding
            && full.release == original.release
            && full.context == original.context,
        "native original retrieval changed the generated evidence attribution",
    )?;
    require(
        full.start <= original.start && full.end >= original.end,
        "native original source does not contain its search evidence window",
    )?;
    require(
        full.encoding.as_str() == "raw_bytes",
        "native linked source control requires actual original byte encoding",
    )?;
    let source =
        std::fs::read(runtime::root("catalog_core").join("api.py")).map_err(ModelError::codec)?;
    // Body offsets are relative to the source range, while ranked windows use artifact offsets.
    let body = &original_response.evidence.body;
    let absolute_start = full
        .start
        .checked_add(body.start)
        .ok_or(ModelError::Conflict("native evidence body start overflow"))?;
    let absolute_end = full
        .start
        .checked_add(body.end)
        .ok_or(ModelError::Conflict("native evidence body end overflow"))?;
    let start = usize::try_from(absolute_start).map_err(ModelError::codec)?;
    let end = usize::try_from(absolute_end).map_err(ModelError::codec)?;
    require(
        source.get(start..end) == Some(body.bytes.as_slice()),
        "generated linked evidence did not return the exact pinned original source bytes",
    )?;
    require(
        start < end && absolute_start <= original.start && absolute_end >= original.end,
        "native original page omitted its linked search evidence window",
    )?;
    let window_start = usize::try_from(original.start).map_err(ModelError::codec)?;
    let window_end = usize::try_from(original.end).map_err(ModelError::codec)?;
    require(
        body.bytes.get(window_start - start..window_end - start)
            == source.get(window_start..window_end),
        "native original page differs from its generated linked evidence window",
    )?;
    Ok(())
}

async fn journey(empty: bool) -> Result<(), ModelError> {
    journey_with_native_queries(empty, false).await
}
async fn journey_with_native_queries(empty: bool, queries: bool) -> Result<(), ModelError> {
    let scratch = tempfile::tempdir().map_err(ModelError::codec)?;
    let mut config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("explicit installed campaign fixture required"),
    ))?;
    let original_selection = config.selection.clone();
    let selection_before = original_selection
        .try_exists()
        .map_err(ModelError::codec)?
        .then(|| std::fs::read(&original_selection))
        .transpose()
        .map_err(ModelError::codec)?;
    config.selection = scratch.path().join("unused-selection.json");
    let native =
        lctx_compiler_store::compiler::NativeCompilerStore::begin(&config, Frontier::Catalog)
            .await?;
    let mut publication_owns_completion = false;
    let mut handle = None;
    let mut reader = None;
    let mut service = None;
    let result=async {
        let workspace=Workspace::new(Arc::new(model()?),WorkspaceOptions::default(),native.clone())?;
        let captured=if empty{
            let root=scratch.path().join("empty-input");std::fs::create_dir(&root).map_err(ModelError::codec)?;
            let capture=cpg_extract::capture::CapturedInput::capture_derived(&root,&[],workspace.budget(),&[],cpg_extract::acquisition::derive_blocks).map_err(|error|ModelError::Cause(Box::new(error)))?;
            Arc::new(cpg_extract::bundle::CapturedInputs::new(vec![cpg_extract::acquisition::AcquiredInput::tree(capture,"compact-empty")],
                cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog,workspace.budget())?))
        }else{runtime::try_capture("catalog_core",Profile::Catalog,workspace.budget())?};
        let library=if empty{"compact-empty"}else{"compact-catalog"};
        let captured=native_library::try_library_fixture_tree(captured,library,workspace.budget())?;
        let settings=ContentHash::of(b"actual-native-compact-analysis-control/v1");
        let prepared=PreparedCompilation::new(Frontier::Catalog,runtime::settings("api"),captured.config().catalog(),None,workspace.budget())?;
        if queries{eprintln!("native linked-analysis control: actual catalog compilation");}
        compilation::compile(&workspace,captured.clone(),Profile::Catalog,settings,Frontier::Catalog,Some(&prepared),None,None).await?;
        drop(prepared);
        let expected=analysis::owner::AnalysisOwner::ALL.into_iter().filter(|kind|kind.can_be_wholly_unrequested())
            .map(|kind|kind.name().to_owned()).collect::<BTreeSet<_>>();
        let contributions=native.contributions(workspace.budget()).await?;
        let actual=contributions.iter().filter(|owner|expected.contains(&owner.spec.producer)).collect::<Vec<_>>();
        require(actual.len()==9,"actual Catalog must complete all nine compact analysis owners")?;
        for owner in actual{
            require(owner.assurance.is_current() && owner.outcome==0,"compact declarations complete physical production without changing semantic NotRequested")?;
            require(owner.outputs.get(UniformNotRequested::NAME).is_some_and(|output|output.rows==1),"one actual captured input/context needs one compact descriptor")?;
            require(owner.outputs.iter().all(|(name,output)|name==UniformNotRequested::NAME || output.rows==0),"compact owner ordinary outputs must be explicitly empty")?;
        }
        // These ordinary consumers must run their real frame logic against compact predecessors.
        for name in [analysis::structural::Invocation::NAME,analysis::catalog_evidence::Invocation::NAME,
            analysis::selection::Invocation::NAME,analysis::synthesis::Invocation::NAME]{
            require(workspace.completed_relations()?.iter().any(|relation|relation.name()==name && relation.rows()>0),
                "actual compact predecessor consumer omitted its ordinary frame")?;
        }
        if queries{eprintln!("native linked-analysis control: independent artifact admission");}
        let admitted=artifact::admit(&workspace,&captured,Frontier::Catalog,Profile::Catalog,settings).await?;
        publication_owns_completion=true;
        if queries{eprintln!("native linked-analysis control: actual unselected publication");}
        handle=Some(lctx_publisher::seal_completed(&admitted,&config,&lctx_serving::native_definitions()).await?);
        reader=Some(NativeReader::connect(&config.endpoint,&config.writer_credentials(),handle.as_ref().unwrap().clone()).await?);
        let current=reader.as_ref().unwrap();
        let inputs=current.records::<input::InputRevision>(RecordSelection::Keys(vec![*captured.inputs()[0].captured().revision().id().bytes()])).await?;
        require(inputs.len()==1,"publication must retain the actual captured input")?;
        let rows=current.records::<UniformNotRequested>(RecordSelection::Scope{field:"input".into(),values:vec![serde_json::to_value(inputs[0].id()).map_err(ModelError::codec)?]}).await?;
        let completed=lctx_native::completed_state::published_completed_state(current,workspace.budget()).await?;
        let owners=completed.admit_uniform_owners(&rows,workspace.budget()).await?;
        let published=owners.owners().iter().filter(|owner|owner.descriptor().kind==UniformKind::AnalysisOwner)
            .map(|owner|owner.descriptor().owner.clone()).collect::<BTreeSet<_>>();
        require(published==expected,"publication must independently admit the exact nine current analysis owners")?;
        let mut changed=rows.iter().find(|row|row.kind==UniformKind::AnalysisOwner).ok_or(ModelError::Conflict("actual compact descriptor absent"))?.clone();
        changed.members=changed.members.checked_add(1).ok_or(ModelError::Conflict("compact descriptor count overflow"))?;
        require(completed.admit_uniform_owners(&[changed],workspace.budget()).await.is_err(),"published compact gate accepted changed actual domain")?;
        drop(owners);drop(completed);
        service=Some(lctx_serving::NativeService::new(current.clone(),Default::default())?);
        let serving=service.as_ref().unwrap();
        let request=serde_json::json!({"library":library,"page":{"size":8,"expanded":true}}).to_string();
        let browse=serving.execute("browse_library",&request).await.map_err(|error|ModelError::Cause(Box::new(error)))?;
        require(!browse.is_empty(),"actual serving compact admission returned no library packet")?;
        let operations=serving.execute("find_operations",&request).await.map_err(|error|ModelError::Cause(Box::new(error)))?;
        let operations:serde_json::Value=serde_json::from_str(&operations).map_err(ModelError::codec)?;
        let items=operations["supported"]["items"].as_array().ok_or(ModelError::Schema("actual operation response inventory"))?;
        require(items.is_empty()==empty,"serving operation inventory differs from the actual captured domain")?;
        if queries{
            eprintln!("native linked-analysis control: public native operation and linked evidence queries");
            linked_native_queries(current,serving,library).await?;
        }
        require(!config.selection.exists(),"compact publication selected itself")?;
        let after=original_selection.try_exists().map_err(ModelError::codec)?.then(||std::fs::read(&original_selection)).transpose().map_err(ModelError::codec)?;
        require(after==selection_before,"compact journey changed an existing selection")?;
        Ok(())
    }.await;
    let mut terminal = completion::Completion::default();
    if let Some(service) = service.take() {
        service.close().await;
        drop(service);
    }
    if let Some(reader) = reader.take() {
        let close = reader.close().await;
        let confirmed = close.is_ok();
        terminal.step("compact native reader close", close);
        if confirmed {
            terminal.step(
                "compact native reader invalidation",
                reader
                    .client()
                    .invalidate()
                    .await
                    .map_err(ModelError::codec),
            );
        }
        drop(reader);
    }
    let confirmed = result
        .as_ref()
        .err()
        .is_none_or(ModelError::permits_storage_cleanup)
        && terminal.local == completion::LocalState::Terminal
        && terminal.remote == completion::RemoteState::Confirmed
        && terminal
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup());
    if !publication_owns_completion && confirmed {
        terminal.step(
            "compact producing attempt abandonment",
            native.abandon().await,
        );
    }
    if confirmed && let Some(handle) = handle {
        terminal.step(
            "compact unselected publication retirement",
            retire_owned(&config, &handle).await,
        );
    }
    completion::complete(result, terminal)
}

#[tokio::test(flavor = "multi_thread")]
async fn current_catalog_compact_owners_cross_producer_compiler_publication_and_serving() {
    journey(false).await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn current_catalog_compact_owners_preserve_explicit_empty_input_across_four_boundaries() {
    journey(true).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn current_catalog_analysis_links_public_native_operations_and_original_evidence() {
    use tracing::instrument::WithSubscriber;
    journey_with_native_queries(false, true)
        .with_subscriber(cpg_extract::logging::dispatch())
        .await
        .unwrap();
}
