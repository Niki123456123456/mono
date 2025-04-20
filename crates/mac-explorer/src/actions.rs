use std::path::Path;
use crate::files::{FileEntry, Restriction};

pub trait GetName: Fn(&Vec<&FileEntry>) -> String {}
impl<F> GetName for F where F: Fn(&Vec<&FileEntry>) -> String {}

pub trait CanExecute: Fn(&Vec<&FileEntry>, bool) -> bool {}
impl<F> CanExecute for F where F: Fn(&Vec<&FileEntry>, bool) -> bool {}

pub trait Execute: Fn(&FileEntry, &mut ActionState) {}
impl<F> Execute for F where F: Fn(&FileEntry, &mut ActionState) {}

#[derive(Default, Debug)]
pub struct ActionState {
    pub relead: bool,
    pub add_entry: Option<(String, bool)>,
    pub extract_zip_archive: Option<ExtractZipArchive>,
    pub zip_dir: Option<ZipDir>,
    pub renaming: Option<Renaming>,
}
#[derive(Debug)]
pub struct ExtractZipArchive {
    pub source: String,
    pub target: String,
    pub strip_toplevel: bool,
}

#[derive(Debug)]
pub struct ZipDir {
    pub source: String,
    pub target: String,
    pub method: zip::CompressionMethod,
}

#[derive(Debug)]
pub struct Renaming {
    pub source_path: String,
    pub new_name: String,
    pub duplicate: bool,
}

pub struct Action {
    pub name: Box<dyn GetName>,
    pub can_execute: Box<dyn CanExecute>,
    pub execute: Box<dyn Execute>,
}

impl Action {
    pub fn new(
        name: impl GetName + 'static,
        can_execute: impl CanExecute + 'static,
        execute: impl Execute + 'static,
    ) -> Self {
        Self {
            name: Box::new(name),
            can_execute: Box::new(can_execute),
            execute: Box::new(execute),
        }
    }

    pub fn constant(
        display_name: &'static str,
        restriction: Restriction,
        execute: impl Execute + 'static,
    ) -> Self {
        return Self::new(
            |e| display_name.to_string(),
            move |e, b| e.iter().all(|e| e.fullfills(&restriction, b)),
            execute,
        );
    }

    pub fn open_with(
        app_name: &'static str,
        display_name: &'static str,
        restriction: Restriction,
    ) -> Self {
        return Self::new(
            |e| display_name.to_string(),
            move |e, b| e.iter().all(|e| e.fullfills(&restriction, b)),
            move |e, s| {
                let _ = std::process::Command::new("open")
                    .arg("-a")
                    .arg(app_name)
                    .arg(&e.path)
                    .status();
            },
        );
    }
}

pub fn actions() -> Vec<Action> {
    let mut actions = vec![];

    /*
    if ui.button("make executable").clicked() {
                                       std::process::Command::new("chmod")
                                           .arg("755")
                                           .arg(entry.path.clone())
                                           .status();
                                       ui.close_menu();
                                   }
    */
    actions.push(Action::constant("add file", Restriction::Main, |e, s| {
        s.add_entry = Some(("".into(), false));
    }));
    actions.push(Action::constant("add dir", Restriction::Main, |e, s| {
        s.add_entry = Some(("".into(), true));
    }));
    actions.push(Action::constant(
        "rename",
        Restriction::Not(Box::new(Restriction::Main)),
        |e, s| {
            s.renaming = Some(Renaming {
                source_path: e.path.to_string(),
                new_name: e.file_name.to_string(),
                duplicate: false,
            });
        },
    ));
    actions.push(Action::constant(
        "duplicate",
        Restriction::Not(Box::new(Restriction::Main)),
        |e, s| {
            let stem = Path::new(&e.file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let extension = Path::new(&e.file_name).extension().and_then(|s| s.to_str()).unwrap_or("");

            let new_name = format!("{} (1){}", stem, extension);

            s.renaming = Some(Renaming {
                source_path: e.path.to_string(),
                new_name: new_name,
                duplicate: true,
            });
        },
    ));
    actions.push(Action::new(
        |e| format!("extract zip archive"),
        |e, m| !m && e.len() == 1 && e[0].file_type.is_file() && e[0].file_name.ends_with(".zip"),
        |e, s| {
            s.extract_zip_archive = Some(ExtractZipArchive {
                source: e.path.to_string(),
                target: e.path[..e.path.len() - 4].to_string(),
                strip_toplevel: true,
            })
        },
    ));
    actions.push(Action::new(
        |e| format!("create zip archive"),
        |e, m| !m && e.len() == 1 && e[0].file_type.is_dir(),
        |e, s| {
            s.zip_dir = Some(ZipDir {
                source: e.path.to_string(),
                target: format!("{}.zip", e.path),
                method: zip::CompressionMethod::Deflated,
            })
        },
    ));
    actions.push(Action::new(
        |e| format!("copy {}", e[0].file_name),
        |e, m| !m && e.len() == 1,
        |e, s| {
            let mut ctx: clipboard::ClipboardContext = clipboard::ClipboardProvider::new().unwrap();
            clipboard::ClipboardProvider::set_contents(&mut ctx, e.file_name.to_string()).unwrap();
        },
    ));
    actions.push(Action::new(
        |e| format!("copy path"),
        |e, m| !m && e.len() == 1,
        |e, s| {
            let mut ctx: clipboard::ClipboardContext = clipboard::ClipboardProvider::new().unwrap();
            clipboard::ClipboardProvider::set_contents(&mut ctx, e.path.to_string()).unwrap();
        },
    ));
    actions.push(Action::open_with(
        "Visual Studio Code",
        "vscode",
        Restriction::None,
    ));
    actions.push(Action::open_with(
        "Google Chrome",
        "chrome",
        Restriction::File,
    ));
    actions.push(Action::open_with(
        "Terminal",
        "terminal",
        Restriction::Folder,
    ));
    actions.push(Action::open_with("Finder", "finder", Restriction::Folder));
    actions.push(Action::open_with("Zed", "zed", Restriction::None));
    actions.push(Action::constant(
        "delete",
        Restriction::Not(Box::new(Restriction::Main)),
        |e, s| {
            if e.file_type.is_file() {
                let _ = std::fs::remove_file(e.path.clone());
            }
            if e.file_type.is_dir() {
                let _ = std::fs::remove_dir_all(e.path.clone());
            }
            s.relead = true;
        },
    ));
    return actions;
}
