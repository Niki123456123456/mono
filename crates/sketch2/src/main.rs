use egui_map_view::{Map, config::OpenStreetMapConfig};

fn main() {
    common::app::run("sketch2", |cc| {
        let mut map = Map::new(OpenStreetMapConfig::default());
        //let mut map = Map::new(TestMapConfig{});
        return Box::new(move |mut ctx| {
            let ui = ctx.get_ui();
            ui.add_sized(ui.available_size_before_wrap(), &mut map);

            egui::Window::new("Map Info").show(ui.ctx(), |ui| {
                ui.label(format!("Zoom: {}", map.zoom));
                ui.label(format!(
                    "Center: {:.4}, {:.4}",
                    map.center.lat, map.center.lon
                ));
                ui.separator();

                if let Some(pos) = map.mouse_pos {
                    ui.label(format!("Mouse: {:.4}, {:.4}", pos.lat, pos.lon));
                } else {
                    ui.label("Mouse: N/A");
                }
            });
        });
    });
}

struct TestMapConfig{

}

impl egui_map_view::config::MapConfig for TestMapConfig {
    fn tile_url(&self, tile: &egui_map_view::TileId) -> String {
        format!("https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile//{}/{}/{}", tile.z, tile.y, tile.x)
    }

    fn attribution(&self) -> Option<&String> {
        None
    }

    fn attribution_url(&self) -> Option<&String> {
        None
    }

    fn default_center(&self) -> (f64, f64) {
         (24.93545, 60.16952)
    }

    fn default_zoom(&self) -> u8 {
        2
    }
}
