use crate::{
    ExpectedOutcome, MAX_CASES, MAX_TARGETS, PreparedProtocol, QuantityValue, RunContext,
    VerificationError, hash, text_ok,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub name: String,
    pub version: String,
    /// Include shared kernels, not only the name of this interface or wrapper.
    pub implementation_lineage: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedOutcome {
    Results {
        quantities: BTreeMap<String, QuantityValue>,
        classifications: BTreeMap<String, String>,
    },
    Rejection {
        code: String,
        detail: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseExecution {
    pub protocol_sha256: String,
    pub engine: EngineIdentity,
    pub outcome: ObservedOutcome,
}

impl CaseExecution {
    fn invalid_reason(&self) -> Option<String> {
        if !text_ok(&self.engine.name)
            || !text_ok(&self.engine.version)
            || self.engine.implementation_lineage.is_empty()
            || self
                .engine
                .implementation_lineage
                .iter()
                .any(|id| !text_ok(id))
        {
            return Some("Identidade ou genealogia do motor ausente.".into());
        }
        match &self.outcome {
            ObservedOutcome::Results {
                quantities,
                classifications,
            } => {
                if quantities.len() + classifications.len() > MAX_TARGETS {
                    return Some("Saída excede o limite de alvos do protocolo.".into());
                }
                for (id, value) in quantities {
                    if !text_ok(id) || !value.value.is_finite() {
                        return Some(format!("Resultado inválido em {id}: {}.", value.value));
                    }
                }
                if classifications
                    .iter()
                    .any(|(id, value)| !text_ok(id) || !text_ok(value))
                {
                    return Some("Classificação vazia ou inválida.".into());
                }
            }
            ObservedOutcome::Rejection { code, detail } if !text_ok(code) || !text_ok(detail) => {
                return Some("Rejeição sem código ou explicação.".into());
            }
            ObservedOutcome::Rejection { .. } => {}
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseDecision {
    Passed,
    Failed,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MetricComparison {
    pub target_id: String,
    pub expected: QuantityValue,
    pub actual: Option<QuantityValue>,
    pub absolute_error: Option<f64>,
    pub accepted_error: f64,
    pub passed: bool,
    pub diagnostic: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ClassificationComparison {
    pub target_id: String,
    pub expected: String,
    pub actual: Option<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordedExecution {
    Completed { execution: CaseExecution },
    Unavailable { diagnostic: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseEvidence {
    pub prepared: PreparedProtocol,
    pub model_sha256: String,
    pub expected_results_sha256: String,
    pub actual_results_sha256: String,
    pub execution: RecordedExecution,
    pub metrics: Vec<MetricComparison>,
    pub classifications: Vec<ClassificationComparison>,
    pub decision: CaseDecision,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SuiteEvidence {
    pub schema_version: u32,
    pub suite_id: String,
    pub context: RunContext,
    pub environment_sha256: String,
    pub cases: Vec<CaseEvidence>,
    pub verification_passed: bool,
    pub engineering_release: bool,
    pub scope: String,
}

/// Created only by run_suite, not by deserializing an untrusted alleged approval.
#[derive(Clone, Debug, Serialize)]
pub struct SuiteReport {
    evidence: SuiteEvidence,
    evidence_sha256: String,
}

impl SuiteReport {
    pub fn evidence(&self) -> &SuiteEvidence {
        &self.evidence
    }
    pub fn sha256(&self) -> &str {
        &self.evidence_sha256
    }
    pub fn passed(&self) -> bool {
        self.evidence.verification_passed
    }

    /// All required cases must exist and pass. A missing case never improves the rate.
    /// This only gates numerical verification; it cannot grant professional release.
    pub fn gate(&self, required_cases: &BTreeSet<String>) -> EvidenceGate {
        let index: BTreeMap<_, _> = self
            .evidence
            .cases
            .iter()
            .map(|case| (case.prepared.protocol().id.clone(), case))
            .collect();
        let missing_cases = required_cases
            .iter()
            .filter(|id| !index.contains_key(*id))
            .cloned()
            .collect();
        let failed_cases = self
            .evidence
            .cases
            .iter()
            .filter(|case| {
                case.decision != CaseDecision::Passed
                    && (case.prepared.protocol().critical
                        || required_cases.contains(&case.prepared.protocol().id))
            })
            .map(|case| case.prepared.protocol().id.clone())
            .collect();
        let mut result = EvidenceGate {
            verification_allowed: false,
            engineering_release: false,
            missing_cases,
            failed_cases,
        };
        result.verification_allowed = !required_cases.is_empty()
            && result.missing_cases.is_empty()
            && result.failed_cases.is_empty();
        result
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvidenceGate {
    pub verification_allowed: bool,
    pub engineering_release: bool,
    pub missing_cases: BTreeSet<String>,
    pub failed_cases: BTreeSet<String>,
}

pub fn run_suite(
    suite_id: &str,
    protocols: &[PreparedProtocol],
    context: RunContext,
    mut executor: impl FnMut(&PreparedProtocol) -> Result<CaseExecution, String>,
) -> Result<SuiteReport, VerificationError> {
    context.validate()?;
    if !text_ok(suite_id) || protocols.is_empty() || protocols.len() > MAX_CASES {
        return Err(VerificationError::InvalidSuite(
            "identidade ou quantidade de casos inválida".into(),
        ));
    }
    let mut ordered: Vec<_> = protocols.iter().collect();
    ordered.sort_by(|a, b| a.protocol().id.cmp(&b.protocol().id));
    if ordered
        .windows(2)
        .any(|pair| pair[0].protocol().id == pair[1].protocol().id)
    {
        return Err(VerificationError::InvalidSuite(
            "IDs de caso repetidos".into(),
        ));
    }
    let environment_sha256 = context.build.sha256()?;
    let mut cases = Vec::with_capacity(ordered.len());
    for prepared in ordered {
        let result = executor(prepared);
        cases.push(compare(prepared, result)?);
    }
    let evidence = SuiteEvidence {
        schema_version: 1,
        suite_id: suite_id.into(),
        context,
        environment_sha256,
        verification_passed: cases.iter().all(|case| case.decision == CaseDecision::Passed),
        cases,
        engineering_release: false,
        scope: "Verificação de software e referências delimitadas. Não é cálculo de projeto, conformidade normativa ou aprovação profissional.".into(),
    };
    let evidence_sha256 = hash(&evidence)?;
    Ok(SuiteReport {
        evidence,
        evidence_sha256,
    })
}

fn compare(
    prepared: &PreparedProtocol,
    result: Result<CaseExecution, String>,
) -> Result<CaseEvidence, VerificationError> {
    let protocol = prepared.protocol();
    let mut evidence = CaseEvidence {
        prepared: prepared.clone(),
        model_sha256: hash(&protocol.inputs)?,
        expected_results_sha256: hash(&protocol.expected)?,
        actual_results_sha256: String::new(),
        execution: RecordedExecution::Unavailable {
            diagnostic: String::new(),
        },
        metrics: Vec::new(),
        classifications: Vec::new(),
        decision: CaseDecision::Blocked,
        diagnostics: Vec::new(),
    };
    let execution = match result {
        Ok(value) => value,
        Err(message) => {
            let message: String = message.chars().take(16_384).collect();
            evidence
                .diagnostics
                .push(format!("Execução sem evidência: {message}"));
            evidence.execution = RecordedExecution::Unavailable {
                diagnostic: message,
            };
            evidence.actual_results_sha256 = hash(&evidence.execution)?;
            return Ok(evidence);
        }
    };
    if let Some(reason) = execution.invalid_reason() {
        evidence.diagnostics.push(reason.clone());
        evidence.execution = RecordedExecution::Unavailable { diagnostic: reason };
        evidence.actual_results_sha256 = hash(&evidence.execution)?;
        return Ok(evidence);
    }
    evidence.execution = RecordedExecution::Completed {
        execution: execution.clone(),
    };
    evidence.actual_results_sha256 = hash(&evidence.execution)?;
    if execution.protocol_sha256 != prepared.sha256() {
        evidence
            .diagnostics
            .push("A saída pertence a outro protocolo ou a outra versão das tolerâncias.".into());
        return Ok(evidence);
    }
    if !protocol
        .reference
        .implementation_lineage
        .is_disjoint(&execution.engine.implementation_lineage)
    {
        evidence.diagnostics.push("Referência e implementação compartilham motor ou núcleo numérico; dois adaptadores não demonstram independência.".into());
        return Ok(evidence);
    }
    evidence.decision = CaseDecision::Passed;
    match (&protocol.expected, &execution.outcome) {
        (
            ExpectedOutcome::Rejection { code: expected },
            ObservedOutcome::Rejection { code: actual, .. },
        ) => {
            if expected != actual {
                evidence.diagnostics.push(format!(
                    "Rejeição diferente: esperada {expected}, recebida {actual}."
                ));
            }
        }
        (
            ExpectedOutcome::Results {
                targets,
                classifications: expected_classes,
            },
            ObservedOutcome::Results {
                quantities,
                classifications,
            },
        ) => {
            for (id, target) in targets {
                let actual = quantities.get(id).copied();
                let same_unit = actual.is_some_and(|value| value.unit == target.expected.unit);
                let error = actual
                    .filter(|_| same_unit)
                    .map(|value| (value.value - target.expected.value).abs());
                let finite_error = error.filter(|value| value.is_finite());
                let passed = finite_error.is_some_and(|value| value <= target.tolerance.limit());
                let diagnostic = if actual.is_none() {
                    "Resultado obrigatório ausente."
                } else if !same_unit {
                    "Unidade diferente da unidade pré-registrada; conversão implícita não é aceita."
                } else if finite_error.is_none() {
                    "Diferença numérica fora do intervalo finito."
                } else if !passed {
                    "Diferença acima da tolerância pré-registrada."
                } else {
                    "Resultado dentro da tolerância pré-registrada."
                };
                evidence.metrics.push(MetricComparison {
                    target_id: id.clone(),
                    expected: target.expected,
                    actual,
                    absolute_error: finite_error,
                    accepted_error: target.tolerance.limit(),
                    passed,
                    diagnostic: diagnostic.into(),
                });
            }
            for (id, expected) in expected_classes {
                let actual = classifications.get(id).cloned();
                evidence.classifications.push(ClassificationComparison {
                    target_id: id.clone(),
                    expected: expected.clone(),
                    passed: actual.as_ref() == Some(expected),
                    actual,
                });
            }
            if quantities.keys().any(|id| !targets.contains_key(id))
                || classifications
                    .keys()
                    .any(|id| !expected_classes.contains_key(id))
            {
                evidence
                    .diagnostics
                    .push("A saída contém alvos que não pertencem ao protocolo fechado.".into());
            }
            if evidence.metrics.iter().any(|item| !item.passed)
                || evidence.classifications.iter().any(|item| !item.passed)
            {
                evidence
                    .diagnostics
                    .push("Um ou mais alvos obrigatórios reprovaram.".into());
            }
        }
        _ => evidence
            .diagnostics
            .push("A execução não produziu o tipo de resultado esperado pelo protocolo.".into()),
    }
    if !evidence.diagnostics.is_empty() {
        evidence.decision = CaseDecision::Failed;
    }
    Ok(evidence)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RegressionReport {
    pub previous_sha256: String,
    pub current_sha256: String,
    pub regressions: BTreeSet<String>,
    pub fixes: BTreeSet<String>,
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub changed_protocols: BTreeSet<String>,
    pub changed_outputs: BTreeSet<String>,
}

pub fn compare_regression(previous: &SuiteReport, current: &SuiteReport) -> RegressionReport {
    let old: BTreeMap<_, _> = previous
        .evidence
        .cases
        .iter()
        .map(|case| (case.prepared.protocol().id.clone(), case))
        .collect();
    let new: BTreeMap<_, _> = current
        .evidence
        .cases
        .iter()
        .map(|case| (case.prepared.protocol().id.clone(), case))
        .collect();
    let mut report = RegressionReport {
        previous_sha256: previous.sha256().into(),
        current_sha256: current.sha256().into(),
        regressions: BTreeSet::new(),
        fixes: BTreeSet::new(),
        added: BTreeSet::new(),
        removed: BTreeSet::new(),
        changed_protocols: BTreeSet::new(),
        changed_outputs: BTreeSet::new(),
    };
    for (id, before) in &old {
        let Some(after) = new.get(id) else {
            report.removed.insert(id.clone());
            continue;
        };
        if before.prepared.sha256() != after.prepared.sha256() {
            report.changed_protocols.insert(id.clone());
        }
        if before.actual_results_sha256 != after.actual_results_sha256 {
            report.changed_outputs.insert(id.clone());
        }
        if before.decision == CaseDecision::Passed && after.decision != CaseDecision::Passed {
            report.regressions.insert(id.clone());
        }
        if before.decision != CaseDecision::Passed && after.decision == CaseDecision::Passed {
            report.fixes.insert(id.clone());
        }
    }
    for id in new.keys().filter(|id| !old.contains_key(*id)) {
        report.added.insert(id.clone());
    }
    report
}
