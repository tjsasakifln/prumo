use prumo_domain::assemblies::*;
use prumo_domain::model::{
    MaterialFamily, MaterialInput, ModelEdit, PhysicalRole, apply_model_edit,
};
use prumo_domain::surfaces::{LocalPoint, SurfaceEdit, SurfaceFrame, SurfaceOpening};
use prumo_domain::units::{DensityInput, DensityUnit, Point3, PressureInput, PressureUnit};
use prumo_domain::{ChangeRecord, LengthUnit, PROJECT_SCHEMA_VERSION, Project};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual}, expected={expected}"
    );
}
fn point(v: [f64; 3]) -> Point3 {
    Point3::try_from(v).unwrap()
}
fn rectangle(w: f64, h: f64) -> Vec<LocalPoint> {
    vec![
        LocalPoint::new(0., 0.),
        LocalPoint::new(w, 0.),
        LocalPoint::new(w, h),
        LocalPoint::new(0., h),
    ]
}
fn fixture() -> Project {
    let mut p = Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Assembly fixture, not engineering approval".into(),
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
                name: "Concrete fixture".into(),
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
fn stair(p: &Project, form: StairForm) -> AssemblySpec {
    AssemblySpec {
        name: "Stair fixture".into(),
        role: PhysicalRole::AnalyticalSource,
        material_id: Some(p.model.materials[0].id),
        environment_id: None,
        shape: AssemblyShape::Stair {
            form,
            flights: vec![StairFlight {
                name: "Flight".into(),
                start_m: Point3::ORIGIN,
                end_m: point([3., 0., 3.]),
                width_m: 1.,
                thickness_m: 0.15,
                steps: 10,
            }],
            landings: vec![Landing {
                name: "Top landing".into(),
                frame: SurfaceFrame {
                    origin_m: point([3., 0., 3.]),
                    ..Default::default()
                },
                outline: rectangle(1., 1.),
                thickness_m: 0.15,
            }],
        },
        supports: vec![],
        continuities: vec![],
    }
}
fn assembly_id(p: &Project) -> Uuid {
    p.objects
        .iter()
        .find(|o| metadata(o).unwrap().is_some())
        .unwrap()
        .id
}

#[test]
fn stair_between_floors_has_no_implicit_links_and_exposes_declared_continuity() {
    let mut p = fixture();
    let mut s = stair(&p, StairForm::Conventional);
    let before = preview(&p, &s).unwrap();
    assert!(before.declared_continuities.is_empty());
    assert!(before.warnings.iter().any(|w| w.contains("continuidade")));
    s.continuities = proposed_continuities(&p, &s).unwrap();
    assert_eq!(s.continuities.len(), 1);
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let r = inspect(&p, assembly_id(&p)).unwrap();
    assert_eq!(r.parts.len(), 2);
    assert_eq!(r.continuities.len(), 1);
    assert!(r.regions.iter().all(|r| r.region_id.is_some()));
    assert!(!r.preview.engineering_release);
    near(r.preview.additional_step_volumes[0].volume_m3, 0.45, 1e-12);
    let stair_normal = r.regions[0].surface.frame.normal();
    near(stair_normal[0], -1. / 2f64.sqrt(), 1e-12);
    near(stair_normal[2], 1. / 2f64.sqrt(), 1e-12);
}

#[test]
fn waist_and_step_wedges_have_disjoint_interiors_in_longitudinal_section() {
    let p = fixture();
    let s = stair(&p, StairForm::Conventional);
    let report = preview(&p, &s).unwrap();
    let waist = prumo_domain::surfaces::preview(&p, &report.parts[0].surface).unwrap();
    // Independent section: z = x is the upper waist plane; its upward normal
    // is (-1, 0, 1)/sqrt(2). The concrete waist lies entirely below it.
    for patch in &waist.patches {
        near(patch.offset_m, -0.075, 1e-12);
        near(patch.offset_m + 0.5 * patch.thickness_m, 0., 1e-12);
        for triangle in &patch.triangles_m {
            for p in triangle {
                near((p[2] - p[0]) / 2f64.sqrt(), -0.075, 1e-12);
            }
        }
    }
    // Each triangular filling wedge has nonnegative signed distance from
    // z = x, with strictly positive distance at its centroid.
    let mut wedge_volume = 0.;
    for i in 0..10 {
        let x = f64::from(i) * 0.3;
        let triangle = [[x, x], [x, x + 0.3], [x + 0.3, x + 0.3]];
        for [x, z] in triangle {
            assert!(z >= x);
        }
        let signed_centroid = triangle
            .iter()
            .map(|[x, z]| (z - x) / 2f64.sqrt())
            .sum::<f64>()
            / 3.;
        assert!(signed_centroid > 0.);
        wedge_volume += 0.5 * 0.3 * 0.3 * 1.;
    }
    near(
        report.additional_step_volumes[0].volume_m3,
        wedge_volume,
        1e-12,
    );
    near(
        waist.quantities.total_volume_m3,
        3. * 2f64.sqrt() * 0.15,
        1e-12,
    );
    let landing = prumo_domain::surfaces::preview(&p, &report.parts[1].surface).unwrap();
    assert!(
        landing
            .patches
            .iter()
            .all(|patch| patch.offset_m + 0.5 * patch.thickness_m <= 1e-12)
    );
}

#[test]
fn part_expansion_and_edge_search_are_bounded_before_geometric_work() {
    let p = fixture();
    let mut oversized = stair(&p, StairForm::Folded);
    if let AssemblyShape::Stair { flights, .. } = &mut oversized.shape {
        flights[0].steps = 128;
    }
    // 256 tread/riser faces plus the landing exceed the whole-assembly bound.
    assert!(matches!(
        preview(&p, &oversized),
        Err(prumo_domain::DomainError::ModelLimit)
    ));
    let circle = (0..512)
        .map(|i| {
            let angle = f64::from(i) * std::f64::consts::TAU / 512.;
            LocalPoint::new(angle.cos(), angle.sin())
        })
        .collect::<Vec<_>>();
    let mut bounded = stair(&p, StairForm::Conventional);
    bounded.shape = AssemblyShape::Core {
        panels: (0..4)
            .map(|i| WallPanel {
                name: format!("Detailed panel {i}"),
                frame: SurfaceFrame {
                    origin_m: point([0., 0., f64::from(i)]),
                    ..Default::default()
                },
                outline: circle.clone(),
                openings: vec![],
                thickness_m: 0.2,
            })
            .collect(),
        coupling_beams: vec![],
    };
    let error = proposed_continuities(&p, &bounded).unwrap_err();
    assert!(error.to_string().contains("milhão de pares"));
}

#[test]
fn cantilever_support_is_explicit_inspectable_and_does_not_imply_solver_result() {
    let mut p = fixture();
    let mat = p.model.materials[0].id;
    edit(
        &mut p,
        ModelEdit::AddPhysical {
            name: "Supporting wall reference".into(),
            discipline: prumo_domain::Discipline::Structure,
            start: prumo_domain::units::PointInput {
                values: [0., 0., -3.],
                unit: LengthUnit::Metre,
            },
            end: prumo_domain::units::PointInput {
                values: [0., 0., 0.],
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::AnalyticalSource,
            material_id: Some(mat),
            environment_id: None,
        },
    );
    let support = p.objects[0].id;
    let mut s = stair(&p, StairForm::SelfSupporting);
    s.supports = vec![AssemblySupport {
        part: 0,
        object_id: support,
        point: LocalPoint::new(0., 0.),
        kind: SupportKind::Fixed,
    }];
    s.continuities = proposed_continuities(&p, &s).unwrap();
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let r = inspect(&p, assembly_id(&p)).unwrap();
    assert_eq!(r.assembly.supports[0].kind, SupportKind::Fixed);
    assert_eq!(r.regions[0].preview.supports[0].object_id, support);
    assert!(r.regions[0].preview.supports[0].material_present);
    assert!(!r.regions[0].preview.engineering_release);
    use prumo_domain::dependencies::{SourceKey, SourceKind};
    for source in [
        SourceKey {
            kind: SourceKind::Physical,
            id: support,
        },
        SourceKey {
            kind: SourceKind::Material,
            id: mat,
        },
        SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: r.regions[0].region_id.unwrap(),
        },
        SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: r.regions[1].region_id.unwrap(),
        },
    ] {
        assert!(r.sources.contains(&source), "missing {source:?}");
    }
}

