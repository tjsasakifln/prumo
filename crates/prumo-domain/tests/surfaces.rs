use prumo_domain::model::{
    MaterialFamily, MaterialInput, ModelEdit, PhysicalRole, apply_model_edit,
};
use prumo_domain::surfaces::*;
use prumo_domain::units::{DensityInput, DensityUnit, Point3, PressureInput, PressureUnit};
use prumo_domain::{ChangeRecord, LengthUnit, PROJECT_SCHEMA_VERSION, Project};
use std::collections::BTreeMap;
use uuid::Uuid;

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual:.16e}, expected {expected:.16e}"
    );
}
fn fixture() -> Project {
    let mut p = Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Surface fixture, geometry only".into(),
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
    };
    edit(
        &mut p,
        ModelEdit::AddMaterial {
            material: MaterialInput {
                name: "Concrete geometric fixture".into(),
                family: MaterialFamily::Concrete,
                elastic_modulus: PressureInput {
                    value: 30.,
                    unit: PressureUnit::Gigapascal,
                },
                density: DensityInput {
                    value: 2500.,
                    unit: DensityUnit::KilogramPerCubicMetre,
                },
                poisson_ratio: 0.2,
            },
        },
    );
    p
}
fn edit(p: &mut Project, edit: ModelEdit) {
    apply_model_edit(p, edit, "fixture").unwrap();
    p.revision += 1;
    p.history.push(ChangeRecord {
        revision: p.revision,
        actor: "fixture".into(),
        operation: "model.edit".into(),
        timestamp_unix_ms: 1,
    });
    p.validate().unwrap();
}
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<LocalPoint> {
    vec![
        LocalPoint::new(x, y),
        LocalPoint::new(x + w, y),
        LocalPoint::new(x + w, y + h),
        LocalPoint::new(x, y + h),
    ]
}
fn slab(p: &Project) -> SurfaceSpec {
    SurfaceSpec {
        name: "Slab fixture".into(),
        role: PhysicalRole::AnalyticalSource,
        material_id: Some(p.model.materials[0].id),
        environment_id: None,
        frame: SurfaceFrame::default(),
        kind: SurfaceKind::SolidSlab {},
        outline: rectangle(0., 0., 8., 8.),
        openings: vec![],
        thickness_m: 0.2,
        regions: vec![],
        supports: vec![],
        edge_conditions: vec![],
    }
}

#[test]
fn concave_slab_and_multiple_openings_conserve_material_area_volume_and_mass() {
    let mut p = fixture();
    let mut s = slab(&p);
    s.outline = vec![
        LocalPoint::new(0., 0.),
        LocalPoint::new(8., 0.),
        LocalPoint::new(8., 4.),
        LocalPoint::new(4., 4.),
        LocalPoint::new(4., 8.),
        LocalPoint::new(0., 8.),
    ];
    s.openings = vec![
        SurfaceOpening {
            name: "Opening 1".into(),
            outline: rectangle(1., 1., 1., 1.),
        },
        SurfaceOpening {
            name: "Opening 2".into(),
            outline: rectangle(5., 1., 1., 2.),
        },
    ];
    let r = preview(&p, &s).unwrap();
    near(r.net_area_m2, 48. - 1. - 2., 1e-10);
    near(r.quantities.total_volume_m3, 9., 1e-10);
    near(r.quantities.total_mass_kg.unwrap(), 22_500., 1e-7);
    let triangle_area: f64 = r
        .patches
        .iter()
        .flat_map(|p| &p.triangles_m)
        .map(|t| {
            ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]))
                * 0.5
        })
        .sum();
    near(triangle_area, 45., 1e-9);
    for t in r.patches.iter().flat_map(|p| &p.triangles_m) {
        let c = [
            (t[0][0] + t[1][0] + t[2][0]) / 3.,
            (t[0][1] + t[1][1] + t[2][1]) / 3.,
        ];
        assert!(!(c[0] > 1. && c[0] < 2. && c[1] > 1. && c[1] < 2.));
        assert!(!(c[0] > 5. && c[0] < 6. && c[1] > 1. && c[1] < 3.));
    }
    let mut covered = s.clone();
    covered.regions.push(ThicknessRegion {
        name: "Uniform replacement with preserved holes".into(),
        outline: covered.outline.clone(),
        thickness_m: 0.3,
        offset_m: 0.,
    });
    let replaced = preview(&p, &covered).unwrap();
    assert_eq!(replaced.patches.len(), 1);
    near(replaced.patches[0].area_m2, 45., 1e-10);
    near(replaced.quantities.total_volume_m3, 13.5, 1e-10);
    assert_eq!(replaced.patches[0].triangles_m, r.patches[0].triangles_m);
    edit(
        &mut p,
        ModelEdit::EditSurfaces {
            command: SurfaceEdit::Create { surface: s },
        },
    );
    let inspected = inspect(&p, p.objects[0].id).unwrap();
    assert!(inspected.region_id.is_some());
    assert!(!inspected.preview.engineering_release);
}

