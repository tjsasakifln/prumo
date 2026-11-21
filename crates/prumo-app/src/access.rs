//! Authorization selected by the trusted host, never loaded from project data.
use crate::{AccessMode, AppError};
use prumo_domain::model::ModelEdit;
use prumo_domain::{
    Command, Connection, Discipline, LengthUnit, Network, PhysicalObject, Port, Project,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessScope {
    Read,
    Model,
    Calculation,
    Documents,
}

/// An in-memory host grant. Deserializing a model or a catalogue never creates one.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectGrant {
    pub project_id: Option<Uuid>,
    pub scopes: Vec<AccessScope>,
    /// None means all disciplines; an empty set grants no discipline.
    pub disciplines: Option<Vec<Discipline>>,
}

/// Deliberately not a Project: histories, catalogues, identities and generic
/// extensions can refer to other disciplines and are absent from this view.
#[derive(Clone, Debug, Serialize)]
pub struct ScopedModelView {
    pub project_id: Uuid,
    pub revision: u64,
    pub display_unit: LengthUnit,
    pub position_unit: &'static str,
    pub disciplines: Option<Vec<Discipline>>,
    pub objects: Vec<PhysicalObject>,
    pub networks: Vec<Network>,
    pub ports: Vec<Port>,
    pub connections: Vec<Connection>,
}

impl ProjectGrant {
    pub fn for_access(access: AccessMode) -> Self {
        let mut scopes = vec![AccessScope::Read];
        if access == AccessMode::ReadWrite {
            scopes.push(AccessScope::Model);
        }
        Self {
            project_id: None,
            scopes,
            disciplines: None,
        }
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if self.project_id.is_some_and(|id| id.is_nil())
            || self
                .scopes
                .iter()
                .enumerate()
                .any(|(i, item)| self.scopes[..i].contains(item))
            || self.disciplines.as_ref().is_some_and(|items| {
                items
                    .iter()
                    .enumerate()
                    .any(|(i, item)| items[..i].contains(item))
            })
            || (self.scopes.iter().any(|scope| *scope != AccessScope::Read)
                && !self.allows(AccessScope::Read))
        {
            return Err(AppError::InvalidInput("Concessão de acesso inválida. Selecione escopos e disciplinas únicos; operação exige leitura.".into()));
        }
        Ok(())
    }

