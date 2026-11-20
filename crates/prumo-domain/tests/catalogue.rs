use prumo_domain::catalogue::sections::{SectionInput, SectionPart, TorsionState};
use prumo_domain::catalogue::*;
use prumo_domain::dependencies::{
    Dependency, InspectionPayload, ResultState, SourceKey, SourceKind,
};
use prumo_domain::model::{
    MaterialFamily, MaterialInput, ModelEdit, PhysicalRole, apply_model_edit,
};
use prumo_domain::units::{DensityInput, DensityUnit, PointInput, PressureInput, PressureUnit};
use prumo_domain::{ChangeRecord, Discipline, LengthUnit, PROJECT_SCHEMA_VERSION, Project};
use std::collections::BTreeMap;
use uuid::Uuid;

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.4e}"
    );
}
fn value(decimal: &str, unit: PhysicalUnit, quantity: PhysicalQuantity) -> PhysicalValue {
    PhysicalValue {
        quantity,
        decimal: decimal.into(),
        unit,
        basis: ValueBasis::Nominal,
        design_basis: None,
    }
}
fn source() -> CatalogSource {
    CatalogSource {
        publisher: "Synthetic test source".into(),
        reference: "original test inputs".into(),
        edition: "1".into(),
        revision: "1".into(),
        data_license: "original fixture".into(),
        original_record: None,
    }
}
fn rectangle_entry(name: &str, width: f64) -> CatalogEntry {
    CatalogEntry {
        id: Uuid::new_v4(),
        revision: 1,
        name: name.into(),
        source: source(),
        applicability: vec![],
        customization: None,
        payload: CatalogPayload::Section {
            shape: SectionShape::Rectangle {
                width_m: width,
                height_m: 0.4,
            },
        },
    }
}
fn isotropic() -> CatalogMaterial {
    CatalogMaterial {
        class: "Generic elastic fixture".into(),
        states: vec![MaterialState {
            name: "reference".into(),
            conditions: vec![],
            elastic: ElasticLaw::Isotropic {
                young_modulus: value(
                    "200",
                    PhysicalUnit::Gigapascal,
                    PhysicalQuantity::ElasticModulus,
                ),
                poisson_ratio: 0.3,
            },
            density: value(
                "7850",
                PhysicalUnit::KilogramPerCubicMetre,
                PhysicalQuantity::Density,
            ),
            properties: BTreeMap::new(),
        }],
    }
}
fn material_entry() -> CatalogEntry {
    CatalogEntry {
        id: Uuid::new_v4(),
        revision: 1,
        name: "Material fixture".into(),
        source: source(),
        applicability: vec![],
        customization: None,
        payload: CatalogPayload::Material {
            material: isotropic(),
        },
    }
}
fn empty() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Catalogue fixture".into(),
        revision: 1,
        units: LengthUnit::Metre,
        objects: vec![],
        environments: vec![],
        networks: vec![],
        ports: vec![],
        connections: vec![],
        history: vec![ChangeRecord {
            revision: 1,
            actor: "fixture".into(),
            operation: "create".into(),
            timestamp_unix_ms: 1,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: BTreeMap::new(),
    }
}
fn stamp(project: &mut Project) {
    project.revision += 1;
    project.history.push(ChangeRecord {
        revision: project.revision,
        actor: "fixture".into(),
        operation: "edit".into(),
        timestamp_unix_ms: project.revision,
    });
    project.validate().unwrap();
}
fn apply(project: &mut Project, edit: ModelEdit) {
    let before = prumo_domain::canonical::source_index_unchecked(project).unwrap();
    apply_model_edit(project, edit, "fixture").unwrap();
    let after = prumo_domain::canonical::source_index_unchecked(project).unwrap();
    project
        .dependencies
        .invalidate_changes(
            &before,
            &after,
            project.revision + 1,
            "fixture",
            "catalogue change",
        )
        .unwrap();
    stamp(project);
}
fn physical(project: &mut Project, start: [f64; 3], end: [f64; 3]) -> Uuid {
    apply(
        project,
        ModelEdit::AddPhysical {
            name: "piece".into(),
            discipline: Discipline::Structure,
            start: PointInput {
                values: start,
                unit: LengthUnit::Metre,
            },
            end: PointInput {
                values: end,
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::VisualOnly,
            material_id: None,
            environment_id: None,
        },
    );
    project.objects.last().unwrap().id
}

#[test]
fn rectangle_circle_i_box_and_asymmetric_polygon_match_closed_geometry() {
    let rectangle = SectionShape::Rectangle {
        width_m: 2.0,
        height_m: 3.0,
    }
    .geometry()
    .unwrap();
    near(rectangle.area_m2, 6.0, 1e-12);
    near(rectangle.ixx_m4, 4.5, 1e-12);
    near(rectangle.iyy_m4, 2.0, 1e-12);
    near(rectangle.centroid_m[0], 1.0, 1e-12);
    near(rectangle.centroid_m[1], 1.5, 1e-12);
    let circle = SectionShape::Circle { diameter_m: 2.0 }.geometry().unwrap();
    near(circle.area_m2, std::f64::consts::PI, 1e-12);
    near(circle.ixx_m4, std::f64::consts::PI / 4.0, 1e-12);
    let section = SectionShape::ISection {
        width_m: 0.2,
        height_m: 0.4,
        flange_m: 0.02,
        web_m: 0.01,
    }
    .geometry()
    .unwrap();
    near(section.area_m2, 2.0 * 0.2 * 0.02 + 0.36 * 0.01, 1e-13);
    near(
        section.ixx_m4,
        (0.2 * 0.4f64.powi(3) - 0.19 * 0.36f64.powi(3)) / 12.0,
        1e-13,
    );
    near(
        section.iyy_m4,
        (2.0 * 0.02 * 0.2f64.powi(3) + 0.36 * 0.01f64.powi(3)) / 12.0,
        1e-13,
    );
    let closed = SectionShape::Box {
        width_m: 0.3,
        height_m: 0.5,
        wall_m: 0.02,
    }
    .geometry()
    .unwrap();
    near(closed.area_m2, 0.3 * 0.5 - 0.26 * 0.46, 1e-13);
    near(
        closed.ixx_m4,
        (0.3 * 0.5f64.powi(3) - 0.26 * 0.46f64.powi(3)) / 12.0,
        1e-13,
    );
    let triangle = SectionShape::Polygon {
        section: SectionInput {
            exterior_m: vec![[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]],
            holes_m: vec![],
        },
    }
    .geometry()
    .unwrap();
    near(triangle.area_m2, 6.0, 1e-12);
    near(triangle.centroid_m[0], 4.0 / 3.0, 1e-12);
    near(triangle.centroid_m[1], 1.0, 1e-12);
    near(triangle.ixx_m4, 4.0 * 27.0 / 36.0, 1e-12);
    near(triangle.iyy_m4, 3.0 * 64.0 / 36.0, 1e-12);
    near(triangle.ixy_m4, -16.0 * 9.0 / 72.0, 1e-12);
}

#[test]
fn compound_section_uses_union_and_parallel_axis_without_double_counting() {
    let shape = SectionShape::Compound {
        parts: vec![
            SectionPart {
                shape: Box::new(SectionShape::Rectangle {
                    width_m: 2.0,
                    height_m: 1.0,
                }),
                translation_m: [0.0, 0.0],
                rotation_rad: 0.0,
            },
            SectionPart {
                shape: Box::new(SectionShape::Rectangle {
                    width_m: 1.0,
                    height_m: 2.0,
                }),
                translation_m: [0.0, 1.0],
                rotation_rad: 0.0,
            },
        ],
    };
    let p = shape.geometry().unwrap();
    near(p.area_m2, 4.0, 1e-12);
    near(p.centroid_m[0], 0.75, 1e-12);
    near(p.centroid_m[1], 1.25, 1e-12);
    near(
        p.ixx_m4,
        2.0 / 12.0 + 2.0 * 0.75f64.powi(2) + 8.0 / 12.0 + 2.0 * 0.75f64.powi(2),
        1e-12,
    );
    near(
        p.ixy_m4,
        2.0 * (1.0 - 0.75) * (0.5 - 1.25) + 2.0 * (0.5 - 0.75) * (2.0 - 1.25),
        1e-12,
    );
    let overlap = SectionShape::Compound {
        parts: vec![
            SectionPart {
                shape: Box::new(SectionShape::Rectangle {
                    width_m: 1.0,
                    height_m: 1.0
                }),
                translation_m: [0.0, 0.0],
                rotation_rad: 0.0
            };
            2
        ],
    };
    assert!(overlap.geometry().is_err());
}

#[test]
fn analytical_torsion_is_not_the_polar_moment_except_for_a_circle() {
    let square = analyse_section(&SectionShape::Rectangle {
        width_m: 1.0,
        height_m: 1.0,
    })
    .unwrap();
    near(square.torsion.j_m4.unwrap(), 0.140577014955, 1e-11);
    assert!(
        (square.torsion.j_m4.unwrap() - square.geometry.ixx_m4 - square.geometry.iyy_m4).abs()
            > 0.02
    );
    let circle = analyse_section(&SectionShape::Circle { diameter_m: 2.0 }).unwrap();
    near(
        circle.torsion.j_m4.unwrap(),
        std::f64::consts::PI / 2.0,
        1e-12,
    );
    assert_eq!(circle.torsion.state, TorsionState::Analytical);
    assert_eq!(
        circle.torsion_assumption,
        "homogeneous_isotropic_saint_venant"
    );
    assert!(!circle.engineering_release);
}

#[test]
fn polygon_warping_converges_to_independent_rectangle_fourier_reference() {
    let p = analyse_section(&SectionShape::Polygon {
        section: SectionInput {
            exterior_m: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
            holes_m: vec![],
        },
    })
    .unwrap();
    let reference = analyse_section(&SectionShape::Rectangle {
        width_m: 2.0,
        height_m: 1.0,
    })
    .unwrap();
    assert_eq!(p.torsion.state, TorsionState::Converged, "{:#?}", p.torsion);
    near(
        p.torsion.j_m4.unwrap(),
        reference.torsion.j_m4.unwrap(),
        0.005 * reference.torsion.j_m4.unwrap(),
    );
    assert!(p.torsion.relative_change.unwrap() <= 0.002);
    assert!(p.torsion.refinements >= 3);
}

#[test]
fn closed_thin_wall_torsion_agrees_with_independent_bredt_limit() {
    let p = analyse_section(&SectionShape::Box {
        width_m: 1.0,
        height_m: 1.0,
        wall_m: 0.02,
    })
    .unwrap();
    let bredt = 4.0 * 0.98f64.powi(4) / (4.0 * 0.98 / 0.02);
    assert_eq!(p.torsion.state, TorsionState::Converged, "{:#?}", p.torsion);
    // Preselected 2% comparison to the thin-wall limit, not an exact thick-wall oracle.
    near(p.torsion.j_m4.unwrap(), bredt, 0.02 * bredt);
}

#[test]
fn nonconverged_thin_i_keeps_estimate_but_never_usable_torsion() {
    let (width, height, flange, web) = (0.2, 0.4, 0.005, 0.003);
    let p = analyse_section(&SectionShape::ISection {
        width_m: width,
        height_m: height,
        flange_m: flange,
        web_m: web,
    })
    .unwrap();
    let thin_plate = (2.0 * width * flange.powi(3) + (height - 2.0 * flange) * web.powi(3)) / 3.0;
    // A resource-limit fixture. Its estimate is plausible against the independent
    // thin-plate limit, but that agreement must never bypass mesh convergence.
    assert_eq!(
        p.torsion.state,
        TorsionState::NotConverged,
        "{:#?}",
        p.torsion
    );
    assert!(p.torsion.j_m4.is_none());
    assert!(!p.engineering_release);
    // The original 5% comparison includes the finite-thickness junction effect.
    near(p.torsion.estimate_m4, thin_plate, 0.05 * thin_plate);
}

#[test]
fn disconnected_compound_torsion_sums_independent_warping_energies() {
    let section = SectionShape::Compound {
        parts: vec![
            SectionPart {
                shape: Box::new(SectionShape::Rectangle {
                    width_m: 2.0,
                    height_m: 1.0,
                }),
                translation_m: [0.0, 0.0],
                rotation_rad: 0.0,
            },
            SectionPart {
                shape: Box::new(SectionShape::Rectangle {
                    width_m: 2.0,
                    height_m: 1.0,
                }),
                translation_m: [0.0, 3.0],
                rotation_rad: 0.0,
            },
        ],
    };
    let p = analyse_section(&section).unwrap();
    let reference = analyse_section(&SectionShape::Rectangle {
        width_m: 2.0,
        height_m: 1.0,
    })
    .unwrap();
    assert_eq!(p.torsion.state, TorsionState::Converged);
    near(
        p.torsion.j_m4.unwrap(),
        2.0 * reference.torsion.j_m4.unwrap(),
        0.005 * 2.0 * reference.torsion.j_m4.unwrap(),
    );
}

#[test]
fn geometry_rejects_impossible_dimensions_holes_and_crossing_faces() {
    for shape in [
        SectionShape::Rectangle {
            width_m: -1.0,
            height_m: 1.0,
        },
        SectionShape::Circle { diameter_m: 0.0 },
        SectionShape::ISection {
            width_m: 1.0,
            height_m: 1.0,
            flange_m: 0.6,
            web_m: 0.1,
        },
        SectionShape::Box {
            width_m: 1.0,
            height_m: 1.0,
            wall_m: 0.5,
        },
        SectionShape::Polygon {
            section: SectionInput {
                exterior_m: vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]],
                holes_m: vec![],
            },
        },
    ] {
        assert!(shape.geometry().is_err());
    }
    let outside = SectionShape::Polygon {
        section: SectionInput {
            exterior_m: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            holes_m: vec![vec![[2.0, 2.0], [3.0, 2.0], [3.0, 3.0], [2.0, 3.0]]],
        },
    };
    assert!(outside.geometry().is_err());
}

