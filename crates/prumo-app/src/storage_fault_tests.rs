//! Real child-process termination, with fault controls compiled only in tests.

use super::{AccessMode, ProjectService};
use crate::storage::SavePhase;
use prumo_domain::{
    LengthUnit,
    storage::{CandidateKind, StorageAction},
};
use std::{fs, process::Command};

const SAVE_PHASES: [SavePhase; 8] = [
    SavePhase::ObjectsInserted,
    SavePhase::RevisionInserted,
    SavePhase::HeadUpdated,
    SavePhase::BeforeCommit,
    SavePhase::DatabaseCommitted,
    SavePhase::AutosavePublished,
    SavePhase::BackupPublished,
    SavePhase::PrimaryPublished,
];
const RECOVERY_PHASES: [SavePhase; 5] = [
    SavePhase::RecoveryPreserved,
    SavePhase::RecoveryAutosavePublished,
    SavePhase::RecoveryDatabaseRestored,
    SavePhase::RecoveryRecorded,
    SavePhase::RecoveryPrimaryPublished,
];

#[test]
fn crash_worker() {
    let Some(path) = std::env::var_os("PRUMO_TEST_CRASH_PROJECT") else {
        return;
    };
    let phase = std::env::var("PRUMO_TEST_CRASH_PHASE").unwrap();
    let fault = SAVE_PHASES
        .into_iter()
        .chain(RECOVERY_PHASES)
        .find(|candidate| format!("{candidate:?}") == phase)
        .unwrap();
    let mut service = ProjectService::open(path, AccessMode::ReadWrite, "fixture:crash").unwrap();
    service.fault = Some(fault);
    if SAVE_PHASES.contains(&fault) {
        service.rename_project(1, "Revisão interrompida").unwrap();
    } else {
        let status = service.storage_status().unwrap();
        let id = status
            .candidates
            .iter()
            .find(|candidate| candidate.kind == CandidateKind::Backup && candidate.revision == 1)
            .unwrap()
            .id
            .clone();
        service
            .execute_storage(StorageAction::Recover {
                candidate_id: id,
                inspection_sha256: status.inspection_sha256,
                confirm: true,
            })
            .unwrap();
    }
    panic!("O ponto de interrupção não foi atingido.");
}

