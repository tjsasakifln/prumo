use super::*;
use crate::{MAX_OBJECTS, MIN_MEMBER_LENGTH_M, insert_id, validate_name};
use nalgebra::{Matrix3, Vector3};
use std::collections::BTreeSet;

pub fn validate_rotation(rotation: [[f64; 3]; 3]) -> Result<(), DomainError> {
    if rotation.iter().flatten().any(|x| !x.is_finite()) {
        return Err(DomainError::InvalidFrame);
    }
    let matrix = Matrix3::from_row_slice(&rotation.concat());
    let error = matrix.transpose() * matrix - Matrix3::identity();
    if error.iter().any(|x| x.abs() > 1.0e-12) || (matrix.determinant() - 1.0).abs() > 1.0e-12 {
        return Err(DomainError::InvalidFrame);
    }
    Ok(())
}

pub fn member_axes(
    project: &Project,
    member: &AnalyticalMember,
) -> Result<[[f64; 3]; 3], DomainError> {
    let start = project
        .model
        .analytical_nodes
        .iter()
        .find(|x| x.id == member.start_node_id)
        .ok_or(DomainError::UnknownReference(member.start_node_id))?;
    let end = project
        .model
        .analytical_nodes
        .iter()
        .find(|x| x.id == member.end_node_id)
        .ok_or(DomainError::UnknownReference(member.end_node_id))?;
    let delta =
        Vector3::from(end.position_m.as_array()) - Vector3::from(start.position_m.as_array());
    if delta.norm() <= MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    let x = delta.normalize();
    let y = Vector3::from(member.local_y);
    if y.iter().any(|x| !x.is_finite())
        || (y.norm_squared() - 1.0).abs() > 1.0e-12
        || x.dot(&y).abs() > 1.0e-12
    {
        return Err(DomainError::InvalidFrame);
    }
    let z = x.cross(&y);
    Ok([[x.x, y.x, z.x], [x.y, y.y, z.y], [x.z, y.z, z.z]])
}

