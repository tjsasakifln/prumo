use prumo_app::{AccessMode, AppError, Command, ProjectService};
use prumo_domain::{
    LengthUnit,
    registry::{NormAccess, NormReferenceInput},
};
use serde_json::json;
use std::fs;

fn service(directory: &tempfile::TempDir, access: AccessMode) -> ProjectService {
    ProjectService::open(
        directory.path().join("normas.prumo"),
        access,
        "fixture:operador",
    )
    .unwrap()
}

fn revised_reference(app: &ProjectService, norm_id: &str, edition: &str) -> NormReferenceInput {
    let catalog = app.registry_catalog().unwrap();
    let norm = catalog
        .norms
        .iter()
        .find(|norm| norm.id == norm_id)
        .unwrap();
    let mut reference = NormReferenceInput::from(norm);
    reference.edition = Some(edition.into());
    reference
}

#[test]
fn legacy_project_without_a_bundle_remains_unpinned_when_reopened() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    let original = app.create("Legado", LengthUnit::Metre).unwrap();
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("normative");
    let original_bytes = serde_json::to_vec(&legacy).unwrap();
    fs::write(app.project_path(), &original_bytes).unwrap();
    let reopened = app.read().unwrap();
    assert!(reopened.normative.active.is_none());
    assert!(reopened.normative.catalog.is_none());
    assert!(
        app.normative_package_preview()
            .unwrap()
            .active_hash
            .is_none()
    );
    assert_eq!(fs::read(app.project_path()).unwrap(), original_bytes);
    assert!(!app.final_result_gate("PAR-06-verify").unwrap().allowed);
}

#[test]
fn actual_registration_and_explicit_migration_preserve_the_original_edition_and_hash() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    app.create("Edições", LengthUnit::Metre).unwrap();
    let candidate = app.available_normative_package().unwrap();
    let pinned = app.pin_normative_package(1, &candidate.hash).unwrap();
    let old_package = pinned.normative.active.unwrap();
    let old_edition = old_package
        .catalog
        .norms
        .iter()
        .find(|norm| norm.id == "nbr-6118")
        .unwrap()
        .edition
        .clone();
    let reference = revised_reference(&app, "nbr-6118", "fixture-new-edition");
    let registered = app
        .execute(Command::RegisterNormReference {
            expected_revision: 2,
            reference,
        })
        .unwrap();
    assert_eq!(registered.normative.active.as_ref().unwrap(), &old_package);
    assert_eq!(registered.normative.registry_changes.len(), 1);
    assert_eq!(
        registered.normative.registry_changes[0]
            .previous
            .as_ref()
            .unwrap()
            .edition,
        old_edition
    );
    let preview = app.normative_package_preview().unwrap();
    assert_eq!(preview.project_revision, Some(3));
    assert_eq!(
        preview.active_hash.as_deref(),
        Some(old_package.hash.as_str())
    );
    assert_ne!(preview.candidate.hash, old_package.hash);
    assert_eq!(
        preview.impact.as_ref().unwrap().changed_norm_ids,
        ["nbr-6118"]
    );
    let unchanged = fs::read(app.project_path()).unwrap();
    assert!(matches!(
        app.migrate_normative_package(3, &old_package.hash, &preview.candidate.hash, false),
        Err(AppError::ConfirmationRequired)
    ));
    assert_eq!(fs::read(app.project_path()).unwrap(), unchanged);
    assert!(matches!(
        app.migrate_normative_package(3, &"0".repeat(64), &preview.candidate.hash, true),
        Err(AppError::PackageConflict)
    ));
    assert_eq!(fs::read(app.project_path()).unwrap(), unchanged);
    let migrated = app
        .migrate_normative_package(3, &old_package.hash, &preview.candidate.hash, true)
        .unwrap();
    assert_eq!(
        migrated.normative.previous.as_slice(),
        std::slice::from_ref(&old_package)
    );
    assert_eq!(migrated.normative.migrations[0], preview.impact.unwrap());
    assert!(migrated.normative.recalculation_required);
    assert_eq!(
        migrated.normative.active.as_ref().unwrap().hash,
        preview.candidate.hash
    );
    assert_eq!(
        migrated.normative.previous[0]
            .catalog
            .norms
            .iter()
            .find(|norm| norm.id == "nbr-6118")
            .unwrap()
            .edition,
        old_edition
    );
    drop(app);
    let reopened = service(&directory, AccessMode::ReadOnly).read().unwrap();
    assert_eq!(reopened, migrated);
    let norm = reopened
        .normative
        .active
        .as_ref()
        .unwrap()
        .catalog
        .norms
        .iter()
        .find(|norm| norm.id == "nbr-6118")
        .unwrap();
    assert_eq!(norm.edition.as_deref(), Some("fixture-new-edition"));
    assert_eq!(norm.access, NormAccess::PublicMetadataOnly);
    assert!(norm.licensed_evidence.is_none() && norm.resolution.is_none());
}

