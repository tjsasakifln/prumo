//! XML is read by an independent parser, not the SVG compositor under test.

use prumo_components::document::{
    DocumentAdapter, DocumentError, PROJECTION_TOLERANCE_MM, SvgDocumentAdapter, fixture_input,
};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

fn xml_elements(content: &str) -> Vec<(String, BTreeMap<String, String>)> {
    let mut reader = Reader::from_str(content);
    let mut elements = Vec::new();
    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) | Event::Empty(element) => {
                let name = std::str::from_utf8(element.name().as_ref())
                    .unwrap()
                    .to_owned();
                let attributes = element
                    .attributes()
                    .map(|attribute| {
                        let attribute = attribute.unwrap();
                        (
                            std::str::from_utf8(attribute.key.as_ref())
                                .unwrap()
                                .to_owned(),
                            attribute
                                .decoded_and_normalized_value(
                                    quick_xml::XmlVersion::Implicit1_0,
                                    reader.decoder(),
                                )
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                elements.push((name, attributes));
            }
            Event::DocType(_) | Event::PI(_) => panic!("unexpected active/external XML content"),
            Event::Eof => break,
            _ => {}
        }
    }
    elements
}

#[test]
fn independent_xml_parser_confirms_paper_size_scale_and_geometry() {
    let adapter: Box<dyn DocumentAdapter> = Box::new(SvgDocumentAdapter);
    let result = adapter.render(&fixture_input()).unwrap();
    let elements = xml_elements(&result.content);
    assert_eq!(elements[0].0, "svg");
    assert_eq!(elements[0].1["width"], "297mm");
    assert_eq!(elements[0].1["height"], "210mm");
    assert_eq!(elements[0].1["viewBox"], "0 0 297 210");
    let expected = [
        ("segment-0", [35.0, 145.0, 65.0, 105.0], 50.0),
        ("segment-1", [65.0, 105.0, 95.0, 105.0], 30.0),
    ];
    for (id, coordinates, length) in expected {
        let (_, attributes) = elements
            .iter()
            .find(|(name, attributes)| {
                name == "line" && attributes.get("id").map(String::as_str) == Some(id)
            })
            .unwrap();
        let actual = ["x1", "y1", "x2", "y2"].map(|name| attributes[name].parse::<f64>().unwrap());
        for (actual, expected) in actual.into_iter().zip(coordinates) {
            assert!((actual - expected).abs() <= PROJECTION_TOLERANCE_MM);
        }
        let actual_length = (actual[2] - actual[0]).hypot(actual[3] - actual[1]);
        assert!((actual_length - length).abs() <= PROJECTION_TOLERANCE_MM);
    }
}

#[test]
fn text_is_escaped_and_only_passive_allowlisted_elements_are_composed() {
    let adapter = SvgDocumentAdapter;
    let mut input = fixture_input();
    input.title = "Controle <script>x</script> & BIM".into();
    input.segments[0].label = "<image href='x'>".into();
    let result = adapter.render(&input).unwrap();
    assert!(
        result
            .content
            .contains("&lt;script&gt;x&lt;/script&gt; &amp; BIM")
    );
    assert!(!result.content.contains("<script>"));
    for (name, attributes) in xml_elements(&result.content) {
        assert!(
            [
                "svg", "title", "desc", "rect", "text", "line", "circle", "g"
            ]
            .contains(&name.as_str())
        );
        assert!(!attributes.contains_key("href"));
        assert!(!attributes.contains_key("xlink:href"));
        assert!(!attributes.keys().any(|key| key.starts_with("on")));
    }
}

#[test]
fn invalid_values_and_oversized_geometry_are_rejected_without_silent_rescaling() {
    let adapter = SvgDocumentAdapter;
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut input = fixture_input();
        input.segments[0].end_m[0] = invalid;
        assert_eq!(adapter.render(&input), Err(DocumentError::InvalidGeometry));
    }
    let mut input = fixture_input();
    input.scale_denominator = 1;
    assert_eq!(adapter.render(&input), Err(DocumentError::OutsideSheet));
    assert_eq!(input.scale_denominator, 1);
    input.scale_denominator = 0;
    assert_eq!(adapter.render(&input), Err(DocumentError::InvalidProfile));
    input = fixture_input();
    input.segments[0].end_m = input.segments[0].start_m;
    assert_eq!(adapter.render(&input), Err(DocumentError::InvalidGeometry));
    input = fixture_input();
    input.title = "W".repeat(64);
    assert_eq!(adapter.render(&input), Err(DocumentError::InvalidText));
    input.title = "W".repeat(40);
    assert!(adapter.render(&input).is_ok());
    for forbidden in ['\u{fffe}', '\u{ffff}'] {
        input = fixture_input();
        input.title = format!("Controle {forbidden}");
        assert_eq!(adapter.render(&input), Err(DocumentError::InvalidText));
        input = fixture_input();
        input.segments[0].label = format!("Peça {forbidden}");
        assert_eq!(adapter.render(&input), Err(DocumentError::InvalidText));
    }
    assert!(adapter.render(&fixture_input()).is_ok());
}

#[test]
fn explicit_scale_is_linear_and_rendering_is_deterministic() {
    let adapter = SvgDocumentAdapter;
    let mut input = fixture_input();
    input.scale_denominator = 200;
    let first = adapter.render(&input).unwrap();
    let second = adapter.render(&input).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.projected_segments[0].start_mm, [35.0, 145.0]);
    assert_eq!(first.projected_segments[0].end_mm, [50.0, 125.0]);
    assert_eq!(first.projected_segments[1].end_mm, [65.0, 125.0]);
}

#[test]
fn document_experiment_produces_inspectable_svg_and_numeric_evidence() {
    let report = prumo_components::document::run_experiment();
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.iterations, 100);
    assert!(report.elapsed_ns > 0);
    let artifact = report.artifact.unwrap();
    assert_eq!(artifact.media_type, "image/svg+xml");
    assert!(!xml_elements(&artifact.content).is_empty());
}
