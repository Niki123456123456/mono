use sha2::Digest;

fn main() {
    common::app::run("tool", |cc| {
        let uuid = uuid::Uuid::new_v4();

        let mut urlencoding = Encoding::new(
            "URL encoding",
            &|data| urlencoding::encode(data).to_string(),
            Some(&|data| urlencoding::decode(data).unwrap_or_default().to_string()),
        );

        let mut base64ncoding = Encoding::new(
            "Base64 encoding",
            &|data| base64::encode(data),
            Some(&|data| {
                String::from_utf8(base64::decode(data).unwrap_or_default()).unwrap_or_default()
            }),
        );

        let mut sha256ncoding = Encoding::new(
            "Sha256 encoding",
            &|data| {
                let mut hasher = sha2::Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                format!("{:x}", result)
            },
            None,
        );

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            ui.horizontal(|ui| {
                ui.label("GUID: ");
                ui.label(uuid.to_string());
            });

            urlencoding.show(ui);
            base64ncoding.show(ui);
            sha256ncoding.show(ui);
        });
    });
}

pub struct Encoding {
    pub original: String,
    pub encoded: String,
    pub title: &'static str,
    pub encode: &'static dyn Fn(&str) -> String,
    pub decode: Option<&'static dyn Fn(&str) -> String>,
}

impl Encoding {
    pub fn new(
        title: &'static str,
        encode: &'static dyn Fn(&str) -> String,
        decode: Option<&'static dyn Fn(&str) -> String>,
    ) -> Self {
        Self {
            original: Default::default(),
            encoded: Default::default(),
            encode,
            decode,
            title,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.title);
        ui.columns(2, |columns: &mut [egui::Ui]| {
            Encoding::show_single_mut(
                &mut self.original,
                &mut self.encoded,
                &mut columns[0],
                self.encode,
                "original",
            );
            if let Some(decode) = self.decode {
                Encoding::show_single_mut(
                    &mut self.encoded,
                    &mut self.original,
                    &mut columns[1],
                    decode,
                    "encoded",
                );
            } else{
                Encoding::show_single(& self.encoded,&mut columns[1], "encoded",);
            }
        });
    }

    pub fn show_single_mut(
        data: &mut String,
        other: &mut String,
        ui: &mut egui::Ui,
        code: &'static dyn Fn(&str) -> String,
        title: &str,
    ) {
        ui.label(title);

        let mut temp = data.clone();
        egui::TextEdit::multiline(&mut temp)
            .desired_width(ui.available_width())
            .show(ui);
        if &temp != data {
            *data = temp;
            *other = (code)(&data);
        }
    }

    pub fn show_single(data: &str, ui: &mut egui::Ui, title: &str) {
        ui.label(title);
        ui.label(data);
    }
}