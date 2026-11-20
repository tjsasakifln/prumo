//! Actual widgets exercise the same queries and edits as external clients.
use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;

fn click(ui: &mut Harness<'_, Desktop>, label: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
}

fn text(ui: &mut Harness<'_, Desktop>, label: &str, value: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
    ui.key_press_modifiers(
        eframe::egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
        eframe::egui::Key::A,
    );
    ui.run();
    ui.get_by_label(label).type_text(value);
    ui.run();
}

#[test]
fn reference_verification_and_geometry_queries_never_edit_the_open_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("geometry-ui.prumo");
    let mut ui = Harness::builder()
        .with_size(eframe::egui::vec2(1500., 2600.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path),
        );
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Adicionar peça e salvar");
    let before = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Geometria e conexões");
    click(&mut ui, "Inspecionar geometria");
    assert_eq!(
        ui.state()
            .view
            .geometry
            .report
            .as_ref()
            .unwrap()
            .object_count,
        1
    );
    click(&mut ui, "Conferir face por coordenadas");
    click(&mut ui, "Conferir orientação e forma");
    assert!(
        ui.state()
            .view
            .geometry
            .face_report
            .as_ref()
            .unwrap()
            .inspection
            .valid
    );
    click(&mut ui, "Verificação com casos de referência");
    click(&mut ui, "Executar casos de referência");
    let report = ui.state().view.geometry.verification.as_ref().unwrap();
    assert_eq!(report["evidence"]["verification_passed"], true);
    assert_eq!(report["evidence"]["engineering_release"], false);
    assert_eq!(ui.state().project.as_ref().unwrap(), &before);
    click(&mut ui, "Salvar tolerâncias");
    assert!(ui.state().view.geometry.verification.is_none());
    click(&mut ui, "Conferir orientação e forma");
    assert!(ui.state().view.geometry.face_report.is_some());
    text(
        &mut ui,
        "Coordenadas da face (JSON)",
        r#"{"vertices_m":[],"expected_normal":[0,0,1]}"#,
    );
    assert!(ui.state().view.geometry.face_report.is_none());
    ui.get_by_label("Conferir orientação e forma").click();
    ui.run();
    assert!(ui.state().view.is_error);
    assert!(ui.state().view.geometry.face_report.is_none());
}

#[test]
fn section_widgets_create_analyse_assign_and_reopen_catalogue_with_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalogue-ui.prumo");
    let mut ui = Harness::builder()
        .with_size(eframe::egui::vec2(1500., 2600.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path.clone()),
        );
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Adicionar peça e salvar");
    click(&mut ui, "Catálogos de materiais e seções");
    click(&mut ui, "Criar seção por dimensões");
    click(&mut ui, "Salvar nova seção");
    let saved = ui.state().project.as_ref().unwrap().clone();
    let catalogue = prumo_domain::catalogue::project_catalogue(&saved).unwrap();
    assert_eq!(catalogue.entries.len(), 1);
    let entry = catalogue
        .entries
        .values()
        .next()
        .unwrap()
        .last()
        .unwrap()
        .entry
        .clone();
    click(&mut ui, "Calcular propriedades da seção");
    let report = ui.state().view.catalogue.section_report.as_ref().unwrap();
    assert!((report["analysis"]["geometry"]["area_m2"].as_f64().unwrap() - 0.08).abs() < 1e-14);
    assert!(report["analysis"]["torsion"]["j_m4"].as_f64().unwrap() > 0.0);
    assert_eq!(
        report["entry_sha256"].as_str().unwrap(),
        entry.sha256().unwrap()
    );
    assert_eq!(ui.state().project.as_ref().unwrap(), &saved);
    click(&mut ui, "Associar entrada à peça");
    assert!(ui.state().view.catalogue.section_report.is_none());
    click(&mut ui, "Reabrir / atualizar");
    drop(ui);
    let reopened = Desktop::new(path);
    let project = reopened.project.unwrap();
    assert_eq!(
        prumo_domain::catalogue::project_catalogue(&project)
            .unwrap()
            .latest(entry.id)
            .unwrap(),
        &entry
    );
    assert_eq!(
        prumo_domain::catalogue::assignments(&project.objects[0]).unwrap()
            [&prumo_domain::catalogue::CatalogKind::Section],
        entry.id
    );
}

#[test]
fn invalid_numeric_text_discards_a_previously_valid_geometry_plan() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-plan-ui.prumo");
    let mut ui = Harness::builder()
        .with_size(eframe::egui::vec2(1500., 2600.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path),
        );
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Adicionar peça e salvar");
    let before = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Geometria e conexões");
    click(&mut ui, "Carregar posição atual");
    text(&mut ui, "X (m)", "4.0000005");
    click(&mut ui, "Conferir correção");
    assert!(ui.state().view.geometry.preview.is_some());
    text(&mut ui, "X (m)", "-");
    assert!(ui.state().view.geometry.preview.is_none());
    assert_eq!(ui.state().project.as_ref().unwrap(), &before);
}

#[test]
fn editing_material_conditions_discards_the_previous_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("material-ui.prumo");
    let mut ui = Harness::builder()
        .with_size(eframe::egui::vec2(1500., 2600.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path),
        );
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Catálogos de materiais e seções");
    click(&mut ui, "Importar ou editar entrada completa");
    click(&mut ui, "Preparar exemplo de material para preencher");
    click(&mut ui, "Salvar entrada conferida");
    click(&mut ui, "Consultar estado do material");
    assert!(ui.state().view.catalogue.material_report.is_some());
    let before = ui.state().project.as_ref().unwrap().clone();
    text(&mut ui, "Condições do material (JSON)", "{");
    assert!(ui.state().view.catalogue.material_report.is_none());
    assert_eq!(ui.state().project.as_ref().unwrap(), &before);
}