#[test]
fn imported_decimals_source_units_and_value_basis_are_preserved() {
    let precise = value(
        "210.123456789012345678901",
        PhysicalUnit::Gigapascal,
        PhysicalQuantity::ElasticModulus,
    );
    let entry = CatalogEntry {
        id: Uuid::new_v4(),
        revision: 1,
        name: "Precise source".into(),
        source: CatalogSource {
            original_record: Some("E=210.123456789012345678901 GPa".into()),
            ..source()
        },
        applicability: vec![],
        customization: None,
        payload: CatalogPayload::Properties {
            values: BTreeMap::from([("E".into(), precise.clone())]),
        },
    };
    let bytes = serde_json::to_vec(&entry).unwrap();
    let imported = CatalogEntry::from_json(&bytes).unwrap();
    assert_eq!(entry, imported);
    assert_eq!(entry.sha256().unwrap(), imported.sha256().unwrap());
    assert_eq!(precise.decimal, "210.123456789012345678901");
    near(precise.si().unwrap(), 210.12345678901235e9, 1e-4);
    let mut characteristic = value("30", PhysicalUnit::Megapascal, PhysicalQuantity::Stress);
    characteristic.basis = ValueBasis::Characteristic;
    assert!(CharacteristicValue::try_from(characteristic.clone()).is_ok());
    assert!(DesignValue::try_from(characteristic.clone()).is_err());
    characteristic.basis = ValueBasis::Design;
    assert!(characteristic.si().is_err());
    characteristic.design_basis =
        Some("Explicit input, method supplied by engineer; not calculated by this test".into());
    assert!(DesignValue::try_from(characteristic).is_ok());
}

