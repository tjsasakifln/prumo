#![forbid(unsafe_code)]

use std::path::PathBuf;

use prumo_app::AccessMode;
use prumo_mcp::{MAX_REQUEST_BYTES, PrumoMcp};
use rmcp::{RoleServer, ServiceExt, transport::async_rw::AsyncRwTransport};

const USAGE: &str = "Prumo MCP local\nUso: prumo-mcp --project <arquivo.prumo> [--allow-write]\nO padrão permite somente leitura. --allow-write autoriza criar e editar somente o projeto selecionado.\nProtocolo: MCP 2026-07-28, descoberta server/discover e stdio.\n";

fn options() -> Result<Option<(PathBuf, AccessMode)>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut project = None;
    let mut writable = false;
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
            _ => return Err("Argumento desconhecido ou repetido. Use --help.".into()),
        }
    }
    let path = project.ok_or("Selecione o projeto com --project <arquivo.prumo>. Use --help.")?;
    let access = if writable {
        AccessMode::ReadWrite
    } else {
        AccessMode::ReadOnly
    };
    Ok(Some((path, access)))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Prumo: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some((path, access)) = options()? else {
        eprint!("{USAGE}");
        return Ok(());
    };
    let server = PrumoMcp::open(path, access, "mcp-local")?;
    let transport =
        AsyncRwTransport::<RoleServer, _, _>::new(tokio::io::stdin(), tokio::io::stdout())
            .with_max_line_length(MAX_REQUEST_BYTES);
    server.serve(transport).await?.waiting().await?;
    Ok(())
}