#[test]
fn sloped_surface_and_dropped_thickness_region_preserve_elevations_and_units() {
    let p = fixture();
    let mut s = slab(&p);
    s.frame = SurfaceFrame {
        origin_m: Point3::try_from([100_000_000., 200_000_000., 5.]).unwrap(),
        u: [0.8, 0., 0.6],
        v: [0., 1., 0.],
    };
    s.outline = rectangle(0., 0., 4., 3.);
    s.openings.push(SurfaceOpening {
        name: "Opening".into(),
        outline: rectangle(2., 1., 1., 1.),
    });
    s.regions.push(ThicknessRegion {
        name: "Drop panel".into(),
        outline: rectangle(0., 0., 1., 1.),
        thickness_m: 0.4,
        offset_m: -0.1,
    });
    let r = preview(&p, &s).unwrap();
    near(r.net_area_m2, 11., 1e-8);
    near(r.quantities.total_volume_m3, 2.4, 1e-8);
    let drop = r
        .patches
        .iter()
        .find(|x| x.component == "Drop panel")
        .unwrap();
    for p in drop.triangles_m.iter().flatten() {
        let xlocal = (p[0] - 100_000_000. - 0.06) / 0.8;
        near(p[2], 5. + 0.6 * xlocal - 0.08, 2e-8);
    }
}

#[test]
fn ribs_count_topping_and_crossings_once_with_independent_rectangle_oracle() {
    let p = fixture();
    let mut s = slab(&p);
    s.outline = rectangle(0., 0., 4., 4.);
    s.thickness_m = 0.1;
    s.kind = SurfaceKind::Ribbed {
        rib_width_m: 0.2,
        spacing_x_m: 2.,
        spacing_y_m: Some(2.),
        extra_depth_m: 0.3,
    };
    let r = preview(&p, &s).unwrap();
    let footprint = 4. * 0.4 + 4. * 0.4 - 0.4 * 0.4;
    near(
        r.quantities.total_volume_m3,
        16. * 0.1 + footprint * 0.3,
        2e-7,
    );
    near(
        r.quantities
            .components
            .iter()
            .filter(|c| c.component.starts_with("Nervura"))
            .map(|c| c.area_m2)
            .sum::<f64>(),
        footprint,
        2e-7,
    );
    s.kind = SurfaceKind::Grillage {
        rib_width_m: 0.2,
        spacing_x_m: 2.,
        spacing_y_m: 2.,
        depth_m: 0.4,
    };
    s.thickness_m = 0.;
    let grid = preview(&p, &s).unwrap();
    near(grid.quantities.total_volume_m3, footprint * 0.4, 2e-7);
    s.openings.push(SurfaceOpening {
        name: "Central crossing removed".into(),
        outline: rectangle(1.9, 1.9, 0.2, 0.2),
    });
    let cut = preview(&p, &s).unwrap();
    near(
        cut.quantities.total_volume_m3,
        (footprint - 0.04) * 0.4,
        3e-7,
    );
}

#[test]
fn edits_keep_identities_and_rebuild_region_while_generic_changes_are_rejected() {
    let mut p = fixture();
    let s = slab(&p);
    edit(
        &mut p,
        ModelEdit::EditSurfaces {
            command: SurfaceEdit::Create { surface: s },
        },
    );
    let id = p.objects[0].id;
    let first = inspect(&p, id).unwrap();
    let source = prumo_domain::canonical::model_snapshot(&p).unwrap().sha256;
    edit(
        &mut p,
        ModelEdit::EditSurfaces {
            command: SurfaceEdit::SetOpenings {
                object_id: id,
                openings: vec![SurfaceOpening {
                    name: "New opening".into(),
                    outline: rectangle(1., 1., 2., 2.),
                }],
            },
        },
    );
    let after = inspect(&p, id).unwrap();
    assert_eq!(after.region_id, first.region_id);
    near(after.preview.net_area_m2, 60., 1e-9);
    assert_ne!(
        source,
        prumo_domain::canonical::model_snapshot(&p).unwrap().sha256
    );
    let mut bad = p.clone();
    bad.objects[0].end_m = Point3::try_from([9., 9., 0.]).unwrap();
    assert!(bad.validate().is_err());
    let mut copied = p.clone();
    let mut object = copied.objects[0].clone();
    object.id = Uuid::new_v4();
    copied.objects.push(object);
    assert!(validate_project(&copied).is_err());
}

