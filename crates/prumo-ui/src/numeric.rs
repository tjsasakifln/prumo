//! Locale-tolerant numeric entry with visible errors and no silent fallback submission.
use egui::{Color32, Id, Ui};
use std::collections::HashMap;

#[derive(Clone)]
struct Entry {
    text: String,
    last_value: f64,
}

// Collapsed controls are not rendered by egui. Keep their invalid text and
// error independently of the per-frame list used by the older, flat forms.
#[derive(Clone)]
struct FormEntry(Entry);

#[derive(Clone, Default)]
struct FormEntries {
    fields: HashMap<Id, String>,
    errors: HashMap<Id, String>,
}

fn active_form_id() -> Id {
    Id::unique("prumo-active-persistent-numeric-form")
}

fn form_id(scope: &str) -> Id {
    Id::unique("prumo-persistent-numeric-form").with(scope)
}

fn active_form(ui: &Ui) -> Option<Id> {
    ui.data(|data| data.get_temp::<Id>(active_form_id()))
}

fn errors_id() -> Id {
    Id::unique("prumo-active-numeric-form-errors")
}

pub(crate) fn begin(ui: &mut Ui) {
    ui.data_mut(|data| {
        data.insert_temp(errors_id(), Vec::<String>::new());
        data.remove::<Id>(active_form_id());
    });
    context(ui, "form".into());
}

pub(crate) fn begin_form(ui: &mut Ui, scope: String) {
    begin(ui);
    ui.data_mut(|data| data.insert_temp(active_form_id(), form_id(&scope)));
}

/// Reset one explicitly selected or replaced form without erasing invalid
/// input in another open panel. A normal frame or collapse must never call it.
pub(crate) fn discard_form(ui: &mut Ui, scope: &str) {
    let id = form_id(scope);
    ui.data_mut(|data| {
        if let Some(form) = data.get_temp::<FormEntries>(id) {
            for field in form.fields.keys() {
                data.remove::<FormEntry>(*field);
            }
        }
        data.remove::<FormEntries>(id);
    });
}

/// Use when a control is actually removed from a draft, not merely collapsed.
pub(crate) fn discard_contexts(ui: &mut Ui, matches: impl Fn(&str) -> bool) {
    if let Some(id) = active_form(ui) {
        ui.data_mut(|data| {
            let mut form = data.get_temp::<FormEntries>(id).unwrap_or_default();
            let removed: Vec<_> = form
                .fields
                .iter()
                .filter(|(_, scope)| matches(scope))
                .map(|(id, _)| *id)
                .collect();
            for field in removed {
                data.remove::<FormEntry>(field);
                form.fields.remove(&field);
                form.errors.remove(&field);
            }
            data.insert_temp(id, form);
        });
    }
}

pub(crate) fn context(ui: &mut Ui, scope: String) {
    ui.data_mut(|data| data.insert_temp(Id::unique("prumo-number-context"), scope));
}

pub(crate) fn discard_edit_buffers(ui: &mut Ui) {
    ui.data_mut(|data| data.remove_by_type::<Entry>());
}

pub(crate) fn errors(ui: &Ui) -> Vec<String> {
    ui.data(|data| {
        if let Some(id) = data.get_temp::<Id>(active_form_id()) {
            let mut errors: Vec<_> = data
                .get_temp::<FormEntries>(id)
                .unwrap_or_default()
                .errors
                .into_values()
                .collect();
            errors.sort();
            errors.dedup();
            errors
        } else {
            data.get_temp::<Vec<String>>(errors_id())
                .unwrap_or_default()
        }
    })
}

pub(crate) fn context_has_errors(ui: &Ui, scope: &str) -> bool {
    active_form(ui).is_some_and(|id| {
        ui.data(|data| {
            let form = data.get_temp::<FormEntries>(id).unwrap_or_default();
            form.errors.keys().any(|id| {
                form.fields
                    .get(id)
                    .is_some_and(|field_scope| field_scope == scope)
            })
        })
    })
}

pub(crate) fn field(ui: &mut Ui, label: &str, value: &mut f64) {
    let scope = ui.data(|data| {
        data.get_temp::<String>(Id::unique("prumo-number-context"))
            .unwrap_or_default()
    });
    let form_id = active_form(ui);
    let id = ui.make_persistent_id(("number", form_id, &scope, label));
    let mut entry = ui
        .data(|data| {
            if form_id.is_some() {
                data.get_temp::<FormEntry>(id).map(|entry| entry.0)
            } else {
                data.get_temp::<Entry>(id)
            }
        })
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
    let parsed = parse(&entry.text);
    match parsed {
        Some(parsed) => {
            *value = parsed;
            entry.last_value = parsed;
        }
        None => {
            ui.colored_label(Color32::LIGHT_RED, "Número inválido");
            if form_id.is_none() {
                ui.data_mut(|data| {
                    let mut errors = data
                        .get_temp::<Vec<String>>(errors_id())
                        .unwrap_or_default();
                    errors.push(label.to_owned());
                    data.insert_temp(errors_id(), errors);
                });
            }
        }
    }
    ui.data_mut(|data| {
        if let Some(form_id) = form_id {
            let mut form = data.get_temp::<FormEntries>(form_id).unwrap_or_default();
            form.fields.insert(id, scope);
            if parsed.is_some() {
                form.errors.remove(&id);
            } else {
                form.errors.insert(id, label.to_owned());
            }
            data.insert_temp(form_id, form);
            data.insert_temp(id, FormEntry(entry));
        } else {
            data.insert_temp(id, entry);
        }
    });
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
    let normalized = text.replace(',', ".");
    let value = normalized.parse::<f64>().ok()?;
    let nonzero_mantissa = normalized
        .split(['e', 'E'])
        .next()
        .unwrap_or_default()
        .bytes()
        .any(|byte| matches!(byte, b'1'..=b'9'));
    (value.is_finite() && !(value == 0. && nonzero_mantissa)).then_some(value)
}
