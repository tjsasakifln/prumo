//! Local MCP adapter. All model operations go through `ProjectService`.
#![forbid(unsafe_code)]

use std::{borrow::Cow, path::PathBuf, sync::Arc};

use prumo_app::{AccessMode, AppError, ProjectService};
use prumo_domain::{LengthUnit, MAX_NAME_CHARACTERS, Project};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig, Tool,
        ToolAnnotations,
    },
    service::RequestContext,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::sync::Semaphore;

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const SUPPORTED_PROTOCOL: &str = "2026-07-28";

/// A server is permanently bound to the path and access granted by its host.
/// No tool accepts paths, actors, permission levels, SQL, or executable code.
#[derive(Clone)]
pub struct PrumoMcp {
    service: Arc<ProjectService>,
    writable: bool,
    available: Arc<Semaphore>,
}

impl PrumoMcp {
    pub fn open(
        path: impl Into<PathBuf>,
        access: AccessMode,
        actor: impl Into<String>,
    ) -> Result<Self, AppError> {
        let writable = matches!(access, AccessMode::ReadWrite);
        Ok(Self {
            service: Arc::new(ProjectService::open(path, access, actor)?),
            writable,
            available: Arc::new(Semaphore::new(4)),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    name: String,
    units: LengthUnit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddMember {
    expected_revision: u64,
    name: String,
    start: [f64; 3],
    end: [f64; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    expected_revision: u64,
    name: String,
}

enum Operation {
    Read,
    Create(Create),
    AddMember(AddMember),
    Rename(Rename),
    AddEnvironment(Rename),
}

fn decode<T: DeserializeOwned>(arguments: Value) -> Result<T, ErrorData> {
    serde_json::from_value(arguments).map_err(|error| {
        ErrorData::invalid_params(
            format!("Entrada inválida: {error}"),
            Some(json!({"code": "invalid_input"})),
        )
    })
}

impl Operation {
    fn parse(request: CallToolRequestParams) -> Result<Self, ErrorData> {
        if request.input_responses.is_some() || request.request_state.is_some() {
            return Err(ErrorData::invalid_params(
                "Esta operação não aceita retomada de tarefa.",
                None,
            ));
        }
        let args = Value::Object(request.arguments.unwrap_or_default());
        match request.name.as_ref() {
            "prumo.project.read" => {
                decode::<Empty>(args)?;
                Ok(Self::Read)
            }
            "prumo.project.create" => decode(args).map(Self::Create),
            "prumo.model.add_member" => decode(args).map(Self::AddMember),
            "prumo.project.rename" => decode(args).map(Self::Rename),
            "prumo.model.add_environment" => decode(args).map(Self::AddEnvironment),
            _ => Err(ErrorData::invalid_params(
                "Ferramenta inexistente.",
                Some(json!({"code":"unknown_tool"})),
            )),
        }
    }

    fn run(self, service: &ProjectService) -> Result<Project, AppError> {
        match self {
            Self::Read => service.read(),
            Self::Create(params) => service.create(params.name, params.units),
            Self::AddMember(params) => service.add_member(
                params.expected_revision,
                params.name,
                params.start,
                params.end,
            ),
            Self::Rename(params) => service.rename_project(params.expected_revision, params.name),
            Self::AddEnvironment(params) => {
                service.add_environment(params.expected_revision, params.name)
            }
        }
    }
}

fn definition(
    name: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
    read_only: bool,
) -> Tool {
    let schema = json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    });
    let mut tool = Tool::new(
        name,
        description,
        schema.as_object().expect("schema is an object").clone(),
    );
    tool.annotations = Some(
        ToolAnnotations::new()
            .read_only(read_only)
            .destructive(!read_only)
            .idempotent(read_only)
            .open_world(false),
    );
    tool
}

/// Exact public operation catalogue. Schemas and deserializers both reject extra fields.
pub fn tool_catalog(writable: bool) -> Vec<Tool> {
    let text = json!({"type":"string", "minLength":1, "maxLength":MAX_NAME_CHARACTERS});
    let revision = json!({"type":"integer", "minimum":1});
    let point = json!({"type":"array", "items":{"type":"number"}, "minItems":3, "maxItems":3});
    let mut tools = vec![definition(
        "prumo.project.read",
        "Lê o projeto selecionado, sua revisão, unidades e identidades. Nenhum cálculo está disponível.",
        json!({}),
        &[],
        true,
    )];
    if writable {
        tools.push(definition(
            "prumo.project.create",
            "Cria o projeto no destino já selecionado pelo usuário. Não substitui projeto existente.",
            json!({"name":text,"units":{"type":"string","enum":["metre","centimetre","millimetre"]}}),
            &["name","units"], false,
        ));
        tools.push(definition(
            "prumo.model.add_member",
            "Adiciona membro físico e identidade analítica. Coordenadas seguem a unidade do projeto. Não realiza dimensionamento.",
            json!({"expected_revision":revision,"name":text,"start":point,"end":point}),
            &["expected_revision","name","start","end"], false,
        ));
        tools.push(definition(
            "prumo.project.rename",
            "Altera o nome do projeto quando a revisão informada ainda é atual.",
            json!({"expected_revision":revision,"name":text}),
            &["expected_revision", "name"],
            false,
        ));
        tools.push(definition(
            "prumo.model.add_environment",
            "Adiciona um ambiente ao projeto na revisão atual, sem cálculo de instalações.",
            json!({"expected_revision":revision,"name":text}),
            &["expected_revision", "name"],
            false,
        ));
    }
    tools
}

impl ServerHandler for PrumoMcp {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2026_07_28)
            .with_server_info(Implementation::new("prumo", env!("CARGO_PKG_VERSION")))
            .with_instructions("Operação local limitada ao projeto selecionado. Alterações exigem autorização de escrita no lançamento e revisão atual. Modelagem inicial, sem cálculo ou emissão técnica.")
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        if request.is_some_and(|request| request.cursor.is_some()) {
            return Err(ErrorData::invalid_params(
                "Cursor inválido: catálogo retornado em uma página.",
                None,
            ));
        }
        Ok(ListToolsResult::with_all_items(tool_catalog(self.writable)))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_catalog(true)
            .into_iter()
            .find(|tool| tool.name == name)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let operation = Operation::parse(request)?;
        let Ok(permit) = self.available.clone().try_acquire_owned() else {
            return Ok(CallToolResult::structured_error(json!({"code":"busy","message":"Projeto ocupado. Aguarde a operação em andamento."})).into());
        };
        let service = self.service.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation.run(&service)
        })
        .await
        .map_err(|_| ErrorData::internal_error("Falha ao executar o comando local.", None))?;
        let response = match result {
            Ok(project) => {
                CallToolResult::structured(json!({"project":project,"calculation_available":false}))
            }
            Err(error) => CallToolResult::structured_error(
                json!({"code":error.code(),"message":error.to_string()}),
            ),
        };
        Ok(response.into())
    }
}
