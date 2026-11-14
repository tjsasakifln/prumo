use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;
use std::time::{Duration, Instant};

#[test]
fn actual_ui_runs_component_worker_and_clears_evidence_after_project_change() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("components.prumo");
    let mut desktop = Desktop::new(path.clone());
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1400.0, 1200.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Criar projeto").click();
        harness.run();
        harness.get_by_label("Verificação de componentes").click();
        harness.run();
        harness
            .get_by_label("Executar ensaios de componentes")
            .click();
        harness.step();
        let deadline = Instant::now() + Duration::from_secs(25);
        while harness
            .query_by_label("Todos os ensaios passaram no domínio descrito.")
            .is_none()
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(10));
            harness.step();
        }
    }
    assert!(!desktop.view.is_error, "{}", desktop.view.notice);
    let report = desktop
        .view
        .components
        .report
        .as_ref()
        .expect("A interface precisa receber o processo real.");
    assert!(report.passed());
    assert_eq!(report.experiments.len(), 6);
    assert_eq!(report.basis.as_ref().unwrap().revision, 1);
    assert!(!report.engineering_release);
    assert_eq!(desktop.project.as_ref().unwrap().revision, 1);
    let before = std::fs::read(&path).unwrap();
    let readonly = prumo_app::ProjectService::open(
        &path,
        prumo_app::AccessMode::ReadOnly,
        "qualification-test",
    )
    .unwrap();
    assert_eq!(readonly.read().unwrap().revision, 1);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    {
        let mut harness = Harness::builder()
            .with_size(eframe::egui::vec2(1400.0, 1200.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Adicionar peça e salvar").click();
        harness.run();
    }
    assert_eq!(desktop.project.as_ref().unwrap().revision, 2);
    assert!(desktop.view.components.report.is_none());
}
