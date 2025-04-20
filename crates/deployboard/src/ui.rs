use egui::{Ui, Widget};

use crate::{adapters::gitlab::FileUpdate, models::{EditorContext, ModalContext}, ui_save_vault::show};

pub fn show_project(
    project: &mut crate::models::DeployProject,
    config: &crate::config::Config,
    ui: &mut Ui,
    envs: &Vec<&String>,
    search: &str, modals : &mut Vec<crate::models::Modal>
) {
    if !search.is_empty() && !project.name.contains(search) {
        return;
    }
    ui.columns(envs.len(), |columns: &mut [Ui]| {
        for (i, &env) in envs.iter().enumerate() {
            let ui = &mut columns[i];
            if let Some(deployment) = project.deployments_by_env.get_mut(env) {
                ui.horizontal(|ui| {
                    if egui::Label::new(">")
                        .selectable(false)
                        .sense(egui::Sense::click())
                        .ui(ui)
                        .clicked()
                    {
                        project.details_open = !project.details_open;
                    }
                    ui.label(&project.name);
                    if let Some(git_project) = &deployment.git_project {
                        ui.hyperlink_to(
                            "src",
                            &format!("{}/-/blob/main/{}", git_project.web_url, deployment.path),
                        );
                    }
                    if let Some(endpoint) = &deployment.argocd_endpoint() {
                        ui.hyperlink_to(
                            "argocd",
                            &format!("{}/applications/argocd/{}{}", endpoint, deployment.argocd_prefix().unwrap_or_default(), deployment.name.to_lowercase()),
                        );
                    }
                });

                if project.details_open {
                    if deployment.content.is_none() {
                        crate::core::fill_deployment(deployment, config, ui.ctx().clone());
                    }
                
                    if let Some(content) = deployment.content.as_mut() {
                        match content.ready_mut() {
                            Some(content) => {
                                if content.images.is_empty() && content.secrets.is_empty() {
                                    ui.label("no images / sercets found");
                                }
                                for image in content.images.iter_mut() {
                                    
                                    let tags : Vec<_> = image.artifact.tags.iter().map(|x|x.name.as_str()).collect();
                                    ui.horizontal(|ui| {
                                        let resp = ui.label(format!(
                                            "{}: {}",
                                            image.identifier.path, tags.join(" | ")
                                        ));
                                        resp.context_menu(|ui|{
                                            ui.hyperlink_to(
                                                "src",
                                                &format!("{}/harbor/projects/{}/repositories/{}/artifacts-tab", config.harbor.connection.endpoint, config.harbor.project_id, urlencoding::encode(&image.identifier.path)),
                                            );
                                           
                                            ui.menu_button("change to image", |ui| {
                                                for artifact in image.artifacts.iter() {
                                                    let tags : Vec<_> = artifact.tags.iter().map(|x|x.name.as_str()).collect();
                                                    if ui.button(format!(
                                                        "{}", tags.join(" | ")
                                                    )).clicked() {
                                                        let gitlab = config.gitlab.clone();
                                                        let deployment_name = deployment.name.clone();
                                                        let deployment_env = deployment.env.clone();
                                                        let image = image.clone();
                                                        let new_artifact = artifact.clone();
                                                        let raw = content.raw.clone();
                                                        let project = deployment.source.gitlab_project.clone();
                                                        let path = deployment.path.clone();
                                                        let mut commit_message = format!("{} {}: update image to {}", deployment_env, deployment_name, new_artifact.tags[0].name);
                                                        modals.push(crate::models::Modal::new(format!("{}:{}->{}", deployment.name, artifact.tags[0].name, image.artifact.tags[0].name), move |ui: &mut Ui, ctx: &mut ModalContext| {
                                                            
                                                            ui.set_width(750.0);

                                                            ui.heading(format!("{} {}: Update to {}:{}", deployment_env, deployment_name, image.identifier.path, new_artifact.tags[0].name));

                                                            ui.horizontal(|ui|{
                                                                ui.label("commit message ");
                                                                egui::TextEdit::singleline(&mut commit_message).desired_width(ui.available_width()).ui(ui);
                                                            });
                                                            ui.columns(2, |columns: &mut [Ui]| {
                                                                show_artifact(&mut columns[0], &image.artifact, "old");
                                                                show_artifact(&mut columns[1], &new_artifact, "new");
                                                            });
                                                            
                                                            ui.separator();
                                                            egui::Sides::new().show(
                                                                ui,
                                                                |_ui| {},
                                                                |ui| {
                                                                    if ui.button("Save").clicked() {
                                                                        let mut yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw).unwrap();

                                                                        let new_image = image.identifier.to_string_with_tag(&new_artifact.tags[0].name);
                                                                        crate::yaml::set_field(&mut yaml, &image.source_path, &serde_yaml::Value::String(new_image.clone()), false);
                                                                        let new_text = serde_yaml::to_string(&yaml).unwrap();
                                                                        ctx.close = true;
                                                                        let update_result = crate::adapters::gitlab::update_file(&gitlab.connection, &project, &path, &FileUpdate{ 
                                                                            branch: "main".to_string(), 
                                                                            commit_message: commit_message.clone(), 
                                                                            content: new_text, 
                                                                            author_email: gitlab.author.email.clone(), 
                                                                            author_name: gitlab.author.name.clone() });
                                                                        if let Err(err) = update_result {
                                                                            ctx.toasts.error(format!("Error Updating {} to {}:\n {}",  deployment_name, new_image, err));
                                                                        } else {
                                                                            ctx.toasts.success(format!("Updating {} to {} succeed!", deployment_name, new_image));
                                                                            ctx.reload = true;
                                                                        }
                                                                    }
                                                                    if ui.button("Cancel").clicked() {
                                                                        ctx.close = true;
                                                                    }
                                                                },
                                                            );
                                                        }));
                                                    }
                                                }
                                            });
                                        });
                                       
                                    });
                                    if let Some(envs_json) = &mut image.envs_json {
                                        show_text(ui, ui.next_auto_id(), |ui|{
                                            
                                        },envs_json, |orginal,new|{
                                            modals.push(crate::ui_save_env::show(config.gitlab.clone(), orginal, new, deployment.source.gitlab_project.clone(), deployment.path.clone(), content.raw.clone(), image.identifier.path.clone(), image.source_path.clone(), deployment.env.clone(), deployment.name.clone()));
                                        });
                                    }
                                    
                                }
                                
                                
                                
                                let vault_path = deployment
                                                            .source
                                                            .vault_path
                                                            .as_ref()
                                                            .or(deployment
                                                                .source
                                                                .vault_paths
                                                                .as_ref()
                                                                .and_then(|x| x.get(&deployment.env)))
                                                            .cloned()
                                                            .unwrap_or_default();
                                let deployment_name = deployment.name.clone();
                                for secret in content.secrets.iter_mut() {
                                    
                                    show_text(ui,ui.next_auto_id(), |ui|{
                                        let resp = ui.label(format!("secrets: {}", secret.vault_name));

                                        resp.context_menu(|ui|{
                                            let vault_path = deployment
                                                .source
                                                .vault_path
                                                .as_ref()
                                                .or(deployment
                                                    .source
                                                    .vault_paths
                                                    .as_ref()
                                                    .and_then(|x| x.get(&deployment.env)))
                                                .cloned().unwrap_or_default();
        
                                            ui.hyperlink_to(
                                                "src",
                                                &format!("{}/ui/vault/secrets/secret/kv/{}/details", config.vault.connection.endpoint, urlencoding::encode(&format!("{}/{}", vault_path, secret.vault_name))),
                                            );
                                            
                                        });
                                    }, &mut secret.secrets, |orginal_secrets,new_secrets|{
                                        modals.push(show(config.vault.connection.clone(), orginal_secrets, new_secrets,  vault_path.clone(), secret.vault_name.clone(), deployment_name.clone()));
                                    });
                                }
                            },
                            None => {
                                ui.horizontal(|ui|{
                                    ui.label("loading");
                                    ui.spinner();
                                });
                                
                            },
                        }
                        
                    }
                }
            }
        }
    });
}

