//! Walls, open cores, stairs and ramps built from traceable surface parts.
//! Continuity and supports are explicit input. Geometric contact alone never
//! creates a structural connection, and these candidates do not run a solver.
use crate::model::{PhysicalRole, record_identity};
use crate::surfaces::{
    self, LocalPoint, SurfaceFrame, SurfaceKind, SurfaceOpening, SurfaceSpec, SurfaceSupport,
    ThicknessRegion,
};
use crate::units::Point3;
use crate::{Discipline, DomainError, ModelState, PhysicalObject, Project, validate_name};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const ASSEMBLY_KEY: &str = "prumo.assembly.v1";
const MAX_PARTS: usize = 256;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WallPanel {
    pub name: String,
    pub frame: SurfaceFrame,
    pub outline: Vec<LocalPoint>,
    pub openings: Vec<SurfaceOpening>,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CouplingBeam {
    pub name: String,
    pub start_m: Point3,
    pub end_m: Point3,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub height_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StairForm {
    Conventional,
    Folded,
    SelfSupporting,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StairFlight {
    pub name: String,
    /// Lower and upper points on the upper face of a conventional stair's waist
    /// or ramp, at the left edge. For folded stairs they locate the step profile.
    pub start_m: Point3,
    pub end_m: Point3,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub width_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
    pub steps: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Landing {
    pub name: String,
    /// Reference plane of the upper face. Material extends against its normal.
    pub frame: SurfaceFrame,
    pub outline: Vec<LocalPoint>,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssemblyShape {
    Wall {
        panel: WallPanel,
    },
    Core {
        panels: Vec<WallPanel>,
        coupling_beams: Vec<CouplingBeam>,
    },
    Stair {
        form: StairForm,
        flights: Vec<StairFlight>,
        landings: Vec<Landing>,
    },
    Ramp {
        flights: Vec<StairFlight>,
        landings: Vec<Landing>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportKind {
    Bearing,
    Fixed,
    Sliding,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblySupport {
    pub part: usize,
    pub object_id: Uuid,
    pub point: LocalPoint,
    pub kind: SupportKind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartContinuity {
    pub from_part: usize,
    pub from_edge: usize,
    pub to_part: usize,
    pub to_edge: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblySpec {
    pub name: String,
    pub role: PhysicalRole,
    pub material_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub shape: AssemblyShape,
    pub supports: Vec<AssemblySupport>,
    pub continuities: Vec<PartContinuity>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssemblyEdit {
    Create {
        assembly: AssemblySpec,
    },
    Update {
        object_id: Uuid,
        assembly: AssemblySpec,
    },
    Adopt {
        assembly: AssemblySpec,
        part_ids: Vec<Uuid>,
    },
    Promote {
        object_id: Uuid,
        material_id: Uuid,
    },
    SetSupports {
        object_id: Uuid,
        supports: Vec<AssemblySupport>,
    },
    SetContinuities {
        object_id: Uuid,
        continuities: Vec<PartContinuity>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyPart {
    pub key: String,
    pub object_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyRecord {
    pub physical_id: Uuid,
    pub assembly: AssemblySpec,
    pub parts: Vec<AssemblyPart>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PartPreview {
    pub key: String,
    pub surface: SurfaceSpec,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StepVolume {
    pub flight: String,
    pub volume_m3: f64,
    pub mass_kg: Option<f64>,
    pub hypothesis: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AssemblyPreview {
    pub parts: Vec<PartPreview>,
    pub declared_continuities: Vec<PartContinuity>,
    pub declared_supports: Vec<AssemblySupport>,
    pub additional_step_volumes: Vec<StepVolume>,
    pub warnings: Vec<String>,
    pub engineering_release: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RegionContinuity {
    pub from_object_id: Uuid,
    pub from_region_id: Option<Uuid>,
    pub to_object_id: Uuid,
    pub to_region_id: Option<Uuid>,
    pub from_edge: usize,
    pub to_edge: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AssemblyInspection {
    pub object_id: Uuid,
    pub assembly: AssemblySpec,
    pub parts: Vec<AssemblyPart>,
    pub regions: Vec<surfaces::SurfaceInspection>,
    pub continuities: Vec<RegionContinuity>,
    pub preview: AssemblyPreview,
    pub sources: Vec<crate::dependencies::SourceKey>,
}

fn invalid(message: &str) -> DomainError {
    DomainError::InvalidModel(message.into())
}
fn encoded<T: Serialize>(value: &T) -> Result<Value, DomainError> {
    serde_json::to_value(value).map_err(|e| invalid(&e.to_string()))
}
fn positive(value: f64) -> Result<(), DomainError> {
    if !value.is_finite() || value <= 0. || value > 1e6 {
        Err(invalid("Dimensão do conjunto inválida."))
    } else {
        Ok(())
    }
}

pub fn metadata(object: &PhysicalObject) -> Result<Option<AssemblyRecord>, DomainError> {
    object
        .extensions
        .get(ASSEMBLY_KEY)
        .map(|v| {
            serde_json::from_value(v.clone())
                .map_err(|e| invalid(&format!("Registro de conjunto inválido: {e}")))
        })
        .transpose()
}

pub fn is_child(project: &Project, id: Uuid) -> Result<bool, DomainError> {
    for object in &project.objects {
        if let Some(record) = metadata(object)?
            && record.parts.iter().any(|p| p.object_id == id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn surface(
    spec: &AssemblySpec,
    name: String,
    frame: SurfaceFrame,
    outline: Vec<LocalPoint>,
    openings: Vec<SurfaceOpening>,
    thickness_m: f64,
) -> SurfaceSpec {
    SurfaceSpec {
        name,
        role: spec.role,
        material_id: spec.material_id,
        environment_id: spec.environment_id,
        frame,
        kind: SurfaceKind::Panel {},
        outline,
        openings,
        thickness_m,
        regions: vec![],
        supports: vec![],
        edge_conditions: vec![],
    }
}
fn rectangle(width: f64, height: f64) -> Vec<LocalPoint> {
    vec![
        LocalPoint::new(0., 0.),
        LocalPoint::new(width, 0.),
        LocalPoint::new(width, height),
        LocalPoint::new(0., height),
    ]
}

/// A flight or landing is entered at its upper face. Its material midsurface
/// therefore lies half a thickness below that face, leaving the step wedges
/// wholly above the waist. The physical contact edges remain explicit input;
/// transferring between offset analytical midsurfaces requires a future solver.
fn below_upper_face(mut surface: SurfaceSpec) -> SurfaceSpec {
    surface.regions.push(ThicknessRegion {
        name: "Lâmina abaixo da face superior declarada".into(),
        outline: surface.outline.clone(),
        thickness_m: surface.thickness_m,
        offset_m: -0.5 * surface.thickness_m,
    });
    surface
}

fn flight_axes(
    flight: &StairFlight,
) -> Result<(Vector3<f64>, Vector3<f64>, f64, f64), DomainError> {
    validate_name(&flight.name)?;
    positive(flight.width_m)?;
    positive(flight.thickness_m)?;
    let delta = Vector3::from(flight.end_m.as_array()) - Vector3::from(flight.start_m.as_array());
    let run = (delta.x * delta.x + delta.y * delta.y).sqrt();
    if run <= 1e-8 || delta.z <= 1e-8 || !delta.norm().is_finite() {
        return Err(invalid(
            "Lance deve subir entre pontos distintos com desenvolvimento horizontal positivo.",
        ));
    }
    let direction = Vector3::new(delta.x / run, delta.y / run, 0.);
    let across = Vector3::new(-direction.y, direction.x, 0.);
    Ok((direction, across, run, delta.z))
}

fn wall_part(
    spec: &AssemblySpec,
    panel: &WallPanel,
    key: String,
) -> Result<PartPreview, DomainError> {
    validate_name(&panel.name)?;
    positive(panel.thickness_m)?;
    Ok(PartPreview {
        key,
        surface: surface(
            spec,
            panel.name.clone(),
            panel.frame.clone(),
            panel.outline.clone(),
            panel.openings.clone(),
            panel.thickness_m,
        ),
    })
}

fn generated_parts(spec: &AssemblySpec) -> Result<Vec<PartPreview>, DomainError> {
    validate_name(&spec.name)?;
    // Bound the expansion before allocating or triangulating any generated part.
    let count = match &spec.shape {
        AssemblyShape::Wall { .. } => Some(1),
        AssemblyShape::Core {
            panels,
            coupling_beams,
        } => panels.len().checked_add(coupling_beams.len()),
        AssemblyShape::Ramp { flights, landings } => flights.len().checked_add(landings.len()),
        AssemblyShape::Stair {
            form,
            flights,
            landings,
        } => {
            if flights.len() > MAX_PARTS || landings.len() > MAX_PARTS {
                return Err(DomainError::ModelLimit);
            }
            flights.iter().try_fold(landings.len(), |count, flight| {
                count.checked_add(if *form == StairForm::Folded {
                    usize::from(flight.steps) * 2
                } else {
                    1
                })
            })
        }
    };
    if count.is_none_or(|n| n == 0 || n > MAX_PARTS) {
        return Err(DomainError::ModelLimit);
    }
    let mut parts = Vec::new();
    match &spec.shape {
        AssemblyShape::Wall { panel } => parts.push(wall_part(spec, panel, "wall-0".into())?),
        AssemblyShape::Core {
            panels,
            coupling_beams,
        } => {
            if panels.len() < 2 || panels.len() + coupling_beams.len() > MAX_PARTS {
                return Err(invalid("Núcleo requer de duas a 256 partes."));
            }
            for (i, panel) in panels.iter().enumerate() {
                parts.push(wall_part(spec, panel, format!("wall-{i}"))?);
            }
            for (i, beam) in coupling_beams.iter().enumerate() {
                validate_name(&beam.name)?;
                positive(beam.height_m)?;
                positive(beam.thickness_m)?;
                let delta =
                    Vector3::from(beam.end_m.as_array()) - Vector3::from(beam.start_m.as_array());
                let length = delta.norm();
                if length <= 1e-8 || delta.z.abs() > 1e-8 {
                    return Err(invalid(
                        "A viga de acoplamento deve ter eixo horizontal não nulo nesta etapa.",
                    ));
                }
                let frame = SurfaceFrame {
                    origin_m: beam.start_m,
                    u: (delta / length).into(),
                    v: [0., 0., 1.],
                };
                parts.push(PartPreview {
                    key: format!("coupling-{i}"),
                    surface: surface(
                        spec,
                        beam.name.clone(),
                        frame,
                        rectangle(length, beam.height_m),
                        vec![],
                        beam.thickness_m,
                    ),
                });
            }
        }
        AssemblyShape::Stair {
            form,
            flights,
            landings,
        } => {
            if flights.is_empty() {
                return Err(invalid("Informe ao menos um lance de escada."));
            }
            for (i, flight) in flights.iter().enumerate() {
                let (direction, across, run, rise) = flight_axes(flight)?;
                if !(1..=128).contains(&flight.steps) {
                    return Err(invalid("O lance deve ter de 1 a 128 degraus."));
                }
                if *form == StairForm::Folded {
                    let tread = run / f64::from(flight.steps);
                    let riser = rise / f64::from(flight.steps);
                    for j in 0..flight.steps {
                        let origin = Vector3::from(flight.start_m.as_array())
                            + direction * (f64::from(j) * tread)
                            + Vector3::z() * (f64::from(j) * riser);
                        let riser_frame = SurfaceFrame {
                            origin_m: point3(origin)?,
                            u: across.into(),
                            v: [0., 0., 1.],
                        };
                        parts.push(PartPreview {
                            key: format!("folded-{i}-riser-{j}"),
                            surface: surface(
                                spec,
                                format!("{} espelho {}", flight.name, j + 1),
                                riser_frame,
                                rectangle(flight.width_m, riser),
                                vec![],
                                flight.thickness_m,
                            ),
                        });
                        let tread_frame = SurfaceFrame {
                            origin_m: point3(origin + Vector3::z() * riser)?,
                            u: direction.into(),
                            v: across.into(),
                        };
                        parts.push(PartPreview {
                            key: format!("folded-{i}-tread-{j}"),
                            surface: surface(
                                spec,
                                format!("{} piso {}", flight.name, j + 1),
                                tread_frame,
                                rectangle(tread, flight.width_m),
                                vec![],
                                flight.thickness_m,
                            ),
                        });
                    }
                } else {
                    let delta = Vector3::from(flight.end_m.as_array())
                        - Vector3::from(flight.start_m.as_array());
                    let length = delta.norm();
                    let frame = SurfaceFrame {
                        origin_m: flight.start_m,
                        u: (delta / length).into(),
                        v: across.into(),
                    };
                    parts.push(PartPreview {
                        key: format!("flight-{i}"),
                        surface: below_upper_face(surface(
                            spec,
                            flight.name.clone(),
                            frame,
                            rectangle(length, flight.width_m),
                            vec![],
                            flight.thickness_m,
                        )),
                    });
                }
            }
            add_landings(&mut parts, spec, landings)?;
        }
        AssemblyShape::Ramp { flights, landings } => {
            if flights.is_empty() {
                return Err(invalid("Informe ao menos um lance de rampa."));
            }
            for (i, flight) in flights.iter().enumerate() {
                let (_, across, _, _) = flight_axes(flight)?;
                if flight.steps != 0 {
                    return Err(invalid("Rampa não possui degraus: informe steps = 0."));
                }
                let delta = Vector3::from(flight.end_m.as_array())
                    - Vector3::from(flight.start_m.as_array());
                let length = delta.norm();
                let frame = SurfaceFrame {
                    origin_m: flight.start_m,
                    u: (delta / length).into(),
                    v: across.into(),
                };
                parts.push(PartPreview {
                    key: format!("ramp-{i}"),
                    surface: below_upper_face(surface(
                        spec,
                        flight.name.clone(),
                        frame,
                        rectangle(length, flight.width_m),
                        vec![],
                        flight.thickness_m,
                    )),
                });
            }
            add_landings(&mut parts, spec, landings)?;
        }
    }
    if parts.len() > MAX_PARTS {
        return Err(DomainError::ModelLimit);
    }
    if spec.supports.len() > MAX_PARTS * 4 || spec.continuities.len() > MAX_PARTS * 4 {
        return Err(DomainError::ModelLimit);
    }
    for support in &spec.supports {
        let part = parts
            .get_mut(support.part)
            .ok_or_else(|| invalid("Apoio referencia parte inexistente."))?;
        part.surface.supports.push(SurfaceSupport {
            object_id: support.object_id,
            point: support.point,
        });
    }
    Ok(parts)
}

fn add_landings(
    parts: &mut Vec<PartPreview>,
    spec: &AssemblySpec,
    landings: &[Landing],
) -> Result<(), DomainError> {
    for (i, landing) in landings.iter().enumerate() {
        validate_name(&landing.name)?;
        positive(landing.thickness_m)?;
        parts.push(PartPreview {
            key: format!("landing-{i}"),
            surface: below_upper_face(surface(
                spec,
                landing.name.clone(),
                landing.frame.clone(),
                landing.outline.clone(),
                vec![],
                landing.thickness_m,
            )),
        });
    }
    Ok(())
}

fn edge(part: &PartPreview, index: usize) -> Result<[Point3; 2], DomainError> {
    let s = &part.surface;
    if index >= s.outline.len() {
        return Err(invalid("Continuidade referencia borda inexistente."));
    }
    Ok([
        s.frame.point(s.outline[index], 0.)?,
        s.frame
            .point(s.outline[(index + 1) % s.outline.len()], 0.)?,
    ])
}
fn edges_match(a: [Point3; 2], b: [Point3; 2], tolerance: f64) -> bool {
    (a[0].distance(b[0]) <= tolerance && a[1].distance(b[1]) <= tolerance)
        || (a[0].distance(b[1]) <= tolerance && a[1].distance(b[0]) <= tolerance)
}

/// Suggestions only. The caller must include the chosen links explicitly in a command.
pub fn proposed_continuities(
    project: &Project,
    spec: &AssemblySpec,
) -> Result<Vec<PartContinuity>, DomainError> {
    let parts = generated_parts(spec)?;
    let mut pairs = 0usize;
    for (i, a) in parts.iter().enumerate() {
        for b in &parts[i + 1..] {
            pairs = pairs
                .checked_add(
                    a.surface
                        .outline
                        .len()
                        .checked_mul(b.surface.outline.len())
                        .ok_or(DomainError::ModelLimit)?,
                )
                .ok_or(DomainError::ModelLimit)?;
            if pairs > 1_000_000 {
                return Err(invalid(
                    "A busca de continuidade excede um milhão de pares de bordas. Declare as uniões específicas ou divida o conjunto.",
                ));
            }
        }
    }
    let tolerance = crate::geometry::project_tolerances(project)?.topology_m;
    let mut result = Vec::new();
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            for a in 0..parts[i].surface.outline.len() {
                for b in 0..parts[j].surface.outline.len() {
                    if edges_match(edge(&parts[i], a)?, edge(&parts[j], b)?, tolerance) {
                        result.push(PartContinuity {
                            from_part: i,
                            from_edge: a,
                            to_part: j,
                            to_edge: b,
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}

pub fn preview(project: &Project, spec: &AssemblySpec) -> Result<AssemblyPreview, DomainError> {
    let parts = generated_parts(spec)?;
    for part in &parts {
        surfaces::preview(project, &part.surface)?;
    }
    let tolerance = crate::geometry::project_tolerances(project)?.topology_m;
    let mut links = BTreeSet::new();
    for c in &spec.continuities {
        if c.from_part == c.to_part {
            return Err(invalid("Uma parte não pode se conectar a si mesma."));
        }
        let a = parts
            .get(c.from_part)
            .ok_or_else(|| invalid("Parte de continuidade inexistente."))?;
        let b = parts
            .get(c.to_part)
            .ok_or_else(|| invalid("Parte de continuidade inexistente."))?;
        if !edges_match(edge(a, c.from_edge)?, edge(b, c.to_edge)?, tolerance) {
            return Err(invalid(
                "As bordas declaradas não coincidem dentro da tolerância topológica. Nenhum vínculo foi criado.",
            ));
        }
        let x = (c.from_part, c.from_edge);
        let y = (c.to_part, c.to_edge);
        if !links.insert((x.min(y), x.max(y))) {
            return Err(invalid("Continuidade repetida."));
        }
    }
    let mut warnings = Vec::new();
    if spec.role == PhysicalRole::VisualOnly {
        warnings.push(
            "Referência arquitetônica: nenhuma região resistente ou apoio está liberado.".into(),
        );
    } else if spec.supports.is_empty() {
        warnings
            .push("Nenhum apoio externo foi declarado. O conjunto não está apto a cálculo.".into());
    }
    if parts.len() > 1 && spec.continuities.is_empty() {
        warnings.push(
            "Partes sem continuidade declarada. A proximidade visual não liga as regiões.".into(),
        );
    }
    if parts.len() > 1 {
        warnings.push("Quantidades nominais por parte: os encontros entre cintura e patamar, paredes ou vigas não foram unidos ou recortados como sólidos. Não some essas quantidades como volume líquido do conjunto sem conferir e descontar as juntas.".into());
    }
    if matches!(
        spec.shape,
        AssemblyShape::Stair { .. } | AssemblyShape::Ramp { .. }
    ) {
        warnings.push("As bordas de cintura, rampa e patamar são referidas às faces superiores declaradas. Seus planos médios têm deslocamentos de espessura; a ligação numérica entre eles ainda requer o modelo de análise posterior.".into());
    }
    let mut step_volumes = Vec::new();
    if let AssemblyShape::Stair { form, flights, .. } = &spec.shape {
        if *form == StairForm::Folded {
            warnings.push("Escada plissada: as faces representam lâminas com bordas declaradas. O encontro em meia-esquadria é hipótese de quantificação, não um recorte de sólido já executado; nenhum volume de acabamento foi presumido.".into());
        } else {
            for flight in flights {
                let (_, _, run, rise) = flight_axes(flight)?;
                let volume = 0.5
                    * (run / f64::from(flight.steps))
                    * (rise / f64::from(flight.steps))
                    * flight.width_m
                    * f64::from(flight.steps);
                let density = spec
                    .material_id
                    .and_then(|id| project.model.materials.iter().find(|m| m.id == id))
                    .map(|m| m.density.si());
                step_volumes.push(StepVolume{flight:flight.name.clone(),volume_m3:volume,mass_kg:density.map(|d|d*volume),hypothesis:"Degraus maciços acima da face superior inclinada da cintura, definida por start/end. A lâmina se estende para baixo dessa face; as cunhas triangulares ocupam apenas o semiespaço acima dela.".into()});
            }
        }
    }
    Ok(AssemblyPreview {
        parts,
        declared_continuities: spec.continuities.clone(),
        declared_supports: spec.supports.clone(),
        additional_step_volumes: step_volumes,
        warnings,
        engineering_release: false,
    })
}

fn reference_endpoints(parts: &[PartPreview]) -> Result<(Point3, Point3), DomainError> {
    let first = parts
        .first()
        .ok_or_else(|| invalid("Conjunto sem partes."))?
        .surface
        .frame
        .point(parts[0].surface.outline[0], 0.)?;
    let mut farthest = first;
    let mut distance = 0.;
    for part in parts {
        for p in &part.surface.outline {
            let point = part.surface.frame.point(*p, 0.)?;
            let d = point.distance(first);
            if d > distance {
                distance = d;
                farthest = point;
            }
        }
    }
    if distance <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    Ok((first, farthest))
}

fn load(project: &Project, id: Uuid) -> Result<AssemblyRecord, DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    metadata(object)?.ok_or_else(|| invalid("A peça não é um conjunto paramétrico."))
}

fn put_assembly(
    project: &mut Project,
    id: Option<Uuid>,
    spec: AssemblySpec,
    adopt: Option<Vec<Uuid>>,
    actor: &str,
    revision: u64,
) -> Result<Uuid, DomainError> {
    let prepared = preview(project, &spec)?;
    let (start, end) = reference_endpoints(&prepared.parts)?;
    let aggregate = id.unwrap_or_else(Uuid::new_v4);
    let previous = id.map(|id| load(project, id)).transpose()?;
    let old_parts: BTreeMap<_, _> = previous
        .as_ref()
        .map(|r| {
            r.parts
                .iter()
                .map(|p| (p.key.clone(), p.object_id))
                .collect()
        })
        .unwrap_or_default();
    if let Some(ids) = &adopt
        && (ids.len() != prepared.parts.len()
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len())
    {
        return Err(invalid(
            "A adoção exige uma peça independente distinta por parte.",
        ));
    }
    let mut parts = Vec::new();
    for (index, part) in prepared.parts.iter().enumerate() {
        let part_id = if let Some(ids) = &adopt {
            let id = ids[index];
            if is_child(project, id)? {
                return Err(invalid("Peça já pertence a outro conjunto."));
            }
            let inspected = surfaces::inspect(project, id)?;
            if inspected.surface != part.surface {
                return Err(invalid(
                    "A peça manual não equivale aos parâmetros da parte a adotar.",
                ));
            }
            id
        } else {
            surfaces::put_surface(
                project,
                old_parts.get(&part.key).copied(),
                part.surface.clone(),
                actor,
                revision,
            )?
        };
        if spec
            .supports
            .iter()
            .any(|s| s.object_id == part_id || s.object_id == aggregate)
        {
            return Err(invalid(
                "Apoios externos não podem apontar para o próprio conjunto. Declare continuidade entre as partes.",
            ));
        }
        parts.push(AssemblyPart {
            key: part.key.clone(),
            object_id: part_id,
        });
    }
    for old_id in old_parts
        .values()
        .filter(|id| !parts.iter().any(|p| p.object_id == **id))
    {
        if project
            .objects
            .iter()
            .filter(|o| o.id != aggregate)
            .any(|o| {
                surfaces::metadata(o)
                    .ok()
                    .flatten()
                    .is_some_and(|r| r.surface.supports.iter().any(|s| s.object_id == *old_id))
            })
        {
            return Err(invalid(
                "Uma parte a retirar é usada como apoio externo. Atualize os vínculos antes de mudar o conjunto.",
            ));
        }
        surfaces::remove_surface(project, *old_id, revision)?;
    }
    let record = AssemblyRecord {
        physical_id: aggregate,
        assembly: spec.clone(),
        parts,
    };
    if let Some(object) = project.objects.iter_mut().find(|o| o.id == aggregate) {
        object.name = spec.name;
        object.start_m = start;
        object.end_m = end;
        object.role = spec.role;
        object.material_id = spec.material_id;
        object.environment_id = spec.environment_id;
        object
            .extensions
            .insert(ASSEMBLY_KEY.into(), encoded(&record)?);
    } else {
        record_identity(project, aggregate, actor, revision, true)?;
        project.objects.push(PhysicalObject {
            id: aggregate,
            name: spec.name,
            discipline: Discipline::Structure,
            start_m: start,
            end_m: end,
            state: ModelState::Modelled,
            role: spec.role,
            material_id: spec.material_id,
            environment_id: spec.environment_id,
            extensions: BTreeMap::from([(ASSEMBLY_KEY.into(), encoded(&record)?)]),
        });
    }
    Ok(aggregate)
}

pub fn apply(
    project: &mut Project,
    command: AssemblyEdit,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    let mut candidate = project.clone();
    match command {
        AssemblyEdit::Create { assembly } => {
            put_assembly(&mut candidate, None, assembly, None, actor, revision)?;
        }
        AssemblyEdit::Update {
            object_id,
            assembly,
        } => {
            put_assembly(
                &mut candidate,
                Some(object_id),
                assembly,
                None,
                actor,
                revision,
            )?;
        }
        AssemblyEdit::Adopt { assembly, part_ids } => {
            put_assembly(
                &mut candidate,
                None,
                assembly,
                Some(part_ids),
                actor,
                revision,
            )?;
        }
        AssemblyEdit::Promote {
            object_id,
            material_id,
        } => {
            let mut spec = load(&candidate, object_id)?.assembly;
            spec.role = PhysicalRole::AnalyticalSource;
            spec.material_id = Some(material_id);
            put_assembly(&mut candidate, Some(object_id), spec, None, actor, revision)?;
        }
        AssemblyEdit::SetSupports {
            object_id,
            supports,
        } => {
            let mut spec = load(&candidate, object_id)?.assembly;
            spec.supports = supports;
            put_assembly(&mut candidate, Some(object_id), spec, None, actor, revision)?;
        }
        AssemblyEdit::SetContinuities {
            object_id,
            continuities,
        } => {
            let mut spec = load(&candidate, object_id)?.assembly;
            spec.continuities = continuities;
            put_assembly(&mut candidate, Some(object_id), spec, None, actor, revision)?;
        }
    }
    surfaces::validate_project(&candidate)?;
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}

pub fn inspect(project: &Project, object_id: Uuid) -> Result<AssemblyInspection, DomainError> {
    let record = load(project, object_id)?;
    let regions = record
        .parts
        .iter()
        .map(|p| surfaces::inspect(project, p.object_id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut connections = Vec::new();
    for c in &record.assembly.continuities {
        connections.push(RegionContinuity {
            from_object_id: record.parts[c.from_part].object_id,
            from_region_id: regions[c.from_part].region_id,
            to_object_id: record.parts[c.to_part].object_id,
            to_region_id: regions[c.to_part].region_id,
            from_edge: c.from_edge,
            to_edge: c.to_edge,
        });
    }
    Ok(AssemblyInspection {
        object_id,
        preview: preview(project, &record.assembly)?,
        assembly: record.assembly,
        parts: record.parts,
        regions,
        continuities: connections,
        sources: object_sources(project, object_id)?,
    })
}

pub fn object_sources(
    project: &Project,
    object_id: Uuid,
) -> Result<Vec<crate::dependencies::SourceKey>, DomainError> {
    let record = load(project, object_id)?;
    let mut sources = BTreeSet::new();
    surfaces::append_object_sources(project, object_id, &mut sources)?;
    for part in &record.parts {
        sources.extend(surfaces::object_sources(project, part.object_id)?);
    }
    Ok(sources.into_iter().collect())
}

pub fn register_ids(_project: &Project, _ids: &mut BTreeSet<Uuid>) -> Result<(), DomainError> {
    Ok(())
}
pub fn active_values(_project: &Project) -> Result<BTreeMap<Uuid, Value>, DomainError> {
    Ok(BTreeMap::new())
}

pub fn validate_project(project: &Project) -> Result<(), DomainError> {
    let mut all_parts = BTreeSet::new();
    for object in &project.objects {
        let Some(record) = metadata(object)? else {
            continue;
        };
        let expected = preview(project, &record.assembly)?;
        let (start, end) = reference_endpoints(&expected.parts)?;
        let a = &record.assembly;
        if record.physical_id != object.id
            || object.discipline != Discipline::Structure
            || object.name != a.name
            || object.role != a.role
            || object.material_id != a.material_id
            || object.environment_id != a.environment_id
            || object.start_m != start
            || object.end_m != end
            || record.parts.len() != expected.parts.len()
        {
            return Err(invalid(
                "O conjunto diverge dos parâmetros. Use o editor de paredes, escadas e rampas.",
            ));
        }
        if project
            .model
            .links
            .iter()
            .any(|l| l.physical_id == object.id)
        {
            return Err(invalid(
                "O conjunto usa as regiões das suas partes, sem membro analítico adicional.",
            ));
        }
        for (part, expected) in record.parts.iter().zip(expected.parts) {
            if part.object_id == object.id
                || !all_parts.insert(part.object_id)
                || part.key != expected.key
                || surfaces::inspect(project, part.object_id)?.surface != expected.surface
            {
                return Err(invalid(
                    "Parte órfã, repetida ou divergente do conjunto. Atualize pelo editor paramétrico.",
                ));
            }
        }
    }
    Ok(())
}

pub fn display_polylines(
    project: &Project,
    object_id: Uuid,
) -> Result<Vec<Vec<Point3>>, DomainError> {
    let Some(object) = project.objects.iter().find(|o| o.id == object_id) else {
        return Ok(vec![]);
    };
    let Some(record) = metadata(object)? else {
        return Ok(vec![]);
    };
    let mut lines = Vec::new();
    if let AssemblyShape::Stair { form, flights, .. } = &record.assembly.shape
        && *form != StairForm::Folded
    {
        for flight in flights {
            let (direction, across, run, rise) = flight_axes(flight)?;
            let tread = run / f64::from(flight.steps);
            let riser = rise / f64::from(flight.steps);
            let mut side = Vec::new();
            let start = Vector3::from(flight.start_m.as_array());
            side.push(flight.start_m);
            for j in 0..flight.steps {
                side.push(point3(
                    start
                        + direction * (f64::from(j) * tread)
                        + Vector3::z() * (f64::from(j + 1) * riser),
                )?);
                side.push(point3(
                    start
                        + direction * (f64::from(j + 1) * tread)
                        + Vector3::z() * (f64::from(j + 1) * riser),
                )?);
            }
            let other = side
                .iter()
                .map(|p| point3(Vector3::from(p.as_array()) + across * flight.width_m))
                .collect::<Result<Vec<_>, _>>()?;
            for (a, b) in side.iter().zip(&other) {
                lines.push(vec![*a, *b]);
            }
            lines.push(side);
            lines.push(other);
        }
    }
    Ok(lines)
}

fn shifted(point: Point3, delta: [f64; 3]) -> Result<Point3, DomainError> {
    point3(Vector3::from(point.as_array()) + Vector3::from(delta))
}
pub fn translate_object(
    project: &mut Project,
    object_id: Uuid,
    delta: [f64; 3],
) -> Result<(), DomainError> {
    let mut spec = load(project, object_id)?.assembly;
    match &mut spec.shape {
        AssemblyShape::Wall { panel } => {
            panel.frame.origin_m = shifted(panel.frame.origin_m, delta)?
        }
        AssemblyShape::Core {
            panels,
            coupling_beams,
        } => {
            for panel in panels {
                panel.frame.origin_m = shifted(panel.frame.origin_m, delta)?;
            }
            for beam in coupling_beams {
                beam.start_m = shifted(beam.start_m, delta)?;
                beam.end_m = shifted(beam.end_m, delta)?;
            }
        }
        AssemblyShape::Stair {
            flights, landings, ..
        }
        | AssemblyShape::Ramp { flights, landings } => {
            for flight in flights {
                flight.start_m = shifted(flight.start_m, delta)?;
                flight.end_m = shifted(flight.end_m, delta)?;
            }
            for landing in landings {
                landing.frame.origin_m = shifted(landing.frame.origin_m, delta)?;
            }
        }
    }
    let revision = project
        .revision
        .checked_add(1)
        .ok_or(DomainError::InvalidHistory)?;
    put_assembly(
        project,
        Some(object_id),
        spec,
        None,
        "deslocamento-parametrico",
        revision,
    )?;
    Ok(())
}

fn point3(value: Vector3<f64>) -> Result<Point3, DomainError> {
    Point3::try_from([value.x, value.y, value.z])
}
