use super::*;
use crate::identity::IfcGuid;
use crate::{Connection, ModelState, Network, Port, validate_name};
use nalgebra::{Matrix3, Vector3};
use std::collections::BTreeSet;

pub fn record_identity(
    project: &mut Project,
    id: Uuid,
    actor: &str,
    revision: u64,
    with_ifc_guid: bool,
) -> Result<(), DomainError> {
    validate_name(actor)?;
    if id.is_nil() || project.model.identities.contains_key(&id) {
        return Err(DomainError::DuplicateIdentity(id));
    }
    project.model.identities.insert(
        id,
        EntityIdentity {
            ifc_guid: with_ifc_guid.then(|| IfcGuid::from_uuid(Uuid::new_v4())),
            origin: SourceOrigin::Native {},
            created_revision: Some(revision),
            created_by: Some(actor.into()),
            retired_revision: None,
        },
    );
    Ok(())
}

fn retire(project: &mut Project, id: Uuid, revision: u64) -> Result<(), DomainError> {
    project
        .model
        .identities
        .get_mut(&id)
        .ok_or(DomainError::UnknownReference(id))?
        .retired_revision = Some(revision);
    Ok(())
}
fn physical(project: &Project, id: Uuid) -> Result<&PhysicalObject, DomainError> {
    project
        .objects
        .iter()
        .find(|x| x.id == id)
        .ok_or(DomainError::UnknownReference(id))
}
fn physical_mut(project: &mut Project, id: Uuid) -> Result<&mut PhysicalObject, DomainError> {
    project
        .objects
        .iter_mut()
        .find(|x| x.id == id)
        .ok_or(DomainError::UnknownReference(id))
}
fn matrix(value: [[f64; 3]; 3]) -> Matrix3<f64> {
    Matrix3::from_row_slice(&value.concat())
}
fn array(value: Vector3<f64>) -> [f64; 3] {
    [value.x, value.y, value.z]
}
fn point(value: Vector3<f64>) -> Result<Point3, DomainError> {
    Point3::try_from(array(value))
}

fn material(id: Uuid, input: MaterialInput) -> Result<Material, DomainError> {
    Ok(Material {
        id,
        name: input.name,
        family: input.family,
        elastic_modulus: input.elastic_modulus.try_into()?,
        density: input.density.try_into()?,
        poisson_ratio: input.poisson_ratio,
    })
}
fn action(id: Uuid, input: NodalActionInput) -> Result<NodalAction, DomainError> {
    Ok(NodalAction {
        id,
        name: input.name,
        application_node_id: input.application_node_id,
        force: input.force.try_into()?,
        couple: input.couple.try_into()?,
        frame: input.frame,
    })
}

fn active_values(project: &Project) -> Result<BTreeMap<Uuid, serde_json::Value>, DomainError> {
    let mut values = BTreeMap::new();
    macro_rules! collect {
        ($items:expr) => {
            for item in $items {
                values.insert(
                    item.id,
                    serde_json::to_value((item, project.model.identities.get(&item.id)))
                        .map_err(|e| DomainError::InvalidModel(e.to_string()))?,
                );
            }
        };
    }
    collect!(&project.objects);
    collect!(&project.environments);
    collect!(&project.networks);
    collect!(&project.ports);
    collect!(&project.connections);
    collect!(&project.model.analytical_nodes);
    collect!(&project.model.analytical_members);
    collect!(&project.model.links);
    collect!(&project.model.materials);
    collect!(&project.model.actions);
    values.extend(crate::layout::active_values(project)?);
    values.extend(crate::surfaces::active_values(project)?);
    values.extend(crate::assemblies::active_values(project)?);
    for (id, versions) in crate::catalogue::project_catalogue(project)?.entries {
        if let Some(version) = versions.last() {
            values.insert(
                id,
                serde_json::to_value(&version.entry)
                    .map_err(|error| DomainError::InvalidModel(error.to_string()))?,
            );
        }
    }
    Ok(values)
}