fn terminate_at(path: &std::path::Path, phase: SavePhase) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "storage_fault_tests::crash_worker",
            "--nocapture",
        ])
        .env("PRUMO_TEST_CRASH_PROJECT", path)
        .env("PRUMO_TEST_CRASH_PHASE", format!("{phase:?}"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "{phase:?}: child unexpectedly succeeded"
    );
    let marker = format!("[prumo-test-crash:{phase:?}]");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&marker),
        "{phase:?}: missing phase marker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            output.status.signal(),
            Some(6),
            "{phase:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(windows)]
    {
        // Rust abort uses fast-fail (0xc0000409) on current Windows; reject the
        // ordinary libtest failure code so a panic cannot count as a kill.
        assert_ne!(
            output.status.code(),
            Some(101),
            "{phase:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_ne!(
            output.status.code(),
            Some(1),
            "{phase:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn abrupt_exit_at_eight_save_phases_retains_a_complete_revision_and_recovers_new_commits() {
    for phase in SAVE_PHASES {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("interruption.prumo");
        let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture:parent").unwrap();
        let before = service
            .create("Revisão anterior", LengthUnit::Metre)
            .unwrap();
        let previous_bytes = fs::read(&path).unwrap();
        terminate_at(&path, phase);
        let reopened =
            ProjectService::open(&path, AccessMode::ReadWrite, "fixture:recovery").unwrap();
        let published = reopened.read().unwrap();
        assert!(
            published == before
                || (published.revision == 2 && published.name == "Revisão interrompida"),
            "{phase:?}"
        );
        if phase != SavePhase::PrimaryPublished {
            assert_eq!(fs::read(&path).unwrap(), previous_bytes, "{phase:?}");
        }
        let status = reopened.storage_status().unwrap();
        let after_commit = matches!(
            phase,
            SavePhase::DatabaseCommitted
                | SavePhase::AutosavePublished
                | SavePhase::BackupPublished
                | SavePhase::PrimaryPublished
        );
        let target = if after_commit { 2 } else { 1 };
        let candidate = status
            .candidates
            .iter()
            .find(|candidate| candidate.recoverable && candidate.revision == target)
            .unwrap();
        let recovered = reopened
            .execute_storage(StorageAction::Recover {
                candidate_id: candidate.id.clone(),
                inspection_sha256: status.inspection_sha256,
                confirm: true,
            })
            .unwrap();
        assert_eq!(recovered.project.revision, target, "{phase:?}");
        assert_eq!(reopened.read().unwrap(), recovered.project, "{phase:?}");
        assert!(recovered.status.package.is_some());
        let edited = reopened
            .rename_project(target, "Edição após recuperação")
            .unwrap();
        assert_eq!(edited.revision, target + 1);
    }
}

#[test]
fn abrupt_exit_at_five_recovery_phases_keeps_the_selected_source_recoverable() {
    for phase in RECOVERY_PHASES {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recovery.prumo");
        let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture:parent").unwrap();
        let first = service
            .create("Revisão preservada", LengthUnit::Metre)
            .unwrap();
        service.rename_project(1, "Outra revisão").unwrap();
        fs::write(&path, b"arquivo adulterado para o ensaio").unwrap();
        terminate_at(&path, phase);
        let reopened =
            ProjectService::open(&path, AccessMode::ReadWrite, "fixture:recovery").unwrap();
        if let Ok(project) = reopened.read() {
            assert_eq!(project, first);
        }
        let status = reopened.storage_status().unwrap();
        let candidate = status
            .candidates
            .iter()
            .find(|candidate| candidate.recoverable && candidate.revision == 1)
            .unwrap();
        let recovered = reopened
            .execute_storage(StorageAction::Recover {
                candidate_id: candidate.id.clone(),
                inspection_sha256: status.inspection_sha256,
                confirm: true,
            })
            .unwrap();
        assert_eq!(recovered.project, first, "{phase:?}");
        assert_eq!(reopened.read().unwrap(), first, "{phase:?}");
    }
}

#[test]
fn first_legacy_edit_cannot_discard_a_committed_revision_after_interruption() {
    let origin = tempfile::tempdir().unwrap();
    let original = ProjectService::open(
        origin.path().join("source.prumo"),
        AccessMode::ReadWrite,
        "fixture:source",
    )
    .unwrap()
    .create("JSON original", LengthUnit::Metre)
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.prumo");
    let bytes = serde_json::to_vec_pretty(&original).unwrap();
    fs::write(&path, &bytes).unwrap();
    terminate_at(&path, SavePhase::DatabaseCommitted);
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture:recovery").unwrap();
    assert_eq!(service.read().unwrap(), original);
    assert!(matches!(
        service.rename_project(1, "Descartar pendência"),
        Err(super::AppError::RecoveryRequired)
    ));
    let status = service.storage_status().unwrap();
    assert!(!status.can_migrate);
    assert!(matches!(
        service.execute_storage(StorageAction::Migrate {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256.clone(),
            confirm: true,
        }),
        Err(super::AppError::RecoveryRequired)
    ));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let candidate = status
        .candidates
        .iter()
        .find(|candidate| {
            candidate.kind == CandidateKind::WorkingDatabase && candidate.revision == 2
        })
        .unwrap();
    let recovered = service
        .execute_storage(StorageAction::Recover {
            candidate_id: candidate.id.clone(),
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    assert_eq!(recovered.project.revision, 2);
    assert_eq!(recovered.project.name, "Revisão interrompida");
}