#[test]
fn invalid_topology_normals_thickness_and_unknown_material_are_diagnosed() {
    let p = fixture();
    let s = slab(&p);
    let mut bad = s.clone();
    bad.outline.reverse();
    assert!(preview(&p, &bad).is_err());
    let mut bad = s.clone();
    bad.thickness_m = f64::NAN;
    assert!(preview(&p, &bad).is_err());
    let mut bad = s.clone();
    bad.material_id = None;
    assert!(preview(&p, &bad).is_err());
    let mut bad = s.clone();
    bad.openings = vec![
        SurfaceOpening {
            name: "A".into(),
            outline: rectangle(1., 1., 3., 3.),
        },
        SurfaceOpening {
            name: "B".into(),
            outline: rectangle(2., 2., 3., 3.),
        },
    ];
    assert!(preview(&p, &bad).is_err());
    let mut bad = s.clone();
    bad.openings = vec![SurfaceOpening {
        name: "Outside".into(),
        outline: rectangle(7., 7., 2., 2.),
    }];
    assert!(preview(&p, &bad).is_err());
    let mut bad = s;
    bad.regions = vec![
        ThicknessRegion {
            name: "A".into(),
            outline: rectangle(1., 1., 2., 2.),
            thickness_m: 0.4,
            offset_m: 0.,
        },
        ThicknessRegion {
            name: "B".into(),
            outline: rectangle(2., 2., 2., 2.),
            thickness_m: 0.3,
            offset_m: 0.,
        },
    ];
    assert!(preview(&p, &bad).is_err());
}