#[test]
fn absolute_gauge_electrical_energy_and_thermal_quantities_do_not_mix() {
    assert!(
        value(
            "-1",
            PhysicalUnit::Pascal,
            PhysicalQuantity::AbsolutePressure
        )
        .si()
        .is_err()
    );
    near(
        value(
            "-1",
            PhysicalUnit::Kilopascal,
            PhysicalQuantity::GaugePressure,
        )
        .si()
        .unwrap(),
        -1000.0,
        0.0,
    );
    assert!(
        value("1", PhysicalUnit::Pascal, PhysicalQuantity::GaugePressure)
            .require(PhysicalQuantity::AbsolutePressure)
            .is_err()
    );
    near(
        value(
            "2",
            PhysicalUnit::LitrePerSecond,
            PhysicalQuantity::VolumeFlow,
        )
        .si()
        .unwrap(),
        0.002,
        1e-15,
    );
    near(
        value("2", PhysicalUnit::KilowattHour, PhysicalQuantity::Energy)
            .si()
            .unwrap(),
        7_200_000.0,
        1e-9,
    );
    assert!(
        value("2", PhysicalUnit::KilowattHour, PhysicalQuantity::Power)
            .si()
            .is_err()
    );
    near(
        value(
            "20",
            PhysicalUnit::DegreeCelsius,
            PhysicalQuantity::AbsoluteTemperature,
        )
        .si()
        .unwrap(),
        293.15,
        1e-12,
    );
    assert!(
        value(
            "20",
            PhysicalUnit::DegreeCelsius,
            PhysicalQuantity::TemperatureDifference
        )
        .si()
        .is_err()
    );
    assert!(
        value("1", PhysicalUnit::Volt, PhysicalQuantity::Current)
            .si()
            .is_err()
    );
    assert!(
        value(
            "1e-999",
            PhysicalUnit::Pascal,
            PhysicalQuantity::GaugePressure
        )
        .si()
        .is_err()
    );
    assert!(
        value("0", PhysicalUnit::Megapascal, PhysicalQuantity::Strength)
            .si()
            .is_err()
    );
    assert!(
        value("-1", PhysicalUnit::Megapascal, PhysicalQuantity::Strength)
            .si()
            .is_err()
    );
    near(
        value("-1", PhysicalUnit::Megapascal, PhysicalQuantity::Stress)
            .si()
            .unwrap(),
        -1e6,
        0.0,
    );
}

