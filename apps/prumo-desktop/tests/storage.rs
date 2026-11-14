//! The native UI diagnoses, migrates and recovers actual on-disk files.
use eframe::egui::vec2;
use egui_kittest::{Harness, kittest::Queryable};
use prumo_app::{AccessMode, ProjectService};
use prumo_desktop::Desktop;
use prumo_domain::storage::{CandidateKind, StorageAction, StorageFormat, StorageState};
use prumo_domain::{LengthUnit, Project};
use serde_json::json;
use std::path::Path;

fn harness(path: &Path) -> Harness<'static, Desktop> {
    Harness::builder()
        .with_size(vec2(1500.0, 2400.0))
        .build_ui_state(
            |ui, desktop| desktop.render(ui),
            Desktop::new(path.to_path_buf()),
        )
}

fn click(ui: &mut Harness<'_, Desktop>, label: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
}

fn legacy(path: &Path) -> Project {
    let seed = tempfile::tempdir().unwrap();
    let service = ProjectService::open(
        seed.path().join("seed.prumo"),
        AccessMode::ReadWrite,
        "legacy-fixture",
    )
    .unwrap();
    service
        .create("Legado de instalações", LengthUnit::Centimetre)
        .unwrap();
    let mut project = service
        .add_member(1, "Peça preservada", [0.0; 3], [400.0, 0.0, 0.0])
        .unwrap();
    project.extensions.insert("future-installations".into(),json!({"version":17,"opaque":{"water":"keep","gas":[1,2,3],"equipment":{"vendor":"fixture"}}}));
    project.validate().unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
    project
}

fn candidate_label(candidate: &prumo_domain::storage::StorageCandidate) -> String {
    format!(
        "{} | revisão {} | {}",
        candidate.label,
        candidate.revision,
        candidate.sha256.chars().take(8).collect::<String>()
    )
}

#[test]
fn actual_ui_opens_legacy_without_writing_then_migrates_and_reopens_package() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.prumo");
    let expected = legacy(&path);
    let original = std::fs::read(&path).unwrap();
    let mut ui = harness(&path);
    assert_eq!(ui.state().project.as_ref(), Some(&expected));
    assert_eq!(
        ui.state().view.storage.status.as_ref().unwrap().format,
        StorageFormat::LegacyJson
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(ui.query_by_label("Arquivo legado preservado").is_some());
    click(
        &mut ui,
        "Confirmo migrar o arquivo e manter a cópia original",
    );
    click(&mut ui, "Migrar arquivo Prumo");
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
    let status = ui.state().view.storage.status.as_ref().unwrap();
    assert_eq!(status.format, StorageFormat::NativePackage);
    assert_eq!(status.state, StorageState::Ready);
    assert!(status.package.is_some());
    assert!(
        status
            .candidates
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::Original
                && candidate.sha256 == prumo_domain::canonical::hash_bytes(&original))
    );
    assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
    let after = ui.state().project.as_ref().unwrap().clone();
    assert_eq!(after.id, expected.id);
    assert_eq!(after.objects, expected.objects);
    assert_eq!(after.extensions, expected.extensions);
    assert_eq!(after.model, expected.model);
    drop(ui);
    let reopened = Desktop::new(path);
    assert_eq!(reopened.project.as_ref(), Some(&after));
    assert_eq!(
        reopened.view.storage.status.unwrap().format,
        StorageFormat::NativePackage
    );
}

