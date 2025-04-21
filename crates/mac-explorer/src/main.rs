pub mod actions;
pub mod files;
pub mod tab;
pub mod tabviewer;
pub mod zip;

use std::{fs, path::Path};

use egui::{Label, Sense, Widget};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};

use crate::{actions::actions,  tab::Tab, tabviewer::AppData};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    pub data: AppData,
    #[serde(skip)]
    pub tabs: DockState<Tab>,
    pub latest_tab_id: u64,
}

impl Default for App {
    fn default() -> Self {
        Self {
            data: Default::default(),
            tabs: DockState::new(vec![]),
            latest_tab_id: 0,
        }
    }
}

fn main() {
    common::app::run("mac-explorer", |cc| {
        
        let mut app: App = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        app.tabs = DockState::new(vec![Tab::new(
            app.data.favorites.first().unwrap_or(&"/".to_string()),
            egui::Id::new(app.latest_tab_id),
        )]);
        app.latest_tab_id += 1;
        //app.tabs.set_active_tab((SurfaceIndex(0), NodeIndex(0), TabIndex(0)));
        app.tabs
            .set_focused_node_and_surface((SurfaceIndex(0), NodeIndex(0)));
        app.data.actions = actions();

        return Box::new(move |mut ctx| {
            app.data.need_save = false;

            let egui_ctx = ctx.ui.ctx().clone();

            if app.tabs.iter_all_tabs().count() == 0 {
                app.tabs = DockState::new(vec![Tab::new(
                    app.data.favorites.first().unwrap_or(&"/".to_string()),
                    egui::Id::new(app.latest_tab_id),
                )]);
                app.tabs
                .set_focused_node_and_surface((SurfaceIndex(0), NodeIndex(0)));
            }
    
            egui::SidePanel::left("favorites_tab").show(&egui_ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("favorites");
                    let mut to_remove = None;
                    for (i, favorite) in app.data.favorites.iter().enumerate() {
                        let path = Path::new(favorite);
                        let file_name = path
                            .file_name()
                            .and_then(|x| Some(x.to_str().unwrap_or_default()))
                            .unwrap_or("unkown");
                        let resp = Label::new(file_name)
                            .sense(Sense::click())
                            .selectable(false)
                            .ui(ui);
                        if resp.clicked() {
                            let tab = app.tabs.find_active_focused();
                            if let Some((rect, tab)) = tab {
                                tab.refresh(favorite);
                            }
                        }
                        resp.context_menu(|ui| {
                            if Label::new("x")
                                .sense(Sense::click())
                                .selectable(false)
                                .ui(ui)
                                .clicked()
                            {
                                ui.close_menu();
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(to_remove) = to_remove {
                        app.data.favorites.remove(to_remove);
                        ctx.save(&app);
                    }
                });
            });
            egui::CentralPanel::default().show(&egui_ctx, |ui| {
                DockArea::new(&mut app.tabs)
                    .show_add_buttons(true)
                    .style({
                        let mut style = Style::from_egui(egui_ctx.style().as_ref());
                        style.tab_bar.fill_tab_bar = true;
                        style
                    })
                    .show(&egui_ctx, &mut app.data);
    
                    app.latest_tab_id += 1;
                    app.data.added_nodes.drain(..).for_each(|(surface, node)| {
                        app.tabs.set_focused_node_and_surface((surface, node));
                        app.tabs.push_to_focused_leaf(Tab::new(
                            app.data.favorites.first().unwrap_or(&"/".to_string()),
                        egui::Id::new(app.latest_tab_id),
                    ));
                });
            });
    
            if let Some((source_path, files)) = &app.data.drag_paths {
                if let Some(dest_path) = &app.data.drop_path {
                    if source_path != dest_path {
                        let command = egui_ctx.input(|i| i.modifiers.command);
                        for (path, file_name) in files.iter() {
                            let target = Path::new(dest_path).join(file_name);
                            if command {
                                if Path::new(path).is_dir() {
                                    let _ = files::copy_dir(Path::new(path), &target);
                                } else {
                                    let _ = fs::copy(path, Path::new(dest_path).join(file_name));
                                }
                                
                            } else {
                                let _ = fs::rename(path, Path::new(dest_path).join(file_name));
                            }
                        }
                    }
                    for ((_, _), tab) in app.tabs.iter_all_tabs_mut() {
                        tab.refresh_hard(tab.path.clone());
                    }
                    app.data.drag_paths = None;
                    app.data.drop_path = None;
                }
            }
    

            if app.data.need_save {
                ctx.save(&app);
            }
        });
    });
}