#[test]
fn import_retains_original_json_when_no_source_record_was_supplied() {
    let raw = serde_json::to_string(&rectangle_entry("Source precision", 0.2))
        .unwrap()
        .replace("\"width_m\":0.2", "\"width_m\":0.200000000000000000001");
    assert!(raw.contains("0.200000000000000000001"));
    let entry = CatalogEntry::from_json(raw.as_bytes()).unwrap();
    assert_eq!(entry.source.original_record.as_deref(), Some(raw.as_str()));
    let second = CatalogEntry::from_json(&serde_json::to_vec(&entry).unwrap()).unwrap();
    assert_eq!(entry, second);
    assert_eq!(entry.sha256().unwrap(), second.sha256().unwrap());
}

#[test]
fn material_entry_global_applicability_is_enforced_before_state_selection() {
    let mut entry = material_entry();
    entry.applicability = vec![ConditionRange {
        quantity: PhysicalQuantity::AbsoluteTemperature,
        minimum: value(
            "273",
            PhysicalUnit::Kelvin,
            PhysicalQuantity::AbsoluteTemperature,
        ),
        maximum: value(
            "310",
            PhysicalUnit::Kelvin,
            PhysicalQuantity::AbsoluteTemperature,
        ),
    }];
    assert!(entry.material_state_for(&BTreeMap::new()).is_err());
    assert!(
        entry
            .material_state_for(&BTreeMap::from([(
                PhysicalQuantity::AbsoluteTemperature,
                value(
                    "311",
                    PhysicalUnit::Kelvin,
                    PhysicalQuantity::AbsoluteTemperature
                )
            )]))
            .is_err()
    );
    assert_eq!(
        entry
            .material_state_for(&BTreeMap::from([(
                PhysicalQuantity::AbsoluteTemperature,
                value(
                    "300",
                    PhysicalUnit::Kelvin,
                    PhysicalQuantity::AbsoluteTemperature
                )
            )]))
            .unwrap()
            .name,
        "reference"
    );
}

