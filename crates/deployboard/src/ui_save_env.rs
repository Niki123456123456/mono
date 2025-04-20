use egui::Widget;

use crate::{models::ModalContext, yaml};

pub fn generate_env(
    key: String,
    value: String,
    config: &crate::adapters::gitlab::Config,
) -> serde_yaml::Value {
    let mut result = serde_yaml::Value::Null;
    yaml::set_field(
        &mut result,
        &yaml::path_from_str(&config.env_name_path),
        &serde_yaml::Value::String(key),
        true,
    );
    yaml::set_field(
        &mut result,
        &yaml::path_from_str(&config.env_value_path),
        &serde_yaml::Value::String(value),
        true,
    );
    return result;
}

pub fn generate_envs(
    values: std::collections::BTreeMap<String, String>,
    config: &crate::adapters::gitlab::Config,
) -> serde_yaml::Value {
    let mut results = vec![];
    for (key, value) in values.into_iter() {
        results.push(generate_env(key, value, config));
    }
    return serde_yaml::Value::Sequence(results);
}

pub fn show(
    config: crate::adapters::gitlab::Config,
    orginal_map: &std::collections::BTreeMap<String, String>,
    new_map: std::collections::BTreeMap<String, String>,
    project_id: String,
    file_path: String,
    raw: String,
    image_path: String,
    image_source_path: crate::yaml::Path,
    deployment_env: String,
    deployment_name: String,
) -> crate::models::Modal {
    let orginal = ReadOnlyTextBuffer {
        text: serde_json::to_string_pretty(&orginal_map).unwrap(),
    };
    let new_data = ReadOnlyTextBuffer {
        text: serde_json::to_string_pretty(&new_map).unwrap(),
    };
    let mut commit_message = format!(
        "{} {}: Update Env for {}",
        deployment_env, deployment_name, image_path
    );
    crate::models::Modal::new(
        format!("env for {}", image_path),
        move |ui: &mut egui::Ui, ctx: &mut ModalContext| {
            ui.set_width(1250.0);

            ui.heading(format!("Update Env for {}", image_path));

            ui.horizontal(|ui|{
                ui.label("commit message ");
                egui::TextEdit::singleline(&mut commit_message).desired_width(ui.available_width()).ui(ui);
            });

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

                        let mut yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw).unwrap();

                        let mut envs_path = crate::yaml::path_from_str(&config.envs_path);
                        crate::yaml::enrich_path_with_indices(&mut envs_path, &image_source_path);
                        let envs = generate_envs(new_map.clone(), &config);
                        crate::yaml::set_field(&mut yaml, &envs_path, &envs, false);
                        let new_text = serde_yaml::to_string(&yaml).unwrap();

                        // println!("{}", new_text);

                        ctx.close = true;
                        let update_result = crate::adapters::gitlab::update_file(
                            &config.connection,
                            &project_id,
                            &file_path,
                            &crate::adapters::gitlab::FileUpdate {
                                branch: "main".to_string(),
                                commit_message: commit_message.clone(),
                                content: new_text,
                                author_email: config.author.email.clone(),
                                author_name: config.author.name.clone(),
                            },
                        );

                        if let Err(err) = update_result {
                            ctx.toasts
                                .error(format!("Error Updating Env for {}:\n {}", image_path, err));
                        } else {
                            ctx.toasts
                                .success(format!("Updating Env for {} succeed!", image_path));
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