pub fn apply_model_edit(
    project: &mut Project,
    edit: ModelEdit,
    actor: &str,
) -> Result<ModelChange, DomainError> {
    validate_name(actor)?;
    let revision = project
        .revision
        .checked_add(1)
        .ok_or(DomainError::InvalidHistory)?;
    let before = active_values(project)?;
    let mut candidate = project.clone();
    edit_inner(&mut candidate, edit, actor, revision)?;
    candidate.validate_model_contents(revision)?;
    let after = active_values(&candidate)?;
    let mut change = ModelChange::default();
    for (id, value) in &after {
        match before.get(id) {
            None => change.created_ids.push(*id),
            Some(old) if old != value => change.changed_ids.push(*id),
            _ => {}
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            change.retired_ids.push(*id)
        }
    }
    if project.units != candidate.units
        || project.extensions != candidate.extensions
        || project.model.engineering_frame != candidate.model.engineering_frame
        || project.model.georeference != candidate.model.georeference
    {
        change.changed_ids.push(project.id)
    }
    *project = candidate;
    Ok(change)
}

fn edit_inner(
    project: &mut Project,
    edit: ModelEdit,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    match edit {
        ModelEdit::EditPrecalculation { command } => {
            crate::precalculation::apply(project, command, actor, revision)?;
        }
        ModelEdit::EditLayout { command } => {
            crate::layout::apply(project, command, actor, revision)?;
        }
        ModelEdit::EditFraming { command } => {
            crate::framing::apply(project, command, actor, revision)?;
        }
        ModelEdit::EditSurfaces { command } => {
            crate::surfaces::apply(project, command, actor, revision)?;
        }
        ModelEdit::EditAssemblies { command } => {
            crate::assemblies::apply(project, command, actor, revision)?;
        }
        ModelEdit::PutCatalogEntry { entry } => {
            crate::catalogue::put_entry(project, entry, actor, revision)?;
            crate::framing::regenerate_layout_dependents(project)?;
        }
        ModelEdit::AssignCatalogEntry {
            object_id,
            entry_id,
        } => {
            crate::catalogue::assign_entry(project, object_id, entry_id)?;
        }
        ModelEdit::ClearCatalogAssignment { object_id, kind } => {
            crate::catalogue::clear_assignment(project, object_id, kind)?;
        }
        ModelEdit::SetGeometryTolerances { tolerances } => {
            tolerances.validate()?;
            project.extensions.insert(
                crate::geometry::TOLERANCES_KEY.into(),
                serde_json::to_value(tolerances)
                    .map_err(|e| DomainError::InvalidModel(e.to_string()))?,
            );
        }
        ModelEdit::ApplyGeometryRepair { plan_hash, repairs } => {
            crate::geometry::apply_repairs(project, &repairs, &plan_hash, actor)?
        }
        ModelEdit::AddPhysical {
            name,
            discipline,
            start,
            end,
            role,
            material_id,
            environment_id,
        } => {
            let id = Uuid::new_v4();
            project.objects.push(PhysicalObject {
                id,
                name,
                discipline,
                start_m: start.to_si()?,
                end_m: end.to_si()?,
                state: ModelState::Modelled,
                role,
                material_id,
                environment_id,
                extensions: BTreeMap::new(),
            });
            record_identity(project, id, actor, revision, true)?;
        }
        ModelEdit::SetPhysicalGeometry {
            object_id,
            start,
            end,
        } => {
            let object = physical_mut(project, object_id)?;
            object.start_m = start.to_si()?;
            object.end_m = end.to_si()?;
        }
        ModelEdit::SetPhysicalRole { object_id, role } => {
            physical_mut(project, object_id)?.role = role
        }
        ModelEdit::SetPhysicalMaterial {
            object_id,
            material_id,
        } => {
            if material_id.is_some()
                && crate::catalogue::assignments(physical(project, object_id)?)?
                    .contains_key(&crate::catalogue::CatalogKind::Material)
            {
                return Err(DomainError::InvalidModel("Remova explicitamente o material do catálogo antes de atribuir material do modelo.".into()));
            }
            physical_mut(project, object_id)?.material_id = material_id;
        }
        ModelEdit::SetDisplayUnit { unit } => project.units = unit,
        ModelEdit::AddAnalyticalNode { name, position } => {
            let id = Uuid::new_v4();
            project.model.analytical_nodes.push(AnalyticalNode {
                id,
                name,
                position_m: position.to_si()?,
            });
            record_identity(project, id, actor, revision, true)?;
        }
        ModelEdit::AddAnalyticalMember {
            name,
            start_node_id,
            end_node_id,
            local_y,
        } => {
            let id = Uuid::new_v4();
            project.model.analytical_members.push(AnalyticalMember {
                id,
                name,
                start_node_id,
                end_node_id,
                local_y,
            });
            record_identity(project, id, actor, revision, true)?;
        }
        ModelEdit::LinkPhysicalAnalytical {
            physical_id,
            analytical_id,
        } => {
            let id = Uuid::new_v4();
            project.model.links.push(PhysicalAnalyticalLink {
                id,
                physical_id,
                analytical_id,
            });
            record_identity(project, id, actor, revision, false)?;
        }
        ModelEdit::UnlinkPhysicalAnalytical { link_id } => {
            if !project.model.links.iter().any(|x| x.id == link_id) {
                return Err(DomainError::UnknownReference(link_id));
            }
            project.model.links.retain(|x| x.id != link_id);
            retire(project, link_id, revision)?;
        }
        ModelEdit::AddMaterial { material: input } => {
            let id = Uuid::new_v4();
            project.model.materials.push(material(id, input)?);
            record_identity(project, id, actor, revision, false)?;
        }
        ModelEdit::SetMaterial {
            material_id,
            material: input,
        } => {
            let entry = project
                .model
                .materials
                .iter_mut()
                .find(|x| x.id == material_id)
                .ok_or(DomainError::UnknownReference(material_id))?;
            *entry = material(material_id, input)?;
        }
        ModelEdit::AddNodalAction { action: input } => {
            let id = Uuid::new_v4();
            project.model.actions.push(action(id, input)?);
            record_identity(project, id, actor, revision, false)?;
        }
        ModelEdit::SetNodalAction {
            action_id,
            action: input,
        } => {
            let entry = project
                .model
                .actions
                .iter_mut()
                .find(|x| x.id == action_id)
                .ok_or(DomainError::UnknownReference(action_id))?;
            *entry = action(action_id, input)?;
        }
        ModelEdit::AddNetwork {
            name,
            discipline,
            kind,
            environment_id,
        } => {
            let id = Uuid::new_v4();
            project.networks.push(Network {
                id,
                name,
                discipline,
                kind,
                environment_id,
                port_ids: Vec::new(),
                extensions: BTreeMap::new(),
            });
            record_identity(project, id, actor, revision, true)?;
        }
        ModelEdit::AddPort {
            object_id,
            discipline,
            kind,
            position,
        } => {
            let id = Uuid::new_v4();
            project.ports.push(Port {
                id,
                object_id,
                discipline,
                kind,
                position_m: position.to_si()?,
                extensions: BTreeMap::new(),
            });
            record_identity(project, id, actor, revision, true)?;
        }
        ModelEdit::AssignPort {
            network_id,
            port_id,
        } => {
            let network = project
                .networks
                .iter_mut()
                .find(|x| x.id == network_id)
                .ok_or(DomainError::UnknownReference(network_id))?;
            network.port_ids.push(port_id);
        }
        ModelEdit::ConnectPorts {
            network_id,
            from_port,
            to_port,
        } => {
            let id = Uuid::new_v4();
            project.connections.push(Connection {
                id,
                network_id,
                from_port,
                to_port,
                extensions: BTreeMap::new(),
            });
            record_identity(project, id, actor, revision, false)?;
        }
        ModelEdit::DisconnectPorts { connection_id } => {
            if !project.connections.iter().any(|x| x.id == connection_id) {
                return Err(DomainError::UnknownReference(connection_id));
            }
            project.connections.retain(|x| x.id != connection_id);
            retire(project, connection_id, revision)?;
        }
        ModelEdit::CopyPhysical {
            source_id,
            name,
            translation,
        } => {
            let mut copy = physical(project, source_id)?.clone();
            copy.id = Uuid::new_v4();
            copy.name = name;
            let delta = Vector3::from(translation.to_si()?.as_array());
            copy.start_m = point(Vector3::from(copy.start_m.as_array()) + delta)?;
            copy.end_m = point(Vector3::from(copy.end_m.as_array()) + delta)?;
            let id = copy.id;
            project.objects.push(copy);
            record_identity(project, id, actor, revision, true)?;
            project
                .model
                .identities
                .get_mut(&id)
                .expect("registered identity")
                .origin = SourceOrigin::Derived {
                operation: "copy_physical".into(),
            };
            project.model.lineage.push(LineageRecord {
                id: Uuid::new_v4(),
                kind: LineageKind::Copy,
                parent_ids: vec![source_id],
                child_ids: vec![id],
                revision,
                actor: actor.into(),
                retired_objects: vec![],
                retired_links: vec![],
            });
        }
        ModelEdit::MergePhysical { source_ids, name } => {
            merge(project, source_ids, name, actor, revision)?
        }
        ModelEdit::SplitPhysical {
            object_id,
            fraction,
            names,
        } => split(project, object_id, fraction, names, actor, revision)?,
        ModelEdit::RigidMove {
            object_ids,
            rotation,
            translation,
        } => {
            validate_rotation(rotation)?;
            move_selected(
                project,
                object_ids,
                matrix(rotation),
                Vector3::from(translation.to_si()?.as_array()),
            )?;
        }
        ModelEdit::ChangeEngineeringFrame { frame } => {
            validate_rotation(frame.rotation_to_world)?;
            let old = &project.model.engineering_frame;
            let inverse = matrix(frame.rotation_to_world).transpose();
            let rotation = inverse * matrix(old.rotation_to_world);
            let translation = inverse
                * (Vector3::from(old.origin_world_m.as_array())
                    - Vector3::from(frame.origin_world_m.as_array()));
            transform_all(project, rotation, translation)?;
            project.model.engineering_frame = frame;
        }
        ModelEdit::SetGeoreference { georeference } => project.model.georeference = georeference,
        ModelEdit::SetSourceOrigin { entity_id, origin } => {
            origin.validate()?;
            project
                .model
                .identities
                .get_mut(&entity_id)
                .ok_or(DomainError::UnknownReference(entity_id))?
                .origin = origin;
        }
    }
    Ok(())
}