#[test]
fn catalogue_commands_and_transactions_preserve_scientific_numbers_through_tagged_json() {
    use prumo_domain::{Command, transactions::TransactionRequest};
    let mut entries = vec![rectangle_entry("Small rectangle", 2e-7), material_entry()];
    let mut compound = rectangle_entry("Compound", 0.1);
    compound.payload = CatalogPayload::Section {
        shape: SectionShape::Compound {
            parts: vec![SectionPart {
                shape: Box::new(SectionShape::Polygon {
                    section: SectionInput {
                        exterior_m: vec![[0.0, 0.0], [2e-7, 0.0], [0.0, 3e-7]],
                        holes_m: vec![],
                    },
                }),
                translation_m: [1e-5, -2e-5],
                rotation_rad: 2e-3,
            }],
        },
    };
    entries.push(compound);
    let mut orthotropic = material_entry();
    if let CatalogPayload::Material { material } = &mut orthotropic.payload {
        let e = value(
            "20.1234567890123456789",
            PhysicalUnit::Gigapascal,
            PhysicalQuantity::ElasticModulus,
        );
        let g = value(
            "7",
            PhysicalUnit::Gigapascal,
            PhysicalQuantity::ElasticModulus,
        );
        material.states[0].elastic = ElasticLaw::Orthotropic {
            young_moduli: Box::new([e.clone(), e.clone(), e]),
            shear_moduli: Box::new([g.clone(), g.clone(), g]),
            poisson_12_23_31: [3e-1, 2e-1, 1e-1],
        };
    }
    entries.push(orthotropic);
    let project = empty();
    let request = TransactionRequest::new(
        &project,
        entries
            .iter()
            .cloned()
            .map(|entry| Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::PutCatalogEntry { entry },
            })
            .collect(),
    );
    let raw = serde_json::to_string(&request)
        .unwrap()
        .replace("\"poisson_ratio\":0.3", "\"poisson_ratio\":3e-1")
        .replace("\"rotation_rad\":0.002", "\"rotation_rad\":2e-3");
    assert!(raw.contains("3e-1"));
    assert!(raw.contains("2e-7"));
    let decoded: TransactionRequest = serde_json::from_str(&raw).unwrap();
    let from_value: TransactionRequest =
        serde_json::from_value(serde_json::from_str(&raw).unwrap()).unwrap();
    for result in [decoded, from_value] {
        for (command, expected) in result.operations.into_iter().zip(&entries) {
            match command {
                Command::EditModel {
                    edit: ModelEdit::PutCatalogEntry { entry },
                    ..
                } => {
                    assert_eq!(&entry, expected);
                    entry.validate().unwrap();
                }
                _ => panic!("Wrong command after JSON decoding"),
            }
        }
    }
}

