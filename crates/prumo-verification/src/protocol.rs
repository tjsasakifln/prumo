use crate::{VerificationError, hash, text_ok};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_CASES: usize = 1_024;
pub const MAX_TARGETS: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityKind {
    Dimensionless,
    Length,
    Area,
    SecondMomentOfArea,
    Angle,
    Force,
    Moment,
    Pressure,
    VolumeFlow,
    Current,
    Voltage,
    Resistance,
    Power,
    Energy,
    AbsoluteTemperature,
    TemperatureDifference,
    Frequency,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    One,
    Metre,
    Millimetre,
    SquareMetre,
    MetreToFourth,
    Radian,
    Newton,
    Kilonewton,
    NewtonMetre,
    Pascal,
    Kilopascal,
    CubicMetrePerSecond,
    LitrePerSecond,
    Ampere,
    Volt,
    Ohm,
    Watt,
    Joule,
    Kelvin,
    DegreeCelsius,
    KelvinDifference,
    Hertz,
}

impl Unit {
    pub const fn quantity(self) -> QuantityKind {
        match self {
            Self::One => QuantityKind::Dimensionless,
            Self::Metre | Self::Millimetre => QuantityKind::Length,
            Self::SquareMetre => QuantityKind::Area,
            Self::MetreToFourth => QuantityKind::SecondMomentOfArea,
            Self::Radian => QuantityKind::Angle,
            Self::Newton | Self::Kilonewton => QuantityKind::Force,
            Self::NewtonMetre => QuantityKind::Moment,
            Self::Pascal | Self::Kilopascal => QuantityKind::Pressure,
            Self::CubicMetrePerSecond | Self::LitrePerSecond => QuantityKind::VolumeFlow,
            Self::Ampere => QuantityKind::Current,
            Self::Volt => QuantityKind::Voltage,
            Self::Ohm => QuantityKind::Resistance,
            Self::Watt => QuantityKind::Power,
            Self::Joule => QuantityKind::Energy,
            Self::Kelvin | Self::DegreeCelsius => QuantityKind::AbsoluteTemperature,
            Self::KelvinDifference => QuantityKind::TemperatureDifference,
            Self::Hertz => QuantityKind::Frequency,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityValue {
    #[serde(deserialize_with = "finite_f64")]
    pub value: f64,
    pub unit: Unit,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    /// Expressed in the target unit, not a hidden SI conversion.
    #[serde(deserialize_with = "finite_f64")]
    pub absolute: f64,
    #[serde(deserialize_with = "finite_f64")]
    pub relative: f64,
    /// Fixed by the protocol, including for zero-valued references.
    #[serde(deserialize_with = "finite_f64")]
    pub reference_scale: f64,
    pub rationale: String,
}

// serde's buffered tagged-enum representation carries arbitrary-precision JSON
// numbers through serde_json::Number. Decode that representation explicitly;
// converting through a generic f64 visitor would reject valid exponent literals.
fn finite_f64<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    number
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            serde::de::Error::custom("A grandeza deve ser finita e representável em f64.")
        })
}

impl Tolerance {
    pub fn limit(&self) -> f64 {
        self.absolute + self.relative * self.reference_scale
    }

