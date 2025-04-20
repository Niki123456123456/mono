pub struct Context<'a> {
    pub ui: &'a mut egui::Ui,
    pub needs_save: &'a mut Option<String>,
}

impl<'a> Context<'a> {
    pub fn save<T: serde::Serialize>(&mut self,value: &T ){
        if let Ok(string) = ron::ser::to_string(value) {
            *self.needs_save = Some(string);
        }
    }
}
struct App {
    update: Box<dyn FnMut(Context)>,
    needs_save: Option<String>,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        f: impl Fn(&eframe::CreationContext<'_>) -> Box<dyn FnMut(Context)>,
    ) -> Self {
        let update = (f)(cc);
        Self {
            update,
            needs_save: None,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(string) = &self.needs_save {
            storage.set_string(eframe::APP_KEY, string.clone());
            self.needs_save = None;
        }
       
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        if self.needs_save.is_some() {
            std::time::Duration::from_secs(0)
        } else {
            std::time::Duration::from_secs(30)
        }
    }
    
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let context = Context {
                ui,
                needs_save: &mut self.needs_save,
            };

            (self.update)(context);
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run(app_name: &str, f: impl Fn(&eframe::CreationContext<'_>) -> Box<dyn FnMut(Context)>) {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport =
        egui::ViewportBuilder::default().with_inner_size(egui::vec2(1200.0, 700.0));
    eframe::run_native(
        app_name,
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, f)))),
    )
    .unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn run(
    app_name: &str,
    f: impl Fn(&eframe::CreationContext<'_>) -> Box<dyn FnMut(Context)> + 'static,
) {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc, f)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
