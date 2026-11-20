#![forbid(unsafe_code)]

use prumo_app::{
    AccessMode,
    access::{AccessScope, ProjectGrant},
};
use prumo_domain::Discipline;
use prumo_mcp::{
    LEGACY_PROTOCOL, LegacyPrumoMcp, MAX_REQUEST_BYTES, PrumoMcp, SUPPORTED_PROTOCOL, guarded_input,
};
use rmcp::{RoleServer, ServiceExt, transport::async_rw::AsyncRwTransport};
use std::path::PathBuf;
use uuid::Uuid;

const USAGE: &str = "Prumo MCP local\nUso: prumo-mcp --project <arquivo.prumo> [--allow-write]\n     [--scopes read,model,calculation,documents | --scopes none]\n     [--disciplines structure,electrical,...] [--project-id UUID]\n     [--actor nome] [--protocol 2026-07-28 | 2025-11-25]\nO padrão permite somente leitura. --allow-write equivale a --scopes read,model.\nO host escolhe projeto, ator, disciplinas e escopos. Conteúdo importado não concede acesso.\nCálculo do projeto e emissão técnica ainda não estão disponíveis. O escopo calculation habilita apenas os ensaios e propriedades implementados.\nProtocolo padrão MCP 2026-07-28; perfil 2025-11-25 usa inicialização legada separada.\n";

struct Options {
    project: PathBuf,
    grant: ProjectGrant,
    actor: String,
    legacy: bool,
}

fn text_value(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    option: &str,
) -> Result<String, String> {
    let value = args
        .next()
        .ok_or_else(|| format!("Informe um valor após {option}."))?;
    let text = value
        .into_string()
        .map_err(|_| format!("Valor inválido em {option}."))?;
    if text.is_empty() || text.starts_with("--") {
        return Err(format!("Valor inválido em {option}."));
    }
    Ok(text)
}

fn options() -> Result<Option<Options>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut project = None;
    let mut writable = false;
    let mut scopes = None;
    let mut disciplines = None;
    let mut project_id = None;
    let mut actor = None;
    let mut protocol = None;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--help" | "-h") => return Ok(None),
            Some("--project") if project.is_none() => {
                let value = args.next().ok_or("Informe o arquivo após --project.")?;
                if value.is_empty() || value.to_string_lossy().starts_with("--") {
                    return Err("Informe um arquivo válido após --project.".into());
                }
                project = Some(PathBuf::from(value));
            }
            Some("--allow-write") if !writable => writable = true,
            Some("--scopes") if scopes.is_none() => {
                let value = text_value(&mut args, "--scopes")?;
                let parsed = if value == "none" {
                    Vec::new()
                } else {
                    value
                        .split(',')
                        .map(|scope| match scope {
                            "read" => Ok(AccessScope::Read),
                            "model" => Ok(AccessScope::Model),
                            "calculation" => Ok(AccessScope::Calculation),
                            "documents" => Ok(AccessScope::Documents),
                            _ => Err(format!("Escopo desconhecido: {scope}.")),
                        })
                        .collect::<Result<Vec<_>, _>>()?
                };
                scopes = Some(parsed);
            }
            Some("--disciplines") if disciplines.is_none() => {
                let value = text_value(&mut args, "--disciplines")?;
                let parsed = value
                    .split(',')
                    .map(|discipline| match discipline {
                        "structure" => Ok(Discipline::Structure),
                        "water_sanitation" => Ok(Discipline::WaterSanitation),
                        "gas" => Ok(Discipline::Gas),
                        "fire_protection" => Ok(Discipline::FireProtection),
                        "electrical" => Ok(Discipline::Electrical),
                        "photovoltaic" => Ok(Discipline::Photovoltaic),
                        "lightning_protection" => Ok(Discipline::LightningProtection),
                        "cabling" => Ok(Discipline::Cabling),
                        "hvac" => Ok(Discipline::Hvac),
                        _ => Err(format!("Disciplina desconhecida: {discipline}.")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                disciplines = Some(parsed);
            }
            Some("--project-id") if project_id.is_none() => {
                project_id = Some(
                    Uuid::parse_str(&text_value(&mut args, "--project-id")?)
                        .map_err(|_| "Identidade de projeto inválida.")?,
                );
            }
            Some("--actor") if actor.is_none() => actor = Some(text_value(&mut args, "--actor")?),
            Some("--protocol") if protocol.is_none() => {
                protocol = Some(text_value(&mut args, "--protocol")?)
            }
            _ => return Err("Argumento desconhecido ou repetido. Use --help.".into()),
        }
    }
    if writable && scopes.is_some() {
        return Err("Escolha --allow-write ou --scopes, sem combinar as duas opções.".into());
    }
    let project =
        project.ok_or("Selecione o projeto com --project <arquivo.prumo>. Use --help.")?;
    let mut grant = ProjectGrant::for_access(if writable {
        AccessMode::ReadWrite
    } else {
        AccessMode::ReadOnly
    });
    if let Some(scopes) = scopes {
        grant.scopes = scopes;
    }
    grant.project_id = project_id;
    grant.disciplines = disciplines;
    grant.validate().map_err(|failure| failure.to_string())?;
    let legacy = match protocol.as_deref().unwrap_or(SUPPORTED_PROTOCOL) {
        SUPPORTED_PROTOCOL => false,
        LEGACY_PROTOCOL => true,
        _ => {
            return Err("Revisão de protocolo não suportada. Use 2026-07-28 ou 2025-11-25.".into());
        }
    };
    Ok(Some(Options {
        project,
        grant,
        actor: actor.unwrap_or_else(|| "mcp-local".into()),
        legacy,
    }))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Prumo: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = options()? else {
        eprint!("{USAGE}");
        return Ok(());
    };
    let server = PrumoMcp::open_scoped(options.project, options.grant, options.actor)?;
    let transport = AsyncRwTransport::<RoleServer, _, _>::new(
        guarded_input(tokio::io::stdin()),
        tokio::io::stdout(),
    )
    .with_max_line_length(MAX_REQUEST_BYTES);
    if options.legacy {
        LegacyPrumoMcp(server)
            .serve(transport)
            .await?
            .waiting()
            .await?;
    } else {
        server.serve(transport).await?.waiting().await?;
    }
    Ok(())
}