fn replacement(
    project: &mut Project,
    parents: Vec<PhysicalObject>,
    children: Vec<PhysicalObject>,
    kind: LineageKind,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    let parent_ids: Vec<_> = parents.iter().map(|x| x.id).collect();
    if project
        .ports
        .iter()
        .any(|port| parent_ids.contains(&port.object_id))
    {
        return Err(DomainError::InvalidModel(
            "Fusão ou divisão com portas exige primeiro resolver explicitamente a conectividade."
                .into(),
        ));
    }
    let retired_links: Vec<_> = project
        .model
        .links
        .iter()
        .filter(|x| parent_ids.contains(&x.physical_id))
        .cloned()
        .collect();
    let analytical_ids: BTreeSet<_> = retired_links.iter().map(|x| x.analytical_id).collect();
    project.objects.retain(|x| !parent_ids.contains(&x.id));
    project
        .model
        .links
        .retain(|x| !parent_ids.contains(&x.physical_id));
    for id in parent_ids.iter().chain(retired_links.iter().map(|x| &x.id)) {
        retire(project, *id, revision)?;
    }
    let child_ids: Vec<_> = children.iter().map(|x| x.id).collect();
    for child in children {
        record_identity(project, child.id, actor, revision, true)?;
        project
            .model
            .identities
            .get_mut(&child.id)
            .expect("registered identity")
            .origin = SourceOrigin::Derived {
            operation: match kind {
                LineageKind::Merge => "merge_physical",
                LineageKind::Split => "split_physical",
                LineageKind::Copy => "copy_physical",
            }
            .into(),
        };
        for analytical_id in &analytical_ids {
            let id = Uuid::new_v4();
            project.model.links.push(PhysicalAnalyticalLink {
                id,
                physical_id: child.id,
                analytical_id: *analytical_id,
            });
            record_identity(project, id, actor, revision, false)?;
        }
        project.objects.push(child);
    }
    project.model.lineage.push(LineageRecord {
        id: Uuid::new_v4(),
        kind,
        parent_ids,
        child_ids,
        revision,
        actor: actor.into(),
        retired_objects: parents,
        retired_links,
    });
    Ok(())
}

