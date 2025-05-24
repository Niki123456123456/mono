use std::collections::{HashMap, HashSet};

use egui::Ui;
// https://github.com/mnaufalhilmym/fexpr/tree/main

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Default)]
struct Storage {
    api_key: String,
}

fn main() {
    common::app::run("moin ai hub", |cc| {
        let mut s: Storage = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let mut promise = if s.api_key.is_empty() {
            None
        } else {
            Some(get_articles_p(&cc.egui_ctx, &s.api_key))
        };

        let mut article_details = HashMap::new();

        let mut cache = egui_commonmark::CommonMarkCache::default();

        return Box::new(move |mut ctx| {
            let mut ui = ctx.get_ui();

            let mut save = false;
            ui.horizontal(|ui| {
                let resp = egui::TextEdit::singleline(&mut s.api_key)
                    .return_key(Some(egui::KeyboardShortcut::new(
                        egui::Modifiers::NONE,
                        egui::Key::Enter,
                    )))
                    .cursor_at_end(true)
                    .hint_text("api key")
                    .show(ui);
                if ui.button("load").clicked() {
                    save = true;
                    promise = Some(get_articles_p(ui.ctx(), &s.api_key));
                }
            });

            if let Some(promise) = &mut promise {
                if let Some(articles) = promise.ready_mut() {
                    match articles {
                        Ok(articles) => {
                            ui.label(format!(
                                "{} articles, {} characters",
                                common::thousands_sep(articles.data.len()),
                                common::thousands_sep(articles.total_character_count)
                            ));
                            ui.horizontal(|ui| {
                                for column in &mut articles.columns {
                                    ui.checkbox(&mut column.enabled, &column.name);
                                }
                            });

                            let mut builder = egui_extras::TableBuilder::new(ui);

                            for column in &articles.columns {
                                if !column.enabled {
                                    continue;
                                }
                                if let Some(width) = column.width {
                                    builder =
                                        builder.column(egui_extras::Column::auto().at_least(width));
                                } else {
                                    builder = builder.column(egui_extras::Column::remainder());
                                }
                            }

                            let table = builder.header(20.0, |mut header| {
                                for column in &articles.columns {
                                    if !column.enabled {
                                        continue;
                                    }
                                    header.col(|ui| {
                                        ui.label(&column.name);
                                    });
                                }
                            });

                            table.body(|mut body| {
                                body.rows(20., articles.data.len(), |mut row| {
                                    let article = &articles.data[row.index()];

                                    for column in &articles.columns {
                                        if !column.enabled {
                                            continue;
                                        }
                                        row.col(|ui| {
                                            if ui.label((column.value)(article)).double_clicked() {
                                                article_details
                                                    .insert(article.id.clone(), article.clone());
                                            }
                                        });
                                    }
                                });
                            });
                        }
                        Err(err) => {
                            ui.label(format!("Error: {}", err));
                        }
                    }
                } else {
                    ui.label("loading articles...");
                }
            } else {
                ui.label("please enter your api key and press load");
            }

            let mut to_remove = vec![];
            for (id, article) in article_details.iter() {
                egui::Window::new(&article.title)
                    .id(egui::Id::new(id))
                    .show(ui.ctx(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "characterCount: {}",
                                common::thousands_sep(article.characterCount)
                            ));
                            ui.label(format!("createdAt: {}", article.createdAt));
                            ui.label(format!("updatedAt: {}", article.updatedAt));
                            if ui.button("close").clicked() {
                                to_remove.push(id.clone());
                            }
                        });

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui_commonmark::CommonMarkViewer::new().show(
                                ui,
                                &mut cache,
                                &article.body,
                            );
                        });
                    });
            }
            for id in to_remove {
                article_details.remove(&id);
            }

            if save {
                ctx.save(&s);
            }
        });
    });
}

struct Column {
    name: String,
    width: Option<f32>,
    value: Box<dyn Fn(&Article) -> String + Send>,
    enabled: bool,
}

impl Column {
    fn new(
        width: Option<f32>,
        name: impl Into<String>,
        value: impl Fn(&Article) -> String + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            value: Box::new(value),
            enabled: true,
        }
    }
    fn new2(
        width: Option<f32>,
        name: impl Into<String>,
        value: Box<dyn Fn(&Article) -> String + Send>,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            value,
            enabled: false,
        }
    }

    fn default() -> Vec<Self> {
        let mut columns = Vec::new();
        columns.push(Column::new(None, "Title", |article| {
            article.title.to_string()
        }));
        columns.push(Column::new(Some(200.), "Channels", |article| {
            article
                .activeOn
                .iter()
                .map(|x| x.channel.clone())
                .collect::<Vec<_>>()
                .join("\n")
        }));
        columns.push(Column::new(Some(400.), "Agents", |article| {
            article
                .activeOn
                .iter()
                .map(|x| x.agent.clone())
                .collect::<Vec<_>>()
                .join("\n")
        }));
        columns.push(Column::new(Some(200.), "characterCount", |article| {
            article.characterCount.to_string()
        }));
        columns.push(Column::new(Some(200.), "createdAt", |article| {
            article.createdAt.to_string()
        }));
        columns.push(Column::new(Some(200.), "updatedAt", |article| {
            article.updatedAt.to_string()
        }));
        return columns;
    }
}

