//! Emit the bounded qualification drawing for visual inspection.
use prumo_components::document::{DocumentAdapter, SvgDocumentAdapter, fixture_input};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = SvgDocumentAdapter.render(&fixture_input())?;
    std::io::stdout().write_all(artifact.content.as_bytes())?;
    Ok(())
}