#[test]
fn polygon_inertia_and_torsion_are_invariant_to_translation_and_winding() {
    let base = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]];
    let shape = SectionShape::Polygon {
        section: SectionInput {
            exterior_m: base.clone(),
            holes_m: vec![],
        },
    };
    let reference = analyse_section(&shape).unwrap();
    let mut shifted: Vec<_> = base.iter().map(|[x, y]| [x + 1e8, y - 1e8]).collect();
    shifted.reverse();
    let translated = analyse_section(&SectionShape::Polygon {
        section: SectionInput {
            exterior_m: shifted,
            holes_m: vec![],
        },
    })
    .unwrap();
    near(
        translated.geometry.area_m2,
        reference.geometry.area_m2,
        1e-12,
    );
    near(translated.geometry.ixx_m4, reference.geometry.ixx_m4, 1e-12);
    near(
        translated.torsion.j_m4.unwrap(),
        reference.torsion.j_m4.unwrap(),
        1e-8,
    );
}

#[test]
fn isotropic_and_orthotropic_materials_reject_nonpositive_energy() {
    let mut material = isotropic();
    material.validate().unwrap();
    if let ElasticLaw::Isotropic { poisson_ratio, .. } = &mut material.states[0].elastic {
        *poisson_ratio = 0.5;
    }
    assert!(material.validate().is_err());
    let e = value(
        "200",
        PhysicalUnit::Gigapascal,
        PhysicalQuantity::ElasticModulus,
    );
    let g = value(
        "80",
        PhysicalUnit::Gigapascal,
        PhysicalQuantity::ElasticModulus,
    );
    let mut law = ElasticLaw::Orthotropic {
        young_moduli: Box::new([e.clone(), e.clone(), e]),
        shear_moduli: Box::new([g.clone(), g.clone(), g]),
        poisson_12_23_31: [0.25; 3],
    };
    law.validate().unwrap();
    if let ElasticLaw::Orthotropic {
        poisson_12_23_31, ..
    } = &mut law
    {
        *poisson_12_23_31 = [0.9; 3];
    }
    assert!(law.validate().is_err());
}