#[test]
fn folded_stair_preserves_orientations_and_connected_tread_riser_edges() {
    let mut p = fixture();
    let mut s = stair(&p, StairForm::Folded);
    if let AssemblyShape::Stair { flights, .. } = &mut s.shape {
        flights[0].steps = 4;
    }
    s.continuities = proposed_continuities(&p, &s).unwrap();
    assert_eq!(s.continuities.len(), 8);
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let r = inspect(&p, assembly_id(&p)).unwrap();
    assert_eq!(r.parts.len(), 9);
    assert_eq!(r.continuities.len(), 8);
    for (i, region) in r.regions.iter().take(8).enumerate() {
        let normal = region.surface.frame.normal();
        if i % 2 == 0 {
            near(normal[0], 1., 1e-12);
        } else {
            near(normal[2], 1., 1e-12);
        }
    }
    assert!(r.preview.additional_step_volumes.is_empty());
    // The aggregate adds no duplicate faces: each real child is drawn once.
    assert!(display_polylines(&p, r.object_id).unwrap().is_empty());
    assert!(r.parts.iter().all(|part| {
        !prumo_domain::surfaces::display_polylines(&p, part.object_id)
            .unwrap()
            .is_empty()
    }));
}

#[test]
fn open_core_with_window_keeps_only_remaining_regions_and_explicit_seams() {
    let mut p = fixture();
    let wall = |name: &str, origin: [f64; 3], u: [f64; 3], width: f64| WallPanel {
        name: name.into(),
        frame: SurfaceFrame {
            origin_m: point(origin),
            u,
            v: [0., 0., 1.],
        },
        outline: rectangle(width, 3.),
        openings: vec![],
        thickness_m: 0.2,
    };
    let mut a = wall("South", [0., 0., 0.], [1., 0., 0.], 4.);
    a.openings.push(SurfaceOpening {
        name: "Window".into(),
        outline: vec![
            LocalPoint::new(1., 1.),
            LocalPoint::new(2., 1.),
            LocalPoint::new(2., 2.),
            LocalPoint::new(1., 2.),
        ],
    });
    let mut s = AssemblySpec {
        name: "U core".into(),
        role: PhysicalRole::AnalyticalSource,
        material_id: Some(p.model.materials[0].id),
        environment_id: None,
        shape: AssemblyShape::Core {
            panels: vec![
                a,
                wall("East", [4., 0., 0.], [0., 1., 0.], 3.),
                wall("North", [4., 3., 0.], [-1., 0., 0.], 4.),
            ],
            coupling_beams: vec![],
        },
        supports: vec![],
        continuities: vec![],
    };
    s.continuities = proposed_continuities(&p, &s).unwrap();
    assert_eq!(s.continuities.len(), 2);
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let r = inspect(&p, assembly_id(&p)).unwrap();
    near(r.regions[0].preview.net_area_m2, 11., 1e-10);
    assert_eq!(r.parts.len(), 3);
    assert_eq!(r.continuities.len(), 2);
    assert!(!r.preview.engineering_release);
}

