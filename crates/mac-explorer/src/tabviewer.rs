use crate::{
    actions::Action,
    files::{self, bytes_to_human_readable},
    tab::{SortingColumn, Tab},
};
use std::path::Path;

use egui::{Id, Key, Label, Modifiers, PointerButton, Rect, Sense, TextEdit, Widget};
use egui_dock::{NodeIndex, SurfaceIndex};
use egui_extras::{Column, TableBuilder};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct AppData {
    pub favorites: Vec<String>,
    #[serde(skip)]
    pub added_nodes: Vec<(SurfaceIndex, NodeIndex)>,
    #[serde(skip)]
    pub actions: Vec<Action>,
    #[serde(skip)]
    pub drag_paths: Option<(String, Vec<(String, String)>)>,
    pub drop_path: Option<String>,
}

fn show_sorting_header(
    header: &mut egui_extras::TableRow<'_, '_>,
    sorting: &mut crate::tab::TabSorting,
    text: &str,
    column: SortingColumn,
) -> bool {
    let mut clicked = false;
    header.col(|ui| {
        let after = if sorting.column == column {
            if sorting.reverse {
                " ⬇"
            } else {
                " ⬆"
            }
        } else {
            ""
        };
        if Label::new(egui::RichText::from(format!("{}{}", text, after)).strong())
            .selectable(false)
            .sense(Sense::click())
            .ui(ui)
            .clicked()
        {
            clicked = true;
            if sorting.column == column {
                sorting.reverse = !sorting.reverse;
            } else {
                sorting.column = column;
                sorting.reverse = false;
            }
        }
    });
    return clicked;
}

impl egui_dock::TabViewer for AppData {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match &tab.info {
            Ok(info) => info.path.clone().into(),
            Err(_) => "invalid path".into(),
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.id
    }

    fn on_add(&mut self, surface: SurfaceIndex, node: NodeIndex) {
        self.added_nodes.push((surface, node));
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.state.relead = false;

        if ui.input(|i| i.pointer.button_clicked(PointerButton::Extra1)) {
            // previous
            if !tab.previous_paths.is_empty() {
                let last = tab.previous_paths.remove(tab.previous_paths.len() - 1);
                let mut new = Tab::new(last.clone(), tab.id);
                new.previous_paths.append(&mut tab.previous_paths);
                new.previous_paths2.append(&mut tab.previous_paths2);
                new.previous_paths2.push(last);
                *tab = new;
            }
        }
        if ui.input(|i| i.pointer.button_clicked(PointerButton::Extra2)) {}
        ui.horizontal(|ui| {
            if ui.button("★").clicked() {
                if !self.favorites.contains(&tab.path) {
                    self.favorites.push(tab.path.clone());
                }
            }
            if ui.button("⬅").clicked() {}
            if ui.button("➡").clicked() {}
            if ui.button("⬆").clicked() {
                let p = tab.path.clone();
                let path = Path::new(&p);
                if let Some(parent) = path.parent() {
                    tab.refresh(parent.to_str().unwrap_or_default());
                }
            }
            if ui.button("⟳").clicked() {
                tab.refresh_hard(tab.path.clone());
            }
            let search_width = 150.0;
            let resp = TextEdit::singleline(&mut tab.path)
                .desired_width(ui.available_width() - search_width)
                .return_key(Some(egui::KeyboardShortcut::new(
                    Modifiers::NONE,
                    Key::Enter,
                )))
                .cursor_at_end(true)
                .show(ui);

            if resp.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                tab.refresh(tab.path.clone());
            }

            TextEdit::singleline(&mut tab.search)
                .return_key(Some(egui::KeyboardShortcut::new(
                    Modifiers::NONE,
                    Key::Enter,
                )))
                .hint_text("search")
                .cursor_at_end(true)
                .desired_width(search_width - 50.)
                .show(ui);

            if !tab.search.is_empty() {
                if ui.button("X").clicked() {
                    tab.search = "".into();
                }
            }
        });

