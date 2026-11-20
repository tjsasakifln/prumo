#![forbid(unsafe_code)]
//! Browser inspection target. The same project and view commands are used natively.
//! HTML contains presentation only; all input, validation and controllers are Rust.
#[cfg(target_arch = "wasm32")]
mod browser {
    use prumo_domain::{
        Project,
        view::{ViewCommand, ViewReport},
    };
    use prumo_ui::{UiAction, ViewportView};
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{JsCast, prelude::*};

    type EventListener = Closure<dyn FnMut(web_sys::Event)>;

    struct WebState {
        project: Option<Project>,
        viewport: ViewportView,
        notice: String,
        graphics_error: Option<String>,
        rendered_revision: Option<(String, u64)>,
    }

    impl Default for WebState {
        fn default() -> Self {
            Self {
                project: None,
                viewport: ViewportView {
                    export_available: false,
                    ..Default::default()
                },
                notice: String::new(),
                graphics_error: None,
                rendered_revision: None,
            }
        }
    }

    thread_local! {
        static STATE: Rc<RefCell<WebState>> = Rc::new(RefCell::new(WebState::default()));
        static RUNNER: eframe::WebRunner = eframe::WebRunner::new();
        static LISTENERS: RefCell<Vec<EventListener>> = RefCell::new(Vec::new());
    }