#[test]
fn actual_ui_creates_backup_and_recovers_when_corruption_prevents_project_opening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recover.prumo");
    let mut ui = harness(&path);
    click(&mut ui, "Criar projeto");
    click(&mut ui, "Adicionar peça e salvar");
    click(&mut ui, "Arquivo e recuperação");
    click(&mut ui, "Criar backup verificado");
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
    let saved = ui.state().project.as_ref().unwrap().clone();
    assert!(
        ui.state()
            .view
            .storage
            .status
            .as_ref()
            .unwrap()
            .candidates
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::Backup && candidate.recoverable)
    );
    drop(ui);
    std::fs::write(&path, b"PK damaged primary package").unwrap();
    let mut ui = harness(&path);
    assert!(ui.state().project.is_none());
    assert!(ui.state().view.is_error);
    assert!(ui.query_by_label("Criar projeto").is_none());
    let candidate = ui
        .state()
        .view
        .storage
        .status
        .as_ref()
        .unwrap()
        .candidates
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Backup && candidate.recoverable)
        .unwrap()
        .clone();
    click(&mut ui, "Candidato de recuperação");
    click(&mut ui, &candidate_label(&candidate));
    click(
        &mut ui,
        "Confirmo restaurar este candidato e preservar o arquivo atual",
    );
    click(&mut ui, "Recuperar candidato selecionado");
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
    let restored = ui.state().project.as_ref().unwrap().clone();
    assert_eq!(restored.id, saved.id);
    assert_eq!(restored.objects, saved.objects);
    assert_eq!(restored.model, saved.model);
    assert_eq!(
        ui.state().view.storage.status.as_ref().unwrap().format,
        StorageFormat::NativePackage
    );
    drop(ui);
    assert_eq!(Desktop::new(path).project.unwrap(), restored);
}

#[test]
fn changed_path_and_diagnosis_expire_storage_confirmation_even_without_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("selected.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture").unwrap();
    service.create("Selecionado", LengthUnit::Metre).unwrap();
    let status = service.storage_status().unwrap();
    service
        .execute_storage(StorageAction::Backup {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256,
        })
        .unwrap();
    std::fs::write(&path, b"corrupt selected file").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut ui = harness(&path);
    assert!(ui.state().project.is_none());
    let status = ui.state().view.storage.status.as_ref().unwrap().clone();
    let candidate = status
        .candidates
        .iter()
        .find(|candidate| candidate.recoverable)
        .unwrap();
    click(&mut ui, "Candidato de recuperação");
    click(&mut ui, &candidate_label(candidate));
    click(
        &mut ui,
        "Confirmo restaurar este candidato e preservar o arquivo atual",
    );
    assert!(ui.state().view.storage.confirm_recovery);
    click(&mut ui, "Inspecionar arquivo");
    assert!(!ui.state().view.storage.confirm_recovery);
    ui.state_mut().view.path = directory
        .path()
        .join("another.prumo")
        .to_string_lossy()
        .into_owned();
    ui.state_mut()
        .dispatch(prumo_ui::UiAction::Storage(Box::new(
            StorageAction::Recover {
                candidate_id: candidate.id.clone(),
                inspection_sha256: status.inspection_sha256,
                confirm: true,
            },
        )));
    assert!(ui.state().view.is_error);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(!directory.path().join("another.prumo").exists());
}

#[test]
fn actual_ui_preserves_future_package_and_offers_diagnosis_instead_of_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("future.prumo");
    let bytes = include_bytes!("../../../crates/prumo-storage/tests/fixtures/future-format.prumo");
    std::fs::write(&path, bytes).unwrap();
    let mut ui = harness(&path);
    assert!(ui.state().project.is_none());
    assert!(ui.state().view.is_error);
    let status = ui.state().view.storage.status.as_ref().unwrap();
    assert_eq!(status.state, StorageState::Unsupported);
    assert!(!status.can_migrate && !status.can_backup && !status.can_recover);
    assert!(
        ui.query_by_label("Versão ou formato incompatível")
            .is_some()
    );
    assert!(ui.query_by_label("Criar projeto").is_none());
    assert!(ui.query_by_label("Migrar arquivo Prumo").is_none());
    click(&mut ui, "Inspecionar arquivo");
    assert_eq!(std::fs::read(&path).unwrap(), bytes.as_slice());
    click(&mut ui, "Reabrir / atualizar");
    assert_eq!(std::fs::read(&path).unwrap(), bytes.as_slice());
}