        if let Ok(entries) = &mut tab.entries {
            let mut new_path = None;

            let ctx = ui.ctx().clone();
            let mut builder = TableBuilder::new(ui)
                .column(Column::remainder())
                .column(Column::auto().at_least(160.))
                .column(Column::auto().at_least(60.))
                .sense(egui::Sense::click());

            let mut resort = false;

            let table = builder.header(20.0, |mut header| {
                resort = resort || show_sorting_header(
                    &mut header,
                    &mut tab.sorting,
                    "Name",
                    SortingColumn::Filename,
                );
                resort = resort || show_sorting_header(
                    &mut header,
                    &mut tab.sorting,
                    "Date modified",
                    SortingColumn::Date,
                );
                resort = resort || show_sorting_header(&mut header, &mut tab.sorting, "Size", SortingColumn::Size);
            });
            if resort {
                files::sort(entries, &tab.sorting);
            }

            table.body(|mut body| {
                if let Some((name, is_dir)) = &mut tab.state.add_entry {
                    let mut close = false;
                    body.row(18.0, |mut row| {
                        row.set_selected(true);
                        row.col(|ui| {
                            let resp = TextEdit::singleline(name)
                                .return_key(Some(egui::KeyboardShortcut::new(
                                    Modifiers::NONE,
                                    Key::Enter,
                                )))
                                .cursor_at_end(true)
                                .desired_width(ui.available_width())
                                .show(ui);

                            if resp.response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                close = true;
                            } else {
                                resp.response.request_focus();
                            }
                        });
                        row.col(|ui| {});
                        row.col(|ui| {});
                    });
                    if close {
                        let path = Path::new(&tab.path).join(name);
                        if *is_dir {
                            let _ = std::fs::create_dir_all(path);
                        } else {
                            let _ = std::fs::File::create(path);
                        }
                        tab.state.relead = true;
                        tab.state.add_entry = None;
                    }
                }

                for (i, entry) in entries.iter().enumerate() {
                    if let Some(rename) = &mut tab.state.renaming {
                        if rename.source_path == entry.path {
                            let mut close = false;
                            body.row(18.0, |mut row| {
                                row.set_selected(true);
                                row.col(|ui| {
                                    let resp = TextEdit::singleline(&mut rename.new_name)
                                        .return_key(Some(egui::KeyboardShortcut::new(
                                            Modifiers::NONE,
                                            Key::Enter,
                                        )))
                                        .cursor_at_end(true)
                                        .desired_width(ui.available_width())
                                        .show(ui);

                                    if resp.response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        close = true;
                                    } else {
                                        resp.response.request_focus();
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(entry.modified.format("%d/%m/%Y %H:%M").to_string());
                                });
                                row.col(|ui| {
                                    if entry.file_type.is_file() {
                                        ui.label(bytes_to_human_readable(entry.len));
                                    }
                                });
                            });
                            if close {
                                if rename.new_name != entry.file_name {
                                    let source = Path::new(&rename.source_path);
                                    let target = Path::new(&tab.info.as_ref().unwrap().path)
                                        .join(&rename.new_name);
                                    if rename.duplicate {
                                        if entry.file_type.is_file() {
                                            let _ = std::fs::copy(&source, &target);
                                        } else if entry.file_type.is_dir() {
                                            let _ = files::copy_dir(&source, &target);
                                        }
                                    } else {
                                        let _ = std::fs::rename(&source, &target);
                                    }
                                }
                                tab.state.relead = true;
                                tab.state.renaming = None;
                            }
                            continue;
                        }
                    }

                    if tab.search.is_empty()
                        || entry
                            .file_name
                            .to_lowercase()
                            .contains(&tab.search.to_lowercase())
                    {
                        body.row(18.0, |mut row| {
                            row.set_selected(tab.selected_entries.contains(&i));
                            row.col(|ui| {
                                let mut text: egui::RichText = entry.file_name.to_string().into();
                                if entry.file_type.is_dir() {
                                    text = text.strong();
                                }
                                Label::new(text).selectable(false).ui(ui);
                            });
                            row.col(|ui| {
                                ui.label(entry.modified.format("%d/%m/%Y %H:%M").to_string());
                            });
                            row.col(|ui| {
                                if entry.file_type.is_file() {
                                    ui.label(bytes_to_human_readable(entry.len));
                                }
                            });

                            let resp = row.response();
                            if resp.double_clicked() && entry.file_type.is_dir() {
                                new_path = Some(entry.path.clone());
                            }

                            let command = ctx.input(|i| i.modifiers.command);
                            let shift = ctx.input(|i| i.modifiers.shift);
                            if resp.clicked() {
                                if shift {
                                    if let Some(first) = tab.last_clicked_entry {
                                        if first >= i {
                                            for x in i..first {
                                                tab.selected_entries.insert(x);
                                            }
                                        } else {
                                            for x in first + 1..=i {
                                                tab.selected_entries.insert(x);
                                            }
                                        }
                                    }
                                } else if command {
                                    if tab.selected_entries.contains(&i) {
                                        tab.selected_entries.remove(&i);
                                    } else {
                                        tab.selected_entries.insert(i);
                                    }
                                } else {
                                    if tab.selected_entries.contains(&i) {
                                        tab.selected_entries.clear();
                                    } else {
                                        tab.selected_entries.clear();
                                        tab.selected_entries.insert(i);
                                    }
                                }
                                tab.last_clicked_entry = Some(i);
                            }

                            let is_main = tab.selected_entries.is_empty();
                            let action_entries: Vec<_> = if is_main {
                                vec![tab.info.as_ref().unwrap()]
                            } else {
                                entries
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, x)| tab.selected_entries.contains(i))
                                    .map(|(i, x)| x)
                                    .collect()
                            };
                            resp.context_menu(|ui| {
                                for action in self.actions.iter() {
                                    if (action.can_execute)(&action_entries, is_main) {
                                        if ui.button((action.name)(&action_entries)).clicked() {
                                            for entry in action_entries.iter() {
                                                (action.execute)(&entry, &mut tab.state);
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });
                            if resp.contains_pointer()
                                && ctx.input(|i| i.pointer.primary_pressed())
                                && !entries.is_empty()
                            {
                                self.drag_paths = Some((
                                    tab.path.to_string(),
                                    entries
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, x)| tab.selected_entries.contains(i))
                                        .map(|(i, x)| (x.path.to_string(), x.file_name.to_string()))
                                        .collect(),
                                ));
                                //println!("drag");
                            }
                            // if resp.contains_pointer()
                            //     && self.drag_paths.is_some()
                            //     && ctx.input(|i| i.pointer.primary_released())
                            // {
                            //     self.drop_path = Some(tab.path.to_string());
                            //     tab.state.relead = true;
                            //     println!("drop");
                            // }
                        });
                    }
                }
            });

