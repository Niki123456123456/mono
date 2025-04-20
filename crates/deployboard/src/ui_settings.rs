pub fn show_settings(app: &mut crate::App, ctx: &mut common::app::Context<'_>) {
    if app.show_settings {
        let mut save = false;
        let modal = egui::Modal::new(egui::Id::new("settings")).show(ctx.ui.ctx(), |ui| {
           
            ui.set_width(ctx.ui.available_width() * 0.9);
            ui.set_height(ctx.ui.ctx().available_rect().height() * 0.9);
            crate::ui::show_text(
                ui, "settings_text",
                |ui| {
                    ui.label("settings");
                },
                &mut app.config,
                |_, _| {
                    save = true;
                },
            );
        });

        if save {
            app.config.orignal_text = app.config.text.clone();
            app.config.orignal_data = serde_json::from_str(&app.config.orignal_text).unwrap();
            app.show_settings = false;
            ctx.save(&app.config.orignal_data);
            app.reload(true);
        }

        if modal.should_close() {
            app.show_settings = false;
        }
    }
}
