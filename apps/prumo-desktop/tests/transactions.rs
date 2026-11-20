//! Drive real widgets through prepare, review, apply, undo, redo and reopening.
use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;

fn click(harness: &mut Harness<'_, Desktop>, label: &str) {
    harness.get_by_label(label).scroll_to_me();
    harness.run();
    harness.get_by_label(label).click();
    harness.run();
    assert!(
        !harness.state().view.is_error,
        "{}",
        harness.state().view.notice
    );
}

#[test]
fn real_widgets_review_an_atomic_batch_then_undo_redo_and_reopen_the_same_entities() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transaction-ui.prumo");
    let mut ui = Harness::builder()
        .with_size(eframe::egui::vec2(1500.0, 2600.0))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(path.clone()),
        );
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Alterações em lote e histórico");
    click(&mut ui, "Preparar alterações em lote antes de salvar");
    click(&mut ui, "Adicionar peça e salvar");
    click(&mut ui, "Nome e ambientes");
    click(&mut ui, "Adicionar ambiente");
    assert_eq!(ui.state().project.as_ref().unwrap().revision, 1);
    assert!(ui.state().project.as_ref().unwrap().objects.is_empty());
    assert_eq!(
        ui.state()
            .view
            .transactions
            .pending
            .as_ref()
            .unwrap()
            .operations
            .len(),
        2
    );
    click(&mut ui, "Conferir prévia do lote");
    let expected_hash = ui
        .state()
        .view
        .transactions
        .preview
        .as_ref()
        .unwrap()
        .after_snapshot_sha256
        .clone();
    assert_eq!(ui.state().project.as_ref().unwrap().revision, 1);
    click(&mut ui, "Aplicar lote conferido");
    let saved = ui.state().project.as_ref().unwrap().clone();
    assert_eq!(saved.revision, 2);
    assert_eq!(saved.objects.len(), 1);
    assert_eq!(saved.environments.len(), 1);
    assert_eq!(
        prumo_domain::canonical::model_snapshot(&saved)
            .unwrap()
            .sha256,
        expected_hash
    );
    click(&mut ui, "Desfazer última alteração");
    assert_eq!(ui.state().project.as_ref().unwrap().revision, 3);
    assert!(ui.state().project.as_ref().unwrap().objects.is_empty());
    assert!(ui.state().project.as_ref().unwrap().environments.is_empty());
    click(&mut ui, "Refazer alteração");
    assert_eq!(ui.state().project.as_ref().unwrap().revision, 4);
    assert_eq!(ui.state().project.as_ref().unwrap().objects, saved.objects);
    assert_eq!(
        ui.state().project.as_ref().unwrap().environments,
        saved.environments
    );
    assert_eq!(ui.state().project.as_ref().unwrap().model, saved.model);
    click(&mut ui, "Reabrir / atualizar");
    drop(ui);
    let reopened = Desktop::new(path);
    assert_eq!(reopened.project.as_ref().unwrap().revision, 4);
    assert_eq!(reopened.project.as_ref().unwrap().objects, saved.objects);
    assert_eq!(reopened.project.as_ref().unwrap().model, saved.model);
}