fn merge(
    project: &mut Project,
    ids: Vec<Uuid>,
    name: String,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    if ids.len() < 2 || ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(DomainError::InvalidModel(
            "Fusão requer pelo menos duas identidades distintas.".into(),
        ));
    }
    let parents: Vec<_> = ids
        .into_iter()
        .map(|id| physical(project, id).cloned())
        .collect::<Result<_, _>>()?;
    let first = &parents[0];
    let origin = Vector3::from(first.start_m.as_array());
    let axis = (Vector3::from(first.end_m.as_array()) - origin).normalize();
    let mut intervals = Vec::new();
    for object in &parents {
        if object.discipline != first.discipline
            || object.role != first.role
            || object.material_id != first.material_id
            || object.environment_id != first.environment_id
            || object.extensions != first.extensions
        {
            return Err(DomainError::InvalidModel("A fusão não pode descartar diferenças de disciplina, papel, material, ambiente ou extensões.".into()));
        }
        let points = [
            Vector3::from(object.start_m.as_array()) - origin,
            Vector3::from(object.end_m.as_array()) - origin,
        ];
        if points.iter().any(|p| p.cross(&axis).norm() > 1.0e-9) {
            return Err(DomainError::InvalidModel(
                "Esta fusão aceita somente peças colineares e contíguas.".into(),
            ));
        }
        let a = points[0].dot(&axis);
        let b = points[1].dot(&axis);
        intervals.push((a.min(b), a.max(b)));
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let low = intervals[0].0;
    let mut high = intervals[0].1;
    for (start, end) in intervals.into_iter().skip(1) {
        if start > high + 1.0e-9 {
            return Err(DomainError::InvalidModel(
                "A fusão não pode preencher uma lacuna geométrica.".into(),
            ));
        }
        high = high.max(end);
    }
    let mut merged = first.clone();
    merged.id = Uuid::new_v4();
    merged.name = name;
    merged.start_m = point(origin + axis * low)?;
    merged.end_m = point(origin + axis * high)?;
    replacement(
        project,
        parents,
        vec![merged],
        LineageKind::Merge,
        actor,
        revision,
    )
}

