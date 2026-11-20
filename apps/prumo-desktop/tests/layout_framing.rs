//! Actual accessible widgets create and revise the same typed model commands.
use egui_kittest::{Harness, kittest::Queryable};
use prumo_app::{AccessMode, ProjectService};
use prumo_desktop::Desktop;
use prumo_domain::{
    Command, LengthUnit,
    catalogue::{CatalogEntry, CatalogPayload, CatalogSource, SectionShape},
    framing, layout,
    model::{MaterialFamily, MaterialInput, ModelEdit},
    units::{DensityInput, DensityUnit, PressureInput, PressureUnit},
};

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
fn harness(path: std::path::PathBuf) -> Harness<'static, Desktop> {
    Harness::builder()
        .with_size(eframe::egui::vec2(1500., 4200.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path),
        )
}
fn setup(path: &std::path::Path) {
    let service = ProjectService::open(path, AccessMode::ReadWrite, "UI fixture").unwrap();
    let mut project = service
        .create("Modelagem por formulários", LengthUnit::Metre)
        .unwrap();
    project = service
        .execute(Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::AddMaterial {
                material: MaterialInput {
                    name: "Aço de ensaio".into(),
                    family: MaterialFamily::Steel,
                    elastic_modulus: PressureInput {
                        value: 200.,
                        unit: PressureUnit::Gigapascal,
                    },
                    density: DensityInput {
                        value: 7850.,
                        unit: DensityUnit::KilogramPerCubicMetre,
                    },
                    poisson_ratio: 0.3,
                },
            },
        })
        .unwrap();
    service
        .execute(Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::PutCatalogEntry {
                entry: CatalogEntry {
                    id: "00112233-4455-4677-8899-aabbccddeeff".parse().unwrap(),
                    revision: 1,
                    name: "Seção de ensaio 20x40".into(),
                    source: CatalogSource {
                        publisher: "Teste independente".into(),
                        reference: "Dimensões originais".into(),
                        edition: "1".into(),
                        revision: "1".into(),
                        data_license: "Fixture própria".into(),
                        original_record: None,
                    },
                    applicability: vec![],
                    customization: None,
                    payload: CatalogPayload::Section {
                        shape: SectionShape::Rectangle {
                            width_m: 0.2,
                            height_m: 0.4,
                        },
                    },
                },
            },
        })
        .unwrap();
}
#[test]
fn level_creation_and_move_are_usable_without_drag_and_require_preview() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("levels.prumo");
    let mut ui = harness(path);
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Eixos, níveis e encaixes");
    text(&mut ui, "Cota do nível", "3 m + 20 cm");
    click(&mut ui, "Criar nível");
    let created = ui.state().project.as_ref().unwrap().clone();
    let state = layout::state(&created).unwrap();
    assert_eq!(state.levels.len(), 1);
    assert_eq!(state.levels[0].elevation_m, 3.2);
    click(&mut ui, "Editar nível Térreo");
    text(&mut ui, "Cota do nível", "4 m");
    click(&mut ui, "Conferir movimento do nível");
    assert_eq!(created, ui.state().project.as_ref().unwrap().clone());
    assert!(ui.state().view.transactions.preview.is_some());
    click(&mut ui, "Alterações em lote e histórico");
    click(&mut ui, "Aplicar lote conferido");
    let after = layout::state(ui.state().project.as_ref().unwrap()).unwrap();
    assert_eq!(after.levels[0].id, state.levels[0].id);
    assert_eq!(after.levels[0].elevation_m, 4.);
}
#[test]
fn ambiguous_measure_stays_visible_and_does_not_create_level() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid.prumo");
    let mut ui = harness(path);
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Eixos, níveis e encaixes");
    text(&mut ui, "Cota do nível", "1.000,50 m");
    let before = ui.state().project.as_ref().unwrap().clone();
    click(&mut ui, "Criar nível");
    assert_eq!(before, ui.state().project.as_ref().unwrap().clone());
    assert!(ui.state().view.layout.message.contains("ambíguo"));
    assert_eq!(ui.state().view.layout.level_elevation, "1.000,50 m");
}
#[test]
fn member_form_creates_real_physical_and_analytical_objects() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("member.prumo");
    setup(&path);
    let mut ui = harness(path.clone());
    click(&mut ui, "Pilares, vigas, barras e treliças");
    click(&mut ui, "Material da peça");
    click(&mut ui, "Aço de ensaio");
    click(&mut ui, "Seção 1");
    click(&mut ui, "Seção de ensaio 20x40");
    click(&mut ui, "Conferir peça física e analítica");
    assert!(ui.state().view.framing.preview.is_some());
    click(&mut ui, "Criar peça conferida");
    let project = ui.state().project.as_ref().unwrap();
    assert_eq!(project.objects.len(), 1);
    assert_eq!(project.model.analytical_members.len(), 1);
    assert_eq!(project.model.links.len(), 1);
    let id = project.objects[0].id;
    assert_eq!(
        framing::inspect(project, id)
            .unwrap()
            .record
            .physical_length_m,
        4.
    );
    drop(ui);
    let reopened = Desktop::new(path);
    assert_eq!(
        framing::inspect(reopened.project.as_ref().unwrap(), id)
            .unwrap()
            .record
            .physical_length_m,
        4.
    );
}
