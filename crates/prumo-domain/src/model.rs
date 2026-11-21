//! Physical semantics, analytical topology and explicit engineering coordinates.
pub use crate::identity::{EntityIdentity, SourceOrigin};
pub use crate::units::{
    Density, DensityInput, Force3, ForceInput, Moment3, MomentInput, Point3, PointInput, Pressure,
    PressureInput,
};
use crate::{Discipline, DomainError, LengthUnit, PhysicalObject, PortKind, Project};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalRole {
    VisualOnly,
    AnalyticalSource,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkKind {
    #[default]
    Network,
    Circuit,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialFamily {
    Concrete,
    Steel,
    Timber,
    Generic,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalNode {
    pub id: Uuid,
    pub name: String,
    pub position_m: Point3,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalMember {
    pub id: Uuid,
    pub name: String,
    pub start_node_id: Uuid,
    pub end_node_id: Uuid,
    #[serde(deserialize_with = "crate::numeric::vector3")]
    pub local_y: [f64; 3],
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalAnalyticalLink {
    pub id: Uuid,
    pub physical_id: Uuid,
    pub analytical_id: Uuid,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    pub id: Uuid,
    pub name: String,
    pub family: MaterialFamily,
    pub elastic_modulus: Pressure,
    pub density: Density,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub poisson_ratio: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialInput {
    pub name: String,
    pub family: MaterialFamily,
    pub elastic_modulus: PressureInput,
    pub density: DensityInput,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub poisson_ratio: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionFrame {
    Engineering {},
    MemberLocal { member_id: Uuid },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodalAction {
    pub id: Uuid,
    pub name: String,
    pub application_node_id: Uuid,
    pub force: Force3,
    pub couple: Moment3,
    pub frame: ActionFrame,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodalActionInput {
    pub name: String,
    pub application_node_id: Uuid,
    pub force: ForceInput,
    pub couple: MomentInput,
    pub frame: ActionFrame,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringFrame {
    pub origin_world_m: Point3,
    /// Row-major proper rotation mapping engineering coordinates to an abstract world.
    #[serde(deserialize_with = "crate::numeric::matrix3")]
    pub rotation_to_world: [[f64; 3]; 3],
}
impl Default for EngineeringFrame {
    fn default() -> Self {
        Self {
            origin_world_m: Point3::ORIGIN,
            rotation_to_world: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Georeference {
    pub crs_identifier: String,
    pub map_origin_m: Point3,
    pub rotation_radians: crate::units::Angle,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub scale: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    Copy,
    Merge,
    Split,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LineageRecord {
    pub id: Uuid,
    pub kind: LineageKind,
    pub parent_ids: Vec<Uuid>,
    pub child_ids: Vec<Uuid>,
    pub revision: u64,
    pub actor: String,
    pub retired_objects: Vec<PhysicalObject>,
    pub retired_links: Vec<PhysicalAnalyticalLink>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticModel {
    pub analytical_nodes: Vec<AnalyticalNode>,
    pub analytical_members: Vec<AnalyticalMember>,
    pub links: Vec<PhysicalAnalyticalLink>,
    pub materials: Vec<Material>,
    pub actions: Vec<NodalAction>,
    pub identities: BTreeMap<Uuid, EntityIdentity>,
    pub lineage: Vec<LineageRecord>,
    pub engineering_frame: EngineeringFrame,
    pub georeference: Option<Georeference>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModelEdit {
    EditPrecalculation {
        command: crate::precalculation::PrecalculationEdit,
    },
    EditLayout {
        command: crate::layout::LayoutEdit,
    },
    EditFraming {
        command: crate::framing::FramingEdit,
    },
    EditSurfaces {
        command: crate::surfaces::SurfaceEdit,
    },
    EditAssemblies {
        command: crate::assemblies::AssemblyEdit,
    },
    PutCatalogEntry {
        entry: crate::catalogue::CatalogEntry,
    },
    AssignCatalogEntry {
        object_id: Uuid,
        entry_id: Uuid,
    },
    ClearCatalogAssignment {
        object_id: Uuid,
        kind: crate::catalogue::CatalogKind,
    },
    SetGeometryTolerances {
        tolerances: crate::geometry::TolerancePolicy,
    },
    ApplyGeometryRepair {
        plan_hash: String,
        repairs: Vec<crate::geometry::GeometryRepair>,
    },
    AddPhysical {
        name: String,
        discipline: Discipline,
        start: PointInput,
        end: PointInput,
        role: PhysicalRole,
        material_id: Option<Uuid>,
        environment_id: Option<Uuid>,
    },
    SetPhysicalGeometry {
        object_id: Uuid,
        start: PointInput,
        end: PointInput,
    },
    SetPhysicalRole {
        object_id: Uuid,
        role: PhysicalRole,
    },
    SetPhysicalMaterial {
        object_id: Uuid,
        #[serde(deserialize_with = "required_option")]
        material_id: Option<Uuid>,
    },
    SetDisplayUnit {
        unit: LengthUnit,
    },
    AddAnalyticalNode {
        name: String,
        position: PointInput,
    },
    AddAnalyticalMember {
        name: String,
        start_node_id: Uuid,
        end_node_id: Uuid,
        #[serde(deserialize_with = "crate::numeric::vector3")]
        local_y: [f64; 3],
    },
    LinkPhysicalAnalytical {
        physical_id: Uuid,
        analytical_id: Uuid,
    },
    UnlinkPhysicalAnalytical {
        link_id: Uuid,
    },
    AddMaterial {
        material: MaterialInput,
    },
    SetMaterial {
        material_id: Uuid,
        material: MaterialInput,
    },
    AddNodalAction {
        action: NodalActionInput,
    },
    SetNodalAction {
        action_id: Uuid,
        action: NodalActionInput,
    },
    AddNetwork {
        name: String,
        discipline: Discipline,
        kind: NetworkKind,
        environment_id: Option<Uuid>,
    },
    AddPort {
        object_id: Uuid,
        discipline: Discipline,
        kind: PortKind,
        position: PointInput,
    },
    AssignPort {
        network_id: Uuid,
        port_id: Uuid,
    },
    ConnectPorts {
        network_id: Uuid,
        from_port: Uuid,
        to_port: Uuid,
    },
    DisconnectPorts {
        connection_id: Uuid,
    },
    CopyPhysical {
        source_id: Uuid,
        name: String,
        translation: PointInput,
    },
    MergePhysical {
        source_ids: Vec<Uuid>,
        name: String,
    },
    SplitPhysical {
        object_id: Uuid,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        fraction: f64,
        names: [String; 2],
    },
    RigidMove {
        object_ids: Vec<Uuid>,
        #[serde(deserialize_with = "crate::numeric::matrix3")]
        rotation: [[f64; 3]; 3],
        translation: PointInput,
    },
    ChangeEngineeringFrame {
        frame: EngineeringFrame,
    },
    SetGeoreference {
        #[serde(deserialize_with = "required_option")]
        georeference: Option<Georeference>,
    },
    SetSourceOrigin {
        entity_id: Uuid,
        origin: SourceOrigin,
    },
}

// Update commands distinguish an explicit removal (null) from a missing field.
// A custom deserializer without a default makes this field required in Serde.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ModelChange {
    pub created_ids: Vec<Uuid>,
    pub changed_ids: Vec<Uuid>,
    pub retired_ids: Vec<Uuid>,
}

// Validation, migration and editing are implemented below. Only the application
// service stamps the project revision/history and persists a completed edit.
#[path = "model_validation.rs"]
mod validation;
pub use validation::{
    force_and_couple_in_engineering, member_axes, resultant_about, validate_rotation,
    validate_semantics,
};
#[path = "model_edit.rs"]
mod editing;
pub use editing::{apply_model_edit, initialize_legacy_member, record_identity};
#[path = "model_legacy.rs"]
mod legacy;
pub use legacy::decode_project;