fn split(
    project: &mut Project,
    id: Uuid,
    fraction: f64,
    names: [String; 2],
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    if !fraction.is_finite() || fraction <= 0.0 || fraction >= 1.0 {
        return Err(DomainError::InvalidQuantity);
    }
    let source = physical(project, id)?.clone();
    let start = Vector3::from(source.start_m.as_array());
    let end = Vector3::from(source.end_m.as_array());
    let middle = point(start + (end - start) * fraction)?;
    let mut left = source.clone();
    let mut right = source.clone();
    left.id = Uuid::new_v4();
    right.id = Uuid::new_v4();
    left.name = names[0].clone();
    right.name = names[1].clone();
    left.end_m = middle;
    right.start_m = middle;
    replacement(
        project,
        vec![source],
        vec![left, right],
        LineageKind::Split,
        actor,
        revision,
    )
}

fn transform_loads(
    project: &mut Project,
    nodes: &BTreeSet<Uuid>,
    rotation: Matrix3<f64>,
) -> Result<(), DomainError> {
    for action in &mut project.model.actions {
        if nodes.contains(&action.application_node_id)
            && matches!(action.frame, ActionFrame::Engineering {})
        {
            action.force = Force3::from_si(array(rotation * Vector3::from(action.force.si())))?;
            action.couple = Moment3::from_si(array(rotation * Vector3::from(action.couple.si())))?;
        }
    }
    Ok(())
}