#[test]
fn opening_at_a_declared_column_reports_missing_contact_without_synthetic_support() {
    let mut p = fixture();
    let mat = p.model.materials[0].id;
    edit(
        &mut p,
        ModelEdit::AddPhysical {
            name: "Column".into(),
            discipline: prumo_domain::Discipline::Structure,
            start: prumo_domain::units::PointInput {
                values: [2., 2., -3.],
                unit: LengthUnit::Metre,
            },
            end: prumo_domain::units::PointInput {
                values: [2., 2., 0.],
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::AnalyticalSource,
            material_id: Some(mat),
            environment_id: None,
        },
    );
    let mut s = slab(&p);
    s.supports.push(SurfaceSupport {
        object_id: p.objects[0].id,
        point: LocalPoint::new(2., 2.),
    });
    s.openings.push(SurfaceOpening {
        name: "Opening at column".into(),
        outline: rectangle(1., 1., 2., 2.),
    });
    let r = preview(&p, &s).unwrap();
    assert!(!r.supports[0].material_present);
    assert!(r.punching_review_required && !r.warnings.is_empty());
    assert!(!r.engineering_release);
}

#[test]
fn tagged_surface_commands_preserve_scientific_numbers_and_reject_underflow() {
    let p = fixture();
    let mut s = slab(&p);
    s.thickness_m = 1e-3;
    let command = ModelEdit::EditSurfaces {
        command: SurfaceEdit::Create { surface: s },
    };
    let text = serde_json::to_string(&command).unwrap();
    let parsed: ModelEdit = serde_json::from_value(serde_json::from_str(&text).unwrap()).unwrap();
    assert_eq!(text, serde_json::to_string(&parsed).unwrap());
    let malicious = text.replace("\"thickness_m\":0.001", "\"thickness_m\":1e-9999");
    assert!(serde_json::from_str::<ModelEdit>(&malicious).is_err());
}

#[test]
fn rib_spacing_extremes_return_diagnostic_without_panicking() {
    let p = fixture();
    let mut s = slab(&p);
    s.kind = SurfaceKind::Ribbed {
        rib_width_m: 1e-301,
        spacing_x_m: 1e-300,
        spacing_y_m: None,
        extra_depth_m: 0.3,
    };
    let value = serde_json::to_value(&s).unwrap();
    let received: SurfaceSpec = serde_json::from_value(value).unwrap();
    let result = std::panic::catch_unwind(|| preview(&p, &received));
    assert!(result.is_ok(), "Numeric range must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn drop_panel_and_ribs_partition_overlapping_volume_once() {
    let p = fixture();
    let mut s = slab(&p);
    s.outline = rectangle(0., 0., 4., 4.);
    s.thickness_m = 0.1;
    s.kind = SurfaceKind::Ribbed {
        rib_width_m: 0.2,
        spacing_x_m: 2.,
        spacing_y_m: Some(2.),
        extra_depth_m: 0.3,
    };
    s.regions.push(ThicknessRegion {
        name: "Drop panel replaces topping and underlying ribs".into(),
        outline: rectangle(1., 1., 2., 2.),
        thickness_m: 0.5,
        offset_m: -0.2,
    });
    let report = preview(&p, &s).unwrap();
    // Rectangle union oracle: topping + ribs + drop replacing topping - ribs
    // already inside the drop. Central rib footprint is 2*.2*2 - .2*.2.
    let expected = 16. * 0.1 + 3.04 * 0.3 + 4. * (0.5 - 0.1) - 0.76 * 0.3;
    near(report.quantities.total_volume_m3, expected, 3e-7);
}

#[test]
fn grillage_void_cannot_receive_material_contact_and_edges_are_explicit() {
    let mut p = fixture();
    let mat = p.model.materials[0].id;
    edit(
        &mut p,
        ModelEdit::AddPhysical {
            name: "Candidate support".into(),
            discipline: prumo_domain::Discipline::Structure,
            start: prumo_domain::units::PointInput {
                values: [1., 1., -2.],
                unit: LengthUnit::Metre,
            },
            end: prumo_domain::units::PointInput {
                values: [1., 1., 0.],
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::AnalyticalSource,
            material_id: Some(mat),
            environment_id: None,
        },
    );
    let id = p.objects[0].id;
    let mut s = slab(&p);
    s.outline = rectangle(0., 0., 4., 4.);
    s.thickness_m = 0.;
    s.kind = SurfaceKind::Grillage {
        rib_width_m: 0.2,
        spacing_x_m: 2.,
        spacing_y_m: 2.,
        depth_m: 0.4,
    };
    s.supports.push(SurfaceSupport {
        object_id: id,
        point: LocalPoint::new(1., 1.),
    });
    let empty = preview(&p, &s).unwrap();
    assert!(!empty.supports[0].material_present);
    assert!(
        empty
            .boundaries
            .iter()
            .all(|edge| matches!(edge.behavior, EdgeBehavior::Free {}))
    );
    s.edge_conditions.push(EdgeCondition {
        edge: 0,
        behavior: EdgeBehavior::CandidateSupport { object_id: id },
    });
    let explicit = preview(&p, &s).unwrap();
    assert!(
        matches!(explicit.boundaries[0].behavior, EdgeBehavior::CandidateSupport{object_id} if object_id==id)
    );
    assert!(
        explicit.boundaries[1..]
            .iter()
            .all(|edge| matches!(edge.behavior, EdgeBehavior::Free {}))
    );
    edit(
        &mut p,
        ModelEdit::EditSurfaces {
            command: SurfaceEdit::Create { surface: s.clone() },
        },
    );
    let created = p
        .objects
        .iter()
        .find(|object| metadata(object).unwrap().is_some())
        .unwrap()
        .id;
    let report = inspect(&p, created).unwrap();
    use prumo_domain::dependencies::{SourceKey, SourceKind};
    for source in [
        SourceKey {
            kind: SourceKind::Physical,
            id,
        },
        SourceKey {
            kind: SourceKind::PhysicalGeometry,
            id,
        },
        SourceKey {
            kind: SourceKind::Material,
            id: mat,
        },
        SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: report.region_id.unwrap(),
        },
        SourceKey {
            kind: SourceKind::Coordinates,
            id: p.id,
        },
    ] {
        assert!(report.sources.contains(&source), "missing {source:?}");
    }
    s.edge_conditions.push(s.edge_conditions[0].clone());
    assert!(preview(&p, &s).is_err());
}