#[test]
fn modifying_landing_preserves_part_ids_and_changes_analytical_geometry() {
    let mut p = fixture();
    let s = stair(&p, StairForm::Conventional);
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let id = assembly_id(&p);
    let first = inspect(&p, id).unwrap();
    let mut changed = first.assembly.clone();
    if let AssemblyShape::Stair { landings, .. } = &mut changed.shape {
        landings[0].outline = rectangle(2., 1.);
    }
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Update {
                object_id: id,
                assembly: changed,
            },
        },
    );
    let last = inspect(&p, id).unwrap();
    assert_eq!(first.parts, last.parts);
    assert_eq!(first.regions[1].region_id, last.regions[1].region_id);
    near(last.regions[1].preview.net_area_m2, 2., 1e-10);
    assert_ne!(
        first.regions[1].preview.patches,
        last.regions[1].preview.patches
    );
    let child = last.parts[1].object_id;
    let before = p.clone();
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::EditSurfaces {
                command: SurfaceEdit::SetOpenings {
                    object_id: child,
                    openings: vec![]
                }
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}

#[test]
fn architecture_promotion_and_manual_adoption_create_no_orphan_parts() {
    let mut p = fixture();
    let mut s = stair(&p, StairForm::Conventional);
    let mat = s.material_id.unwrap();
    s.role = PhysicalRole::VisualOnly;
    s.material_id = None;
    let parts = preview(&p, &s).unwrap().parts;
    let mut ids = Vec::new();
    for part in &parts {
        edit(
            &mut p,
            ModelEdit::EditSurfaces {
                command: SurfaceEdit::Create {
                    surface: part.surface.clone(),
                },
            },
        );
        ids.push(p.objects.last().unwrap().id);
    }
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Adopt {
                assembly: s,
                part_ids: ids.clone(),
            },
        },
    );
    let id = assembly_id(&p);
    assert_eq!(p.objects.len(), 3);
    let adopted = inspect(&p, id).unwrap();
    assert_eq!(
        adopted
            .parts
            .iter()
            .map(|p| p.object_id)
            .collect::<Vec<_>>(),
        ids
    );
    assert!(adopted.regions.iter().all(|r| r.region_id.is_none()));
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Promote {
                object_id: id,
                material_id: mat,
            },
        },
    );
    let promoted = inspect(&p, id).unwrap();
    assert_eq!(p.objects.len(), 3);
    assert!(promoted.regions.iter().all(|r| r.region_id.is_some()));
    assert_eq!(
        promoted
            .parts
            .iter()
            .map(|p| p.object_id)
            .collect::<Vec<_>>(),
        ids
    );
    let linked: BTreeSet<_> = p.model.links.iter().map(|l| l.physical_id).collect();
    assert_eq!(linked, ids.into_iter().collect());
}