    fn validate(&self) -> bool {
        self.absolute.is_finite()
            && self.absolute >= 0.0
            && self.relative.is_finite()
            && (0.0..=1.0).contains(&self.relative)
            && self.reference_scale.is_finite()
            && self.reference_scale > 0.0
            && self.limit().is_finite()
            && text_ok(&self.rationale)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub quantity: QuantityKind,
    pub expected: QuantityValue,
    pub tolerance: Tolerance,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpectedOutcome {
    Results {
        targets: BTreeMap<String, Target>,
        /// Exact categorical agreement. Numeric tolerance never relaxes a decision.
        classifications: BTreeMap<String, String>,
    },
    Rejection {
        code: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceOrigin {
    pub title: String,
    pub citation: String,
    pub method: String,
    pub derivation: String,
    pub data_license: String,
    /// Includes the actual engine and any upstream numerical kernels.
    pub implementation_lineage: BTreeSet<String>,
    pub independence_assessment: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseProtocol {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub domain: String,
    pub requirements: BTreeSet<String>,
    pub critical: bool,
    pub inputs: BTreeMap<String, QuantityValue>,
    pub assumptions: Vec<String>,
    pub reference: ReferenceOrigin,
    pub expected: ExpectedOutcome,
}

impl CaseProtocol {
    /// Boundary for imported JSON; rejects reserved serialization markers before
    /// decoding numbers, then validates the physical quantity/tolerance schema.
    pub fn from_json(bytes: &[u8]) -> Result<Self, VerificationError> {
        prumo_domain::canonical::validate_json_representation(bytes)
            .map_err(|error| VerificationError::InvalidProtocol(error.to_string()))?;
        let protocol: Self = serde_json::from_slice(bytes)?;
        protocol.validate()?;
        Ok(protocol)
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        let invalid = |detail: &str| VerificationError::InvalidProtocol(detail.into());
        if self.schema_version != PROTOCOL_VERSION
            || !text_ok(&self.id)
            || !text_ok(&self.title)
            || !text_ok(&self.domain)
            || self.requirements.is_empty()
            || self.requirements.iter().any(|id| !text_ok(id))
            || self.assumptions.is_empty()
            || self.assumptions.iter().any(|item| !text_ok(item))
        {
            return Err(invalid(
                "identidade, versão, requisitos ou hipóteses ausentes",
            ));
        }
        if self.inputs.is_empty()
            || self.inputs.len() > MAX_TARGETS
            || self
                .inputs
                .iter()
                .any(|(id, value)| !text_ok(id) || !value.value.is_finite())
        {
            return Err(invalid("entradas não finitas, vazias ou acima do limite"));
        }
        let reference = &self.reference;
        if [
            &reference.title,
            &reference.citation,
            &reference.method,
            &reference.derivation,
            &reference.data_license,
            &reference.independence_assessment,
        ]
        .into_iter()
        .any(|value| !text_ok(value))
            || reference.implementation_lineage.is_empty()
            || reference
                .implementation_lineage
                .iter()
                .any(|id| !text_ok(id))
        {
            return Err(invalid(
                "origem, método, licença ou genealogia da referência ausente",
            ));
        }
        match &self.expected {
            ExpectedOutcome::Results {
                targets,
                classifications,
            } => {
                if targets.len() + classifications.len() == 0
                    || targets.len() + classifications.len() > MAX_TARGETS
                    || classifications
                        .iter()
                        .any(|(id, value)| !text_ok(id) || !text_ok(value))
                    || targets.iter().any(|(id, target)| {
                        !text_ok(id)
                            || !target.expected.value.is_finite()
                            || target.quantity != target.expected.unit.quantity()
                            || !target.tolerance.validate()
                    })
                {
                    return Err(invalid("alvo, unidade, decisão ou tolerância inválidos"));
                }
            }
            ExpectedOutcome::Rejection { code } if !text_ok(code) => {
                return Err(invalid("código de rejeição ausente"));
            }
            ExpectedOutcome::Rejection { .. } => {}
        }
        Ok(())
    }

    pub fn sha256(&self) -> Result<String, VerificationError> {
        self.validate()?;
        hash(self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    /// Reproducibility/software review only, no professional qualification implied.
    Software,
    /// A technical record may be linked, but this crate cannot approve engineering release.
    TechnicalRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolReview {
    pub protocol_sha256: String,
    pub reviewer: String,
    pub competence_and_scope: String,
    pub record: String,
    pub kind: ReviewKind,
    pub independence_confirmed: bool,
}

/// Immutable after review. It deliberately does not implement Deserialize.
/// An executor only receives a prepared case, so it cannot widen tolerances after observing outputs.
#[derive(Clone, Debug, Serialize)]
pub struct PreparedProtocol {
    protocol: CaseProtocol,
    protocol_sha256: String,
    review: ProtocolReview,
}

impl PreparedProtocol {
    pub fn new(protocol: CaseProtocol, review: ProtocolReview) -> Result<Self, VerificationError> {
        let protocol_sha256 = protocol.sha256()?;
        if review.protocol_sha256 != protocol_sha256
            || !review.independence_confirmed
            || !text_ok(&review.reviewer)
            || !text_ok(&review.competence_and_scope)
            || !text_ok(&review.record)
        {
            return Err(VerificationError::InvalidReview(protocol.id.clone()));
        }
        Ok(Self {
            protocol,
            protocol_sha256,
            review,
        })
    }

    pub fn protocol(&self) -> &CaseProtocol {
        &self.protocol
    }
    pub fn sha256(&self) -> &str {
        &self.protocol_sha256
    }
    pub fn review(&self) -> &ProtocolReview {
        &self.review
    }
}
