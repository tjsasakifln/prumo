//! User-visible controls author real surfaces and assemblies, then reopen storage.
use eframe::egui::{Key, Modifiers, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use prumo_desktop::Desktop;
use prumo_domain::{assemblies, surfaces};
use std::path::Path;

fn harness(path: &Path) -> Harness<'static, Desktop> {
    Harness::builder()
        .with_size(vec2(1500., 2800.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path.to_path_buf()),
        )
}

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
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
        Key::A,
    );
    ui.run();
    ui.get_by_label(label).type_text(value);
    ui.run();
}

fn choose(ui: &mut Harness<'_, Desktop>, label: &str, option: &str) {
    click(ui, label);
    ui.get_by_role_and_label(eframe::egui::accesskit::Role::Button, option)
        .scroll_to_me();
    ui.run();
    ui.get_by_role_and_label(eframe::egui::accesskit::Role::Button, option)
        .click();
    ui.run();
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
}

fn material(ui: &mut Harness<'_, Desktop>) {
    click(ui, "Criar projeto");
    click(ui, "Modelo físico e analítico");
    click(ui, "Materiais");
    text(ui, "Nome do material", "Concreto da tarefa");
    choose(ui, "Unidade do módulo", "GPa");
    text(ui, "Módulo de elasticidade", "25");
    text(ui, "Massa específica (kg/m³)", "2500");
    click(ui, "Cadastrar material");
    click(ui, "Modelo físico e analítico");
}

fn disabled(ui: &Harness<'_, Desktop>, label: &str) {
    assert!(
        ui.get_by_label(label).accesskit_node().is_disabled(),
        "{label} deveria estar bloqueado"
    );
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn slab_opening_widgets_change_material_area_and_reopen_with_stable_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("surface-widgets.prumo");
    let mut ui = harness(&path);
    material(&mut ui);
    click(&mut ui, "Lajes, painéis e aberturas");
    text(&mut ui, "Nome da superfície", "Laje com abertura");
    choose(&mut ui, "Material da peça", "Concreto da tarefa");
    text(&mut ui, "Espessura da superfície (m)", "0,20");
    click(&mut ui, "Aberturas");
    click(&mut ui, "Adicionar abertura retangular");
    text(&mut ui, "Nome da abertura", "Passagem de instalações");
    let before_preview = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Conferir prévia da superfície");
    assert_eq!(ui.state().project.as_ref().unwrap(), &before_preview);
    click(&mut ui, "Aplicar superfície conferida");
    let saved = ui.state().project.as_ref().unwrap();
    let object = saved
        .objects
        .iter()
        .find(|o| o.name == "Laje com abertura")
        .unwrap();
    let first = surfaces::inspect(saved, object.id).unwrap();
    near(first.preview.net_area_m2, 23.);
    near(first.preview.quantities.total_volume_m3, 4.6);
    near(first.preview.quantities.total_mass_kg.unwrap(), 11500.);
    assert_eq!(saved.objects.len(), 1);
    assert!(first.region_id.is_some());

    choose(&mut ui, "Superfície para editar", "Laje com abertura");
    text(&mut ui, "Vértice 2: X (m)", "2.5");
    text(&mut ui, "Vértice 3: X (m)", "2.5");
    click(&mut ui, "Conferir prévia da superfície");
    click(&mut ui, "Aplicar superfície conferida");
    let updated = surfaces::inspect(ui.state().project.as_ref().unwrap(), first.object_id).unwrap();
    near(updated.preview.net_area_m2, 22.);
    near(updated.preview.quantities.total_mass_kg.unwrap(), 11000.);
    assert_eq!(updated.region_id, first.region_id);
    let snapshot = ui.state().project.as_ref().unwrap().clone();
    drop(ui);
    let reopened = Desktop::new(path);
    assert_eq!(reopened.project.as_ref().unwrap(), &snapshot);
    assert_eq!(
        surfaces::inspect(reopened.project.as_ref().unwrap(), first.object_id).unwrap(),
        updated
    );
}

