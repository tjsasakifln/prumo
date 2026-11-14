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
        reopened.project.as_ref().unwrap().objects[0].analytical_id,
        before.objects[0].analytical_id
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
    desktop.dispatch(prumo_ui::UiAction::Command(prumo_domain::Command::Create {
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
    desktop.dispatch(prumo_ui::UiAction::Command(prumo_domain::Command::Create {
        name: "Original".into(),
        units: prumo_domain::LengthUnit::Metre,
    }));
    desktop.view.path = directory
        .path()
        .join("another.prumo")
        .to_string_lossy()
        .into_owned();
    desktop.dispatch(prumo_ui::UiAction::Command(
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
