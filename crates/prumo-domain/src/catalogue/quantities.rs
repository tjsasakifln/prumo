use super::{finite, invalid};
use crate::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalQuantity {
    Dimensionless,
    Length,
    Area,
    SecondMomentOfArea,
    ElasticModulus,
    Stress,
    Strength,
    Density,
    VolumeFlow,
    AbsolutePressure,
    GaugePressure,
    Voltage,
    Current,
    Power,
    Energy,
    AbsoluteTemperature,
    TemperatureDifference,
    ThermalConductivity,
    HeatCapacity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalUnit {
    One,
    Metre,
    Millimetre,
    SquareMetre,
    SquareMillimetre,
    MetreToFourth,
    MillimetreToFourth,
    Pascal,
    Kilopascal,
    Megapascal,
    Gigapascal,
    KilogramPerCubicMetre,
    CubicMetrePerSecond,
    LitrePerSecond,
    Volt,
    Ampere,
    Watt,
    Kilowatt,
    Joule,
    KilowattHour,
    Kelvin,
    DegreeCelsius,
    KelvinDifference,
    WattPerMetreKelvin,
    JoulePerKilogramKelvin,
}

impl PhysicalUnit {
    pub fn accepts(self, quantity: PhysicalQuantity) -> bool {
        use PhysicalQuantity as Q;
        match self {
            Self::One => quantity == Q::Dimensionless,
            Self::Metre | Self::Millimetre => quantity == Q::Length,
            Self::SquareMetre | Self::SquareMillimetre => quantity == Q::Area,
            Self::MetreToFourth | Self::MillimetreToFourth => quantity == Q::SecondMomentOfArea,
            Self::Pascal | Self::Kilopascal | Self::Megapascal | Self::Gigapascal => matches!(
                quantity,
                Q::ElasticModulus
                    | Q::Stress
                    | Q::Strength
                    | Q::AbsolutePressure
                    | Q::GaugePressure
            ),
            Self::KilogramPerCubicMetre => quantity == Q::Density,
            Self::CubicMetrePerSecond | Self::LitrePerSecond => quantity == Q::VolumeFlow,
            Self::Volt => quantity == Q::Voltage,
            Self::Ampere => quantity == Q::Current,
            Self::Watt | Self::Kilowatt => quantity == Q::Power,
            Self::Joule | Self::KilowattHour => quantity == Q::Energy,
            Self::Kelvin | Self::DegreeCelsius => quantity == Q::AbsoluteTemperature,
            Self::KelvinDifference => quantity == Q::TemperatureDifference,
            Self::WattPerMetreKelvin => quantity == Q::ThermalConductivity,
            Self::JoulePerKilogramKelvin => quantity == Q::HeatCapacity,
        }
    }

    fn to_si(self, value: f64) -> f64 {
        match self {
            Self::Millimetre | Self::LitrePerSecond => value * 1e-3,
            Self::SquareMillimetre => value * 1e-6,
            Self::MillimetreToFourth => value * 1e-12,
            Self::Kilopascal | Self::Kilowatt => value * 1e3,
            Self::Megapascal => value * 1e6,
            Self::Gigapascal => value * 1e9,
            Self::KilowattHour => value * 3_600_000.0,
            Self::DegreeCelsius => value + 273.15,
            _ => value,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueBasis {
    Nominal,
    Characteristic,
    Design,
    Measured,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalValue {
    pub quantity: PhysicalQuantity,
    /// Exact source spelling is retained. Computation is explicit binary64 SI.
    pub decimal: String,
    pub unit: PhysicalUnit,
    pub basis: ValueBasis,
    /// Required for a design value; never inferred from characteristic strength.
    pub design_basis: Option<String>,
}

impl PhysicalValue {
    pub fn si(&self) -> Result<f64, DomainError> {
        if self.decimal.len() > 128
            || self.decimal.trim() != self.decimal
            || !self.unit.accepts(self.quantity)
        {
            return Err(invalid(
                "Grandeza, unidade ou representação decimal inválida.",
            ));
        }
        let number: serde_json::Number = serde_json::from_str(&self.decimal)
            .map_err(|_| invalid("A propriedade deve conservar um número decimal válido."))?;
        let raw = number
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| invalid("Propriedade fora do intervalo finito."))?;
        if raw == 0.0
            && number
                .to_string()
                .chars()
                .any(|ch| ('1'..='9').contains(&ch))
        {
            // Check the mantissa, not exponent digits (zero times any power remains zero).
            let text = number.to_string();
            if text
                .split(['e', 'E'])
                .next()
                .unwrap_or("")
                .chars()
                .any(|ch| ('1'..='9').contains(&ch))
            {
                return Err(invalid(
                    "A conversão da propriedade perderia um valor não nulo por underflow.",
                ));
            }
        }
        if self.basis == ValueBasis::Design
            && self
                .design_basis
                .as_ref()
                .is_none_or(|text| text.trim().is_empty())
        {
            return Err(invalid("Valor de cálculo exige método e base declarados."));
        }
        if self.basis != ValueBasis::Design && self.design_basis.is_some() {
            return Err(invalid(
                "Uma base de cálculo não pode alterar silenciosamente a natureza do valor.",
            ));
        }
        let value = self.unit.to_si(raw);
        if raw != 0.0 && value == 0.0 && self.unit != PhysicalUnit::DegreeCelsius {
            return Err(invalid(
                "A conversão SI apagaria uma propriedade não nula por underflow.",
            ));
        }
        if !value.is_finite() || value.abs() > 1e30 {
            return Err(invalid("Propriedade fora do domínio numérico SI."));
        }
        use PhysicalQuantity as Q;
        if matches!(
            self.quantity,
            Q::Length
                | Q::Area
                | Q::SecondMomentOfArea
                | Q::ElasticModulus
                | Q::Strength
                | Q::Density
                | Q::ThermalConductivity
                | Q::HeatCapacity
        ) && value <= 0.0
        {
            return Err(invalid(
                "Propriedade física deve ser estritamente positiva.",
            ));
        }
        if matches!(
            self.quantity,
            Q::AbsolutePressure | Q::AbsoluteTemperature | Q::Power | Q::Energy
        ) && value < 0.0
        {
            return Err(invalid(
                "Grandeza absoluta ou nominal não pode ser negativa.",
            ));
        }
        Ok(value)
    }

    pub fn require(&self, quantity: PhysicalQuantity) -> Result<f64, DomainError> {
        if self.quantity != quantity {
            return Err(invalid(
                "Tipo físico incompatível com a propriedade solicitada.",
            ));
        }
        self.si()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacteristicValue(PhysicalValue);
#[derive(Clone, Debug, PartialEq)]
pub struct DesignValue(PhysicalValue);

impl TryFrom<PhysicalValue> for CharacteristicValue {
    type Error = DomainError;
    fn try_from(value: PhysicalValue) -> Result<Self, Self::Error> {
        value.si()?;
        if value.basis != ValueBasis::Characteristic {
            return Err(invalid("A propriedade não é um valor característico."));
        }
        Ok(Self(value))
    }
}
impl TryFrom<PhysicalValue> for DesignValue {
    type Error = DomainError;
    fn try_from(value: PhysicalValue) -> Result<Self, Self::Error> {
        value.si()?;
        if value.basis != ValueBasis::Design {
            return Err(invalid("A propriedade não é um valor de cálculo."));
        }
        Ok(Self(value))
    }
}
impl CharacteristicValue {
    pub fn value(&self) -> &PhysicalValue {
        &self.0
    }
}
impl DesignValue {
    pub fn value(&self) -> &PhysicalValue {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionRange {
    pub quantity: PhysicalQuantity,
    pub minimum: PhysicalValue,
    pub maximum: PhysicalValue,
}

impl ConditionRange {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.minimum.require(self.quantity)? > self.maximum.require(self.quantity)? {
            return Err(invalid("Intervalo de aplicação invertido."));
        }
        Ok(())
    }
    pub fn contains(&self, value: &PhysicalValue) -> Result<bool, DomainError> {
        self.validate()?;
        let actual = value.require(self.quantity)?;
        Ok(actual >= self.minimum.si()? && actual <= self.maximum.si()?)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ElasticLaw {
    Isotropic {
        young_modulus: PhysicalValue,
        #[serde(deserialize_with = "finite")]
        poisson_ratio: f64,
    },
    Orthotropic {
        /// E1, E2 and E3 along the declared material axes.
        young_moduli: Box<[PhysicalValue; 3]>,
        /// G12, G23 and G31 in that order.
        shear_moduli: Box<[PhysicalValue; 3]>,
        /// nu_ij is transverse strain in j per axial strain in i; reciprocal
        /// coefficients follow nu_ij/Ei = nu_ji/Ej.
        #[serde(deserialize_with = "triple")]
        poisson_12_23_31: [f64; 3],
    },
}

fn triple<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[f64; 3], D::Error> {
    let values = <[serde_json::Number; 3]>::deserialize(d)?;
    let mut result = [0.0; 3];
    for (index, value) in values.into_iter().enumerate() {
        result[index] = value
            .as_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| serde::de::Error::custom("Coeficiente não finito."))?;
    }
    Ok(result)
}

impl ElasticLaw {
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Isotropic {
                young_modulus,
                poisson_ratio,
            } => {
                young_modulus.require(PhysicalQuantity::ElasticModulus)?;
                if !poisson_ratio.is_finite() || *poisson_ratio <= -1.0 || *poisson_ratio >= 0.5 {
                    return Err(invalid(
                        "Coeficiente de Poisson isotrópico fora de -1 < nu < 0,5.",
                    ));
                }
            }
            Self::Orthotropic {
                young_moduli,
                shear_moduli,
                poisson_12_23_31,
            } => {
                let e = [
                    young_moduli[0].require(PhysicalQuantity::ElasticModulus)?,
                    young_moduli[1].require(PhysicalQuantity::ElasticModulus)?,
                    young_moduli[2].require(PhysicalQuantity::ElasticModulus)?,
                ];
                for g in shear_moduli.iter() {
                    g.require(PhysicalQuantity::ElasticModulus)?;
                }
                if poisson_12_23_31.iter().any(|x| !x.is_finite()) {
                    return Err(invalid("Coeficientes ortotrópicos não finitos."));
                }
                let [v12, v23, v31] = *poisson_12_23_31;
                let scale = e.into_iter().fold(f64::INFINITY, f64::min);
                let compliance = nalgebra::Matrix3::new(
                    scale / e[0],
                    -scale * v12 / e[0],
                    -scale * v31 / e[2],
                    -scale * v12 / e[0],
                    scale / e[1],
                    -scale * v23 / e[1],
                    -scale * v31 / e[2],
                    -scale * v23 / e[1],
                    scale / e[2],
                );
                if compliance.cholesky().is_none() {
                    return Err(invalid(
                        "Propriedades ortotrópicas não produzem energia elástica positiva com reciprocidade.",
                    ));
                }
            }
        }
        Ok(())
    }
}
