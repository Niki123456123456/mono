use std::collections::{HashMap, HashSet};

use common::query::{Operator, Query, Value};
use egui::{Ui, Widget};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Default)]
struct Storage {
    api_key: String,
    query: String,
}

#[derive(Debug)]
pub struct TabSorting {
    pub reverse: bool,
    pub column: String,
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

        let mut sorting = TabSorting {
            reverse: false,
            column: "title".to_string(),
        };

        let mut q = common::query::try_parse_query(&s.query);

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
                            ui.horizontal(|ui| {
                                // return;
                                let mut query = s.query.clone();
                                let resp = egui::TextEdit::singleline(&mut query)
                                    .return_key(Some(egui::KeyboardShortcut::new(
                                        egui::Modifiers::NONE,
                                        egui::Key::Enter,
                                    )))
                                    .cursor_at_end(true)
                                    .hint_text("search query")
                                    .show(ui);
                                if query != s.query {
                                    s.query = query;
                                    q = common::query::try_parse_query(&s.query);
                                    q = Some(Query::Comparison {
                                        left: "source".into(),
                                        op: Operator::Eq,
                                        right: Value::String("dv_docs".into()),
                                    });
                                    q = Some(Query::Comparison {
                                        left: "source".into(),
                                        op: Operator::Eq,
                                        right: Value::String("SalesforceKnowledgeBase".into()),
                                    });
                                    println!("query: {:?}", q);
                                    filter_articles(
                                        &articles.data,
                                        &articles.columns,
                                        &q,
                                        &mut articles.filtered_data,
                                        &mut articles.filtered_character_count,
                                    );
                                    sort_articles(
                                        &articles.data,
                                        &articles.columns,
                                        &sorting,
                                        &mut articles.filtered_data,
                                    );
                                }
                            });

