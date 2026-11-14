//! Evidence from bounded integration experiments, distinct from project results.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub origin: String,
    pub license: String,
    pub language: String,
    pub boundary: String,
    pub scope: String,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
    pub absolute_error: Option<f64>,
    pub tolerance: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeArtifact {
    pub media_type: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentReport {
    pub component: ComponentDescriptor,
    pub checks: Vec<QualificationCheck>,
    pub iterations: u32,
    pub elapsed_ns: u64,
    pub artifact: Option<ProbeArtifact>,
}

impl ExperimentReport {
    pub fn passed(&self) -> bool {
        !self.checks.is_empty()
            && self.iterations > 0
            && self.checks.iter().all(|check| {
                check.passed
                    && check
                        .absolute_error
                        .is_none_or(|value| value.is_finite() && value >= 0.0)
                    && check
                        .tolerance
                        .is_none_or(|value| value.is_finite() && value >= 0.0)
                    && match (check.absolute_error, check.tolerance) {
                        (Some(error), Some(tolerance)) => error <= tolerance,
                        (None, None) => true,
                        _ => false,
                    }
            })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentBasis {
    pub project_id: Uuid,
    pub revision: u64,
    pub content_sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationReport {
    pub protocol_version: u32,
    pub suite_id: String,
    pub request_id: Uuid,
    pub basis: Option<ExperimentBasis>,
    pub platform: String,
    pub executable_sha256: String,
    pub executable_bytes: u64,
    pub elapsed_ns: u64,
    pub experiments: Vec<ExperimentReport>,
    pub engineering_release: bool,
}

impl QualificationReport {
    pub fn passed(&self) -> bool {
        self.protocol_version == 1
            && !self.engineering_release
            && !self.experiments.is_empty()
            && self.experiments.iter().all(ExperimentReport::passed)
    }
}