#[test]
fn inspection_invalidates_editor_when_same_path_contains_another_project_or_changed_bytes() {
    let first = tempfile::tempdir().unwrap();
    let first_path = first.path().join("first.prumo");
    let first_project = ProjectService::open(&first_path, AccessMode::ReadWrite, "fixture")
        .unwrap()
        .create("Primeiro projeto", LengthUnit::Metre)
        .unwrap();
    let second = tempfile::tempdir().unwrap();
    let second_path = second.path().join("second.prumo");
    let second_project = ProjectService::open(&second_path, AccessMode::ReadWrite, "fixture")
        .unwrap()
        .create("Segundo projeto", LengthUnit::Metre)
        .unwrap();
    assert_eq!(first_project.revision, second_project.revision);
    assert_ne!(first_project.id, second_project.id);

    let selected = tempfile::tempdir().unwrap();
    let path = selected.path().join("selected.prumo");
    std::fs::copy(&first_path, &path).unwrap();
    let mut ui = harness(&path);
    assert_eq!(ui.state().project.as_ref(), Some(&first_project));
    click(&mut ui, "Arquivo e recuperação");
    let bytes = std::fs::read(&second_path).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    click(&mut ui, "Inspecionar arquivo");
    assert!(ui.state().project.is_none());
    assert_eq!(
        ui.state().view.storage.status.as_ref().unwrap().project_id,
        Some(second_project.id)
    );
    assert!(
        ui.query_by_label("Reabra o projeto para continuar")
            .is_some()
    );
    assert!(ui.query_by_label("Adicionar peça e salvar").is_none());
    assert!(ui.query_by_label("Criar projeto").is_none());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    click(&mut ui, "Reabrir / atualizar");
    assert_eq!(ui.state().project.as_ref(), Some(&second_project));

    // Even an identity and revision copied unchanged do not make new bytes equal.
    let mut changed = second_project;
    changed.name = "Conteúdo substituído na mesma revisão".into();
    changed.validate().unwrap();
    let bytes = serde_json::to_vec_pretty(&changed).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    click(&mut ui, "Inspecionar arquivo");
    assert!(ui.state().project.is_none());
    assert!(ui.query_by_label("Adicionar peça e salvar").is_none());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    click(&mut ui, "Reabrir / atualizar");
    assert_eq!(ui.state().project.as_ref(), Some(&changed));
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
}

#[test]
fn direct_edit_of_portable_project_refuses_external_identity_or_byte_replacement_before_inspection()
{
    for same_identity in [false, true] {
        let fixtures = tempfile::tempdir().unwrap();
        let first_path = fixtures.path().join("first.prumo");
        let first = ProjectService::open(&first_path, AccessMode::ReadWrite, "fixture")
            .unwrap()
            .create("Projeto exibido", LengthUnit::Metre)
            .unwrap();
        let (replacement, bytes) = if same_identity {
            let mut replacement = first.clone();
            replacement.name = "Outros bytes, mesmo UUID e revisão".into();
            replacement
                .extensions
                .insert("future-data".into(), json!({"preserve":17}));
            replacement.validate().unwrap();
            let bytes = serde_json::to_vec_pretty(&replacement).unwrap();
            (replacement, bytes)
        } else {
            let second_path = fixtures.path().join("second.prumo");
            let replacement = ProjectService::open(&second_path, AccessMode::ReadWrite, "fixture")
                .unwrap()
                .create("Outro projeto", LengthUnit::Metre)
                .unwrap();
            assert_ne!(first.id, replacement.id);
            let bytes = std::fs::read(second_path).unwrap();
            (replacement, bytes)
        };
        assert_eq!(first.revision, replacement.revision);
        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("selected.prumo");
        std::fs::copy(first_path, &path).unwrap();
        let mut ui = harness(&path);
        assert_eq!(ui.state().project.as_ref(), Some(&first));
        // The portable file has only been read, so no local working DB can
        // incidentally detect the replacement on behalf of the host's basis.
        assert_eq!(std::fs::read_dir(selected.path()).unwrap().count(), 1);
        std::fs::write(&path, &bytes).unwrap();
        click(&mut ui, "Adicionar peça e salvar");
        assert!(ui.state().view.is_error);
        assert!(ui.state().project.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(ui.query_by_label("Adicionar peça e salvar").is_none());
        assert!(
            ui.query_by_label("Reabra o projeto para continuar")
                .is_some()
        );
        click(&mut ui, "Reabrir / atualizar");
        assert_eq!(ui.state().project.as_ref(), Some(&replacement));
        assert!(ui.state().project.as_ref().unwrap().objects.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
