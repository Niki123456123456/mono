mod adapters {
    pub mod argocd;
    pub mod gitlab;
    pub mod harbor;
    pub mod http;
    pub mod vault;
}
mod config;
mod core;
mod models;
mod ui;
mod ui_save_env;
mod ui_save_vault;
mod ui_settings;
mod yaml;

use common::RemoveWhere;
use egui::{RichText, Widget};

// cargo bundle --bin deployboard ; ln -s /Applications target/release/bundle/osx/Applications ; hdiutil create -volname "deployboard" -srcfolder target/release/bundle/osx -ov -format UDZO deployboard.dmg
fn main() {
    common::app::run("deployboard", |cc| {
        let mut app = App::new(cc);
        return Box::new(move |ctx: common::app::Context<'_>| {
            app.update(ctx);
        });
    });
}

pub struct App {
    pub config: crate::models::EditorContext<crate::config::Config>,
    pub show_settings: bool,
    pub project_by_name:
        poll_promise::Promise<std::collections::BTreeMap<String, crate::models::DeployProject>>,
    pub env_settings: Vec<bool>,
    pub search: String,
    pub modals: Vec<crate::models::Modal>,
    pub toasts: egui_notify::Toasts,
    pub egui_ctx: egui::Context,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config: crate::config::Config = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let (sender, promise) = poll_promise::Promise::new();

        sender.send(Default::default());

        let mut app = App {
            config: crate::models::EditorContext::new(config).always_saveable(),
            project_by_name: promise,
            env_settings: vec![],
            search: String::default(),
            modals: vec![],
            toasts: egui_notify::Toasts::default(),
            show_settings: false,
            egui_ctx: cc.egui_ctx.clone(),
        };

        app.reload(true);
        return app;
    }

    pub fn reload(&mut self, vault: bool) {
        if vault {
            if let Ok(token) =
                crate::adapters::vault::get_token(&self.config.vault.connection.endpoint)
            {
                let t = &mut self.config.vault.connection.token;
                *t = token;
            }
        }
        let config = self.config.clone();
        let (sender, promise) = poll_promise::Promise::new();
        let ctx = self.egui_ctx.clone();
        common::execute(async move {
            let mut projects_by_name = crate::core::get_projects(&config, ctx).await;

            sender.send(projects_by_name);
        });
        self.project_by_name = promise;
        self.env_settings = self.config.envs.iter().map(|x| true).collect();

        // for (name, project) in self.project_by_name.iter() {
        //     if let Some(new_project) = projects_by_name.get_mut(name) {
        //         new_project.details_open = project.details_open;
        //     }
        // }
    }

    pub fn update(&mut self, mut ctx: common::app::Context<'_>) {
        crate::ui_settings::show_settings(self, &mut ctx);

        self.egui_ctx = ctx.ui.ctx().clone();

        

        let mut show_settings = false;
        egui::Sides::new().show(
            ctx.ui,
            |ui| {
                ui.horizontal(|ui| {
                    if ui.button("⟳").clicked() {
                        self.reload(false);
                    }
                    for (i, enabled) in self.env_settings.iter_mut().enumerate() {
                        if ui
                            .selectable_label(*enabled, &self.config.envs[i])
                            .clicked()
                        {
                            *enabled = !*enabled;
                        }
                    }
                    egui::TextEdit::singleline(&mut self.search)
                        .return_key(Some(egui::KeyboardShortcut::new(
                            egui::Modifiers::NONE,
                            egui::Key::Enter,
                        )))
                        .cursor_at_end(true)
                        .hint_text("search")
                        .ui(ui);

                    if !self.search.is_empty() && ui.button("x").clicked() {
                        self.search = Default::default();
                    }
                });
            },
            |ui| {
                if ui.button("⚙").clicked() {
                    show_settings = true;
                }
            },
        );

        if show_settings {
            self.show_settings = true;
        }

        if !self.env_settings.is_empty() {
            let envs: Vec<_> = self
                .config
                .envs
                .iter()
                .enumerate()
                .filter_map(|(i, t)| if self.env_settings[i] { Some(t) } else { None })
                .collect();
            ctx.ui.columns(envs.len(), |columns: &mut [egui::Ui]| {
                for (i, &env) in envs.iter().enumerate() {
                    let ui = &mut columns[i];
                    ui.label(RichText::new(env).strong());
                }
            });
            egui::ScrollArea::vertical().show(ctx.ui, |ui| {
                if let Some(project_by_name) = self.project_by_name.ready_mut() {
                    for (_, project) in project_by_name.iter_mut() {
                        crate::ui::show_project(
                            project,
                            &self.config,
                            ui,
                            &envs,
                            &self.search,
                            &mut self.modals,
                        );
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("loading");
                        ui.spinner();
                    });
                }
            });
        }

        let mut reload = false;
        let egui_ctx = ctx.ui.ctx();
        self.modals.remove_where(|modal| {
            let mut m_ctx = crate::models::ModalContext {
                close: false,
                reload: false,
                toasts: &mut self.toasts,
            };
            let resp = egui::Modal::new(egui::Id::new(&modal.id)).show(egui_ctx, |ui| {
                (modal.ui)(ui, &mut m_ctx);
            });
            if m_ctx.reload {
                reload = true;
            }
            if m_ctx.close || resp.should_close() {
                return true;
            }
            return false;
        });

        if reload {
            self.reload(false);
        }

        self.toasts.show(&self.egui_ctx);
    }
}