#[test]
fn preview_is_complete_after_two_edits_and_stale_confirmation_cannot_migrate() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    app.create("Prévia", LengthUnit::Metre).unwrap();
    let package = app.available_normative_package().unwrap();
    app.pin_normative_package(1, &package.hash).unwrap();
    app.register_norm_reference(2, revised_reference(&app, "nbr-6118", "fixture-a"))
        .unwrap();
    let first_preview = app.normative_package_preview().unwrap();
    app.register_norm_reference(3, revised_reference(&app, "nbr-8800", "fixture-b"))
        .unwrap();
    let preview = app.normative_package_preview().unwrap();
    assert_eq!(
        preview.impact.as_ref().unwrap().changed_norm_ids,
        ["nbr-6118", "nbr-8800"]
    );
    let before = fs::read(app.project_path()).unwrap();
    assert!(matches!(
        app.migrate_normative_package(3, &package.hash, &first_preview.candidate.hash, true),
        Err(AppError::Conflict { .. })
    ));
    assert!(matches!(
        app.migrate_normative_package(4, &package.hash, &first_preview.candidate.hash, true),
        Err(AppError::PackageConflict)
    ));
    assert_eq!(fs::read(app.project_path()).unwrap(), before);
    let migrated = app
        .migrate_normative_package(4, &package.hash, &preview.candidate.hash, true)
        .unwrap();
    assert_eq!(migrated.normative.migrations[0], preview.impact.unwrap());
}

#[test]
fn every_normative_write_enforces_read_only_authorization_and_hash_freshness() {
    let directory = tempfile::tempdir().unwrap();
    let writer = service(&directory, AccessMode::ReadWrite);
    writer.create("Autorizações", LengthUnit::Metre).unwrap();
    let candidate = writer.available_normative_package().unwrap();
    let reader = service(&directory, AccessMode::ReadOnly);
    let reference = revised_reference(&writer, "nbr-6118", "fixture-new");
    let commands = [
        Command::RegisterNormReference {
            expected_revision: 1,
            reference: reference.clone(),
        },
        Command::PinNormativePackage {
            expected_revision: 1,
            package_hash: candidate.hash.clone(),
        },
        Command::MigrateNormativePackage {
            expected_revision: 1,
            from_package_hash: candidate.hash.clone(),
            to_package_hash: "0".repeat(64),
            confirm: true,
        },
    ];
    let before = fs::read(writer.project_path()).unwrap();
    for command in commands {
        assert!(matches!(
            reader.execute(command),
            Err(AppError::PermissionDenied)
        ));
    }
    assert_eq!(fs::read(writer.project_path()).unwrap(), before);
    writer.register_norm_reference(1, reference).unwrap();
    assert!(matches!(
        writer.pin_normative_package(2, &candidate.hash),
        Err(AppError::PackageConflict)
    ));
    let new_candidate = writer.available_normative_package().unwrap();
    writer
        .pin_normative_package(2, &new_candidate.hash)
        .unwrap();
    assert!(matches!(
        writer.pin_normative_package(3, &new_candidate.hash),
        Err(AppError::PackageAlreadyPinned)
    ));
}

#[test]
fn package_tampering_is_rejected_and_public_input_cannot_claim_a_license() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    app.create("Integridade", LengthUnit::Metre).unwrap();
    let package = app.available_normative_package().unwrap();
    let project = app.pin_normative_package(1, &package.hash).unwrap();
    let reference = revised_reference(&app, "nbr-6118", "fixture");
    let mut forged = serde_json::to_value(&reference).unwrap();
    forged["access"] = json!("licensed_text_consulted");
    assert!(serde_json::from_value::<NormReferenceInput>(forged).is_err());
    let mut tampered = serde_json::to_value(project).unwrap();
    tampered["normative"]["active"]["catalog"]["norms"][0]["edition"] = json!("tampered");
    let bytes = serde_json::to_vec(&tampered).unwrap();
    fs::write(app.project_path(), &bytes).unwrap();
    assert_eq!(app.read().unwrap_err().code(), "invalid_project");
    assert!(app.register_norm_reference(2, reference).is_err());
    assert_eq!(fs::read(app.project_path()).unwrap(), bytes);
}

#[test]
fn final_result_query_cannot_turn_planned_inventory_into_a_calculated_result() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    app.create("Bloqueios", LengthUnit::Metre).unwrap();
    let package = app.available_normative_package().unwrap();
    let id = package.catalog.capabilities[0].id.clone();
    app.pin_normative_package(1, &package.hash).unwrap();
    let gate = app.final_result_gate(&id).unwrap();
    assert!(!gate.allowed);
    for code in [
        "result_missing",
        "capability_not_released",
        "qualification_incomplete",
    ] {
        assert!(gate.reasons.iter().any(|reason| reason.code == code));
    }
}