#[test]
fn condition_selection_requires_complete_unambiguous_applicability() {
    let mut material = isotropic();
    let condition = |min: &str, max: &str| ConditionRange {
        quantity: PhysicalQuantity::AbsoluteTemperature,
        minimum: value(
            min,
            PhysicalUnit::Kelvin,
            PhysicalQuantity::AbsoluteTemperature,
        ),
        maximum: value(
            max,
            PhysicalUnit::Kelvin,
            PhysicalQuantity::AbsoluteTemperature,
        ),
    };
    material.states[0].conditions = vec![condition("270", "300")];
    let mut hot = material.states[0].clone();
    hot.name = "hot".into();
    hot.conditions = vec![condition("310", "350")];
    material.states.push(hot);
    let conditions = |temp: &str| {
        BTreeMap::from([(
            PhysicalQuantity::AbsoluteTemperature,
            value(
                temp,
                PhysicalUnit::Kelvin,
                PhysicalQuantity::AbsoluteTemperature,
            ),
        )])
    };
    assert_eq!(
        material.state_for(&conditions("290")).unwrap().name,
        "reference"
    );
    assert_eq!(material.state_for(&conditions("330")).unwrap().name, "hot");
    assert!(material.state_for(&conditions("305")).is_err());
    assert!(material.state_for(&BTreeMap::new()).is_err());
    material.states[1].conditions = vec![condition("290", "350")];
    assert!(material.state_for(&conditions("295")).is_err());
}

#[test]
fn condition_selection_rejects_invalid_extra_values_even_without_a_matching_range() {
    let material = isotropic();
    let wrong_unit = value("3", PhysicalUnit::Volt, PhysicalQuantity::Current);
    assert!(
        material
            .state_for(&BTreeMap::from([(PhysicalQuantity::Current, wrong_unit)]))
            .is_err()
    );
    let wrong_quantity = value("3", PhysicalUnit::Volt, PhysicalQuantity::Voltage);
    assert!(
        material
            .state_for(&BTreeMap::from([(
                PhysicalQuantity::Current,
                wrong_quantity
            )]))
            .is_err()
    );
}

fn record_for(project: &mut Project, entry_id: Uuid) -> Uuid {
    let snapshot = prumo_domain::canonical::model_snapshot(project).unwrap();
    let index = prumo_domain::canonical::source_index_unchecked(project).unwrap();
    project
        .dependencies
        .snapshots
        .entry(snapshot.sha256.clone())
        .or_insert_with(|| snapshot.clone());
    let id = project
        .dependencies
        .record(
            "Catalogue-dependent fixture".into(),
            "catalogue-test".into(),
            project.revision,
            project.revision + 1,
            snapshot.sha256,
            vec![
                Dependency::source(
                    SourceKey {
                        kind: SourceKind::CatalogEntry,
                        id: entry_id,
                    },
                    &index,
                )
                .unwrap(),
            ],
            InspectionPayload::ExternalEvidence {
                artifact_sha256: "1".repeat(64),
                media_type: "application/json".into(),
            },
        )
        .unwrap();
    stamp(project);
    id
}

#[test]
fn catalogue_revision_invalidates_only_affected_results_and_retains_history() {
    let mut project = empty();
    let a = rectangle_entry("A", 0.2);
    let b = rectangle_entry("B", 0.3);
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry { entry: a.clone() },
    );
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry { entry: b.clone() },
    );
    let ra = record_for(&mut project, a.id);
    let rb = record_for(&mut project, b.id);
    let before = prumo_domain::canonical::source_index_unchecked(&project).unwrap();
    let mut revised = a.clone();
    revised.revision = 2;
    revised.source.revision = "2".into();
    revised.payload = CatalogPayload::Section {
        shape: SectionShape::Rectangle {
            width_m: 0.25,
            height_m: 0.4,
        },
    };
    apply(&mut project, ModelEdit::PutCatalogEntry { entry: revised });
    let after = prumo_domain::canonical::source_index_unchecked(&project).unwrap();
    let metadata = SourceKey {
        kind: SourceKind::ProjectMetadata,
        id: project.id,
    };
    assert_eq!(before[&metadata], after[&metadata]);
    assert_eq!(
        project
            .dependencies
            .results
            .iter()
            .find(|r| r.id == ra)
            .unwrap()
            .state,
        ResultState::Stale
    );
    assert_eq!(
        project
            .dependencies
            .results
            .iter()
            .find(|r| r.id == rb)
            .unwrap()
            .state,
        ResultState::Current
    );
    let catalogue = project_catalogue(&project).unwrap();
    assert_eq!(catalogue.entries[&a.id].len(), 2);
    assert_eq!(catalogue.entries[&a.id][0].entry, a);
    let snapshot = prumo_domain::canonical::model_snapshot(&project).unwrap();
    assert!(snapshot.canonical_json.contains(CATALOGUE_KEY));
}

