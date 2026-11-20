//! Reproducible native render of a modelling fixture, never an engineering result.
use prumo_desktop::Desktop;
use prumo_domain::{Command, LengthUnit, view::ViewCommand};
use prumo_ui::UiAction;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let mut desktop = Desktop::new(directory.path().join("fixture.prumo"));
    desktop.dispatch(UiAction::command(Command::Create {
        name: "Navegação de referência, sem cálculo".into(),
        units: LengthUnit::Metre,
    }));
    for (name, start, end) in [
        ("Viga térrea", [0.0, 0.0, 0.0], [4.0, 0.0, 0.0]),
        ("Pilar inclinado", [0.0, 0.0, 0.0], [0.6, 0.0, 3.0]),
        ("Viga superior", [0.6, 0.0, 3.0], [4.6, 0.0, 3.0]),
        ("Travessa", [4.6, 0.0, 3.0], [4.6, 3.0, 3.0]),
    ] {
        desktop.dispatch(UiAction::command(Command::AddMember {
            expected_revision: desktop.project.as_ref().unwrap().revision,
            name: name.into(),
            start,
            end,
        }));
    }
    if desktop.view.is_error {
        return Err(desktop.view.notice.into());
    }
    if let Some(json_path) = std::env::args().nth(1) {
        let project = desktop.project.as_ref().ok_or("Projeto não criado.")?;
        std::fs::write(&json_path, serde_json::to_string_pretty(project)?)?;
        std::fs::write(
            format!("{json_path}.snapshot.json"),
            serde_json::to_string_pretty(&prumo_domain::canonical::model_snapshot(project)?)?,
        )?;
    }
    if std::env::var_os("PRUMO_VIEW_EXPORT_ONLY").is_some() {
        return Ok(());
    }
    let object_id = desktop.project.as_ref().unwrap().objects[2].id;
    desktop.dispatch(UiAction::View(ViewCommand::Select {
        object_id,
        reveal: true,
    }));
    desktop.dispatch(UiAction::View(ViewCommand::Fit {}));
    if std::env::var_os("PRUMO_VIEW_CONTRAST").is_some() {
        desktop.dispatch(UiAction::View(ViewCommand::SetPreferences {
            scale: 1.5,
            high_contrast: true,
        }));
    }
    desktop.view.viewport.three_views = true;
    if let Some(destination) = std::env::var_os("PRUMO_VIEW_MESH_EXPORT") {
        export_egui_meshes(&mut desktop, std::path::Path::new(&destination))?;
        return Ok(());
    }
    let screenshot = std::env::var_os("PRUMO_VIEW_SCREENSHOT").map(std::path::PathBuf::from);
    let capture_requested = screenshot.is_some();
    if let Some(path) = &screenshot
        && let Some(parent) = path.parent()
    {
        std::fs::create_dir_all(parent)?;
    }
    let outcome = Arc::new(Mutex::new(None));
    let outcome_for_app = outcome.clone();
    let scale = desktop.view.viewport.state.scale as f32;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1600.0 * scale, 1800.0 * scale]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Prumo | Ensaio de navegação sem cálculo",
        options,
        Box::new(move |_| {
            Ok(Box::new(NativeGallery {
                desktop,
                screenshot,
                requested: false,
                frames: 0,
                started: std::time::Instant::now(),
                outcome: outcome_for_app,
            }))
        }),
    )?;
    if capture_requested {
        outcome
            .lock()
            .map_err(|_| "Estado da captura indisponível")?
            .take()
            .ok_or("Janela encerrada sem receber a captura do backend")??;
    }
    Ok(())
}

/// Requests a screenshot through the distributed Glow backend. A completed CPU
/// layout pass alone never marks this evidence successful.
struct NativeGallery {
    desktop: Desktop,
    screenshot: Option<std::path::PathBuf>,
    requested: bool,
    frames: usize,
    started: std::time::Instant,
    outcome: Arc<Mutex<Option<Result<(), String>>>>,
}

