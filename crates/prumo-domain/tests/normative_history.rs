use prumo_domain::registry::{
    NormativePackage, NormativeState, RegistryCatalog, RegistryError, compare_packages,
};

#[test]
fn every_historical_package_is_revalidated_before_comparing_migration_content() {
    let catalog: RegistryCatalog =
        serde_json::from_str(include_str!("../../prumo-registry/data/catalogue.json")).unwrap();
    let first = NormativePackage::from_catalog(&catalog).unwrap();
    let mut second_catalog = catalog.clone();
    second_catalog.version = "fixture-next".into();
    second_catalog.norms[0]
        .title
        .push_str("; alteração sintética de ensaio");
    let second = NormativePackage::from_catalog(&second_catalog).unwrap();
    let mut third_catalog = second_catalog.clone();
    third_catalog.version = "fixture-third".into();
    third_catalog.norms[1]
        .title
        .push_str("; segunda alteração sintética");
    let third = NormativePackage::from_catalog(&third_catalog).unwrap();
    let state = NormativeState {
        catalog: Some(third_catalog),
        active: Some(third.clone()),
        previous: vec![first.clone(), second.clone()],
        migrations: vec![
            compare_packages(&first, &second, 2).unwrap(),
            compare_packages(&second, &third, 3).unwrap(),
        ],
        registry_changes: vec![],
        recalculation_required: true,
    };
    state.validate(3).unwrap();
    for index in 0..2 {
        let mut modified = state.clone();
        modified.previous[index].catalog.norms[0].edition =
            Some("edição adulterada sem novo hash".into());
        assert!(
            matches!(modified.validate(3), Err(RegistryError::HashMismatch)),
            "pacote histórico {index}"
        );
        assert!(matches!(
            compare_packages(&modified.previous[index], &third, 4),
            Err(RegistryError::HashMismatch)
        ));
    }
    let mut modified = state.clone();
    modified.active.as_mut().unwrap().catalog.norms[0]
        .title
        .push_str(" adulteração");
    assert!(matches!(
        modified.validate(3),
        Err(RegistryError::HashMismatch)
    ));
    let mut modified = state;
    modified.migrations[0].changed_norm_ids.clear();
    assert!(matches!(
        modified.validate(3),
        Err(RegistryError::InvalidHistory)
    ));
}