            if let Some(new_path) = new_path {
                tab.refresh(new_path);
            }
        }

        if let Ok(entries) = &tab.entries {
            let resp = ui.interact(
                Rect::from_points(&[
                    ui.next_widget_position(),
                    ui.next_widget_position() + ui.available_size(),
                ]),
                Id::new(format!("{:? }post table", tab.id)),
                Sense::click_and_drag(),
            );
            let is_main = tab.selected_entries.is_empty();
            let action_entries: Vec<_> = if is_main {
                vec![tab.info.as_ref().unwrap()]
            } else {
                entries
                    .iter()
                    .enumerate()
                    .filter(|(i, x)| tab.selected_entries.contains(i))
                    .map(|(i, x)| x)
                    .collect()
            };
            resp.context_menu(|ui| {
                for action in self.actions.iter() {
                    if (action.can_execute)(&action_entries, is_main) {
                        if ui.button((action.name)(&action_entries)).clicked() {
                            for entry in action_entries.iter() {
                                (action.execute)(&entry, &mut tab.state);
                            }
                            ui.close_menu();
                        }
                    }
                }
            });

            if resp.contains_pointer()
                && self.drag_paths.is_some()
                && ui.input(|i| i.pointer.primary_released())
            {
                self.drop_path = Some(tab.path.to_string());
                tab.state.relead = true;
                //println!("drop");
            }
        }

        if let Some(zip) = &mut tab.state.extract_zip_archive {
            egui::Window::new("extract zip archive")
                .default_width(ui.available_width())
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.label(format!("source: {}", zip.source));
                        ui.horizontal(|ui| {
                            ui.label("target:");
                            let resp = TextEdit::singleline(&mut zip.target)
                                .return_key(Some(egui::KeyboardShortcut::new(
                                    Modifiers::NONE,
                                    Key::Enter,
                                )))
                                .cursor_at_end(true)
                                .desired_width(ui.available_width())
                                .show(ui);
                            if resp.response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let contents = std::fs::read(zip.source.clone()).unwrap();
                                let _ = zip_extract::extract(
                                    std::io::Cursor::new(contents),
                                    Path::new(&zip.target),
                                    zip.strip_toplevel,
                                );
                                tab.state.relead = true;
                            }
                        });
                        ui.checkbox(&mut zip.strip_toplevel, "strip toplevel");
                    });
                });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                tab.state.extract_zip_archive = None;
            }
        }
        if let Some(zip) = &mut tab.state.zip_dir {
            egui::Window::new("create zip archive")
                .default_width(ui.available_width())
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.label(format!("source: {}", zip.source));
                        ui.horizontal(|ui| {
                            ui.label("target:");
                            let resp = TextEdit::singleline(&mut zip.target)
                                .return_key(Some(egui::KeyboardShortcut::new(
                                    Modifiers::NONE,
                                    Key::Enter,
                                )))
                                .cursor_at_end(true)
                                .desired_width(ui.available_width())
                                .show(ui);
                            if resp.response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let _ = crate::zip::zip_dir(
                                    Path::new(&zip.source),
                                    Path::new(&zip.target),
                                    zip.method,
                                );
                                tab.state.relead = true;
                            }
                        });
                        egui::ComboBox::from_label("compression method")
                            .selected_text(&zip.method.to_string())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut zip.method,
                                    zip::CompressionMethod::Stored,
                                    zip::CompressionMethod::Stored.to_string(),
                                );
                                ui.selectable_value(
                                    &mut zip.method,
                                    zip::CompressionMethod::Deflated,
                                    zip::CompressionMethod::Deflated.to_string(),
                                );
                                // ui.selectable_value(&mut zip.method, zip::CompressionMethod::Bzip2, zip::CompressionMethod::Bzip2.to_string());
                                // ui.selectable_value(&mut zip.method, zip::CompressionMethod::Zstd, zip::CompressionMethod::Zstd.to_string());
                                // ui.selectable_value(&mut zip.method, zip::CompressionMethod::Lzma, zip::CompressionMethod::Lzma.to_string());
                                // ui.selectable_value(&mut zip.method, zip::CompressionMethod::Xz, zip::CompressionMethod::Xz.to_string());
                            });
                    });
                });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                tab.state.zip_dir = None;
            }
        }

        if tab.state.relead {
            tab.refresh_hard(tab.path.clone());
        }
    }
}