                            ui.label(format!(
                                "{} / {} articles, {} / {} characters",
                                common::thousands_sep(articles.filtered_data.len()),
                                common::thousands_sep(articles.data.len()),
                                common::thousands_sep(articles.filtered_character_count),
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
                                        let after = if sorting.column == column.name {
                                            if sorting.reverse { " ⬇" } else { " ⬆" }
                                        } else {
                                            ""
                                        };
                                        if egui::Label::new(
                                            egui::RichText::from(format!(
                                                "{}{}",
                                                column.name, after
                                            ))
                                            .strong(),
                                        )
                                        .selectable(false)
                                        .sense(egui::Sense::click())
                                        .ui(ui)
                                        .clicked()
                                        {
                                            if sorting.column == column.name {
                                                sorting.reverse = !sorting.reverse;
                                            } else {
                                                sorting.column = column.name.clone();
                                                sorting.reverse = false;
                                            }
                                            sort_articles(
                                                &articles.data,
                                                &articles.columns,
                                                &sorting,
                                                &mut articles.filtered_data,
                                            );
                                        }
                                    });
                                }
                            });

                            table.body(|mut body| {
                                body.rows(20., articles.filtered_data.len(), |mut row| {
                                    let article =
                                        &articles.data[articles.filtered_data[row.index()]];

                                    for column in &articles.columns {
                                        if !column.enabled {
                                            continue;
                                        }
                                        row.col(|ui| {
                                            if ui
                                                .label((column.value)(article).to_string())
                                                .double_clicked()
                                            {
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
                            ui.label(format!("id: {}", article.id));
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

                        ui.heading("Content:");
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
    value: Box<dyn Fn(&Article) -> Value + Send>,
    enabled: bool,
}

impl Column {
    fn new(
        width: Option<f32>,
        name: impl Into<String>,
        enabled: bool,
        value: impl Fn(&Article) -> Value + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            value: Box::new(value),
            enabled,
        }
    }
    fn new2(
        width: Option<f32>,
        name: impl Into<String>,
        value: Box<dyn Fn(&Article) -> Value + Send>,
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
        columns.push(Column::new(None, "title", true, |article| {
            Value::String(article.title.to_string())
        }));
        columns.push(Column::new(Some(200.), "channel", false, |article| {
            Value::Array(
                article
                    .activeOn
                    .iter()
                    .map(|x| Value::String(x.channel.clone()))
                    .collect(),
            )
        }));
        columns.push(Column::new(Some(400.), "agent", false, |article| {
            Value::Array(
                article
                    .activeOn
                    .iter()
                    .map(|x| Value::String(x.agent.clone()))
                    .collect(),
            )
        }));
        columns.push(Column::new(Some(200.), "characterCount", true, |article| {
            Value::Number(article.characterCount as i64)
        }));
        columns.push(Column::new(Some(200.), "createdAt", true, |article| {
            Value::String(article.createdAt.to_string())
        }));
        columns.push(Column::new(Some(200.), "updatedAt", false, |article| {
            Value::String(article.updatedAt.to_string())
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
        let mut articles = get_articles(&api_key).await;
        sender.send(articles);
        ctx.request_repaint();
    });
    return p;
}

async fn get_articles(api_key: &str) -> Result<Articles, String> {
    let mut request = ehttp::Request::get("https://api.moin.ai/api/v1/knowledge");
    request.headers.insert("x-api-key", api_key);

    let response = common::http::fetch(&request).await?;
    let mut articles =
        serde_json::from_slice::<Articles>(&response.bytes).map_err(|e| e.to_string())?;

    let meta_keys: HashSet<String> = articles
        .data
        .iter()
        .flat_map(|article| article.metadata.keys().cloned())
        .collect();

    
    let mut meta_keys =meta_keys.into_iter().collect::<Vec<_>>();
    meta_keys.sort();

    let mut all_columns = Column::default();
    all_columns.extend(meta_keys.into_iter().map(|key| {
        let key_cloned = key.clone();
        Column::new2(
            Some(300.),
            key_cloned.clone(),
            Box::new(move |article| {
                article
                    .metadata
                    .get(&key_cloned)
                    .map(|x| x.to_column())
                    .unwrap_or(Value::None)
            }),
        )
    }));
    articles.columns = all_columns;
    articles.total_character_count = articles.data.iter().map(|x| x.characterCount).sum();
    articles.filtered_character_count = articles.total_character_count;
    articles.filtered_data = (0..articles.data.len()).collect();
    let c = articles.columns.iter().find(|c| c.name == "title").unwrap();
    articles.data.sort_by(|a, b| {
        let a_value = (c.value)(a);
        let b_value = (c.value)(b);
        // if sorting.reverse {
        //     b_value.cmp(&a_value)
        // } else {
        //     a_value.cmp(&b_value)
        // }
        a_value.cmp(&b_value)
    });

    Ok(articles)
}

fn sort_articles(
    data: &Vec<Article>,
    columns: &Vec<Column>,
    sorting: &TabSorting,
    filtered_data: &mut Vec<usize>,
) {
    let c = columns.iter().find(|c| c.name == sorting.column).unwrap();
    filtered_data.sort_by(|&a, &b| {
        let a_value = (c.value)(&data[a]);
        let b_value = (c.value)(&data[b]);
        if sorting.reverse {
            b_value.cmp(&a_value)
        } else {
            a_value.cmp(&b_value)
        }
    });
}

fn filter_articles(
    data: &Vec<Article>,
    columns: &Vec<Column>,
    q: &Option<Query>,
    filtered_data: &mut Vec<usize>,
    filtered_character_count: &mut usize,
) {
    if let Some(query) = q {
        filtered_data.clear();
        filtered_data.extend(0..data.len());
        filtered_data.retain(|&index| {
            let article = &data[index];
            article_matches(article, query, columns)
        });
    } else {
        filtered_data.clear();
        filtered_data.extend(0..data.len());
    }
     *filtered_character_count = filtered_data.iter().map(|x| data[*x].characterCount).sum();
}

fn article_matches(arc: &Article, q: &Query, columns: &Vec<Column>) -> bool {
    match q {
        Query::Logical { left, op, right } => {
            let left_matches = article_matches(arc, left, columns);
            let right_matches = article_matches(arc, right, columns);
            match op {
                common::query::LogicalOperator::And => left_matches && right_matches,
                common::query::LogicalOperator::Or => left_matches || right_matches,
            }
        }
        Query::Comparison { left, op, right } => {
            let field_value = &columns
                .iter()
                .find(|c| &c.name == left)
                .and_then(|c| Some((c.value)(arc)))
                .unwrap_or(Value::None);
            match op {
                common::query::Operator::Eq => field_value == right,
                common::query::Operator::Ne => field_value != right,
                common::query::Operator::Gt => field_value > right,
                common::query::Operator::Lt => field_value < right,
                common::query::Operator::Ge => field_value >= right,
                common::query::Operator::Le => field_value <= right,
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Articles {
    data: Vec<Article>,
    #[serde(skip)]
    columns: Vec<Column>,
    #[serde(skip)]
    total_character_count: usize,
    #[serde(skip)]
    filtered_character_count: usize,
    #[serde(skip)]
    filtered_data: Vec<usize>,
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

impl NumberOrString {
    fn to_column(&self) -> Value {
        match self {
            NumberOrString::Number(n) => Value::Number(*n),
            NumberOrString::String(s) => Value::String(s.clone()),
        }
    }
}
