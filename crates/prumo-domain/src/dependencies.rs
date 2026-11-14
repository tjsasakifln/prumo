//! Traceable model measurements and a directed dependency graph, never engineering approval.
use crate::canonical::{
    ModelSnapshot, canonical_json, digest, hash_bytes, model_snapshot, source_index_unchecked,
};
use crate::model::PhysicalRole;
use crate::{DomainError, Project, validate_name};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use uuid::Uuid;

pub const MAX_RESULTS: usize = 20_000;
pub const MAX_DEPENDENCY_EDGES: usize = 100_000;
pub const MAX_INVALIDATIONS: usize = 10_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ProjectMetadata,
    Membership,
    Physical,
    PhysicalGeometry,
    AnalyticalNode,
    AnalyticalMember,
    Link,
    Material,
    Action,
    Environment,
    Port,
    Network,
    Connection,
    Coordinates,
    Georeference,
    Normative,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceKey {
    pub kind: SourceKind,
    pub id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDigest {
    pub source: SourceKey,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Dependency {
    Source {
        source: SourceKey,
        sha256: String,
    },
    Result {
        result_id: Uuid,
        payload_sha256: String,
    },
}

impl Dependency {
    pub fn source(
        source: SourceKey,
        index: &BTreeMap<SourceKey, String>,
    ) -> Result<Self, DomainError> {
        Ok(Self::Source {
            source,
            sha256: index
                .get(&source)
                .ok_or_else(|| invalid("Fonte de resultado inexistente."))?
                .clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultState {
    Current,
    Stale,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InspectionPayload {
    PhysicalLength {
        object_id: Uuid,
        length_m: f64,
    },
    ModelSummary {
        physical_objects: usize,
        visual_objects: usize,
        analytical_members: usize,
        materials: usize,
        actions: usize,
        networks: usize,
        connections: usize,
        total_length_m: f64,
    },
    /// Metadata for future producers; no public command accepts or approves this evidence.
    ExternalEvidence {
        artifact_sha256: String,
        media_type: String,
    },
}

impl InspectionPayload {
    fn validate(&self) -> Result<(), DomainError> {
        let length = match self {
            Self::PhysicalLength {
                object_id,
                length_m,
            } => {
                if object_id.is_nil() || *length_m <= 0.0 {
                    return Err(invalid("Medida geométrica inválida."));
                }
                Some(*length_m)
            }
            Self::ModelSummary {
                physical_objects,
                visual_objects,
                analytical_members,
                materials,
                actions,
                networks,
                connections,
                total_length_m,
            } => {
                if visual_objects > physical_objects
                    || [
                        physical_objects,
                        analytical_members,
                        materials,
                        actions,
                        networks,
                        connections,
                    ]
                    .into_iter()
                    .any(|count| *count > 160_000)
                {
                    return Err(invalid("Contagem de inspeção inválida."));
                }
                Some(*total_length_m)
            }
            Self::ExternalEvidence {
                artifact_sha256,
                media_type,
            } => {
                validate_hash(artifact_sha256)?;
                validate_name(media_type)?;
                None
            }
        };
        if length.is_some_and(|length| !length.is_finite() || !(0.0..=1.0e15).contains(&length)) {
            return Err(invalid(
                "Comprimento de inspeção não finito ou fora do domínio.",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResultTrace {
    pub id: Uuid,
    pub name: String,
    pub producer: String,
    pub input_revision: u64,
    pub created_revision: u64,
    pub input_snapshot_sha256: String,
    pub dependencies: Vec<Dependency>,
    pub payload: InspectionPayload,
    pub payload_sha256: String,
    pub state: ResultState,
    pub invalidated_revision: Option<u64>,
    pub engineering_release: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceChange {
    pub source: SourceKey,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidationEvent {
    pub id: Uuid,
    pub revision: u64,
    pub actor: String,
    pub reason: String,
    pub changes: Vec<SourceChange>,
    pub result_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyGraph {
    pub snapshots: BTreeMap<String, ModelSnapshot>,
    pub results: Vec<ResultTrace>,
    pub invalidations: Vec<InvalidationEvent>,
}

fn invalid(message: &str) -> DomainError {
    DomainError::InvalidDependency(message.into())
}

fn validate_hash(hash: &str) -> Result<(), DomainError> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Err(invalid(
            "A identidade do conteúdo deve ser SHA-256 hexadecimal.",
        ))
    } else {
        Ok(())
    }
}

impl DependencyGraph {
    pub fn validate(
        &self,
        revision: u64,
        sources: &BTreeMap<SourceKey, String>,
    ) -> Result<(), DomainError> {
        if self.results.len() > MAX_RESULTS
            || self.invalidations.len() > MAX_INVALIDATIONS
            || self
                .results
                .iter()
                .map(|result| result.dependencies.len())
                .sum::<usize>()
                > MAX_DEPENDENCY_EDGES
        {
            return Err(invalid(
                "O grafo excede os limites de resultados, relações ou histórico.",
            ));
        }
        let source_ids: BTreeSet<_> = sources.keys().map(|source| source.id).collect();
        let project_id = sources
            .keys()
            .find(|source| source.kind == SourceKind::ProjectMetadata)
            .map(|source| source.id)
            .ok_or_else(|| invalid("Identidade do projeto ausente."))?;
        let mut historical_sources = BTreeMap::new();
        for (hash, snapshot) in &self.snapshots {
            validate_hash(hash)?;
            let value: serde_json::Value = serde_json::from_str(&snapshot.canonical_json)
                .map_err(|_| invalid("Conteúdo de snapshot inválido."))?;
            if snapshot.version != 1
                || snapshot.project_id != project_id
                || snapshot.source_revision == 0
                || snapshot.source_revision > revision
                || snapshot.sha256 != *hash
                || hash_bytes(snapshot.canonical_json.as_bytes()) != *hash
                || canonical_json(&value)? != snapshot.canonical_json
                || value.get("source_digests")
                    != Some(
                        &serde_json::to_value(&snapshot.sources)
                            .map_err(|_| invalid("Índice histórico não serializável."))?,
                    )
            {
                return Err(invalid(
                    "Snapshot histórico perdeu identidade ou conteúdo canônico.",
                ));
            }
            let mut indexed_sources = BTreeMap::new();
            for source in &snapshot.sources {
                validate_hash(&source.sha256)?;
                if source.source.id.is_nil()
                    || indexed_sources
                        .insert(source.source, source.sha256.as_str())
                        .is_some()
                {
                    return Err(invalid("Fonte histórica nula ou duplicada."));
                }
            }
            historical_sources.insert(hash.as_str(), indexed_sources);
        }
        let by_id: BTreeMap<_, _> = self
            .results
            .iter()
            .map(|result| (result.id, result))
            .collect();
        if by_id.len() != self.results.len() {
            return Err(invalid("Resultado com identidade repetida."));
        }
        let mut indegree = BTreeMap::new();
        let mut children: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
        for result in &self.results {
            if result.id.is_nil()
                || source_ids.contains(&result.id)
                || result.input_revision == 0
                || result.input_revision > result.created_revision
                || result.created_revision > revision
                || result.engineering_release
            {
                return Err(invalid(
                    "Identidade, revisão ou estado de resultado inválido.",
                ));
            }
            validate_name(&result.name)?;
            validate_name(&result.producer)?;
            validate_hash(&result.input_snapshot_sha256)?;
            validate_hash(&result.payload_sha256)?;
            let snapshot = self
                .snapshots
                .get(&result.input_snapshot_sha256)
                .ok_or_else(|| invalid("Resultado sem snapshot imutável de origem."))?;
            if snapshot.source_revision > result.input_revision {
                return Err(invalid("Snapshot posterior à base do resultado."));
            }
            let snapshot_sources = historical_sources
                .get(result.input_snapshot_sha256.as_str())
                .ok_or_else(|| invalid("Índice histórico do resultado ausente."))?;
            result.payload.validate()?;
            if digest(&result.payload)? != result.payload_sha256 {
                return Err(invalid("O conteúdo do resultado foi alterado."));
            }
            match (result.state, result.invalidated_revision) {
                (ResultState::Current, None) => {}
                (ResultState::Stale, Some(at))
                    if at >= result.created_revision && at <= revision => {}
                _ => return Err(invalid("Estado de obsolescência incoerente.")),
            }
            if result.dependencies.is_empty() {
                return Err(invalid("Resultado sem dependências declaradas."));
            }
            let mut seen_sources = BTreeSet::new();
            let mut seen_results = BTreeSet::new();
            let mut degree = 0usize;
            for dependency in &result.dependencies {
                match dependency {
                    Dependency::Source { source, sha256 } => {
                        validate_hash(sha256)?;
                        if source.id.is_nil() || !seen_sources.insert(*source) {
                            return Err(invalid("Fonte nula ou repetida."));
                        }
                        if snapshot_sources.get(source).copied() != Some(sha256.as_str()) {
                            return Err(invalid(
                                "Dependência diverge do snapshot imutável de origem.",
                            ));
                        }
                        if result.state == ResultState::Current
                            && sources.get(source) != Some(sha256)
                        {
                            return Err(invalid(
                                "Um resultado atual usa uma fonte ausente ou alterada.",
                            ));
                        }
                    }
                    Dependency::Result {
                        result_id,
                        payload_sha256,
                    } => {
                        validate_hash(payload_sha256)?;
                        let parent = by_id
                            .get(result_id)
                            .ok_or_else(|| invalid("Dependência de resultado órfã."))?;
                        if !seen_results.insert(*result_id)
                            || parent.payload_sha256 != *payload_sha256
                            || parent.created_revision > result.created_revision
                            || (result.state == ResultState::Current
                                && parent.state != ResultState::Current)
                        {
                            return Err(invalid(
                                "Dependência de resultado inválida ou desatualizada.",
                            ));
                        }
                        degree += 1;
                        children.entry(*result_id).or_default().push(result.id);
                    }
                }
            }
            indegree.insert(result.id, degree);
        }
        // Iterative traversal also bounds stack usage for deliberately deep hostile graphs.
        let mut ready: VecDeque<Uuid> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop_front() {
            visited += 1;
            if let Some(dependents) = children.get(&id) {
                for child in dependents {
                    let degree = indegree
                        .get_mut(child)
                        .ok_or_else(|| invalid("Grafo inconsistente."))?;
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push_back(*child);
                    }
                }
            }
        }
        if visited != self.results.len() {
            return Err(invalid("Ciclo no grafo de dependências de resultados."));
        }
        let mut event_ids = BTreeSet::new();
        let mut invalidation_causes = BTreeSet::new();
        for event in &self.invalidations {
            if event.id.is_nil()
                || !event_ids.insert(event.id)
                || by_id.contains_key(&event.id)
                || source_ids.contains(&event.id)
                || event.revision == 0
                || event.revision > revision
                || event.changes.is_empty()
                || event.result_ids.is_empty()
            {
                return Err(invalid("Evento de invalidação inválido."));
            }
            validate_name(&event.actor)?;
            validate_name(&event.reason)?;
            let mut changed = BTreeSet::new();
            for change in &event.changes {
                if change.source.id.is_nil()
                    || !changed.insert(change.source)
                    || change.before_sha256 == change.after_sha256
                {
                    return Err(invalid("Alteração de fonte inválida."));
                }
                for hash in [&change.before_sha256, &change.after_sha256]
                    .into_iter()
                    .flatten()
                {
                    validate_hash(hash)?;
                }
            }
            let mut affected = BTreeSet::new();
            for id in &event.result_ids {
                let result = by_id
                    .get(id)
                    .ok_or_else(|| invalid("Evento aponta resultado inexistente."))?;
                if !affected.insert(*id)
                    || result.created_revision > event.revision
                    || result.state != ResultState::Stale
                {
                    return Err(invalid("Evento de invalidação contradiz o resultado."));
                }
                invalidation_causes.insert((*id, event.revision));
            }
        }
        for result in self
            .results
            .iter()
            .filter(|result| result.state == ResultState::Stale)
        {
            if !result
                .invalidated_revision
                .is_some_and(|at| invalidation_causes.contains(&(result.id, at)))
            {
                return Err(invalid("Resultado obsoleto sem causa rastreável."));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        name: String,
        producer: String,
        input_revision: u64,
        created_revision: u64,
        input_snapshot_sha256: String,
        dependencies: Vec<Dependency>,
        payload: InspectionPayload,
    ) -> Result<Uuid, DomainError> {
        if self.results.len() >= MAX_RESULTS {
            return Err(invalid("Limite de resultados atingido."));
        }
        if !self.snapshots.contains_key(&input_snapshot_sha256) {
            return Err(invalid("Capture o snapshot antes do resultado."));
        }
        payload.validate()?;
        let id = Uuid::new_v4();
        self.results.push(ResultTrace {
            id,
            name,
            producer,
            input_revision,
            created_revision,
            input_snapshot_sha256,
            dependencies,
            payload_sha256: digest(&payload)?,
            payload,
            state: ResultState::Current,
            invalidated_revision: None,
            engineering_release: false,
        });
        Ok(id)
    }

    pub fn invalidate_changes(
        &mut self,
        before: &BTreeMap<SourceKey, String>,
        after: &BTreeMap<SourceKey, String>,
        revision: u64,
        actor: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        let keys: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
        let changes: Vec<_> = keys
            .into_iter()
            .filter(|key| before.get(key) != after.get(key))
            .map(|source| SourceChange {
                source,
                before_sha256: before.get(&source).cloned(),
                after_sha256: after.get(&source).cloned(),
            })
            .collect();
        let changed: BTreeSet<_> = changes.iter().map(|change| change.source).collect();
        let mut affected = BTreeSet::new();
        for result in &self.results {
            if result.dependencies.iter().any(|dependency| matches!(dependency, Dependency::Source {source, ..} if changed.contains(source))) {
                affected.insert(result.id);
            }
        }
        let mut children: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
        for result in &self.results {
            for dependency in &result.dependencies {
                if let Dependency::Result { result_id, .. } = dependency {
                    children.entry(*result_id).or_default().push(result.id);
                }
            }
        }
        let mut ready: VecDeque<_> = affected.iter().copied().collect();
        while let Some(id) = ready.pop_front() {
            if let Some(dependents) = children.get(&id) {
                for child in dependents {
                    if affected.insert(*child) {
                        ready.push_back(*child);
                    }
                }
            }
        }
        if affected.is_empty() {
            return Ok(());
        }
        if self.invalidations.len() >= MAX_INVALIDATIONS {
            return Err(invalid("Limite de histórico de invalidação atingido."));
        }
        for result in &mut self.results {
            if affected.contains(&result.id) {
                result.state = ResultState::Stale;
                result.invalidated_revision.get_or_insert(revision);
            }
        }
        self.invalidations.push(InvalidationEvent {
            id: Uuid::new_v4(),
            revision,
            actor: actor.into(),
            reason: reason.into(),
            changes,
            result_ids: affected.into_iter().collect(),
        });
        Ok(())
    }
}

/// Produces real geometric measurements. It does not verify a structural design.
pub fn inspect_model(project: &mut Project) -> Result<(), DomainError> {
    let snapshot = model_snapshot(project)?;
    let index = source_index_unchecked(project)?;
    let created_revision = project
        .revision
        .checked_add(1)
        .ok_or_else(|| invalid("Limite de revisão atingido."))?;
    let mut ordered: Vec<_> = project.objects.iter().collect();
    ordered.sort_by_key(|object| object.id);
    let mut graph = project.dependencies.clone();
    graph
        .snapshots
        .entry(snapshot.sha256.clone())
        .or_insert_with(|| snapshot.clone());
    let mut summary_dependencies = vec![Dependency::source(
        SourceKey {
            kind: SourceKind::Membership,
            id: project.id,
        },
        &index,
    )?];
    let mut total = 0.0;
    let mut correction = 0.0;
    for object in &ordered {
        let length_m = object.length_m();
        // Stable UUID order and compensated addition avoid storage-order dependent totals.
        let adjusted = length_m - correction;
        let next = total + adjusted;
        correction = (next - total) - adjusted;
        total = next;
        let dependencies = vec![
            Dependency::source(
                SourceKey {
                    kind: SourceKind::PhysicalGeometry,
                    id: object.id,
                },
                &index,
            )?,
            Dependency::source(
                SourceKey {
                    kind: SourceKind::Coordinates,
                    id: project.id,
                },
                &index,
            )?,
        ];
        let id = graph.record(
            format!(
                "Comprimento: {}",
                object.name.chars().take(140).collect::<String>()
            ),
            "prumo-model-inspection/1".into(),
            project.revision,
            created_revision,
            snapshot.sha256.clone(),
            dependencies,
            InspectionPayload::PhysicalLength {
                object_id: object.id,
                length_m,
            },
        )?;
        let result = graph
            .results
            .last()
            .ok_or_else(|| invalid("Resultado de inspeção ausente."))?;
        summary_dependencies.push(Dependency::Result {
            result_id: id,
            payload_sha256: result.payload_sha256.clone(),
        });
    }
    graph.record(
        "Resumo geométrico do modelo".into(),
        "prumo-model-inspection/1".into(),
        project.revision,
        created_revision,
        snapshot.sha256,
        summary_dependencies,
        InspectionPayload::ModelSummary {
            physical_objects: ordered.len(),
            visual_objects: ordered
                .iter()
                .filter(|object| object.role == PhysicalRole::VisualOnly)
                .count(),
            analytical_members: project.model.analytical_members.len(),
            materials: project.model.materials.len(),
            actions: project.model.actions.len(),
            networks: project.networks.len(),
            connections: project.connections.len(),
            total_length_m: total,
        },
    )?;
    graph.validate(created_revision, &index)?;
    project.dependencies = graph;
    Ok(())
}
