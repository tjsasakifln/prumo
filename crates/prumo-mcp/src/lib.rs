//! Local MCP adapter. All model operations go through `ProjectService`.
#![forbid(unsafe_code)]

use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use prumo_app::access::{AccessScope, ProjectGrant};
use prumo_app::{AccessMode, AppError, Command, ProjectService};
use prumo_domain::model::ModelEdit;
use prumo_domain::registry::NormReferenceInput;
use prumo_domain::storage::StorageAction;
use prumo_domain::transactions::{
    TransactionHistoryRequest, TransactionRequest, TransactionResult,
};
use prumo_domain::{LengthUnit, MAX_NAME_CHARACTERS, Project};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, Implementation,
        ListResourcesResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
        ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
        ResourceContents, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::sync::Semaphore;

mod authoring_schema;
mod catalogue_schema;
mod extensions;
mod input;
mod legacy;
mod model_schema;
mod resources;
mod surface_assembly_schema;
mod transaction_schema;
pub use legacy::{LEGACY_PROTOCOL, LegacyPrumoMcp};

pub use input::{GuardedInput, guarded_input};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const SUPPORTED_PROTOCOL: &str = "2026-07-28";

/// A server is permanently bound to the path and access granted by its host.
/// No tool accepts paths, actors, permission levels, SQL, or executable code.
#[derive(Clone)]
pub struct PrumoMcp {
    service: Arc<ProjectService>,
    available: Arc<Semaphore>,
}

impl PrumoMcp {
    pub fn open(
        path: impl Into<PathBuf>,
        access: AccessMode,
        actor: impl Into<String>,
    ) -> Result<Self, AppError> {
        Self::open_scoped(path, ProjectGrant::for_access(access), actor)
    }