pub fn show_artifact(ui: &mut Ui, artifact: & crate::adapters::harbor::Artifact, text : &str) {
    let tags : Vec<_> = artifact.tags.iter().map(|x|x.name.as_str()).collect();
    ui.label(text);
    ui.label( tags.join(" "));
    ui.label( artifact.push_time.format("%d.%m.%Y %H:%M").to_string());
}


pub fn show_text<T: serde::de::DeserializeOwned + serde::Serialize>(ui: &mut Ui, id_salt: impl std::hash::Hash, title: impl FnOnce(&mut Ui), context: & mut EditorContext<T>, save : impl FnOnce(&T,T)) {

    egui::Sides::new().show(
        ui,title,
        |ui| {
            if context.has_changes() || context.always_saveable {
                let result = serde_json::from_str::<T>(&context.text);
                if  ui.add_enabled(result.is_ok(),  egui::Button::new("Save")).clicked() {
                    (save)(&context.orignal_data, result.unwrap());
                }

                if ui.button("Reset").clicked() {
                    context.reset();
                }
            }
            
        },
    );

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

    egui::ScrollArea::vertical().id_salt(id_salt).max_height(ui.available_width()).show(ui, |ui| {
        egui::TextEdit::multiline(&mut context.text)
        .font(egui::TextStyle::Monospace)
        .code_editor()
        .desired_width(ui.available_width())
        .desired_rows(5)
        .layouter(&mut layouter)
        .ui(ui);
    });

   
}

