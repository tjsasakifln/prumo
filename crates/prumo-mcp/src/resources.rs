//! Only opaque URIs for the bound project are resolvable. Never filesystem paths.
use super::{
    AppError, CacheScope, ErrorData, ListResourcesResult, PaginatedRequestParams, PrumoMcp,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents,
};
use prumo_app::access::AccessScope;
use rmcp::model::ErrorCode;
use serde_json::json;

fn error(error: AppError) -> ErrorData {
    ErrorData::new(
        ErrorCode(-32001),
        error.to_string(),
        Some(json!({"code":error.code()})),
    )
}

pub(super) async fn list(
    server: &PrumoMcp,
    request: Option<PaginatedRequestParams>,
) -> Result<ListResourcesResult, ErrorData> {
    if request.is_some_and(|request| request.cursor.is_some()) {
        return Err(ErrorData::invalid_params(
            "Cursor inválido: os recursos são retornados em uma página.",
            None,
        ));
    }
    server
        .service
        .grant()
        .check_scope(AccessScope::Read)
        .map_err(error)?;
    let permit = server.available.clone().try_acquire_owned().map_err(|_| {
        ErrorData::new(
            ErrorCode(-32001),
            "Projeto ocupado.",
            Some(json!({"code":"busy"})),
        )
    })?;
    let service = server.service.clone();
    let resources = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let model = match service.read_scoped_model() {
            Ok(model) => model,
            Err(AppError::NotFound) => return Ok(Vec::new()),
            Err(failure) => return Err(failure),
        };
        let prefix = format!("prumo://project/{}", model.project_id);
        let mut resources = vec![Resource::new(format!("{prefix}/model"), "Modelo autorizado")
            .with_mime_type("application/json")
            .with_description("Peças e conexões das disciplinas autorizadas; posições em metros e revisão de origem.")];
        if service.grant().all_disciplines() {
            resources.push(Resource::new(format!("{prefix}/snapshot"), "Snapshot do projeto")
                .with_mime_type("application/json")
                .with_description("Entradas canônicas e SHA-256. Exige leitura de todas as disciplinas."));
            resources.push(Resource::new(format!("{prefix}/results"), "Rastreabilidade das medidas")
                .with_mime_type("application/json")
                .with_description("Medidas geométricas, base e invalidações; não é cálculo nem emissão técnica."));
        }
        Ok(resources)
    }).await.map_err(|_| ErrorData::internal_error("Falha ao consultar recursos.", None))?.map_err(error)?;
    Ok(ListResourcesResult::with_all_items(resources)
        .with_ttl_ms(0)
        .with_cache_scope(CacheScope::Private))
}

pub(super) async fn read(
    server: &PrumoMcp,
    request: ReadResourceRequestParams,
) -> Result<ReadResourceResponse, ErrorData> {
    if request.input_responses.is_some() || request.request_state.is_some() {
        return Err(ErrorData::invalid_params(
            "O recurso não aceita retomada de tarefa.",
            None,
        ));
    }
    server
        .service
        .grant()
        .check_scope(AccessScope::Read)
        .map_err(error)?;
    let permit = server.available.clone().try_acquire_owned().map_err(|_| {
        ErrorData::new(
            ErrorCode(-32001),
            "Projeto ocupado.",
            Some(json!({"code":"busy"})),
        )
    })?;
    let service = server.service.clone();
    let uri = request.uri;
    let (uri, value) = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        // Every returned value and its revision come from one read.
        let value = if uri.ends_with("/model") {
            let model = service.read_scoped_model()?;
            if uri != format!("prumo://project/{}/model", model.project_id) {
                return Err(AppError::PermissionDenied);
            }
            json!({"model":model,"calculation_available":false})
        } else {
            service.grant().check_all_disciplines()?;
            let project = service.read()?;
            let prefix = format!("prumo://project/{}", project.id);
            if uri == format!("{prefix}/snapshot") {
                let snapshot = prumo_domain::canonical::model_snapshot(&project)
                    .map_err(|failure| AppError::InvalidInput(failure.to_string()))?;
                json!({"snapshot":snapshot,"calculation_available":false})
            } else if uri == format!("{prefix}/results") {
                json!({"project_id":project.id,"revision":project.revision,
                    "units":{"length":"m"},"dependencies":project.dependencies,
                    "calculation_available":false})
            } else {
                return Err(AppError::PermissionDenied);
            }
        };
        Ok((uri, value))
    })
    .await
    .map_err(|_| ErrorData::internal_error("Falha ao ler recurso.", None))?
    .map_err(error)?;
    let text = serde_json::to_string(&value)
        .map_err(|_| ErrorData::internal_error("Falha ao representar recurso.", None))?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri).with_mime_type("application/json"),
    ])
    .with_ttl_ms(0)
    .with_cache_scope(CacheScope::Private)
    .into())
}