pub fn validate_semantics(
    project: &Project,
    revision: u64,
    ids: &mut BTreeSet<Uuid>,
) -> Result<(), DomainError> {
    let model = &project.model;
    if model.analytical_nodes.len() > MAX_OBJECTS * 4
        || model.analytical_members.len() > MAX_OBJECTS * 4
        || model.links.len() > MAX_OBJECTS * 16
        || model.materials.len() > MAX_OBJECTS
        || model.actions.len() > MAX_OBJECTS * 16
        || model.lineage.len() > MAX_OBJECTS * 4
        || model.identities.len() > MAX_OBJECTS * 64
    {
        return Err(DomainError::ModelLimit);
    }
    validate_rotation(model.engineering_frame.rotation_to_world)?;
    if let Some(reference) = &model.georeference {
        validate_name(&reference.crs_identifier)?;
        if !reference.scale.is_finite() || reference.scale <= 0.0 || reference.scale > 1.0e6 {
            return Err(DomainError::InvalidFrame);
        }
    }
    for node in &model.analytical_nodes {
        insert_id(ids, node.id)?;
        validate_name(&node.name)?;
    }
    for material in &model.materials {
        insert_id(ids, material.id)?;
        validate_name(&material.name)?;
        if !material.poisson_ratio.is_finite()
            || material.poisson_ratio <= -1.0
            || material.poisson_ratio >= 0.5
        {
            return Err(DomainError::InvalidQuantity);
        }
    }
    for member in &model.analytical_members {
        insert_id(ids, member.id)?;
        validate_name(&member.name)?;
        if member.start_node_id == member.end_node_id {
            return Err(DomainError::DegenerateMember);
        }
        member_axes(project, member)?;
    }
    let analytical_regions = crate::surfaces::analytical_region_ids(project)?;
    let mut pairs = BTreeSet::new();
    for link in &model.links {
        insert_id(ids, link.id)?;
        let physical = project
            .objects
            .iter()
            .find(|x| x.id == link.physical_id)
            .ok_or(DomainError::UnknownReference(link.physical_id))?;
        if physical.role != PhysicalRole::AnalyticalSource {
            return Err(DomainError::InvalidModel(
                "Objeto apenas visual não recebe vínculo analítico.".into(),
            ));
        }
        if !model
            .analytical_members
            .iter()
            .any(|x| x.id == link.analytical_id)
            && !analytical_regions.contains(&link.analytical_id)
        {
            return Err(DomainError::UnknownReference(link.analytical_id));
        }
        if !pairs.insert((link.physical_id, link.analytical_id)) {
            return Err(DomainError::InvalidModel(
                "Vínculo físico-analítico repetido.".into(),
            ));
        }
    }
    for action in &model.actions {
        insert_id(ids, action.id)?;
        validate_name(&action.name)?;
        if !model
            .analytical_nodes
            .iter()
            .any(|x| x.id == action.application_node_id)
        {
            return Err(DomainError::UnknownReference(action.application_node_id));
        }
        if let ActionFrame::MemberLocal { member_id } = action.frame {
            let member = model
                .analytical_members
                .iter()
                .find(|x| x.id == member_id)
                .ok_or(DomainError::UnknownReference(member_id))?;
            if action.application_node_id != member.start_node_id
                && action.application_node_id != member.end_node_id
            {
                return Err(DomainError::InvalidModel(
                    "A ação local deve usar o membro incidente ao nó de aplicação.".into(),
                ));
            }
        }
    }
    let active = ids.clone();
    let mut archived = BTreeSet::new();
    let mut last_event_revision = 0;
    for event in &model.lineage {
        insert_id(ids, event.id)?;
        validate_name(&event.actor)?;
        if model.identities.contains_key(&event.id) {
            return Err(DomainError::DuplicateIdentity(event.id));
        }
        if event.revision <= last_event_revision
            || event.revision > revision
            || event.parent_ids.is_empty()
            || event.child_ids.is_empty()
        {
            return Err(DomainError::InvalidModel(
                "Histórico de identidade inválido.".into(),
            ));
        }
        last_event_revision = event.revision;
        for id in event.parent_ids.iter().chain(&event.child_ids) {
            if !model.identities.contains_key(id) {
                return Err(DomainError::UnknownReference(*id));
            }
        }
        let cardinality = match event.kind {
            LineageKind::Copy => event.parent_ids.len() == 1 && event.child_ids.len() == 1,
            LineageKind::Merge => event.parent_ids.len() >= 2 && event.child_ids.len() == 1,
            LineageKind::Split => event.parent_ids.len() == 1 && event.child_ids.len() == 2,
        };
        if !cardinality
            || event.parent_ids.iter().collect::<BTreeSet<_>>().len() != event.parent_ids.len()
            || event.child_ids.iter().collect::<BTreeSet<_>>().len() != event.child_ids.len()
            || event.parent_ids.iter().any(|x| event.child_ids.contains(x))
        {
            return Err(DomainError::InvalidModel(
                "Cardinalidade da proveniência inválida.".into(),
            ));
        }
        for id in &event.child_ids {
            let identity = &model.identities[id];
            if identity.created_revision != Some(event.revision)
                || identity.created_by.as_deref() != Some(event.actor.as_str())
            {
                return Err(DomainError::InvalidModel(
                    "A identidade filha deve nascer na revisão e autoria do evento.".into(),
                ));
            }
        }
        for id in &event.parent_ids {
            let identity = &model.identities[id];
            if identity
                .created_revision
                .is_some_and(|created| created > event.revision)
                || match event.kind {
                    LineageKind::Copy => identity
                        .retired_revision
                        .is_some_and(|retired| retired <= event.revision),
                    _ => identity.retired_revision != Some(event.revision),
                }
            {
                return Err(DomainError::InvalidModel(
                    "A identidade de origem não existia no estado exigido pelo evento.".into(),
                ));
            }
        }
        let retired_ids: BTreeSet<_> = event.retired_objects.iter().map(|x| x.id).collect();
        let expected_retired: BTreeSet<_> = if matches!(event.kind, LineageKind::Copy) {
            BTreeSet::new()
        } else {
            event.parent_ids.iter().copied().collect()
        };
        if retired_ids != expected_retired
            || (matches!(event.kind, LineageKind::Copy) && !event.retired_links.is_empty())
        {
            return Err(DomainError::InvalidModel(
                "O histórico deve preservar os objetos retirados pelo evento.".into(),
            ));
        }
        for object in &event.retired_objects {
            if !event.parent_ids.contains(&object.id)
                || active.contains(&object.id)
                || !archived.insert(object.id)
            {
                return Err(DomainError::InvalidModel(
                    "Objeto retirado tem identidade incoerente.".into(),
                ));
            }
            validate_name(&object.name)?;
            if object.length_m() <= MIN_MEMBER_LENGTH_M {
                return Err(DomainError::DegenerateMember);
            }
        }
        for link in &event.retired_links {
            if active.contains(&link.id)
                || !archived.insert(link.id)
                || !event.parent_ids.contains(&link.physical_id)
                || model
                    .identities
                    .get(&link.id)
                    .is_none_or(|identity| identity.retired_revision != Some(event.revision))
                || !model
                    .analytical_members
                    .iter()
                    .any(|x| x.id == link.analytical_id)
            {
                return Err(DomainError::InvalidModel(
                    "Vínculo retirado tem identidade incoerente.".into(),
                ));
            }
        }
    }
    let mut guids = BTreeSet::new();
    for (id, identity) in &model.identities {
        if id.is_nil() {
            return Err(DomainError::DuplicateIdentity(*id));
        }
        identity.origin.validate()?;
        match (&identity.created_by, identity.created_revision) {
            (Some(actor), Some(created)) if created > 0 && created <= revision => {
                validate_name(actor)?
            }
            (None, None) => {}
            _ => {
                return Err(DomainError::InvalidModel(
                    "Proveniência de criação inconsistente.".into(),
                ));
            }
        }
        if let Some(retired) = identity.retired_revision {
            if retired == 0
                || retired > revision
                || active.contains(id)
                || identity
                    .created_revision
                    .is_some_and(|created| retired <= created)
            {
                return Err(DomainError::InvalidModel(
                    "Identidade retirada ainda aparece ativa.".into(),
                ));
            }
        } else if !active.contains(id) {
            return Err(DomainError::UnknownReference(*id));
        }
        if let Some(guid) = &identity.ifc_guid
            && (guid.to_uuid().is_nil() || !guids.insert(guid.clone()))
        {
            return Err(DomainError::InvalidIfcGuid);
        }
    }
    for id in active {
        if id != project.id && !model.identities.contains_key(&id) {
            return Err(DomainError::InvalidModel(format!(
                "Entidade {id} sem origem registrada."
            )));
        }
    }
    Ok(())
}