fn move_selected(
    project: &mut Project,
    object_ids: Vec<Uuid>,
    rotation: Matrix3<f64>,
    translation: Vector3<f64>,
) -> Result<(), DomainError> {
    let selected: BTreeSet<_> = object_ids.iter().copied().collect();
    if selected.is_empty() || selected.len() != object_ids.len() {
        return Err(DomainError::InvalidModel(
            "Selecione peças distintas para mover.".into(),
        ));
    }
    for id in &selected {
        physical(project, *id)?;
    }
    let members: BTreeSet<_> = project
        .model
        .links
        .iter()
        .filter(|x| selected.contains(&x.physical_id))
        .map(|x| x.analytical_id)
        .collect();
    if project
        .model
        .links
        .iter()
        .any(|x| members.contains(&x.analytical_id) && !selected.contains(&x.physical_id))
    {
        return Err(DomainError::SharedTopology);
    }
    let nodes: BTreeSet<_> = project
        .model
        .analytical_members
        .iter()
        .filter(|x| members.contains(&x.id))
        .flat_map(|x| [x.start_node_id, x.end_node_id])
        .collect();
    if project.model.analytical_members.iter().any(|x| {
        !members.contains(&x.id)
            && (nodes.contains(&x.start_node_id) || nodes.contains(&x.end_node_id))
    }) {
        return Err(DomainError::SharedTopology);
    }
    for object in &mut project.objects {
        if selected.contains(&object.id) {
            object.start_m =
                point(rotation * Vector3::from(object.start_m.as_array()) + translation)?;
            object.end_m = point(rotation * Vector3::from(object.end_m.as_array()) + translation)?;
        }
    }
    for port in &mut project.ports {
        if selected.contains(&port.object_id) {
            port.position_m =
                point(rotation * Vector3::from(port.position_m.as_array()) + translation)?;
        }
    }
    for node in &mut project.model.analytical_nodes {
        if nodes.contains(&node.id) {
            node.position_m =
                point(rotation * Vector3::from(node.position_m.as_array()) + translation)?;
        }
    }
    for member in &mut project.model.analytical_members {
        if members.contains(&member.id) {
            member.local_y = array(rotation * Vector3::from(member.local_y));
        }
    }
    transform_loads(project, &nodes, rotation)
}

fn transform_all(
    project: &mut Project,
    rotation: Matrix3<f64>,
    translation: Vector3<f64>,
) -> Result<(), DomainError> {
    for object in &mut project.objects {
        object.start_m = point(rotation * Vector3::from(object.start_m.as_array()) + translation)?;
        object.end_m = point(rotation * Vector3::from(object.end_m.as_array()) + translation)?;
    }
    for port in &mut project.ports {
        port.position_m =
            point(rotation * Vector3::from(port.position_m.as_array()) + translation)?;
    }
    for node in &mut project.model.analytical_nodes {
        node.position_m =
            point(rotation * Vector3::from(node.position_m.as_array()) + translation)?;
    }
    for member in &mut project.model.analytical_members {
        member.local_y = array(rotation * Vector3::from(member.local_y));
    }
    let nodes = project
        .model
        .analytical_nodes
        .iter()
        .map(|x| x.id)
        .collect();
    transform_loads(project, &nodes, rotation)
}

/// Legacy quick creation receives a real member and explicit one-to-one link.
pub fn initialize_legacy_member(
    project: &mut Project,
    physical_id: Uuid,
    analytical_id: Option<Uuid>,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    let object = physical(project, physical_id)?.clone();
    let start_id = Uuid::new_v4();
    let end_id = Uuid::new_v4();
    let member_id = analytical_id.unwrap_or_else(Uuid::new_v4);
    let link_id = Uuid::new_v4();
    project.model.analytical_nodes.extend([
        AnalyticalNode {
            id: start_id,
            name: "Nó inicial".into(),
            position_m: object.start_m,
        },
        AnalyticalNode {
            id: end_id,
            name: "Nó final".into(),
            position_m: object.end_m,
        },
    ]);
    let axis = (Vector3::from(object.end_m.as_array()) - Vector3::from(object.start_m.as_array()))
        .normalize();
    let basis = if axis.y.abs() < 0.9 {
        Vector3::y()
    } else {
        Vector3::z()
    };
    let y = (basis - axis * axis.dot(&basis)).normalize();
    project.model.analytical_members.push(AnalyticalMember {
        id: member_id,
        name: object.name,
        start_node_id: start_id,
        end_node_id: end_id,
        local_y: array(y),
    });
    project.model.links.push(PhysicalAnalyticalLink {
        id: link_id,
        physical_id,
        analytical_id: member_id,
    });
    if !project.model.identities.contains_key(&physical_id) {
        record_identity(project, physical_id, actor, revision, true)?;
    }
    for id in [start_id, end_id, member_id, link_id] {
        record_identity(project, id, actor, revision, id != link_id)?;
    }
    Ok(())
}