    fn document() -> Result<web_sys::Document, JsValue> {
        web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| JsValue::from_str("Documento do navegador indisponível."))
    }
    fn element(id: &str) -> Result<web_sys::Element, JsValue> {
        document()?
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("Controle ausente: {id}")))
    }
    fn report_error(message: String) {
        STATE.with(|state| state.borrow_mut().notice = message);
        refresh_dom();
    }

    #[wasm_bindgen]
    pub fn import_project(json: &str) -> Result<(), JsValue> {
        if json.len() > 32 * 1024 * 1024 {
            return Err(JsValue::from_str(
                "Projeto acima do limite de 32 MiB desta vista.",
            ));
        }
        prumo_domain::canonical::validate_json_representation(json.as_bytes())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let project: Project = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Projeto inválido: {e}")))?;
        project
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.viewport.state.synchronize(&project);
            state.viewport.state.graphics_failure = state.graphics_error.clone();
            state.project = Some(project);
            state.notice = "Projeto aberto. Consulta de modelagem, sem cálculo.".into();
            state.rendered_revision = None;
        });
        refresh_dom();
        Ok(())
    }

    #[wasm_bindgen]
    pub fn view_report() -> Result<String, JsValue> {
        STATE.with(|state| {
            let state = state.borrow();
            let project = state
                .project
                .as_ref()
                .ok_or_else(|| JsValue::from_str("Abra um projeto."))?;
            let report = state
                .viewport
                .state
                .report(project)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            serde_json::to_string(&report).map_err(|e| JsValue::from_str(&e.to_string()))
        })
    }

    #[wasm_bindgen]
    pub fn view_command(json: &str) -> Result<String, JsValue> {
        if json.len() > 64 * 1024 {
            return Err(JsValue::from_str("Comando de vista acima de 64 KiB."));
        }
        prumo_domain::canonical::validate_json_representation(json.as_bytes())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let command: ViewCommand =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let report = apply(command)?;
        refresh_dom();
        serde_json::to_string(&report).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    fn apply(command: ViewCommand) -> Result<ViewReport, JsValue> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let WebState {
                project, viewport, ..
            } = &mut *state;
            viewport
                .state
                .apply(
                    project
                        .as_ref()
                        .ok_or_else(|| JsValue::from_str("Abra um projeto."))?,
                    command,
                )
                .map_err(|e| JsValue::from_str(&e.to_string()))
        })
    }

    fn refresh_dom() {
        let result: Result<(), JsValue> = STATE.with(|state| {
            let mut state = state.borrow_mut();
            element("project-status")?.set_text_content(Some(&state.notice));
            let Some(project) = state.project.as_ref() else {
                return Ok(());
            };
            let report = state
                .viewport
                .state
                .report(project)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            if let Some(body) = document()?.body() {
                body.set_attribute(
                    "data-high-contrast",
                    if report.state.high_contrast {
                        "true"
                    } else {
                        "false"
                    },
                )?;
                body.style()
                    .set_property("font-size", &format!("{}%", report.state.scale * 100.0))?;
            }
            let key = (project.id.to_string(), project.revision);
            let list = element("object-list")?.dyn_into::<web_sys::HtmlSelectElement>()?;
            if state.rendered_revision.as_ref() != Some(&key) {
                list.set_text_content(None);
                for object in &report.objects {
                    let option = document()?.create_element("option")?;
                    option.set_attribute("value", &object.id.to_string())?;
                    option.set_text_content(Some(&format!(
                        "{} | {}",
                        object.name,
                        ViewportView::discipline_label(object.discipline)
                    )));
                    list.append_child(&option)?;
                }
                state.rendered_revision = Some(key);
            }
            if let Some(selected) = report.objects.iter().find(|o| o.selected) {
                list.set_value(&selected.id.to_string());
                element("object-details")?.set_text_content(Some(&format!(
                    "Selecionado: {}\nIdentidade: {}\n{}\n{}\nInício: {:?} m\nFim: {:?} m\nRelações analíticas: {}",
                    selected.name,
                    selected.id,
                    selected.visibility_reason,
                    selected.status,
                    selected.start_m.as_array(),
                    selected.end_m.as_array(),
                    selected.analytical_ids.len()
                )));
            } else {
                element("object-details")?.set_text_content(Some("Nenhuma seleção."));
            }
            element("graphics-status")?.set_text_content(
                state
                    .graphics_error
                    .as_deref()
                    .or(report.state.graphics_failure.as_deref()),
            );
            Ok(())
        });
        if let Err(error) = result {
            web_sys::console::error_1(&error);
        }
    }

    fn listen(
        id: &str,
        event: &str,
        callback: impl FnMut(web_sys::Event) + 'static,
    ) -> Result<(), JsValue> {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(web_sys::Event)>);
        element(id)?.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
        LISTENERS.with(|listeners| listeners.borrow_mut().push(closure));
        Ok(())
    }

    struct Viewer;
    impl eframe::App for Viewer {
        fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
            let actions = STATE.with(|state| {
                let mut state = state.borrow_mut();
                let WebState {
                    project, viewport, ..
                } = &mut *state;
                ui.ctx().set_zoom_factor(viewport.state.scale as f32);
                ui.ctx().set_visuals(if viewport.state.high_contrast {
                    let mut visuals = eframe::egui::Visuals::dark();
                    visuals.override_text_color = Some(eframe::egui::Color32::WHITE);
                    visuals.panel_fill = eframe::egui::Color32::BLACK;
                    visuals
                } else {
                    eframe::egui::Visuals::dark()
                });
                let mut actions = Vec::new();
                eframe::egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(project) = project {
                        viewport.show(ui, project, &mut actions);
                    } else {
                        ui.heading("Abra seu projeto no painel ao lado");
                        ui.label("Copie o JSON da aplicação local. A identidade, revisão e geometria são conferidas antes de mostrar o modelo.");
                    }
                });
                actions
            });
            for action in actions {
                if let UiAction::View(command) = action
                    && let Err(error) = apply(command)
                {
                    report_error(
                        error
                            .as_string()
                            .unwrap_or_else(|| "Comando de vista recusado.".into()),
                    );
                }
            }
            refresh_dom();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    async fn restart() -> Result<(), JsValue> {
        let canvas = element("prumo-canvas")?.dyn_into::<web_sys::HtmlCanvasElement>()?;
        let runner = RUNNER.with(Clone::clone);
        runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_| Ok(Box::new(Viewer))),
            )
            .await?;
        STATE.with(|state| state.borrow_mut().graphics_error = None);
        if STATE.with(|state| state.borrow().project.is_some()) {
            apply(ViewCommand::RestoreGraphics {})?;
        }
        refresh_dom();
        Ok(())
    }

    #[wasm_bindgen(start)]
    pub async fn start() -> Result<(), JsValue> {
        listen("project-file", "change", |_| {
            let input = element("project-file").and_then(|e| {
                e.dyn_into::<web_sys::HtmlInputElement>()
                    .map_err(Into::into)
            });
            let file = input
                .ok()
                .and_then(|input| input.files())
                .and_then(|files| files.get(0));
            if let Some(file) = file {
                wasm_bindgen_futures::spawn_local(async move {
                    if file.size() > 32.0 * 1024.0 * 1024.0 {
                        report_error("Projeto acima do limite de 32 MiB desta vista.".into());
                        return;
                    }
                    let result = wasm_bindgen_futures::JsFuture::from(file.text())
                        .await
                        .and_then(|value| import_project(&value.as_string().unwrap_or_default()));
                    if let Err(error) = result {
                        report_error(error.as_string().unwrap_or_else(|| {
                            "Não foi possível ler o arquivo selecionado.".into()
                        }));
                    }
                });
            }
        })?;
        listen("load-project", "click", |_| {
            let result = element("project-json")
                .and_then(|e| {
                    e.dyn_into::<web_sys::HtmlTextAreaElement>()
                        .map_err(Into::into)
                })
                .and_then(|input| import_project(&input.value()));
            if let Err(error) = result {
                report_error(
                    error
                        .as_string()
                        .unwrap_or_else(|| "Não foi possível abrir o projeto.".into()),
                );
            }
        })?;
        listen("select-object", "click", |_| {
            let result = (|| -> Result<(), JsValue> {
                let list = element("object-list")?.dyn_into::<web_sys::HtmlSelectElement>()?;
                let object_id = list
                    .value()
                    .parse()
                    .map_err(|_| JsValue::from_str("Selecione uma peça da lista."))?;
                apply(ViewCommand::Select {
                    object_id,
                    reveal: true,
                })?;
                apply(ViewCommand::LocateSelected {})?;
                Ok(())
            })();
            if let Err(error) = result {
                report_error(
                    error
                        .as_string()
                        .unwrap_or_else(|| "Seleção recusada.".into()),
                );
            } else {
                refresh_dom();
            }
        })?;
        for (id, command) in [
            ("previous-object", ViewCommand::SelectNext { reverse: true }),
            ("next-object", ViewCommand::SelectNext { reverse: false }),
            ("hide-object", ViewCommand::HideSelected {}),
            ("isolate-object", ViewCommand::IsolateSelected {}),
            ("show-all", ViewCommand::ShowAll {}),
            ("fit", ViewCommand::Fit {}),
        ] {
            listen(id, "click", move |_| {
                if let Err(error) = apply(command.clone()) {
                    report_error(
                        error
                            .as_string()
                            .unwrap_or_else(|| "Comando recusado.".into()),
                    );
                } else {
                    refresh_dom();
                }
            })?;
        }
        listen("prumo-canvas", "webglcontextlost", |event| {
            event.prevent_default();
            STATE.with(|state| {
                state.borrow_mut().graphics_error = Some(
                    "Contexto gráfico interrompido. Projeto e navegação textual preservados."
                        .into(),
                )
            });
            let _ = apply(ViewCommand::GraphicsUnavailable {
                reason: "Contexto gráfico interrompido. Projeto e navegação textual preservados."
                    .into(),
            });
            refresh_dom();
        })?;
        for (id, event) in [
            ("prumo-canvas", "webglcontextrestored"),
            ("restore-graphics", "click"),
        ] {
            listen(id, event, |_| {
                wasm_bindgen_futures::spawn_local(async {
                    if let Err(error) = restart().await {
                        report_error(format!("Restauração gráfica recusada: {error:?}"));
                    }
                })
            })?;
        }
        match restart().await {
            Ok(()) => {
                document()?
                    .body()
                    .ok_or_else(|| JsValue::from_str("Documento sem corpo."))?
                    .set_attribute("data-prumo-ready", "true")?;
                Ok(())
            }
            Err(error) => {
                STATE.with(|state| state.borrow_mut().graphics_error = Some("Não foi possível iniciar a vista gráfica. A navegação textual permanece disponível.".into()));
                element("graphics-status")?.set_text_content(Some("Não foi possível iniciar a vista gráfica. A navegação textual permanece disponível."));
                report_error(format!("Inicialização gráfica recusada: {error:?}"));
                Err(error)
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::{import_project, start, view_command, view_report};