    pub fn allows(&self, scope: AccessScope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn all_disciplines(&self) -> bool {
        self.disciplines.as_ref().is_none_or(|items| {
            [
                Discipline::Structure,
                Discipline::WaterSanitation,
                Discipline::Gas,
                Discipline::FireProtection,
                Discipline::Electrical,
                Discipline::Photovoltaic,
                Discipline::LightningProtection,
                Discipline::Cabling,
                Discipline::Hvac,
            ]
            .iter()
            .all(|discipline| items.contains(discipline))
        })
    }

    pub fn allows_discipline(&self, discipline: Discipline) -> bool {
        self.disciplines
            .as_ref()
            .is_none_or(|items| items.contains(&discipline))
    }

    pub fn check_scope(&self, scope: AccessScope) -> Result<(), AppError> {
        if self.allows(scope) {
            Ok(())
        } else {
            Err(AppError::PermissionDenied)
        }
    }

    pub fn check_project(&self, project: &Project) -> Result<(), AppError> {
        if self.project_id.is_some_and(|id| id != project.id) {
            Err(AppError::PermissionDenied)
        } else {
            Ok(())
        }
    }

    pub fn check_full_read(&self, project: &Project) -> Result<(), AppError> {
        self.check_scope(AccessScope::Read)?;
        self.check_project(project)?;
        self.check_all_disciplines()
    }

    pub fn check_all_disciplines(&self) -> Result<(), AppError> {
        if self.all_disciplines() {
            Ok(())
        } else {
            Err(AppError::PermissionDenied)
        }
    }

    pub fn check_discipline(&self, discipline: Discipline) -> Result<(), AppError> {
        if self.allows_discipline(discipline) {
            Ok(())
        } else {
            Err(AppError::PermissionDenied)
        }
    }

    pub fn check_command(&self, project: &Project, command: &Command) -> Result<(), AppError> {
        self.check_scope(AccessScope::Read)?;
        self.check_scope(AccessScope::Model)?;
        self.check_project(project)?;
        match command {
            Command::EditModel { edit, .. } => self.check_model_edit(project, edit),
            Command::AddMember { .. } => self.check_discipline(Discipline::Structure),
            _ => self.check_all_disciplines(),
        }
    }

    pub fn check_model_edit(&self, project: &Project, edit: &ModelEdit) -> Result<(), AppError> {
        self.check_scope(AccessScope::Read)?;
        self.check_scope(AccessScope::Model)?;
        self.check_project(project)?;
        match edit {
            ModelEdit::EditLayout { .. }
            | ModelEdit::EditPrecalculation { .. } => self.check_all_disciplines(),
            ModelEdit::EditFraming { .. }
            | ModelEdit::EditSurfaces { .. }
            | ModelEdit::EditAssemblies { .. } => self.check_discipline(Discipline::Structure),
            ModelEdit::AddPhysical { discipline, .. }
            | ModelEdit::AddNetwork { discipline, .. } => self.check_discipline(*discipline),
            ModelEdit::SetPhysicalGeometry { object_id, .. }
            | ModelEdit::SetPhysicalRole { object_id, .. }
            | ModelEdit::SetPhysicalMaterial { object_id, .. }
            | ModelEdit::AssignCatalogEntry { object_id, .. }
            | ModelEdit::ClearCatalogAssignment { object_id, .. }
            | ModelEdit::SplitPhysical { object_id, .. } => self.check_object(project, *object_id),
            ModelEdit::CopyPhysical { source_id, .. } => self.check_object(project, *source_id),
            ModelEdit::MergePhysical { source_ids, .. } => source_ids
                .iter()
                .try_for_each(|id| self.check_object(project, *id)),
            ModelEdit::RigidMove { object_ids, .. } => object_ids
                .iter()
                .try_for_each(|id| self.check_object(project, *id)),
            ModelEdit::AddAnalyticalNode { .. }
            | ModelEdit::AddAnalyticalMember { .. }
            | ModelEdit::AddNodalAction { .. }
            | ModelEdit::SetNodalAction { .. } => self.check_discipline(Discipline::Structure),
            ModelEdit::LinkPhysicalAnalytical { physical_id, .. } => {
                self.check_discipline(Discipline::Structure)?;
                self.check_object(project, *physical_id)
            }
            ModelEdit::UnlinkPhysicalAnalytical { link_id } => {
                self.check_discipline(Discipline::Structure)?;
                let link = project
                    .model
                    .links
                    .iter()
                    .find(|link| link.id == *link_id)
                    .ok_or(AppError::PermissionDenied)?;
                self.check_object(project, link.physical_id)
            }
            ModelEdit::AddPort {
                object_id,
                discipline,
                ..
            } => {
                self.check_discipline(*discipline)?;
                self.check_object(project, *object_id)
            }
            ModelEdit::AssignPort {
                network_id,
                port_id,
            } => {
                self.check_network(project, *network_id)?;
                self.check_port(project, *port_id)
            }
            ModelEdit::ConnectPorts {
                network_id,
                from_port,
                to_port,
            } => {
                self.check_network(project, *network_id)?;
                self.check_port(project, *from_port)?;
                self.check_port(project, *to_port)
            }
            ModelEdit::DisconnectPorts { connection_id } => {
                let connection = project
                    .connections
                    .iter()
                    .find(|connection| connection.id == *connection_id)
                    .ok_or(AppError::PermissionDenied)?;
                self.check_network(project, connection.network_id)?;
                self.check_port(project, connection.from_port)?;
                self.check_port(project, connection.to_port)
            }
            // Shared metadata and materials can affect every discipline.
            ModelEdit::SetDisplayUnit { .. }
            | ModelEdit::PutCatalogEntry { .. }
            | ModelEdit::AddMaterial { .. }
            | ModelEdit::SetMaterial { .. }
            | ModelEdit::ChangeEngineeringFrame { .. }
            | ModelEdit::SetGeoreference { .. }
            | ModelEdit::SetSourceOrigin { .. }
            | ModelEdit::SetGeometryTolerances { .. }
            | ModelEdit::ApplyGeometryRepair { .. } => self.check_all_disciplines(),
        }
    }

    /// Defense in depth for batches, undo/redo and indirect cross-discipline effects.
    pub fn check_delta(&self, before: &Project, after: &Project) -> Result<(), AppError> {
        self.check_scope(AccessScope::Read)?;
        self.check_scope(AccessScope::Model)?;
        self.check_project(before)?;
        self.check_project(after)?;
        if before.id != after.id || before.schema_version != after.schema_version {
            return Err(AppError::PermissionDenied);
        }
        if self.all_disciplines() {
            return Ok(());
        }
        if before.name != after.name
            || before.units != after.units
            || before.environments != after.environments
            || before.normative != after.normative
            || before.extensions != after.extensions
            || before.model.materials != after.model.materials
            || before.model.engineering_frame != after.model.engineering_frame
            || before.model.georeference != after.model.georeference
        {
            return Err(AppError::PermissionDenied);
        }
        for object in changed(&before.objects, &after.objects, |item| item.id) {
            self.check_discipline(object.discipline)?;
        }
        for network in changed(&before.networks, &after.networks, |item| item.id) {
            self.check_discipline(network.discipline)?;
        }
        for port in changed(&before.ports, &after.ports, |item| item.id) {
            self.check_discipline(port.discipline)?;
            self.check_entity_either(before, after, port.object_id)?;
        }
        for connection in changed(&before.connections, &after.connections, |item| item.id) {
            self.check_entity_either(before, after, connection.network_id)?;
            self.check_entity_either(before, after, connection.from_port)?;
            self.check_entity_either(before, after, connection.to_port)?;
        }
        if before.model.analytical_nodes != after.model.analytical_nodes
            || before.model.analytical_members != after.model.analytical_members
            || before.model.actions != after.model.actions
        {
            self.check_discipline(Discipline::Structure)?;
            // Analytical edits can invalidate linked physical entities in another discipline.
            for project in [before, after] {
                for link in &project.model.links {
                    self.check_object(project, link.physical_id)?;
                }
            }
        }
        for link in changed(&before.model.links, &after.model.links, |item| item.id) {
            self.check_discipline(Discipline::Structure)?;
            self.check_entity_either(before, after, link.physical_id)?;
        }
        for id in before
            .model
            .identities
            .keys()
            .chain(after.model.identities.keys())
        {
            if before.model.identities.get(id) != after.model.identities.get(id) {
                self.check_entity_either(before, after, *id)?;
            }
        }
        for lineage in changed(&before.model.lineage, &after.model.lineage, |item| item.id) {
            for object in &lineage.retired_objects {
                self.check_discipline(object.discipline)?;
            }
            for id in lineage.parent_ids.iter().chain(&lineage.child_ids) {
                if !lineage
                    .retired_objects
                    .iter()
                    .any(|object| object.id == *id)
                {
                    self.check_entity_either(before, after, *id)?;
                }
            }
            if !lineage.retired_links.is_empty() {
                self.check_discipline(Discipline::Structure)?;
            }
        }
        Ok(())
    }

    pub fn project_view(&self, project: &Project) -> Result<ScopedModelView, AppError> {
        self.check_scope(AccessScope::Read)?;
        self.check_project(project)?;
        let mut objects: Vec<_> = project
            .objects
            .iter()
            .filter(|object| self.allows_discipline(object.discipline))
            .cloned()
            .collect();
        for object in &mut objects {
            object.extensions.clear();
            object.material_id = None;
            object.environment_id = None;
        }
        objects.sort_by_key(|object| object.id);
        let object_ids: BTreeSet<_> = objects.iter().map(|object| object.id).collect();
        let mut ports: Vec<_> = project
            .ports
            .iter()
            .filter(|port| {
                self.allows_discipline(port.discipline) && object_ids.contains(&port.object_id)
            })
            .cloned()
            .collect();
        for port in &mut ports {
            port.extensions.clear();
        }
        ports.sort_by_key(|port| port.id);
        let port_ids: BTreeSet<_> = ports.iter().map(|port| port.id).collect();
        let mut networks: Vec<_> = project
            .networks
            .iter()
            .filter(|network| self.allows_discipline(network.discipline))
            .cloned()
            .collect();
        for network in &mut networks {
            network.extensions.clear();
            network.environment_id = None;
            network.port_ids.retain(|id| port_ids.contains(id));
        }
        networks.sort_by_key(|network| network.id);
        let network_ids: BTreeSet<_> = networks.iter().map(|network| network.id).collect();
        let mut connections: Vec<_> = project
            .connections
            .iter()
            .filter(|connection| {
                network_ids.contains(&connection.network_id)
                    && port_ids.contains(&connection.from_port)
                    && port_ids.contains(&connection.to_port)
            })
            .cloned()
            .collect();
        for connection in &mut connections {
            connection.extensions.clear();
        }
        connections.sort_by_key(|connection| connection.id);
        Ok(ScopedModelView {
            project_id: project.id,
            revision: project.revision,
            display_unit: project.units,
            position_unit: "m",
            disciplines: self.disciplines.clone(),
            objects,
            networks,
            ports,
            connections,
        })
    }

    fn check_object(&self, project: &Project, id: Uuid) -> Result<(), AppError> {
        self.check_discipline(
            project
                .objects
                .iter()
                .find(|object| object.id == id)
                .ok_or(AppError::PermissionDenied)?
                .discipline,
        )
    }

    fn check_network(&self, project: &Project, id: Uuid) -> Result<(), AppError> {
        self.check_discipline(
            project
                .networks
                .iter()
                .find(|network| network.id == id)
                .ok_or(AppError::PermissionDenied)?
                .discipline,
        )
    }

    fn check_port(&self, project: &Project, id: Uuid) -> Result<(), AppError> {
        let port = project
            .ports
            .iter()
            .find(|port| port.id == id)
            .ok_or(AppError::PermissionDenied)?;
        self.check_discipline(port.discipline)?;
        self.check_object(project, port.object_id)
    }

    fn check_entity_either(
        &self,
        before: &Project,
        after: &Project,
        id: Uuid,
    ) -> Result<(), AppError> {
        let mut found = false;
        for project in [before, after] {
            if project.objects.iter().any(|object| object.id == id) {
                found = true;
                self.check_object(project, id)?;
            } else if project.networks.iter().any(|network| network.id == id) {
                found = true;
                self.check_network(project, id)?;
            } else if project.ports.iter().any(|port| port.id == id) {
                found = true;
                self.check_port(project, id)?;
            } else if let Some(link) = project.model.links.iter().find(|link| link.id == id) {
                found = true;
                self.check_discipline(Discipline::Structure)?;
                self.check_object(project, link.physical_id)?;
            } else if let Some(connection) = project
                .connections
                .iter()
                .find(|connection| connection.id == id)
            {
                found = true;
                self.check_network(project, connection.network_id)?;
                self.check_port(project, connection.from_port)?;
                self.check_port(project, connection.to_port)?;
            } else if project
                .model
                .analytical_nodes
                .iter()
                .any(|node| node.id == id)
                || project
                    .model
                    .analytical_members
                    .iter()
                    .any(|member| member.id == id)
                || project.model.actions.iter().any(|action| action.id == id)
                || prumo_domain::surfaces::analytical_region_ids(project)
                    .map_err(crate::invalid_input)?
                    .contains(&id)
            {
                found = true;
                self.check_discipline(Discipline::Structure)?;
            }
        }
        if found {
            Ok(())
        } else {
            self.check_all_disciplines()
        }
    }
}

fn changed<'a, T: PartialEq>(
    before: &'a [T],
    after: &'a [T],
    id: impl Fn(&T) -> Uuid,
) -> Vec<&'a T> {
    let old: BTreeMap<_, _> = before.iter().map(|item| (id(item), item)).collect();
    let new: BTreeMap<_, _> = after.iter().map(|item| (id(item), item)).collect();
    before
        .iter()
        .filter(|item| new.get(&id(item)).is_none_or(|existing| *existing != *item))
        .chain(
            after
                .iter()
                .filter(|item| old.get(&id(item)).is_none_or(|existing| *existing != *item)),
        )
        .collect()
}