#[test]
fn stale_revision_invalid_history_and_unknown_assignment_cannot_mutate_project() {
    let mut project = empty();
    let entry = rectangle_entry("A", 0.2);
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry.clone(),
        },
    );
    let before = serde_json::to_string(&project).unwrap();
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::PutCatalogEntry { entry },
            "fixture"
        )
        .is_err()
    );
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::AssignCatalogEntry {
                object_id: Uuid::new_v4(),
                entry_id: Uuid::new_v4()
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_string(&project).unwrap());
    let mut catalogue = project_catalogue(&project).unwrap();
    catalogue.entries.values_mut().next().unwrap()[0].entry_sha256 = "0".repeat(64);
    project.extensions.insert(
        CATALOGUE_KEY.into(),
        serde_json::to_value(catalogue).unwrap(),
    );
    assert!(project.validate().is_err());
}

#[test]
fn customization_creates_new_identity_and_preserves_the_catalogue_source() {
    let mut project = empty();
    let original = rectangle_entry("Source", 0.2);
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: original.clone(),
        },
    );
    let mut custom = rectangle_entry("Explicit customization", 0.22);
    custom.customization = Some(CatalogCustomization {
        base_id: original.id,
        base_revision: 1,
        base_sha256: original.sha256().unwrap(),
        reason: "Project-specific width, original source retained".into(),
    });
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: custom.clone(),
        },
    );
    let catalogue = project_catalogue(&project).unwrap();
    assert_eq!(catalogue.latest(original.id).unwrap(), &original);
    assert_eq!(catalogue.latest(custom.id).unwrap(), &custom);
    let mut bad = custom;
    bad.id = original.id;
    assert!(bad.validate().is_err());
}

#[test]
fn legacy_and_catalogue_materials_cannot_both_be_effective() {
    let mut project = empty();
    let object = physical(&mut project, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let entry = material_entry();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry.clone(),
        },
    );
    apply(
        &mut project,
        ModelEdit::AddMaterial {
            material: MaterialInput {
                name: "Legacy".into(),
                family: MaterialFamily::Generic,
                elastic_modulus: PressureInput {
                    value: 200.0,
                    unit: PressureUnit::Gigapascal,
                },
                density: DensityInput {
                    value: 7850.0,
                    unit: DensityUnit::KilogramPerCubicMetre,
                },
                poisson_ratio: 0.3,
            },
        },
    );
    let legacy = project.model.materials[0].id;
    apply(
        &mut project,
        ModelEdit::SetPhysicalMaterial {
            object_id: object,
            material_id: Some(legacy),
        },
    );
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::AssignCatalogEntry {
                object_id: object,
                entry_id: entry.id
            },
            "fixture"
        )
        .is_err()
    );
    apply(
        &mut project,
        ModelEdit::SetPhysicalMaterial {
            object_id: object,
            material_id: None,
        },
    );
    apply(
        &mut project,
        ModelEdit::AssignCatalogEntry {
            object_id: object,
            entry_id: entry.id,
        },
    );
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::SetPhysicalMaterial {
                object_id: object,
                material_id: Some(legacy)
            },
            "fixture"
        )
        .is_err()
    );
    apply(
        &mut project,
        ModelEdit::ClearCatalogAssignment {
            object_id: object,
            kind: CatalogKind::Material,
        },
    );
    apply(
        &mut project,
        ModelEdit::SetPhysicalMaterial {
            object_id: object,
            material_id: Some(legacy),
        },
    );
}

#[test]
fn merge_rejects_different_catalogue_assignments() {
    let mut project = empty();
    let a = physical(&mut project, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let b = physical(&mut project, [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
    let ca = rectangle_entry("A", 0.2);
    let cb = rectangle_entry("B", 0.3);
    for (object, entry) in [(a, ca), (b, cb)] {
        let id = entry.id;
        apply(&mut project, ModelEdit::PutCatalogEntry { entry });
        apply(
            &mut project,
            ModelEdit::AssignCatalogEntry {
                object_id: object,
                entry_id: id,
            },
        );
    }
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::MergePhysical {
                source_ids: vec![a, b],
                name: "merged".into()
            },
            "fixture"
        )
        .is_err()
    );
}
