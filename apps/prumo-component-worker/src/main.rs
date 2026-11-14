#![forbid(unsafe_code)]
use prumo_components::process::{MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, WorkerRequest};
use std::io::{Read, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("Qualificação: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "Prumo: ensaios delimitados de componentes\nUso: prumo-component-worker --qualification-report\nSem argumentos: uma requisição JSON em stdin e uma resposta em stdout.\nOs ensaios não calculam nem aprovam projetos de engenharia."
        );
        return Ok(());
    }
    let request = if args == ["--qualification-report"] {
        WorkerRequest::new(None)
    } else if args.is_empty() {
        let mut input = Vec::new();
        std::io::stdin()
            .lock()
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_to_end(&mut input)?;
        if input.len() > MAX_INPUT_BYTES {
            return Err("Entrada excede o limite.".into());
        }
        serde_json::from_slice::<WorkerRequest>(&input)?
    } else {
        return Err("Argumento desconhecido.".into());
    };
    request.validate()?;
    let report = prumo_components::run_suite(request)?;
    let output = serde_json::to_vec(&report)?;
    if output.len() > MAX_OUTPUT_BYTES {
        return Err("Resposta excede o limite.".into());
    }
    std::io::stdout().lock().write_all(&output)?;
    Ok(())
}
