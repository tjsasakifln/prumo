//! Ensaio isolado do componente uom 0.38.0, anterior à integração do domínio.
//! Configuração reproduzível em docs/componentes/PRU-004.md.
//! Não representa um cálculo disponível no produto.

/// Quantidades incompatíveis não devem ser intercambiáveis.
/// ```compile_fail
/// use uom::si::{f64::{Force, Torque}, force::newton};
/// let moment: Torque = Force::new::<newton>(1.0);
/// ```
/// Dimensões iguais não tornam energia e momento a mesma grandeza.
/// ```compile_fail
/// use uom::si::{f64::{Energy, Torque}, energy::joule};
/// let moment: Torque = Energy::new::<joule>(1.0);
/// ```
pub fn contract() {}

#[cfg(test)]
mod tests {
    use uom::si::{
        angle::{degree, radian},
        f64::*,
        force::{kilonewton, newton},
        length::{meter, millimeter},
        pressure::{megapascal, pascal},
        thermodynamic_temperature::{degree_celsius, kelvin},
        torque::{kilonewton_meter, newton_meter},
    };

    #[test]
    fn closed_conversion_references() {
        assert_eq!(Length::new::<millimeter>(1000.0).get::<meter>(), 1.0);
        assert_eq!(Force::new::<kilonewton>(2.0).get::<newton>(), 2000.0);
        assert_eq!(
            Torque::new::<kilonewton_meter>(2.0).get::<newton_meter>(),
            2000.0
        );
        assert_eq!(
            Pressure::new::<megapascal>(2.0).get::<pascal>(),
            2_000_000.0
        );
        assert!((Angle::new::<degree>(180.0).get::<radian>() - std::f64::consts::PI).abs() < 1e-14);
        assert!(
            (ThermodynamicTemperature::new::<degree_celsius>(0.0).get::<kelvin>() - 273.15).abs()
                < 1e-12
        );
        assert_eq!(
            TemperatureInterval::new::<uom::si::temperature_interval::degree_celsius>(10.0)
                .get::<uom::si::temperature_interval::kelvin>(),
            10.0,
        );
    }

    #[test]
    fn domain_must_supply_finiteness_and_physical_range_guards() {
        assert!(Force::new::<newton>(f64::NAN).get::<newton>().is_nan());
        assert!(ThermodynamicTemperature::new::<kelvin>(-1.0).get::<kelvin>() < 0.0);
    }
}
