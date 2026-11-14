//! Dimensional inputs are explicit; persisted quantities are canonical SI.
use crate::{DomainError, LengthUnit, validate_point};
use serde::{Deserialize, Serialize};
use std::ops::Index;
use uom::si::{
    f64 as si,
    force::{kilonewton, newton},
    length::{centimeter, meter, millimeter},
    mass_density::kilogram_per_cubic_meter,
    pressure::{gigapascal, kilopascal, megapascal, pascal},
    torque::{kilonewton_meter, newton_meter},
};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "[f64; 3]", into = "[f64; 3]")]
pub struct Point3([f64; 3]);
impl Point3 {
    pub const ORIGIN: Self = Self([0.0; 3]);
    pub const fn as_array(self) -> [f64; 3] {
        self.0
    }
    pub fn distance(self, other: Self) -> f64 {
        self.0
            .into_iter()
            .zip(other.0)
            .map(|(a, b)| (b - a).powi(2))
            .sum::<f64>()
            .sqrt()
    }
}
impl TryFrom<[f64; 3]> for Point3 {
    type Error = DomainError;
    fn try_from(value: [f64; 3]) -> Result<Self, Self::Error> {
        validate_point(value)?;
        Ok(Self(value.map(clean_zero)))
    }
}
impl From<Point3> for [f64; 3] {
    fn from(value: Point3) -> Self {
        value.0
    }
}
impl Index<usize> for Point3 {
    type Output = f64;
    fn index(&self, index: usize) -> &f64 {
        &self.0[index]
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PointInput {
    pub values: [f64; 3],
    pub unit: LengthUnit,
}
impl PointInput {
    pub fn to_si(self) -> Result<Point3, DomainError> {
        let values = self.values.map(|value| {
            match self.unit {
                LengthUnit::Metre => si::Length::new::<meter>(value),
                LengthUnit::Centimetre => si::Length::new::<centimeter>(value),
                LengthUnit::Millimetre => si::Length::new::<millimeter>(value),
            }
            .get::<meter>()
        });
        Point3::try_from(values)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForceUnit {
    Newton,
    Kilonewton,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MomentUnit {
    NewtonMetre,
    KilonewtonMetre,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureUnit {
    Pascal,
    Kilopascal,
    Megapascal,
    Gigapascal,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityUnit {
    KilogramPerCubicMetre,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForceInput {
    pub values: [f64; 3],
    pub unit: ForceUnit,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MomentInput {
    pub values: [f64; 3],
    pub unit: MomentUnit,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PressureInput {
    pub value: f64,
    pub unit: PressureUnit,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DensityInput {
    pub value: f64,
    pub unit: DensityUnit,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "ForceInput", into = "ForceInput")]
pub struct Force3([f64; 3]);
impl Force3 {
    pub const fn si(self) -> [f64; 3] {
        self.0
    }
    pub fn from_si(value: [f64; 3]) -> Result<Self, DomainError> {
        bounded(value, 1.0e15)?;
        Ok(Self(value.map(clean_zero)))
    }
}
impl TryFrom<ForceInput> for Force3 {
    type Error = DomainError;
    fn try_from(value: ForceInput) -> Result<Self, Self::Error> {
        Self::from_si(value.values.map(|x| {
            match value.unit {
                ForceUnit::Newton => si::Force::new::<newton>(x),
                ForceUnit::Kilonewton => si::Force::new::<kilonewton>(x),
            }
            .get::<newton>()
        }))
    }
}
impl From<Force3> for ForceInput {
    fn from(value: Force3) -> Self {
        Self {
            values: value.0,
            unit: ForceUnit::Newton,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "MomentInput", into = "MomentInput")]
/// A force vector cannot be assigned as a free moment, even with the same values.
/// ```compile_fail
/// use prumo_domain::units::{Force3, Moment3};
/// let force = Force3::from_si([0.0, 1.0, 0.0]).unwrap();
/// let moment: Moment3 = force;
/// ```
pub struct Moment3([f64; 3]);
impl Moment3 {
    pub const fn si(self) -> [f64; 3] {
        self.0
    }
    pub fn from_si(value: [f64; 3]) -> Result<Self, DomainError> {
        bounded(value, 1.0e18)?;
        Ok(Self(value.map(clean_zero)))
    }
}
impl TryFrom<MomentInput> for Moment3 {
    type Error = DomainError;
    fn try_from(value: MomentInput) -> Result<Self, Self::Error> {
        Self::from_si(value.values.map(|x| {
            match value.unit {
                MomentUnit::NewtonMetre => si::Torque::new::<newton_meter>(x),
                MomentUnit::KilonewtonMetre => si::Torque::new::<kilonewton_meter>(x),
            }
            .get::<newton_meter>()
        }))
    }
}
impl From<Moment3> for MomentInput {
    fn from(value: Moment3) -> Self {
        Self {
            values: value.0,
            unit: MomentUnit::NewtonMetre,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "PressureInput", into = "PressureInput")]
pub struct Pressure(f64);
impl Pressure {
    pub const fn si(self) -> f64 {
        self.0
    }
}
impl TryFrom<PressureInput> for Pressure {
    type Error = DomainError;
    fn try_from(value: PressureInput) -> Result<Self, Self::Error> {
        let si = match value.unit {
            PressureUnit::Pascal => si::Pressure::new::<pascal>(value.value),
            PressureUnit::Kilopascal => si::Pressure::new::<kilopascal>(value.value),
            PressureUnit::Megapascal => si::Pressure::new::<megapascal>(value.value),
            PressureUnit::Gigapascal => si::Pressure::new::<gigapascal>(value.value),
        }
        .get::<pascal>();
        positive(si, 1.0e15)?;
        Ok(Self(si))
    }
}
impl From<Pressure> for PressureInput {
    fn from(value: Pressure) -> Self {
        Self {
            value: value.0,
            unit: PressureUnit::Pascal,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "DensityInput", into = "DensityInput")]
pub struct Density(f64);
impl Density {
    pub const fn si(self) -> f64 {
        self.0
    }
}
impl TryFrom<DensityInput> for Density {
    type Error = DomainError;
    fn try_from(value: DensityInput) -> Result<Self, Self::Error> {
        let si = si::MassDensity::new::<kilogram_per_cubic_meter>(value.value)
            .get::<kilogram_per_cubic_meter>();
        positive(si, 1.0e8)?;
        Ok(Self(si))
    }
}
impl From<Density> for DensityInput {
    fn from(value: Density) -> Self {
        Self {
            value: value.0,
            unit: DensityUnit::KilogramPerCubicMetre,
        }
    }
}

fn bounded(values: [f64; 3], maximum: f64) -> Result<(), DomainError> {
    if values
        .into_iter()
        .any(|x| !x.is_finite() || x.abs() > maximum)
    {
        Err(DomainError::InvalidQuantity)
    } else {
        Ok(())
    }
}
fn positive(value: f64, maximum: f64) -> Result<(), DomainError> {
    if !value.is_finite() || value <= 0.0 || value > maximum {
        Err(DomainError::InvalidQuantity)
    } else {
        Ok(())
    }
}

fn clean_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

// These canonical SI scalars have distinct Rust types. Their wire fields must
// explicitly name SI (as rotation_radians does); user conversion uses new(value,unit).
macro_rules! scalar_si {
    ($name:ident,$minimum:expr,$maximum:expr) => {
        #[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
        #[serde(try_from = "f64", into = "f64")]
        pub struct $name(f64);
        impl $name {
            pub const fn si(self) -> f64 {
                self.0
            }
        }
        impl TryFrom<f64> for $name {
            type Error = DomainError;
            fn try_from(value: f64) -> Result<Self, Self::Error> {
                if !value.is_finite() || !($minimum..=$maximum).contains(&value) {
                    return Err(DomainError::InvalidQuantity);
                }
                Ok(Self(clean_zero(value)))
            }
        }
        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
scalar_si!(Length, 0.0, 1.0e12);
scalar_si!(Mass, 0.0, 1.0e18);
scalar_si!(AbsoluteTemperature, 0.0, 1.0e6);
scalar_si!(TemperatureInterval, -1.0e6, 1.0e6);
scalar_si!(Time, 0.0, 1.0e12);
scalar_si!(Angle, -1.0e9, 1.0e9);
scalar_si!(Stiffness, 0.0, 1.0e24);
scalar_si!(Stress, -1.0e15, 1.0e15);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MassUnit {
    Kilogram,
    Tonne,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureUnit {
    Kelvin,
    DegreeCelsius,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AngleUnit {
    Radian,
    Degree,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StiffnessUnit {
    NewtonPerMetre,
    KilonewtonPerMetre,
}

impl Length {
    pub fn new(value: f64, unit: LengthUnit) -> Result<Self, DomainError> {
        let quantity = match unit {
            LengthUnit::Metre => si::Length::new::<meter>(value),
            LengthUnit::Centimetre => si::Length::new::<centimeter>(value),
            LengthUnit::Millimetre => si::Length::new::<millimeter>(value),
        };
        Self::try_from(quantity.get::<meter>())
    }
}
impl Mass {
    pub fn new(value: f64, unit: MassUnit) -> Result<Self, DomainError> {
        use uom::si::mass::{kilogram, ton};
        let quantity = match unit {
            MassUnit::Kilogram => si::Mass::new::<kilogram>(value),
            MassUnit::Tonne => si::Mass::new::<ton>(value),
        };
        Self::try_from(quantity.get::<kilogram>())
    }
}
impl AbsoluteTemperature {
    pub fn new(value: f64, unit: TemperatureUnit) -> Result<Self, DomainError> {
        use uom::si::thermodynamic_temperature::{degree_celsius, kelvin};
        let quantity = match unit {
            TemperatureUnit::Kelvin => si::ThermodynamicTemperature::new::<kelvin>(value),
            TemperatureUnit::DegreeCelsius => {
                si::ThermodynamicTemperature::new::<degree_celsius>(value)
            }
        };
        Self::try_from(quantity.get::<kelvin>())
    }
}
impl TemperatureInterval {
    pub fn new(value: f64, unit: TemperatureUnit) -> Result<Self, DomainError> {
        use uom::si::temperature_interval::{degree_celsius, kelvin};
        let quantity = match unit {
            TemperatureUnit::Kelvin => si::TemperatureInterval::new::<kelvin>(value),
            TemperatureUnit::DegreeCelsius => si::TemperatureInterval::new::<degree_celsius>(value),
        };
        Self::try_from(quantity.get::<kelvin>())
    }
}
impl Time {
    pub fn new(value: f64, unit: TimeUnit) -> Result<Self, DomainError> {
        use uom::si::time::{hour, minute, second};
        let quantity = match unit {
            TimeUnit::Second => si::Time::new::<second>(value),
            TimeUnit::Minute => si::Time::new::<minute>(value),
            TimeUnit::Hour => si::Time::new::<hour>(value),
        };
        Self::try_from(quantity.get::<second>())
    }
}
impl Angle {
    pub fn new(value: f64, unit: AngleUnit) -> Result<Self, DomainError> {
        use uom::si::angle::{degree, radian};
        let quantity = match unit {
            AngleUnit::Radian => si::Angle::new::<radian>(value),
            AngleUnit::Degree => si::Angle::new::<degree>(value),
        };
        Self::try_from(quantity.get::<radian>())
    }
}
impl Stiffness {
    pub fn new(value: f64, unit: StiffnessUnit) -> Result<Self, DomainError> {
        let force = match unit {
            StiffnessUnit::NewtonPerMetre => si::Force::new::<newton>(value),
            StiffnessUnit::KilonewtonPerMetre => si::Force::new::<kilonewton>(value),
        };
        let ratio = force / si::Length::new::<meter>(1.0);
        // uom establishes M T^-2 in its SI base. Our semantic wrapper identifies
        // spring stiffness without exposing an unrelated equal-dimension quantity.
        Self::try_from(ratio.value)
    }
}
impl Stress {
    pub fn new(value: f64, unit: PressureUnit) -> Result<Self, DomainError> {
        let pressure = match unit {
            PressureUnit::Pascal => si::Pressure::new::<pascal>(value),
            PressureUnit::Kilopascal => si::Pressure::new::<kilopascal>(value),
            PressureUnit::Megapascal => si::Pressure::new::<megapascal>(value),
            PressureUnit::Gigapascal => si::Pressure::new::<gigapascal>(value),
        };
        Self::try_from(pressure.get::<pascal>())
    }
}
