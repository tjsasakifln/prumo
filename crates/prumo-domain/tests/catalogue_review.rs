use prumo_domain::catalogue::sections::{SectionInput, TorsionState};
use prumo_domain::catalogue::{SectionShape, analyse_section};

fn ellipse(a: f64, b: f64, vertices: usize) -> Vec<[f64; 2]> {
    (0..vertices)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / vertices as f64;
            [a * angle.cos(), b * angle.sin()]
        })
        .collect()
}

#[test]
fn polygon_torsion_matches_an_independent_ellipse_solution_and_fourth_power_scaling() {
    // Closed solution: J = pi*a^3*b^3/(a^2+b^2). The 0.5% limit is
    // selected before execution and includes the 128-sided boundary approximation.
    let shape = SectionShape::Polygon {
        section: SectionInput {
            exterior_m: ellipse(2.0, 1.0, 128),
            holes_m: vec![],
        },
    };
    let result = analyse_section(&shape).unwrap();
    assert_eq!(
        result.torsion.state,
        TorsionState::Converged,
        "{:#?}",
        result.torsion
    );
    let expected = std::f64::consts::PI * 8.0 / 5.0;
    let actual = result.torsion.j_m4.unwrap();
    assert!(
        (actual - expected).abs() <= 0.005 * expected,
        "{actual} versus {expected}"
    );
    assert!(actual < 0.9 * (result.geometry.ixx_m4 + result.geometry.iyy_m4));
    let scaled = analyse_section(&SectionShape::Polygon {
        section: SectionInput {
            exterior_m: ellipse(0.5, 0.25, 128),
            holes_m: vec![],
        },
    })
    .unwrap();
    assert_eq!(scaled.torsion.state, TorsionState::Converged);
    assert!((scaled.torsion.j_m4.unwrap() * 256.0 - actual).abs() <= 1e-9 * actual);
    assert!(
        (scaled.geometry.area_m2 * 16.0 - result.geometry.area_m2).abs()
            <= 1e-12 * result.geometry.area_m2
    );
    assert!(!result.engineering_release && !scaled.engineering_release);
}

#[test]
fn an_annular_section_keeps_its_hole_and_matches_the_independent_circular_solution() {
    // Circular annulus: J = pi*(R^4-r^4)/2. The polygon uses 64 vertices
    // per ring; 0.5% is fixed for geometric approximation plus numerical error.
    let mut hole = ellipse(0.5, 0.5, 64);
    hole.reverse();
    let result = analyse_section(&SectionShape::Polygon {
        section: SectionInput {
            exterior_m: ellipse(1.0, 1.0, 64),
            holes_m: vec![hole],
        },
    })
    .unwrap();
    assert_eq!(
        result.torsion.state,
        TorsionState::Converged,
        "{:#?}",
        result.torsion
    );
    let expected = std::f64::consts::PI * (1.0 - 0.5_f64.powi(4)) / 2.0;
    let actual = result.torsion.j_m4.unwrap();
    assert!(
        (actual - expected).abs() <= 0.005 * expected,
        "{actual} versus {expected}"
    );
    let expected_area = std::f64::consts::PI * (1.0 - 0.5_f64.powi(2));
    assert!((result.geometry.area_m2 - expected_area).abs() <= 0.002 * expected_area);
    assert!(result.geometry.area_m2 < 0.8 * std::f64::consts::PI);
    assert!(!result.engineering_release);
}
