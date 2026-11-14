use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;

#[test]
fn real_ui_creates_edits_reopens_and_rejects_zero_length() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ui.prumo");
    let mut desktop = Desktop::new(path.clone());
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1120.0, 780.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Criar projeto").click();
        harness.run();
        harness.get_by_label("Adicionar peça e salvar").click();
        harness.run();
        harness.get_by_label("Reabrir / atualizar").click();
        harness.run();
    }
    let before = desktop.project.as_ref().unwrap().clone();
    assert_eq!(before.revision, 2);
    assert_eq!(before.objects.len(), 1);
    drop(desktop);
    let mut reopened = Desktop::new(path);
    assert_eq!(reopened.project.as_ref().unwrap().id, before.id);
    assert_eq!(
        reopened.project.as_ref().unwrap().model.links,
        before.model.links
    );
    reopened.view.end = reopened.view.start;
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1120.0, 780.0))
            .build_ui(|ui| reopened.render(ui));
        harness.get_by_label("Adicionar peça e salvar").click();
        harness.run();
    }
    assert!(reopened.view.is_error);
    assert_eq!(reopened.project.as_ref().unwrap().revision, 2);
}

#[test]
fn invalid_path_does_not_silently_write_previous_project() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = Desktop::new(directory.path().join("original.prumo"));
    desktop.dispatch(prumo_ui::UiAction::command(prumo_domain::Command::Create {
        name: "Original".into(),
        units: prumo_domain::LengthUnit::Metre,
    }));
    desktop.dispatch(prumo_ui::UiAction::Open(
        directory
            .path()
            .join("missing")
            .join("project.prumo")
            .to_string_lossy()
            .into_owned(),
    ));
    assert!(desktop.view.is_error);
}

#[test]
fn typing_another_path_blocks_edits_of_the_loaded_project() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = Desktop::new(directory.path().join("original.prumo"));
    desktop.dispatch(prumo_ui::UiAction::command(prumo_domain::Command::Create {
        name: "Original".into(),
        units: prumo_domain::LengthUnit::Metre,
    }));
    desktop.view.path = directory
        .path()
        .join("another.prumo")
        .to_string_lossy()
        .into_owned();
    desktop.dispatch(prumo_ui::UiAction::command(
        prumo_domain::Command::RenameProject {
            expected_revision: 1,
            name: "Wrong target".into(),
        },
    ));
    assert!(desktop.view.is_error);
    let stored = prumo_app::ProjectService::open(
        directory.path().join("original.prumo"),
        prumo_app::AccessMode::ReadOnly,
        "test",
    )
    .unwrap()
    .read()
    .unwrap();
    assert_eq!(stored.name, "Original");
    assert_eq!(stored.revision, 1);
}

#[test]
fn corrupted_file_is_reported_on_startup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.prumo");
    std::fs::write(&path, b"not a project").unwrap();
    let desktop = Desktop::new(path);
    assert!(desktop.view.is_error);
    assert!(desktop.project.is_none());
}

#[test]
fn real_ui_pins_registers_previews_and_explicitly_migrates_normative_package() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("normative.prumo");
    let mut desktop = Desktop::new(path.clone());
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1500.0, 2600.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Criar projeto").click();
        harness.run();
        harness.get_by_label("Normas e capacidades").click();
        harness.run();
        harness.get_by_label("Fixar pacote de referências").click();
        harness.run();
        harness
            .get_by_label("Cadastrar ou atualizar referência normativa")
            .click();
        harness.run();
        harness.get_by_label("Nova referência").click();
        harness.run();
        for (label, value) in [
            ("Identificador da referência", "ui-fixture"),
            ("Designação", "Referência de ensaio da interface"),
            ("Edição", "2026"),
            (
                "Título da norma",
                "Metadados fictícios para testar cadastro",
            ),
            ("Publicador", "Fixture do teste"),
            ("Fonte pública", "https://example.com/normative-fixture"),
            ("Domínio de aplicação", "structure"),
            (
                "Responsável pela interpretação",
                "Equipe de teste, sem aprovação",
            ),
            (
                "Emendas, separadas por ponto e vírgula",
                "Emenda de ensaio A; Emenda de ensaio B",
            ),
        ] {
            harness.get_by_label(label).click();
            harness.run();
            if label.starts_with("Emendas") {
                for part in ["Emenda de ensaio A", ";", " Emenda de ensaio B"] {
                    harness.get_by_label(label).type_text(part);
                    harness.run();
                }
            } else {
                harness.get_by_label(label).type_text(value);
                harness.run();
            }
        }
        harness
            .get_by_label("Registrar referência e nova versão")
            .click();
        harness.run();
        assert!(
            harness
                .query_by_label("Referência alterada: ui-fixture")
                .is_some()
        );
    }
    assert!(!desktop.view.is_error, "{}", desktop.view.notice);
    let before = desktop.project.as_ref().unwrap().clone();
    assert_eq!(before.revision, 3);
    let pinned = before.normative.active.as_ref().unwrap();
    assert!(
        !pinned
            .catalog
            .norms
            .iter()
            .any(|norm| norm.id == "ui-fixture")
    );
    assert!(before.normative.previous.is_empty());
    let candidate = desktop.view.registry.candidate.as_ref().unwrap();
    let added = candidate
        .catalog
        .norms
        .iter()
        .find(|norm| norm.id == "ui-fixture")
        .unwrap();
    assert_eq!(added.amendments.len(), 2);
    assert_eq!(
        added.access,
        prumo_domain::registry::NormAccess::PublicMetadataOnly
    );
    assert_ne!(pinned.hash, candidate.hash);
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1500.0, 2600.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Normas e capacidades").click();
        harness.run();
        harness
            .get_by_label("Confirmo a mudança do pacote e a necessidade de recálculo")
            .click();
        harness.run();
        harness.get_by_label("Migrar pacote do projeto").click();
        harness.run();
    }
    assert!(!desktop.view.is_error, "{}", desktop.view.notice);
    drop(desktop);
    let reopened = Desktop::new(path);
    let after = reopened.project.as_ref().unwrap();
    assert_eq!(after.revision, 4);
    assert_eq!(after.normative.previous, vec![pinned.clone()]);
    assert!(after.normative.recalculation_required);
    assert_eq!(
        after.normative.migrations[0].changed_norm_ids,
        ["ui-fixture"]
    );
}

#[test]
fn real_ui_reports_capability_block_and_clears_catalog_between_projects() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = Desktop::new(directory.path().join("capability.prumo"));
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1500.0, 1600.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Criar projeto").click();
        harness.run();
        harness.get_by_label("Normas e capacidades").click();
        harness.run();
        harness.get_by_label("Filtrar capacidades").click();
        harness.run();
        harness
            .get_by_label("Filtrar capacidades")
            .type_text("PAR-01-model");
        harness.run();
        harness
            .get_by_label("PAR-01-model | Modelar: Concepção e modelagem")
            .click();
        harness.run();
    }
    let gate = desktop.view.registry.gate.as_ref().unwrap();
    assert!(!gate.allowed);
    assert!(!gate.reasons.is_empty());
    desktop.dispatch(prumo_ui::UiAction::Open(
        directory
            .path()
            .join("new.prumo")
            .to_string_lossy()
            .into_owned(),
    ));
    assert!(desktop.view.registry.gate.is_none());
    assert!(desktop.view.registry.catalog.is_none());
}