fn get_articles_p(
    ctx: &egui::Context,
    api_key: &String,
) -> poll_promise::Promise<Result<Articles, std::string::String>> {
    let (sender, p) = poll_promise::Promise::new();
    let ctx = ctx.clone();
    let api_key = api_key.clone();
    common::execute(async move {
        let mut articles = get_articles(&api_key);
        sender.send(articles);
        ctx.request_repaint();
    });
    return p;
}

fn get_articles(api_key: &str) -> Result<Articles, String> {
    let mut request = ehttp::Request::get("https://api.moin.ai/api/v1/knowledge");
    request.headers.insert("x-api-key", api_key);

    let response = ehttp::fetch_blocking(&request)?;
    let mut articles =
        serde_json::from_slice::<Articles>(&response.bytes).map_err(|e| e.to_string())?;

    let meta_keys: HashSet<String> = articles
        .data
        .iter()
        .flat_map(|article| article.metadata.keys().cloned())
        .collect();

    let mut all_columns = Column::default();
    all_columns.extend(meta_keys.iter().map(|key| {
        let key_cloned = key.clone();
        Column::new2(
            Some(300.),
            key_cloned.clone(),
            Box::new(move |article| {
                article
                    .metadata
                    .get(&key_cloned)
                    .map(|x| x.to_string())
                    .unwrap_or_default()
            }),
        )
    }));
    articles.columns = all_columns;
    articles.total_character_count = articles.data.iter().map(|x| x.characterCount).sum();

    Ok(articles)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Articles {
    data: Vec<Article>,
    #[serde(skip)]
    columns: Vec<Column>,
    #[serde(skip)]
    total_character_count: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Article {
    id: String,
    title: String,
    body: String,
    activeOn: Vec<ActiveOn>,
    updatedAt: String,
    createdAt: String,
    characterCount: usize,
    metadata: std::collections::HashMap<String, NumberOrString>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct ActiveOn {
    agent: String,
    channel: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum NumberOrString {
    Number(i64),
    String(String),
}
impl ToString for NumberOrString {
    fn to_string(&self) -> String {
        match self {
            NumberOrString::Number(n) => n.to_string(),
            NumberOrString::String(s) => s.clone(),
        }
    }
}
// "id": "6830bbe0e111df4f457f3af8",
//             "status": "active",
//             "title": "Fehlermeldung: Could not open listener on port 3404",
//             "body": "\n\nExternal-Article-Source: https://kb.d-velop.de/s/article/ka0Sd000000INt8IAG\n\nKomponente: d.ecs rendition service\n\nVersion: 4.2.0 - 4.4.1\n\nUnterKategorie: KonvertierenRendering\n\nArtikelNr.: 000001341\n\n---\n\n\n\n## Fehlermeldung: Could not open listener on port 3404\n\n\n\n## Inhalt\n\nDie Anwendung d.ecs rendition service meldet in der d.3-Logdatei nach dem Startvorgang den folgenden Fehler:\n\n```\nCould not open listener on port 3404\n```\n\nDie Fehlermeldung kann folgende Ursachen haben:\n\n- Der Port 3404 auf dem d.ecs rendition service-Server wird bereits verwendet.\n- Der angegebene d.ecs rendition service-Benutzer besitzt keine lokalen Administratorberechtigungen.\n\n## Auflösung\n\nZum Beheben des Fehlers prüfen Sie bitte Folgendes:\n\n- Prüfen Sie den verwendeten Benutzer unter **d.ecs rendition service service configuration**. Dieser Benutzer muss Mitglied der lokalen Administratorengruppe sein.\n- Starten Sie die Eingabeaufforderung (Kommandozeile) als Administrator und geben Sie folgenden Befehl ein: \n  \n  ```\n  netstat -ab > c:\\temp\\ports.txt\n  ```\n  \n  Prüfen Sie in der **ports.txt**-Datei, ob der Port 3404 bereits verwendet wird.\n\nSobald der Port wieder frei ist und der Benutzer entsprechende Berechtigungen hat, sollte d.ecs rendition service erfolgreich starten.\n\n## Voraussetzungen\n\nSie müssen Administratorrechte auf dem Server mit d.ecs rendition service haben.",
//             "metadata": {
//                 "source": "SalesforceKnowledgeBase",
//                 "sourceId": "SalesforceKnowledgeBase_000001341",
//                 "url": "https://kb.d-velop.de/s/article/ka0Sd000000INt8IAG"
//             },
//             "activeOn": [
//                 {
//                     "agent": "faq_d_velop_documents_produktanfragen",
//                     "channel": "null"
//                 }
//             ],
//             "createdAt": "2025-05-23T18:18:08.149Z",
//             "updatedAt": "2025-05-23T18:18:08.185Z",
//             "characterCount": 1376
