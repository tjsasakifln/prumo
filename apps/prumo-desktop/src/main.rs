#![forbid(unsafe_code)]
use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut path = PathBuf::from("meu-projeto.prumo");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                let Some(value) = args.next() else {
                    eprintln!("Informe o arquivo após --project.");
                    std::process::exit(2);
                };
                path = PathBuf::from(value);
            }
            "--help" | "-h" => {
                println!(
                    "Prumo\nUso: prumo-desktop [--project arquivo.prumo]\nCria, salva e reabre projetos locais. Cálculos ainda indisponíveis."
                );
                return Ok(());
            }
            _ => {
                eprintln!("Argumento desconhecido: {arg}");
                std::process::exit(2);
            }
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([760.0, 600.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Prumo | Projeto BIM",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(prumo_desktop::Desktop::new(path)))
        }),
    )
}
