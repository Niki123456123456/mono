#[derive(Clone)]
pub struct View {
    pub textures: Option<TexturesContainer>,
    pub size: egui::Vec2,
}

#[derive(Clone)]
pub struct TexturesContainer {
    pub texture: three_d::Texture2D,
    pub depth_texture: three_d::DepthTexture2D,
    pub texture_id: egui::TextureId,
}

fn create_textures(
    context: &three_d::Context,
    size: egui::Vec2,
    frame: &mut eframe::Frame,
) -> TexturesContainer {
    let texture: three_d::Texture2D = three_d::Texture2D::new_empty::<[u8; 4]>(
        context,
        size.x as u32,
        size.y as u32,
        three_d::Interpolation::Linear,
        three_d::Interpolation::Linear,
        None,
        three_d::Wrapping::ClampToEdge,
        three_d::Wrapping::ClampToEdge,
    );
    let depth_texture: three_d::DepthTexture2D = three_d::DepthTexture2D::new::<f32>(
        context,
        size.x as u32,
        size.y as u32,
        three_d::Wrapping::ClampToEdge,
        three_d::Wrapping::ClampToEdge,
    );
    let texture_id = frame.register_native_glow_texture(texture.id());
    return TexturesContainer {
        texture,
        depth_texture,
        texture_id: texture_id,
    };
}

impl Default for View {
    fn default() -> Self {
        Self {
            textures: None,
            size: egui::Vec2::ZERO,
        }
    }
}
impl View {
    pub fn render(
        &mut self,
        frame: &mut eframe::Frame,
        context: &three_d::Context,
        size: egui::Vec2,
        clear_color: egui::Color32,
        clear_depth: f32,
        render: impl FnOnce(&three_d::Context, three_d::Viewport),
    ) {
        if size != self.size || self.textures.is_none() {
            self.size = size;
            self.textures = Some(create_textures(context, size, frame));
        }

        if let Some(tex) = &mut self.textures {
            let render_target = three_d::RenderTarget::new(
                tex.texture.as_color_target(None),
                tex.depth_texture.as_depth_target(),
            );

            render_target.clear(three_d::ClearState::color_and_depth(
                clear_color.r() as f32 / 255.0,
                clear_color.g() as f32 / 255.0,
                clear_color.b() as f32 / 255.0,
                clear_color.a() as f32 / 255.0,
                clear_depth,
            ));

            let _ = render_target.write(|| {
                let result: Result<(), three_d::CoreError> = Ok(());
                let viewport =
                    three_d::Viewport::new_at_origo(self.size.x as u32, self.size.y as u32);
                context.set_viewport(viewport);
                (render)(context, viewport);
                return result;
            });
        }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if let Some(tex) = &self.textures {
            ui.image(egui::load::SizedTexture::new(tex.texture_id, self.size));
        }
    }
}

pub fn show(
    id_source: impl std::hash::Hash,
    ui: &mut egui::Ui,
    frame: &mut eframe::Frame,
    size: egui::Vec2,
    render: impl FnOnce(&three_d::Context, three_d::Viewport),
) {
    show_advanced(
        id_source,
        ui,
        frame,
        size,
        egui::Color32::TRANSPARENT,
        1.,
        render,
    );
}

pub fn show_advanced(
    id_source: impl std::hash::Hash,
    ui: &mut egui::Ui,
    frame: &mut eframe::Frame,
    size: egui::Vec2,
    clear_color: egui::Color32,
    clear_depth: f32,
    render: impl FnOnce(&three_d::Context, three_d::Viewport),
) {
    let id = egui::Id::new(id_source);
    let ctx = ui.ctx().clone();

    let context: three_d::Context = get_or_insert_context(ui.ctx(), frame.gl().unwrap());

    let mut view: View = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    view.render(frame, &context, size, clear_color, clear_depth, render);
    view.show(ui);
    ctx.data_mut(|d| d.insert_temp(id, view));
}

pub fn get_or_insert_context(
    ctx: &egui::Context,
    gl: &std::sync::Arc<eframe::glow::Context>,
) -> three_d::Context {
    let ctx = ctx.clone();
    let mut context: Option<three_d::Context> =
        ctx.data(|d| d.get_temp(egui::Id::new("three_d::Context")));
    if context.is_none() {
        context = Some(three_d::Context::from_gl_context(gl.clone()).unwrap());
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("three_d::Context"), context.clone().unwrap())
        });
    }
    return context.unwrap();
}