impl eframe::App for NativeGallery {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.desktop.render(ui);
        self.frames += 1;
        let Some(path) = self.screenshot.as_ref() else {
            return;
        };
        if self.started.elapsed() > std::time::Duration::from_secs(20) {
            *self.outcome.lock().expect("Capture state") =
                Some(Err("Backend não entregou uma captura em 20 segundos".into()));
            ui.ctx()
                .send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            return;
        }
        if !self.requested && self.frames >= 5 {
            self.requested = true;
            let path = path.clone();
            let outcome = self.outcome.clone();
            let ctx = ui.ctx().clone();
            let report = self
                .desktop
                .view
                .viewport
                .state
                .report(self.desktop.project.as_ref().expect("Gallery project"));
            let pass = ui.ctx().cumulative_pass_nr();
            ui.ctx().request_screenshot(move |image| {
                let result = (|| -> Result<(), String> {
                    let report = report.map_err(|error| error.to_string())?;
                    if image.width() < 640 || image.height() < 480 {
                        return Err("Captura menor que a janela de ensaio".into());
                    }
                    let first = image.pixels.first().ok_or("Captura vazia")?;
                    if image.pixels.iter().filter(|pixel| *pixel != first).count() < 1000 {
                        return Err(
                            "Captura sem variação suficiente para mostrar a interface".into()
                        );
                    }
                    let bytes: Vec<u8> = image
                        .pixels
                        .iter()
                        .flat_map(|pixel| pixel.to_srgba_unmultiplied())
                        .collect();
                    image::save_buffer_with_format(
                        &path,
                        &bytes,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                        image::ImageFormat::Png,
                    )
                    .map_err(|error| error.to_string())?;
                    let png = std::fs::read(&path).map_err(|error| error.to_string())?;
                    let executable = std::env::current_exe()
                        .and_then(std::fs::read)
                        .map_err(|error| error.to_string())?;
                    let record = serde_json::json!({
                        "evidence":"eframe screenshot callback after real native backend painting",
                        "native_backend_executed":true,"web_browser_executed":false,
                        "renderer":"eframe 0.37.0 glow",
                        "human_screen_reader_tested":false,"native_context_loss_tested":false,
                        "platform":std::env::consts::OS,
                        "width":image.width(),"height":image.height(),
                        "requested_after_egui_pass":pass,
                        "image_sha256":prumo_domain::canonical::hash_bytes(&png),
                        "executable_sha256":prumo_domain::canonical::hash_bytes(&executable),
                        "source_commit":std::env::var("GITHUB_SHA").ok(),
                        "view_report":report,
                    });
                    let json =
                        serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?;
                    std::fs::write(path.with_extension("json"), json)
                        .map_err(|error| error.to_string())?;
                    Ok(())
                })();
                *outcome.lock().expect("Capture state") = Some(result);
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            });
        }
        ui.ctx().request_repaint();
    }
}

/// Administrative evidence: exports the actual egui meshes and font atlas, not a
/// reconstruction of the screen. It does not start a native window or GPU driver.
fn export_egui_meshes(
    desktop: &mut Desktop,
    destination: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use eframe::egui::{self, epaint};
    use std::collections::BTreeMap;
    std::fs::create_dir_all(destination)?;
    let scale = desktop.view.viewport.state.scale as f32;
    let size = egui::vec2(1600.0 * scale, 1800.0 * scale);
    let ctx = egui::Context::default();
    let mut textures = BTreeMap::<String, egui::ColorImage>::new();
    let mut last = None;
    for frame in 0..5 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                // RawInput is in logical points; the exported image is pixels.
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    size / ctx.zoom_factor(),
                )),
                time: Some(frame as f64 / 60.0),
                ..Default::default()
            },
            |ui| desktop.render(ui),
        );
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                let key = format!("{id:?}");
                let epaint::ImageData::Color(image) = &delta.image;
                if let Some([x, y]) = delta.pos {
                    let target = textures.get_mut(&key).ok_or("Textura parcial sem base")?;
                    for row in 0..image.size[1] {
                        let offset = (y + row) * target.size[0] + x;
                        target.pixels[offset..offset + image.size[0]].copy_from_slice(
                            &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                        );
                    }
                } else {
                    textures.insert(key, (**image).clone());
                }
            }
        }
        for id in &output.textures_delta.free {
            textures.remove(&format!("{id:?}"));
        }
        output.textures_delta.clear();
        last = Some(output);
    }
    let output = last.ok_or("Nenhum quadro produzido")?;
    let mut texture_records = Vec::new();
    for (index, (id, image)) in textures.iter().enumerate() {
        let file = format!("texture-{index}.rgba");
        let bytes: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect();
        std::fs::write(destination.join(&file), bytes)?;
        texture_records.push(serde_json::json!({"id":id,"file":file,"size":image.size}));
    }
    let mut meshes = Vec::new();
    for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
        let epaint::Primitive::Mesh(mesh) = primitive.primitive else {
            return Err("Callback gráfico não suportado por este exportador de ensaio".into());
        };
        let vertices: Vec<_> = mesh
            .vertices
            .iter()
            .map(|v| serde_json::json!([v.pos.x, v.pos.y, v.uv.x, v.uv.y, v.color.to_array()]))
            .collect();
        meshes.push(serde_json::json!({
            "texture":format!("{:?}",mesh.texture_id),
            "clip":[primitive.clip_rect.min.x,primitive.clip_rect.min.y,primitive.clip_rect.max.x,primitive.clip_rect.max.y],
            "vertices":vertices,"indices":mesh.indices,
        }));
    }
    let record = serde_json::json!({
        "evidence":"actual egui meshes and font atlas, CPU headless export",
        "native_backend_executed":false,"web_browser_executed":false,
        "width":size.x as u32,"height":size.y as u32,
        "pixels_per_point":output.pixels_per_point,
        "scale":desktop.view.viewport.state.scale,
        "high_contrast":desktop.view.viewport.state.high_contrast,
        "view_report":desktop.view.viewport.state.report(desktop.project.as_ref().ok_or("Projeto ausente")?)?,
        "textures":texture_records,"meshes":meshes,
    });
    std::fs::write(
        destination.join("egui-meshes.json"),
        serde_json::to_vec(&record)?,
    )?;
    Ok(())
}
