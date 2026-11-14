//! Locale-tolerant numeric entry with visible errors and no silent fallback submission.
use egui::{Color32, Id, Ui};

#[derive(Clone)]
struct Entry {
    text: String,
    last_value: f64,
}

fn errors_id() -> Id {
    Id::unique("prumo-active-numeric-form-errors")
}

pub(crate) fn begin(ui: &mut Ui) {
    ui.data_mut(|data| data.insert_temp(errors_id(), Vec::<String>::new()));
    context(ui, "form".into());
}

pub(crate) fn context(ui: &mut Ui, scope: String) {
    ui.data_mut(|data| data.insert_temp(Id::unique("prumo-number-context"), scope));
}

pub(crate) fn discard_edit_buffers(ui: &mut Ui) {
    ui.data_mut(|data| data.remove_by_type::<Entry>());
}

pub(crate) fn errors(ui: &Ui) -> Vec<String> {
    ui.data(|data| {
        data.get_temp::<Vec<String>>(errors_id())
            .unwrap_or_default()
    })
}

pub(crate) fn field(ui: &mut Ui, label: &str, value: &mut f64) {
    let scope = ui.data(|data| {
        data.get_temp::<String>(Id::unique("prumo-number-context"))
            .unwrap_or_default()
    });
    let id = ui.make_persistent_id(("number", scope, label));
    let mut entry = ui
        .data(|data| data.get_temp::<Entry>(id))
        .unwrap_or_else(|| Entry {
            text: value.to_string(),
            last_value: *value,
        });
    // Selection of another entity is an explicit reload of its values.
    if entry.last_value.to_bits() != value.to_bits() {
        entry = Entry {
            text: value.to_string(),
            last_value: *value,
        };
    }
    let caption = ui.label(label);
    ui.add(
        egui::TextEdit::singleline(&mut entry.text)
            .id(id)
            .desired_width(80.0),
    )
    .labelled_by(caption.id)
    .on_hover_text("Use vírgula ou ponto decimal. Não use espaços nem separadores de milhares.");
    match parse(&entry.text) {
        Some(parsed) => {
            *value = parsed;
            entry.last_value = parsed;
        }
        None => {
            ui.colored_label(Color32::LIGHT_RED, "Número inválido");
            ui.data_mut(|data| {
                let mut errors = data
                    .get_temp::<Vec<String>>(errors_id())
                    .unwrap_or_default();
                errors.push(label.to_owned());
                data.insert_temp(errors_id(), errors);
            });
        }
    }
    ui.data_mut(|data| data.insert_temp(id, entry));
}

fn parse(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty()
        || text.chars().any(char::is_whitespace)
        || (text.contains(',') && text.contains('.'))
        || text.matches(',').count() > 1
    {
        return None;
    }
    let value = text.replace(',', ".").parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}
