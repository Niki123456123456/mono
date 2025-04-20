use egui::Widget;

use crate::models::ModalContext;

pub fn show(
    config: crate::adapters::vault::ConnectionConfig,
    orginal: &std::collections::BTreeMap<String, String>,
    new: std::collections::BTreeMap<String, String>,
    vault_path: String,
    vault_name: String,
    deployment_name: String,
) -> crate::models::Modal {
    let orginal = ReadOnlyTextBuffer {
        text: serde_json::to_string_pretty(&orginal).unwrap(),
    };
    let new_data = ReadOnlyTextBuffer {
        text: serde_json::to_string_pretty(&new).unwrap(),
    };
    crate::models::Modal::new(
        format!("sercret: {}/{}", vault_path, vault_name),
        move |ui: &mut egui::Ui, ctx: &mut ModalContext| {
            ui.set_width(1250.0);

            ui.heading(format!(
                "Update {} / {} for {}",
                vault_path, vault_name, deployment_name
            ));

            ui.columns(2, |columns: &mut [egui::Ui]| {
                show_vault_secrets(&mut columns[0], "old", &orginal);
                show_vault_secrets(&mut columns[1], "new", &new_data);
            });

            ui.separator();
            egui::Sides::new().show(
                ui,
                |_ui| {},
                |ui| {
                    if ui.button("Save").clicked() {
                        ctx.close = true;
                        let update_result =crate::adapters::vault::update_secret(
                            &config,
                            &format!("{}/{}", vault_path, vault_name),
                            &new,
                        );
                        if let Err(err) = update_result {
                            ctx.toasts.error(format!("Error Updating {} / {}:\n {}",  vault_path, vault_name, err));
                        } else {
                            ctx.toasts.success(format!("Updating {} / {} succeed!",  vault_path, vault_name));
                            ctx.reload = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        ctx.close = true;
                    }
                },
            );
        },
    )
}

pub fn show_vault_secrets(ui: &mut egui::Ui, text: &str, json: &ReadOnlyTextBuffer) {
    ui.label(text);
    show_text(ui, json);
}

pub fn show_text(ui: &mut egui::Ui, text: &ReadOnlyTextBuffer) {
    let theme = egui_extras::syntax_highlighting::CodeTheme::default();

    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
        let mut layout_job = egui_extras::syntax_highlighting::highlight(
            ui.ctx(),
            ui.style(),
            &theme,
            string,
            "json",
        );
        layout_job.wrap.max_width = wrap_width;
        layout_job.wrap.overflow_character = None;
        ui.fonts(|f| f.layout_job(layout_job))
    };

    egui::TextEdit::multiline(&mut text.clone())
        .font(egui::TextStyle::Monospace)
        .code_editor()
        .desired_width(ui.available_width())
        .desired_rows(5)
        .layouter(&mut layouter)
        .ui(ui);
}

#[derive(Clone)]
pub struct ReadOnlyTextBuffer {
    pub text: String,
}

impl egui::TextBuffer for ReadOnlyTextBuffer {
    fn is_mutable(&self) -> bool {
        false
    }

    fn as_str(&self) -> &str {
        &self.text.as_str()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        0
    }

    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {}
}