#[test]
fn independently_entered_wall_matches_parametric_geometry_and_retains_adopted_links() {
    use prumo_domain::surfaces::{SurfaceKind, SurfaceSpec};
    let mut manual = fixture();
    let mut parametric = manual.clone();
    let material_id = Some(manual.model.materials[0].id);
    // Coordinates are entered independently of the assembly generator: a
    // four by three metre wall in XZ at Y=5, normal -Y, with a 1 m² window.
    let frame = SurfaceFrame {
        origin_m: point([2., 5., 0.]),
        u: [1., 0., 0.],
        v: [0., 0., 1.],
    };
    let outline = vec![
        LocalPoint::new(0., 0.),
        LocalPoint::new(4., 0.),
        LocalPoint::new(4., 3.),
        LocalPoint::new(0., 3.),
    ];
    let openings = vec![SurfaceOpening {
        name: "Janela".into(),
        outline: vec![
            LocalPoint::new(1., 1.),
            LocalPoint::new(2., 1.),
            LocalPoint::new(2., 2.),
            LocalPoint::new(1., 2.),
        ],
    }];
    let entered = SurfaceSpec {
        name: "Parede manual".into(),
        role: PhysicalRole::AnalyticalSource,
        material_id,
        environment_id: None,
        frame: frame.clone(),
        kind: SurfaceKind::Panel {},
        outline: outline.clone(),
        openings: openings.clone(),
        thickness_m: 0.2,
        regions: vec![],
        supports: vec![],
        edge_conditions: vec![],
    };
    edit(
        &mut manual,
        ModelEdit::EditSurfaces {
            command: SurfaceEdit::Create { surface: entered },
        },
    );
    let manual_id = manual.objects[0].id;
    let original = prumo_domain::surfaces::inspect(&manual, manual_id).unwrap();
    let original_link = manual.model.links[0].clone();
    let assembly = AssemblySpec {
        name: "Conjunto equivalente".into(),
        role: PhysicalRole::AnalyticalSource,
        material_id,
        environment_id: None,
        shape: AssemblyShape::Wall {
            panel: WallPanel {
                name: "Parede manual".into(),
                frame,
                outline,
                openings,
                thickness_m: 0.2,
            },
        },
        supports: vec![],
        continuities: vec![],
    };
    edit(
        &mut manual,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Adopt {
                assembly: assembly.clone(),
                part_ids: vec![manual_id],
            },
        },
    );
    edit(
        &mut parametric,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly },
        },
    );
    let adopted = inspect(&manual, assembly_id(&manual)).unwrap();
    let generated = inspect(&parametric, assembly_id(&parametric)).unwrap();
    assert_eq!(adopted.parts.len(), 1);
    assert_eq!(adopted.parts[0].object_id, manual_id);
    assert_eq!(adopted.regions[0].region_id, original.region_id);
    assert_eq!(manual.model.links, vec![original_link]);
    assert_eq!(adopted.regions[0].surface, generated.regions[0].surface);
    assert_eq!(adopted.regions[0].preview, generated.regions[0].preview);
    near(adopted.regions[0].preview.net_area_m2, 11., 1e-10);
    near(
        adopted.regions[0].preview.quantities.total_volume_m3,
        2.2,
        1e-10,
    );
    assert_eq!(adopted.regions[0].surface.frame.normal(), [0., -1., 0.]);
    assert_eq!(manual.objects.len(), 2);
    assert_eq!(parametric.objects.len(), 2);
    assert_eq!(parametric.model.links.len(), 1);
    assert_eq!(
        parametric.model.links[0].physical_id,
        generated.parts[0].object_id
    );
    assert_eq!(
        Some(parametric.model.links[0].analytical_id),
        generated.regions[0].region_id
    );
}