#[test]
fn stair_and_landing_widgets_preserve_parts_and_explicit_continuity_after_edit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("assembly-widgets.prumo");
    let mut ui = harness(&path);
    material(&mut ui);
    click(&mut ui, "Paredes, núcleos, escadas e rampas");
    text(&mut ui, "Nome do conjunto", "Escada entre pavimentos");
    choose(&mut ui, "Material da peça", "Concreto da tarefa");
    click(&mut ui, "Apoios e continuidade declarada");
    click(&mut ui, "Identificar bordas coincidentes para conferir");
    click(&mut ui, "Conferir prévia do conjunto");
    click(&mut ui, "Aplicar conjunto conferido");
    let saved = ui.state().project.as_ref().unwrap();
    let id = saved
        .objects
        .iter()
        .find(|o| o.name == "Escada entre pavimentos")
        .unwrap()
        .id;
    let first = assemblies::inspect(saved, id).unwrap();
    assert_eq!(first.parts.len(), 2);
    assert_eq!(first.continuities.len(), 1);
    near(first.regions[1].preview.net_area_m2, 1.44);

    choose(&mut ui, "Conjunto para editar", "Escada entre pavimentos");
    click(&mut ui, "Patamares");
    text(&mut ui, "Vértice 2: X (m)", "2,4");
    text(&mut ui, "Vértice 3: X (m)", "2,4");
    click(&mut ui, "Conferir prévia do conjunto");
    click(&mut ui, "Aplicar conjunto conferido");
    let updated = assemblies::inspect(ui.state().project.as_ref().unwrap(), id).unwrap();
    near(updated.regions[1].preview.net_area_m2, 2.88);
    assert_eq!(updated.parts, first.parts);
    assert_eq!(updated.continuities, first.continuities);
    assert_eq!(updated.regions[1].region_id, first.regions[1].region_id);
    assert_eq!(ui.state().project.as_ref().unwrap().objects.len(), 3);
    assert!(!updated.preview.engineering_release);
    let snapshot = ui.state().project.as_ref().unwrap().clone();
    drop(ui);
    let reopened = Desktop::new(path);
    assert_eq!(reopened.project.as_ref().unwrap(), &snapshot);
    assert_eq!(
        assemblies::inspect(reopened.project.as_ref().unwrap(), id).unwrap(),
        updated
    );
}

#[test]
fn invalid_coordinates_and_collapsed_landing_thickness_cannot_apply_old_previews() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-parametric-widgets.prumo");
    let mut ui = harness(&path);
    material(&mut ui);
    click(&mut ui, "Lajes, painéis e aberturas");
    choose(&mut ui, "Material da peça", "Concreto da tarefa");
    click(&mut ui, "Conferir prévia da superfície");
    let before = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Posição, inclinação e eixos locais");
    text(&mut ui, "Origem: Z (m)", "-");
    click(&mut ui, "Posição, inclinação e eixos locais");
    disabled(&ui, "Conferir prévia da superfície");
    disabled(&ui, "Aplicar superfície conferida");
    click(&mut ui, "Conferir prévia da superfície");
    click(&mut ui, "Aplicar superfície conferida");
    assert_eq!(ui.state().project.as_ref().unwrap(), &before);
    click(&mut ui, "Posição, inclinação e eixos locais");
    text(&mut ui, "Origem: Z (m)", "0");
    click(&mut ui, "Posição, inclinação e eixos locais");
    click(&mut ui, "Conferir prévia da superfície");
    click(&mut ui, "Aplicar superfície conferida");
    assert_eq!(ui.state().project.as_ref().unwrap().objects.len(), 1);
    click(&mut ui, "Lajes, painéis e aberturas");

    click(&mut ui, "Paredes, núcleos, escadas e rampas");
    choose(&mut ui, "Material da peça", "Concreto da tarefa");
    click(&mut ui, "Conferir prévia do conjunto");
    let before = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Patamares");
    text(&mut ui, "Espessura do patamar (m)", "-");
    click(&mut ui, "Patamares");
    disabled(&ui, "Conferir prévia do conjunto");
    disabled(&ui, "Aplicar conjunto conferido");
    click(&mut ui, "Conferir prévia do conjunto");
    click(&mut ui, "Aplicar conjunto conferido");
    assert_eq!(ui.state().project.as_ref().unwrap(), &before);
    // An explicit reload resets only this form, so the user can recover from
    // invalid hidden input without reopening the whole application.
    choose(&mut ui, "Conjunto para editar", "Novo conjunto");
    choose(&mut ui, "Material da peça", "Concreto da tarefa");
    click(&mut ui, "Conferir prévia do conjunto");
    click(&mut ui, "Aplicar conjunto conferido");
    assert_eq!(ui.state().project.as_ref().unwrap().objects.len(), 4);
}