pub fn force_and_couple_in_engineering(
    project: &Project,
    action: &NodalAction,
) -> Result<([f64; 3], [f64; 3]), DomainError> {
    match action.frame {
        ActionFrame::Engineering {} => Ok((action.force.si(), action.couple.si())),
        ActionFrame::MemberLocal { member_id } => {
            let member = project
                .model
                .analytical_members
                .iter()
                .find(|x| x.id == member_id)
                .ok_or(DomainError::UnknownReference(member_id))?;
            let matrix = Matrix3::from_row_slice(&member_axes(project, member)?.concat());
            let f = matrix * Vector3::from(action.force.si());
            let m = matrix * Vector3::from(action.couple.si());
            Ok(([f.x, f.y, f.z], [m.x, m.y, m.z]))
        }
    }
}

/// Resultant about a declared point; a free couple does not acquire a translation term.
pub fn resultant_about(
    project: &Project,
    action: &NodalAction,
    reference: Point3,
) -> Result<([f64; 3], [f64; 3]), DomainError> {
    let node = project
        .model
        .analytical_nodes
        .iter()
        .find(|x| x.id == action.application_node_id)
        .ok_or(DomainError::UnknownReference(action.application_node_id))?;
    let (force, couple) = force_and_couple_in_engineering(project, action)?;
    let arm = Vector3::from(node.position_m.as_array()) - Vector3::from(reference.as_array());
    let moment = arm.cross(&Vector3::from(force)) + Vector3::from(couple);
    Ok((force, [moment.x, moment.y, moment.z]))
}