#[test]
fn ramp_and_updates_to_number_of_parts_do_not_leave_active_orphans() {
    let mut p = fixture();
    let mut s = stair(&p, StairForm::Conventional);
    if let AssemblyShape::Stair {
        flights, landings, ..
    } = s.shape
    {
        let mut f = flights;
        f[0].steps = 0;
        s.shape = AssemblyShape::Ramp {
            flights: f,
            landings,
        };
    }
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Create { assembly: s },
        },
    );
    let id = assembly_id(&p);
    let first = inspect(&p, id).unwrap();
    let removed = first.parts[1].object_id;
    let mut changed = first.assembly;
    if let AssemblyShape::Ramp { landings, .. } = &mut changed.shape {
        landings.clear();
    }
    edit(
        &mut p,
        ModelEdit::EditAssemblies {
            command: AssemblyEdit::Update {
                object_id: id,
                assembly: changed,
            },
        },
    );
    assert_eq!(p.objects.len(), 2);
    assert!(!p.objects.iter().any(|o| o.id == removed));
    assert!(p.model.identities[&removed].retired_revision.is_some());
    p.validate().unwrap();
}

#[test]
fn duplicated_or_remote_continuities_and_zero_width_are_rejected_atomically() {
    let mut p = fixture();
    let s = stair(&p, StairForm::Conventional);
    let mut bad = s.clone();
    bad.continuities = vec![PartContinuity {
        from_part: 0,
        from_edge: 0,
        to_part: 1,
        to_edge: 0,
    }];
    assert!(preview(&p, &bad).is_err());
    let mut bad = s.clone();
    let c = proposed_continuities(&p, &bad).unwrap()[0].clone();
    bad.continuities = vec![c.clone(), c];
    assert!(preview(&p, &bad).is_err());
    let mut bad = s;
    if let AssemblyShape::Stair { flights, .. } = &mut bad.shape {
        flights[0].width_m = 0.;
    }
    let original = serde_json::to_value(&p).unwrap();
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::EditAssemblies {
                command: AssemblyEdit::Create { assembly: bad }
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&p).unwrap(), original);
}
