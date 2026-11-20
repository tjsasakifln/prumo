//! Project-scoped presentation state shared by native, web and MCP clients.
//! Coordinates stay in f64 until the final UI raster boundary. No operation
//! creates an engineering connection, changes a project or approves a result.
use crate::{DomainError, PhysicalObject, Project, units::Point3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    #[default]
    Plan,
    Section,
    ThreeD,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CutAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCut {
    pub axis: CutAxis,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub min_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub max_m: f64,
}

impl ViewCut {
    fn index(self) -> usize {
        match self.axis {
            CutAxis::X => 0,
            CutAxis::Y => 1,
            CutAxis::Z => 2,
        }
    }
    fn validate(self) -> Result<(), DomainError> {
        if !self.min_m.is_finite()
            || !self.max_m.is_finite()
            || self.min_m.abs() > crate::MAX_COORDINATE_M
            || self.max_m.abs() > crate::MAX_COORDINATE_M
            || self.min_m > self.max_m
        {
            return Err(invalid(
                "O intervalo de corte deve ter limites finitos e mínimo não superior ao máximo.",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ViewState {
    pub project_id: Option<Uuid>,
    pub selected_id: Option<Uuid>,
    pub active_view: ViewKind,
    pub hidden_ids: BTreeSet<Uuid>,
    pub isolated_id: Option<Uuid>,
    pub cut: Option<ViewCut>,
    #[serde(deserialize_with = "crate::numeric::vector3")]
    pub center_m: [f64; 3],
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub span_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub yaw_degrees: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub pitch_degrees: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub scale: f64,
    pub high_contrast: bool,
    pub graphics_failure: Option<String>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            project_id: None,
            selected_id: None,
            active_view: ViewKind::ThreeD,
            hidden_ids: BTreeSet::new(),
            isolated_id: None,
            cut: None,
            center_m: [0.0; 3],
            span_m: 10.0,
            yaw_degrees: 45.0,
            pitch_degrees: 30.0,
            scale: 1.0,
            high_contrast: false,
            graphics_failure: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ViewCommand {
    SetView {
        view: ViewKind,
    },
    Select {
        object_id: Uuid,
        reveal: bool,
    },
    SelectNext {
        reverse: bool,
    },
    ClearSelection {},
    LocateSelected {},
    HideSelected {},
    IsolateSelected {},
    ShowAll {},
    SetCut {
        cut: Option<ViewCut>,
    },
    Fit {},
    Pan {
        #[serde(default)]
        view: ViewKind,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        right_m: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        up_m: f64,
    },
    Zoom {
        #[serde(deserialize_with = "crate::numeric::scalar")]
        factor: f64,
    },
    SetCamera {
        #[serde(deserialize_with = "crate::numeric::scalar")]
        yaw_degrees: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        pitch_degrees: f64,
    },
    SetPreferences {
        #[serde(deserialize_with = "crate::numeric::scalar")]
        scale: f64,
        high_contrast: bool,
    },
    GraphicsUnavailable {
        reason: String,
    },
    RestoreGraphics {},
}

#[derive(Clone, Debug, Serialize)]
pub struct ViewObject {
    pub id: Uuid,
    pub name: String,
    pub discipline: crate::Discipline,
    pub selected: bool,
    pub visible: bool,
    pub visibility_reason: String,
    pub start_m: Point3,
    pub end_m: Point3,
    pub analytical_ids: Vec<Uuid>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ViewReport {
    pub project_id: Uuid,
    pub source_revision: u64,
    pub snapshot_sha256: String,
    pub state: ViewState,
    pub objects: Vec<ViewObject>,
    pub views: Vec<ViewKind>,
    pub changes_project: bool,
    pub calculation_available: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectedSegment {
    pub object_id: Uuid,
    pub start_px: [f64; 2],
    pub end_px: [f64; 2],
    pub depth_m: f64,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectedGuide {
    pub label: String,
    pub start_px: [f64; 2],
    pub end_px: [f64; 2],
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderFrame {
    pub view: ViewKind,
    pub width_px: f64,
    pub height_px: f64,
    pub pixels_per_metre: f64,
    pub segments: Vec<ProjectedSegment>,
    pub guides: Vec<ProjectedGuide>,
}

fn invalid(message: &str) -> DomainError {
    DomainError::InvalidModel(message.into())
}

impl ViewState {
    pub fn validate(&self) -> Result<(), DomainError> {
        crate::validate_point(self.center_m)?;
        if !self.span_m.is_finite()
            || !(1.0e-6..=4.0e9).contains(&self.span_m)
            || !self.yaw_degrees.is_finite()
            || !(-360.0..=360.0).contains(&self.yaw_degrees)
            || !self.pitch_degrees.is_finite()
            || !(-89.0..=89.0).contains(&self.pitch_degrees)
            || !self.scale.is_finite()
            || !(0.75..=2.0).contains(&self.scale)
        {
            return Err(invalid(
                "Câmera ou escala da interface fora do intervalo permitido.",
            ));
        }
        if let Some(cut) = self.cut {
            cut.validate()?;
        }
        if let Some(reason) = &self.graphics_failure {
            crate::validate_name(reason)?;
        }
        Ok(())
    }

    /// Reopening another project must never leak selection or visibility state.
    pub fn synchronize(&mut self, project: &Project) {
        if self.project_id != Some(project.id) {
            let scale = self.scale;
            let high_contrast = self.high_contrast;
            *self = Self {
                project_id: Some(project.id),
                scale,
                high_contrast,
                ..Self::default()
            };
            self.fit(project, None);
        }
        let ids: BTreeSet<_> = project.objects.iter().map(|o| o.id).collect();
        self.hidden_ids.retain(|id| ids.contains(id));
        if self.selected_id.is_some_and(|id| !ids.contains(&id)) {
            self.selected_id = None;
        }
        if self.isolated_id.is_some_and(|id| !ids.contains(&id)) {
            self.isolated_id = None;
        }
    }

    pub fn apply(
        &mut self,
        project: &Project,
        command: ViewCommand,
    ) -> Result<ViewReport, DomainError> {
        let mut next = self.clone();
        next.synchronize(project);
        next.validate()?;
        match command {
            ViewCommand::SetView { view } => next.active_view = view,
            ViewCommand::Select { object_id, reveal } => {
                let object = project
                    .objects
                    .iter()
                    .find(|o| o.id == object_id)
                    .ok_or(DomainError::UnknownReference(object_id))?;
                next.selected_id = Some(object_id);
                if reveal {
                    for id in family_ids(project, object_id) {
                        next.hidden_ids.remove(&id);
                    }
                    if next.isolated_id.is_some_and(|id| id != object_id) {
                        next.isolated_id = None;
                    }
                    if !next.in_cut(project, object) {
                        next.cut = None;
                    }
                }
            }
            ViewCommand::SelectNext { reverse } => {
                if !project.objects.is_empty() {
                    let ids: Vec<_> = project.objects.iter().map(|o| o.id).collect();
                    let index = next
                        .selected_id
                        .and_then(|id| ids.iter().position(|v| *v == id));
                    let target = match (index, reverse) {
                        (Some(i), true) => (i + ids.len() - 1) % ids.len(),
                        (Some(i), false) => (i + 1) % ids.len(),
                        (None, true) => ids.len() - 1,
                        (None, false) => 0,
                    };
                    next.apply(
                        project,
                        ViewCommand::Select {
                            object_id: ids[target],
                            reveal: true,
                        },
                    )?;
                }
            }
            ViewCommand::ClearSelection {} => next.selected_id = None,
            ViewCommand::LocateSelected {} => {
                let id = next
                    .selected_id
                    .ok_or_else(|| invalid("Selecione uma peça para localizar."))?;
                next.apply(
                    project,
                    ViewCommand::Select {
                        object_id: id,
                        reveal: true,
                    },
                )?;
                next.fit(project, Some(id));
            }
            ViewCommand::HideSelected {} => {
                let id = next
                    .selected_id
                    .ok_or_else(|| invalid("Selecione uma peça para ocultar."))?;
                next.hidden_ids.extend(family_ids(project, id));
            }
            ViewCommand::IsolateSelected {} => {
                let id = next
                    .selected_id
                    .ok_or_else(|| invalid("Selecione uma peça para isolar."))?;
                next.apply(
                    project,
                    ViewCommand::Select {
                        object_id: id,
                        reveal: true,
                    },
                )?;
                next.isolated_id = Some(id);
                next.fit(project, Some(id));
            }
            ViewCommand::ShowAll {} => {
                next.hidden_ids.clear();
                next.isolated_id = None;
                next.cut = None;
                next.fit(project, None);
            }
            ViewCommand::SetCut { cut } => {
                if let Some(cut) = cut {
                    cut.validate()?;
                }
                next.cut = cut;
            }
            ViewCommand::Fit {} => next.fit(project, None),
            ViewCommand::Pan {
                view,
                right_m,
                up_m,
            } => {
                if !right_m.is_finite() || !up_m.is_finite() {
                    return Err(invalid(
                        "O deslocamento da vista deve ser finito e estar em metros.",
                    ));
                }
                let (right, up) = match view {
                    ViewKind::Plan => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                    ViewKind::Section => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
                    ViewKind::ThreeD => {
                        let (sy, cy) = next.yaw_degrees.to_radians().sin_cos();
                        let (sp, cp) = next.pitch_degrees.to_radians().sin_cos();
                        ([cy, -sy, 0.0], [-sp * sy, -sp * cy, cp])
                    }
                };
                for i in 0..3 {
                    next.center_m[i] += right[i] * right_m + up[i] * up_m;
                }
            }
            ViewCommand::Zoom { factor } => {
                if !factor.is_finite() || !(0.01..=100.0).contains(&factor) {
                    return Err(invalid("O fator de zoom deve estar entre 0,01 e 100."));
                }
                next.span_m = (next.span_m / factor).clamp(1.0e-6, 4.0e9);
            }
            ViewCommand::SetCamera {
                yaw_degrees,
                pitch_degrees,
            } => {
                next.yaw_degrees = yaw_degrees;
                next.pitch_degrees = pitch_degrees;
            }
            ViewCommand::SetPreferences {
                scale,
                high_contrast,
            } => {
                next.scale = scale;
                next.high_contrast = high_contrast;
            }
            ViewCommand::GraphicsUnavailable { reason } => {
                crate::validate_name(&reason)?;
                next.graphics_failure = Some(reason);
            }
            ViewCommand::RestoreGraphics {} => {
                next.graphics_failure = None;
                next.fit(project, None);
            }
        }
        next.validate()?;
        let report = next.report(project)?;
        *self = next;
        Ok(report)
    }

    fn in_cut(&self, project: &Project, object: &PhysicalObject) -> bool {
        self.cut.is_none_or(|cut| {
            let mut points = object_polylines(project, object)
                .unwrap_or_else(|_| vec![vec![object.start_m, object.end_m]]);
            if points.is_empty() {
                points.push(vec![object.start_m, object.end_m]);
            }
            let axis = cut.index();
            let lo = points
                .iter()
                .flatten()
                .map(|p| p[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = points
                .iter()
                .flatten()
                .map(|p| p[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            lo <= cut.max_m && hi >= cut.min_m
        })
    }

    pub fn visibility(&self, project: &Project, object: &PhysicalObject) -> (bool, &'static str) {
        if self.hidden_ids.contains(&object.id) {
            (false, "Oculto pelo usuário")
        } else if self
            .isolated_id
            .is_some_and(|id| !family_ids(project, id).contains(&object.id))
        {
            (false, "Fora do isolamento")
        } else if !self.in_cut(project, object) {
            (false, "Fora do intervalo de corte")
        } else {
            (true, "Visível")
        }
    }

    pub fn report(&self, project: &Project) -> Result<ViewReport, DomainError> {
        self.validate()?;
        if self.project_id.is_some_and(|id| id != project.id) {
            return Err(invalid("O estado da vista pertence a outro projeto."));
        }
        Ok(ViewReport {
            project_id: project.id,
            source_revision: project.revision,
            snapshot_sha256: crate::canonical::model_snapshot(project)?.sha256,
            state: self.clone(),
            objects: project
                .objects
                .iter()
                .map(|object| {
                    let (visible, reason) = self.visibility(project, object);
                    ViewObject {
                        id: object.id,
                        name: object.name.clone(),
                        discipline: object.discipline,
                        selected: self.selected_id == Some(object.id),
                        visible,
                        visibility_reason: reason.into(),
                        start_m: object.start_m,
                        end_m: object.end_m,
                        analytical_ids: project
                            .model
                            .links
                            .iter()
                            .filter(|l| l.physical_id == object.id)
                            .map(|l| l.analytical_id)
                            .collect(),
                        status: if object.role == crate::model::PhysicalRole::VisualOnly {
                            "Referência visual, sem comportamento resistente"
                        } else {
                            "Modelado, não calculado"
                        }
                        .into(),
                    }
                })
                .collect(),
            views: vec![ViewKind::Plan, ViewKind::Section, ViewKind::ThreeD],
            changes_project: false,
            calculation_available: false,
        })
    }

    fn fit(&mut self, project: &Project, object_id: Option<Uuid>) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for object in &project.objects {
            if object_id.is_some_and(|id| !family_ids(project, id).contains(&object.id))
                || !self.visibility(project, object).0
            {
                continue;
            }
            for point in object_polylines(project, object)
                .unwrap_or_else(|_| vec![vec![object.start_m, object.end_m]])
                .into_iter()
                .flatten()
            {
                let p = point.as_array();
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
            }
        }
        if object_id.is_none()
            && let Ok(layout) = crate::layout::state(project)
        {
            for axis in &layout.axes {
                if let Ok(axis) = crate::layout::resolve_axis(project, axis.id) {
                    for t in [0.0, axis.extent_m] {
                        let p: [f64; 3] =
                            std::array::from_fn(|i| axis.origin_m[i] + t * axis.direction[i]);
                        for i in 0..3 {
                            lo[i] = lo[i].min(p[i]);
                            hi[i] = hi[i].max(p[i]);
                        }
                    }
                }
            }
            for level in &layout.levels {
                if !lo[0].is_finite() {
                    lo = [-5.0, -5.0, level.elevation_m];
                    hi = [5.0, 5.0, level.elevation_m];
                }
                lo[2] = lo[2].min(level.elevation_m);
                hi[2] = hi[2].max(level.elevation_m);
            }
        }
        if lo[0].is_finite() {
            for i in 0..3 {
                self.center_m[i] = lo[i] + (hi[i] - lo[i]) * 0.5;
            }
            self.span_m = ((hi[0] - lo[0]).hypot(hi[1] - lo[1]).hypot(hi[2] - lo[2]) * 1.35)
                .clamp(1.0, 4.0e9);
        }
    }

    pub fn frame(
        &self,
        project: &Project,
        kind: ViewKind,
        width_px: f64,
        height_px: f64,
    ) -> Result<RenderFrame, DomainError> {
        self.validate()?;
        if !width_px.is_finite()
            || !height_px.is_finite()
            || !(1.0..=100_000.0).contains(&width_px)
            || !(1.0..=100_000.0).contains(&height_px)
        {
            return Err(invalid(
                "As dimensões da vista são inválidas. Restaure a vista para continuar.",
            ));
        }
        if self.project_id.is_some_and(|id| id != project.id) {
            return Err(invalid("A vista pertence a outro projeto."));
        }
        let scale = (width_px.min(height_px) - 32.0).max(1.0) / self.span_m;
        let mut segments = Vec::new();
        let selected_ids = self
            .selected_id
            .map(|id| family_ids(project, id))
            .unwrap_or_default();
        for object in &project.objects {
            if !self.visibility(project, object).0 {
                continue;
            }
            for line in object_polylines(project, object)? {
                for pair in line.windows(2) {
                    let start = pair[0].as_array();
                    let end = pair[1].as_array();
                    let (start, end) = match self.cut {
                        Some(cut) => match clip_segment(start, end, cut) {
                            Some(pair) => pair,
                            None => continue,
                        },
                        None => (start, end),
                    };
                    let a = self.project_point(start, kind);
                    let b = self.project_point(end, kind);
                    segments.push(ProjectedSegment {
                        object_id: object.id,
                        start_px: [
                            width_px * 0.5 + a[0] * scale,
                            height_px * 0.5 - a[1] * scale,
                        ],
                        end_px: [
                            width_px * 0.5 + b[0] * scale,
                            height_px * 0.5 - b[1] * scale,
                        ],
                        depth_m: (a[2] + b[2]) * 0.5,
                        selected: selected_ids.contains(&object.id),
                    });
                }
            }
        }

        // Far geometry first. Selection is additionally labelled and outlined by the renderer.
        segments.sort_by(|a, b| {
            a.depth_m
                .total_cmp(&b.depth_m)
                .then(a.object_id.cmp(&b.object_id))
        });
        let mut guides = Vec::new();
        let layout = crate::layout::state(project)?;
        let screen = |p: [f64; 3]| {
            let p = self.project_point(p, kind);
            [
                width_px * 0.5 + p[0] * scale,
                height_px * 0.5 - p[1] * scale,
            ]
        };
        if kind != ViewKind::Section {
            for axis in &layout.axes {
                let axis = crate::layout::resolve_axis(project, axis.id)?;
                let end =
                    std::array::from_fn(|i| axis.origin_m[i] + axis.extent_m * axis.direction[i]);
                guides.push(ProjectedGuide {
                    label: format!("Eixo {}", axis.name),
                    start_px: screen(axis.origin_m.as_array()),
                    end_px: screen(end),
                });
            }
        }
        if kind != ViewKind::Plan {
            for level in &layout.levels {
                guides.push(ProjectedGuide {
                    label: format!("{}: {:.3} m", level.name, level.elevation_m),
                    start_px: screen([
                        self.center_m[0] - self.span_m * 0.45,
                        self.center_m[1],
                        level.elevation_m,
                    ]),
                    end_px: screen([
                        self.center_m[0] + self.span_m * 0.45,
                        self.center_m[1],
                        level.elevation_m,
                    ]),
                });
            }
        }
        Ok(RenderFrame {
            view: kind,
            width_px,
            height_px,
            pixels_per_metre: scale,
            segments,
            guides,
        })
    }

    pub fn project_point(&self, point: [f64; 3], kind: ViewKind) -> [f64; 3] {
        let p = [
            point[0] - self.center_m[0],
            point[1] - self.center_m[1],
            point[2] - self.center_m[2],
        ];
        match kind {
            ViewKind::Plan => [p[0], p[1], p[2]],
            ViewKind::Section => [p[0], p[2], -p[1]],
            ViewKind::ThreeD => {
                let (sy, cy) = self.yaw_degrees.to_radians().sin_cos();
                let (sp, cp) = self.pitch_degrees.to_radians().sin_cos();
                [
                    cy * p[0] - sy * p[1],
                    -sp * sy * p[0] - sp * cy * p[1] + cp * p[2],
                    cp * sy * p[0] + cp * cy * p[1] + sp * p[2],
                ]
            }
        }
    }
}

impl RenderFrame {
    /// Returns all candidates rather than silently losing overlapping objects.
    /// Radius is expressed in logical pixels, independent of display density.
    pub fn pick_candidates(
        &self,
        point: [f64; 2],
        radius_px: f64,
    ) -> Result<Vec<Uuid>, DomainError> {
        if point.iter().any(|v| !v.is_finite())
            || !radius_px.is_finite()
            || !(0.5..=64.0).contains(&radius_px)
        {
            return Err(invalid("Ponto ou raio de seleção inválido."));
        }
        let mut hits: Vec<_> = self
            .segments
            .iter()
            .filter_map(|s| {
                let d = segment_distance(point, s.start_px, s.end_px);
                (d <= radius_px).then_some((d, -s.depth_m, s.object_id))
            })
            .collect();
        hits.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.cmp(&b.2))
        });
        let mut seen = BTreeSet::new();
        Ok(hits
            .into_iter()
            .filter_map(|(_, _, id)| seen.insert(id).then_some(id))
            .collect())
    }
}

fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = d[0] * d[0] + d[1] * d[1];
    let t = if len > 0.0 {
        ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

fn clip_segment(a: [f64; 3], b: [f64; 3], cut: ViewCut) -> Option<([f64; 3], [f64; 3])> {
    let i = cut.index();
    let d = b[i] - a[i];
    if d.abs() < f64::EPSILON {
        return (a[i] >= cut.min_m && a[i] <= cut.max_m).then_some((a, b));
    }
    let t1 = (cut.min_m - a[i]) / d;
    let t2 = (cut.max_m - a[i]) / d;
    let lo = t1.min(t2).max(0.0);
    let hi = t1.max(t2).min(1.0);
    if lo > hi {
        None
    } else {
        Some((
            std::array::from_fn(|j| a[j] + lo * (b[j] - a[j])),
            std::array::from_fn(|j| a[j] + hi * (b[j] - a[j])),
        ))
    }
}

/// Every parametric contour uses the same physical object identity as the tree.
fn object_polylines(
    project: &Project,
    object: &PhysicalObject,
) -> Result<Vec<Vec<Point3>>, DomainError> {
    if object.extensions.contains_key(crate::framing::FRAMING_KEY) {
        return crate::framing::display_polylines(project, object.id);
    }
    if object.extensions.contains_key(crate::surfaces::SURFACE_KEY) {
        return crate::surfaces::display_polylines(project, object.id);
    }
    if object
        .extensions
        .contains_key(crate::assemblies::ASSEMBLY_KEY)
    {
        return crate::assemblies::display_polylines(project, object.id);
    }
    Ok(vec![vec![object.start_m, object.end_m]])
}

fn family_ids(project: &Project, id: Uuid) -> BTreeSet<Uuid> {
    let mut ids = BTreeSet::from([id]);
    if let Some(object) = project.objects.iter().find(|object| object.id == id)
        && let Ok(Some(assembly)) = crate::assemblies::metadata(object)
    {
        ids.extend(assembly.parts.iter().map(|part| part.object_id));
    }
    ids
}