    pub fn open_scoped(
        path: impl Into<PathBuf>,
        grant: ProjectGrant,
        actor: impl Into<String>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            service: Arc::new(ProjectService::open_scoped(path, grant, actor)?),
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Gate {
    capability_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectModel {
    expected_revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditModel {
    expected_revision: u64,
    edit: ModelEdit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageMigration {
    expected_revision: u64,
    inspection_sha256: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageBackup {
    expected_revision: u64,
    inspection_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageRecovery {
    candidate_id: String,
    inspection_sha256: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterNorm {
    expected_revision: u64,
    reference: NormReferenceInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PinNorms {
    expected_revision: u64,
    package_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrateNorms {
    expected_revision: u64,
    from_package_hash: String,
    to_package_hash: String,
    confirm: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyTransaction {
    plan_sha256: String,
}

enum Operation {
    Read,
    ModelQuery,
    AccessInfo,
    TransactionPreview(Box<TransactionRequest>),
    TransactionApply(ApplyTransaction),
    TransactionHistory,
    TransactionUndo(TransactionHistoryRequest),
    TransactionRedo(TransactionHistoryRequest),
    Unavailable(AccessScope),
    Extension(extensions::Operation),
    Create(Create),
    AddMember(AddMember),
    Rename(Rename),
    AddEnvironment(Rename),
    Registry,
    Coverage,
    NormativePackage,
    Gate(Gate),
    RegisterNorm(Box<RegisterNorm>),
    PinNorms(PinNorms),
    MigrateNorms(MigrateNorms),
    Components,
    QualifyComponents,
    ModelSnapshot,
    DependencyTrace,
    InspectModel(InspectModel),
    EditModel(Box<EditModel>),
    StorageStatus,
    Storage(Box<StorageAction>),
}

pub(crate) fn decode<T: DeserializeOwned>(arguments: Value) -> Result<T, ErrorData> {
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
            "prumo.model.query" => {
                decode::<Empty>(args)?;
                Ok(Self::ModelQuery)
            }
            "prumo.access.read" => {
                decode::<Empty>(args)?;
                Ok(Self::AccessInfo)
            }
            "prumo.model.preview_change" => {
                decode(args).map(|request| Self::TransactionPreview(Box::new(request)))
            }
            "prumo.model.apply_change" => decode(args).map(Self::TransactionApply),
            "prumo.model.history" => {
                decode::<Empty>(args)?;
                Ok(Self::TransactionHistory)
            }
            "prumo.model.undo" => decode(args).map(Self::TransactionUndo),
            "prumo.model.redo" => decode(args).map(Self::TransactionRedo),
            "prumo.analysis.submit" => Ok(Self::Unavailable(AccessScope::Calculation)),
            "prumo.documents.issue" => Ok(Self::Unavailable(AccessScope::Documents)),
            "prumo.project.read" => {
                decode::<Empty>(args)?;
                Ok(Self::Read)
            }
            "prumo.project.create" => decode(args).map(Self::Create),
            "prumo.model.add_member" => decode(args).map(Self::AddMember),
            "prumo.project.rename" => decode(args).map(Self::Rename),
            "prumo.model.add_environment" => decode(args).map(Self::AddEnvironment),
            "prumo.registry.read" => {
                decode::<Empty>(args)?;
                Ok(Self::Registry)
            }
            "prumo.registry.audit" => {
                decode::<Empty>(args)?;
                Ok(Self::Coverage)
            }
            "prumo.norms.package" => {
                decode::<Empty>(args)?;
                Ok(Self::NormativePackage)
            }
            "prumo.results.gate" => decode(args).map(Self::Gate),
            "prumo.norms.register" => {
                decode(args).map(|params| Self::RegisterNorm(Box::new(params)))
            }
            "prumo.norms.pin" => decode(args).map(Self::PinNorms),
            "prumo.norms.migrate" => decode(args).map(Self::MigrateNorms),
            "prumo.components.list" => {
                decode::<Empty>(args)?;
                Ok(Self::Components)
            }
            "prumo.components.qualify" => {
                decode::<Empty>(args)?;
                Ok(Self::QualifyComponents)
            }
            "prumo.model.snapshot" => {
                decode::<Empty>(args)?;
                Ok(Self::ModelSnapshot)
            }
            "prumo.results.trace" => {
                decode::<Empty>(args)?;
                Ok(Self::DependencyTrace)
            }
            "prumo.model.inspect" => decode(args).map(Self::InspectModel),
            "prumo.model.edit" => decode(args).map(|params| Self::EditModel(Box::new(params))),
            "prumo.storage.status" => {
                decode::<Empty>(args)?;
                Ok(Self::StorageStatus)
            }
            "prumo.storage.migrate" => {
                let params: StorageMigration = decode(args)?;
                Ok(Self::Storage(Box::new(StorageAction::Migrate {
                    expected_revision: params.expected_revision,
                    inspection_sha256: params.inspection_sha256,
                    confirm: params.confirm,
                })))
            }
            "prumo.storage.backup" => {
                let params: StorageBackup = decode(args)?;
                Ok(Self::Storage(Box::new(StorageAction::Backup {
                    expected_revision: params.expected_revision,
                    inspection_sha256: params.inspection_sha256,
                })))
            }
            "prumo.storage.recover" => {
                let params: StorageRecovery = decode(args)?;
                Ok(Self::Storage(Box::new(StorageAction::Recover {
                    candidate_id: params.candidate_id,
                    inspection_sha256: params.inspection_sha256,
                    confirm: params.confirm,
                })))
            }
            name => match extensions::Operation::parse(name, args) {
                Some(result) => result.map(Self::Extension),
                None => Err(ErrorData::invalid_params(
                    "Ferramenta inexistente.",
                    Some(json!({"code":"unknown_tool"})),
                )),
            },
        }
    }

    fn required_scope(&self) -> AccessScope {
        match self {
            Self::Create(_)
            | Self::AddMember(_)
            | Self::Rename(_)
            | Self::AddEnvironment(_)
            | Self::RegisterNorm(_)
            | Self::PinNorms(_)
            | Self::MigrateNorms(_)
            | Self::InspectModel(_)
            | Self::EditModel(_)
            | Self::Storage(_)
            | Self::TransactionPreview(_)
            | Self::TransactionApply(_)
            | Self::TransactionUndo(_)
            | Self::TransactionRedo(_) => AccessScope::Model,
            Self::Unavailable(scope) => *scope,
            Self::Extension(operation) => operation.required_scope(),
            _ => AccessScope::Read,
        }
    }

    fn needs_all_disciplines(&self) -> bool {
        match self {
            Self::ModelQuery | Self::AccessInfo | Self::AddMember(_) | Self::EditModel(_) => false,
            Self::Extension(operation) => operation.needs_all_disciplines(),
            _ => true,
        }
    }

    fn run(self, service: &ProjectService, cancel: &AtomicBool) -> Result<Value, AppError> {
        service.grant().check_scope(self.required_scope())?;
        service.grant().check_scope(AccessScope::Read)?;
        if self.needs_all_disciplines() {
            service.grant().check_all_disciplines()?;
        }
        let project = match self {
            Self::ModelQuery => return service.read_scoped_model().map(|model| json!({"model":model,"calculation_available":false})),
            Self::AccessInfo => return Ok(json!({"grant":service.grant(),"calculation_available":false,"document_issue_available":false})),
            Self::TransactionPreview(request) => return service.preview_transaction(*request).map(|preview| json!({"preview":preview,"calculation_available":false})),
            Self::TransactionApply(request) => return transaction_response(service.apply_transaction(&request.plan_sha256)?, service),
            Self::TransactionHistory => return service.transaction_history().map(|history| json!({"history":history})),
            Self::TransactionUndo(request) => return transaction_response(service.undo_transaction(request)?, service),
            Self::TransactionRedo(request) => return transaction_response(service.redo_transaction(request)?, service),
            Self::Unavailable(_) => return Err(AppError::CapabilityUnavailable("Capacidade ainda não implementada; a concessão de acesso não libera cálculo ou emissão.".into())),
            Self::Extension(operation) => return operation.run(service, cancel),
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
            Self::Registry => {
                return service
                    .registry_catalog()
                    .map(|catalog| json!({"catalog":catalog}));
            }
            Self::Coverage => return service.coverage_audit().map(|audit| json!({"audit":audit})),
            Self::NormativePackage => {
                return service.normative_package_preview().map(|preview| {
                    json!({
                        "package":preview.candidate,
                        "project_revision":preview.project_revision,
                        "active_hash":preview.active_hash,
                        "impact":preview.impact
                    })
                });
            }
            Self::Gate(params) => {
                return service
                    .final_result_gate(&params.capability_id)
                    .map(|gate| json!({"gate":gate}));
            }
            Self::Components => return Ok(json!({"components":service.component_inventory()})),
            Self::QualifyComponents => {
                return service
                    .qualify_components_with_cancel(cancel)
                    .map(|report| json!({"qualification":report,"calculation_available":false}));
            }
            Self::ModelSnapshot => {
                return service
                    .model_snapshot()
                    .map(|snapshot| json!({"snapshot":snapshot,"calculation_available":false}));
            }
            Self::DependencyTrace => {
                return service
                    .dependency_report()
                    .map(|dependencies| json!({"dependencies":dependencies,"calculation_available":false}));
            }
            Self::InspectModel(params) => service.inspect_model(params.expected_revision),
            Self::EditModel(params) => service.execute(Command::EditModel {
                expected_revision: params.expected_revision,
                edit: params.edit,
            }),
            Self::StorageStatus => {
                return service
                    .storage_status()
                    .map(|status| json!({"storage":status}));
            }
            Self::Storage(action) => {
                return service.execute_storage(*action).map(|result| {
                    json!({
                        "project":result.project,"storage":result.status,"changed":result.changed,
                        "message":result.message,"calculation_available":false
                    })
                });
            }
            Self::RegisterNorm(params) => service.execute(Command::RegisterNormReference {
                expected_revision: params.expected_revision,
                reference: params.reference,
            }),
            Self::PinNorms(params) => service.execute(Command::PinNormativePackage {
                expected_revision: params.expected_revision,
                package_hash: params.package_hash,
            }),
            Self::MigrateNorms(params) => service.execute(Command::MigrateNormativePackage {
                expected_revision: params.expected_revision,
                from_package_hash: params.from_package_hash,
                to_package_hash: params.to_package_hash,
                confirm: params.confirm,
            }),
        }?;
        project_response(project, service)
    }
}

fn project_response(project: Project, service: &ProjectService) -> Result<Value, AppError> {
    if service.grant().all_disciplines() {
        Ok(json!({"project":project,"calculation_available":false}))
    } else {
        Ok(json!({"model":service.grant().project_view(&project)?,"calculation_available":false}))
    }
}

fn transaction_response(
    result: TransactionResult,
    service: &ProjectService,
) -> Result<Value, AppError> {
    let mut response = project_response(result.project, service)?;
    response["receipt"] = serde_json::to_value(result.receipt)?;
    response["replayed"] = json!(result.replayed);
    Ok(response)
}

pub(crate) fn definition(
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
fn base_tool_catalog(writable: bool) -> Vec<Tool> {
    let text = json!({"type":"string", "minLength":1, "maxLength":MAX_NAME_CHARACTERS});
    let revision = json!({"type":"integer", "minimum":1});
    let point = json!({"type":"array", "items":{"type":"number"}, "minItems":3, "maxItems":3});
    let mut tools = vec![definition(
        "prumo.project.read",
        "Lê o projeto selecionado, sua revisão, unidades e identidades. Não calcula o projeto nem libera emissão técnica.",
        json!({}),
        &[],
        true,
    )];
    tools.extend([
        definition("prumo.registry.read","Consulta as capacidades, fontes e referências normativas do projeto. Metadados públicos não equivalem a texto licenciado.",json!({}),&[],true),
        definition("prumo.registry.audit","Audita vínculos com as 72 famílias, issues, fontes e normas, separando erros de cobertura e pendências técnicas.",json!({}),&[],true),
        definition("prumo.norms.package","Consulta o pacote candidato, revisão atual, hash ativo e impacto previsto da migração. Preserva o projeto; confira o impacto antes de confirmar.",json!({}),&[],true),
        definition("prumo.results.gate","Explica por que uma capacidade pode ou não emitir resultado final no pacote fixado do projeto. Não calcula nem concede aprovação.",json!({"capability_id":text}),&["capability_id"],true),
        definition("prumo.components.list","Consulta componentes, versões, procedência, licenças e limites dos adaptadores instalados.",json!({}),&[],true),
        definition("prumo.components.qualify","Executa a bateria fixa de qualificação em processo isolado, com prazo e recursos limitados. Retorna medidas e artefato de ensaio; não altera o projeto nem libera cálculo.",json!({}),&[],true),
        definition("prumo.model.snapshot","Consulta o snapshot canônico completo dos dados de entrada, sua revisão, SHA-256 e fontes. Não altera o modelo.",json!({}),&[],true),
        definition("prumo.results.trace","Consulta dependências, medidas geométricas e causas de invalidação. Nenhuma inspeção equivale a dimensionamento.",json!({}),&[],true),
        definition("prumo.storage.status","Inspeciona o arquivo selecionado, formato, integridade, revisão e candidatos de recuperação, inclusive quando o projeto não abre. Não altera arquivos ou cria backups.",json!({}),&[],true),
    ]);
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
        let nullable_text = json!({"type":["string","null"],"maxLength":MAX_NAME_CHARACTERS});
        let long_text = json!({"type":"string","minLength":1,"maxLength":1000});
        let identifier =
            json!({"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"});
        let texts = json!({"type":"array","items":long_text});
        let hash = json!({"type":"string","pattern":"^[a-f0-9]{64}$"});
        let reference = json!({
            "type":"object","additionalProperties":false,
            "properties":{
                "id":identifier,"designation":long_text,"part":nullable_text,"edition":nullable_text,
                "amendments":texts,"errata":texts,"title":long_text,"publisher":long_text,
                "source_url":{"type":"string","format":"uri","pattern":"^https?://","maxLength":2048},
                "domain":text,"interpretation_owner_role":long_text
            },
            "required":["id","designation","part","edition","amendments","errata","title","publisher","source_url","domain","interpretation_owner_role"]
        });
        tools.push(definition("prumo.norms.register","Registra referência pública no catálogo candidato, sempre sem licença ou aprovação técnica. Preserva o pacote ativo até migração explícita.",json!({"expected_revision":revision,"reference":reference}),&["expected_revision","reference"],false));
        tools.push(definition("prumo.norms.pin","Fixa pela primeira vez o pacote candidato identificado pelo hash exibido. Não aprova capacidades nem substitui pacote já fixado.",json!({"expected_revision":revision,"package_hash":hash}),&["expected_revision","package_hash"],false));
        tools.push(definition("prumo.norms.migrate","Migra explicitamente entre os hashes exibidos, registra impactos e exige recálculo. Revisão, origem, destino e confirmação devem coincidir.",json!({"expected_revision":revision,"from_package_hash":hash,"to_package_hash":hash,"confirm":{"type":"boolean"}}),&["expected_revision","from_package_hash","to_package_hash","confirm"],false));
        tools.push(definition(
            "prumo.model.add_environment",
            "Adiciona um ambiente ao projeto na revisão atual, sem cálculo de instalações.",
            json!({"expected_revision":revision,"name":text}),
            &["expected_revision", "name"],
            false,
        ));
        tools.push(definition(
            "prumo.model.inspect",
            "Atualiza e salva medidas geométricas do modelo, vinculadas ao snapshot e às dependências. Não calcula resistência nem aprova elementos.",
            json!({"expected_revision":revision}),
            &["expected_revision"],
            false,
        ));
        tools.push(definition(
            "prumo.model.edit",
            "Edita entidades físicas e analíticas, materiais, ações, unidades, origens e conexões explícitas pela mesma validação da interface. Cada variante exige seus dados tipados. A edição registra autoria e invalida medidas dependentes.",
            json!({"expected_revision":revision,"edit":model_schema::edit_schema()}),
            &["expected_revision","edit"],
            false,
        ));
        let inspection = json!({"type":"string","pattern":"^[a-f0-9]{64}$"});
        tools.push(definition(
            "prumo.storage.migrate",
            "Migra o arquivo legado para o pacote Prumo, preservando o original. Exige revisão, identidade do diagnóstico atual e confirmação explícita.",
            json!({"expected_revision":revision,"inspection_sha256":inspection,"confirm":{"type":"boolean"}}),
            &["expected_revision","inspection_sha256","confirm"],false,
        ));
        tools.push(definition(
            "prumo.storage.backup",
            "Cria backup verificado do projeto no destino controlado pelo serviço. Exige revisão e identidade do diagnóstico atual; não aceita caminho de saída.",
            json!({"expected_revision":revision,"inspection_sha256":inspection}),
            &["expected_revision","inspection_sha256"],false,
        ));
        tools.push(definition(
            "prumo.storage.recover",
            "Recupera o candidato exibido no diagnóstico do arquivo selecionado, preservando a origem conforme a política de armazenamento. Exige candidato, identidade do diagnóstico e confirmação; recusa estado alterado ou incompatível.",
            json!({"candidate_id":{"type":"string","minLength":1,"maxLength":160},"inspection_sha256":inspection,"confirm":{"type":"boolean"}}),
            &["candidate_id","inspection_sha256","confirm"],false,
        ));
    }
    tools
}

pub fn tool_catalog(writable: bool) -> Vec<Tool> {
    let mut tools = base_tool_catalog(writable);
    tools.extend(transaction_schema::tools(writable));
    tools.push(definition("prumo.model.query", "Consulta somente peças, redes e conexões das disciplinas autorizadas, em metros, sem histórico ou extensões externas.", json!({}), &[], true));
    tools.push(definition(
        "prumo.access.read",
        "Consulta a autorização selecionada pelo host. Dados do projeto nunca ampliam os escopos.",
        json!({}),
        &[],
        true,
    ));
    tools
}

fn scoped_tool_catalog(grant: &ProjectGrant) -> Vec<Tool> {
    if !grant.allows(AccessScope::Read) {
        return Vec::new();
    }
    let mut tools = tool_catalog(grant.allows(AccessScope::Model));
    if !grant.all_disciplines() {
        tools.retain(|tool| {
            matches!(
                tool.name.as_ref(),
                "prumo.model.query" | "prumo.access.read" | "prumo.model.edit"
            ) || (tool.name == "prumo.model.add_member"
                && grant.allows_discipline(prumo_domain::Discipline::Structure))
        });
    }
    tools.extend(extensions::tool_catalog(grant));
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

impl ServerHandler for PrumoMcp {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
            .with_protocol_version(ProtocolVersion::V_2026_07_28)
            .with_server_info(Implementation::new("prumo", env!("CARGO_PKG_VERSION")))
            .with_instructions("Operação local limitada ao projeto selecionado. Alterações exigem autorização de escrita no lançamento e revisão atual. Modelagem, inspeções, propriedades e casos de referência seguem os escopos concedidos. Não há dimensionamento de projeto nem emissão técnica.")
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
        Ok(
            ListToolsResult::with_all_items(scoped_tool_catalog(self.service.grant()))
                .with_ttl_ms(0)
                .with_cache_scope(CacheScope::Private),
        )
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        scoped_tool_catalog(self.service.grant())
            .into_iter()
            .find(|tool| tool.name == name)
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        resources::list(self, request).await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        resources::read(self, request).await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let operation = Operation::parse(request)?;
        let Ok(permit) = self.available.clone().try_acquire_owned() else {
            return Ok(CallToolResult::structured_error(json!({"code":"busy","message":"Projeto ocupado. Aguarde a operação em andamento."})).into());
        };
        let service = self.service.clone();
        let cancelled = Arc::new(AtomicBool::new(context.ct.is_cancelled()));
        let _cancel_on_drop = CancelOnDrop(cancelled.clone());
        let worker_cancelled = cancelled.clone();
        let mut work = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation.run(&service, &worker_cancelled)
        });
        let result = tokio::select! {
            result = &mut work => result,
            _ = context.ct.cancelled() => {
                cancelled.store(true, Ordering::Release);
                work.await
            }
        }
        .map_err(|_| ErrorData::internal_error("Falha ao executar o comando local.", None))?;
        let response = match result {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => CallToolResult::structured_error(
                json!({"code":error.code(),"message":error.to_string()}),
            ),
        };
        Ok(response.into())
    }
}

/// Dropping a cancelled/disconnected request also stops its isolated experiment.
/// Ordinary model transactions do not consume this flag and remain atomic.
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
