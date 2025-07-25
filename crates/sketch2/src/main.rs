use egui::{Pos2, Sense, Vec2};

fn main() {
    common::app::run("sketch2", |cc| {
        let mut presentation = Presentation::default();
        let mut current_builder: Option<Box<BuilderFn>> = None;
        return Box::new(move |ctx| {
            let ui = ctx.ui;

            let builders = builders();

            
            if let Some(slide) = presentation.slides.first_mut() {
                ui.horizontal(|ui| {
                    for builder in &builders {
                        if ui.button(builder.name).clicked() {
                            current_builder = Some((builder.create)());
                        }
                    }
                });

                let av_size = ui.available_size();
                let pos = ui.next_widget_position();
                slide.render(av_size, pos, ui, &mut current_builder);
            }
        });
    });
}

pub struct Presentation {
    pub slides: Vec<Slide>,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            slides: vec![Slide::default()],
        }
    }
}

pub struct Slide {
    pub size: Vec2, // in inches
    pub current_zoom: f32,
    pub current_delta_screen: Vec2,
    pub hover_position_screen: Pos2,
    pub elements: Vec<Box<dyn Element>>,
}

impl Default for Slide {
    fn default() -> Self {
        Self {
            size: Vec2::new(13.33, 7.5),
            current_zoom: 1.0,
            current_delta_screen: Vec2::ZERO,
            hover_position_screen: Pos2::ZERO,
            elements: vec![],
        }
    }
}

impl Slide {
    pub fn render(
        &mut self,
        av_size: Vec2,
        pos: Pos2,
        ui: &mut egui::Ui,
        current_builder: &mut Option<Box<BuilderFn>>,
    ) {
        let old_zoom = self.current_zoom;
        ui.ctx().input(|i| {
            self.current_zoom += self.current_zoom * i.smooth_scroll_delta.y * 0.001;
        });

        let av_rect = egui::Rect::from_min_size(pos, av_size);

        let center = av_rect.center();
        let scale2 = (av_size * 0.9) / self.size;
        let scale = scale2.x.min(scale2.y) * self.current_zoom;
        let old_scale = scale2.x.min(scale2.y) * old_zoom;

        

        ui.ctx().input(|i| {
            self.hover_position_screen =
                i.pointer.hover_pos().unwrap_or(self.hover_position_screen);
            if i.pointer.button_down(egui::PointerButton::Primary) {
                self.current_delta_screen += i.pointer.delta();
                let hover_position_world = self.hover_position_screen / scale;
                let hover_position_world_old = self.hover_position_screen / old_scale;
                self.current_delta_screen += hover_position_world - hover_position_world_old;
            }
        });
        let scaled_size = self.size * scale;

        let primary_clicked = ui.ctx().input(|i| {
            i.pointer.primary_clicked()
        });

        let mut element_context = ElementContext {
            hover_position_world: self.hover_position_screen / scale - self.current_delta_screen,
            scale,
            primary_clicked,
            current_delta_screen: self.current_delta_screen,
        };

        let rect = egui::Rect::from_center_size(center + self.current_delta_screen, scaled_size);
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.set_clip_rect(av_rect);
            ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
            if let Some(builder) = current_builder {
                if let Some(element) = builder(ui, &element_context) {
                    self.elements.push(element);
                    *current_builder = None;
                }
            }
        });
    }
}

pub struct ElementContext {
    pub hover_position_world: Pos2,
    pub scale: f32,
    pub current_delta_screen: Vec2,
    pub primary_clicked: bool,
}

pub trait Element {
    fn render(&mut self, ctx: &ElementContext, ui: &mut egui::Ui);
}

pub struct Rectangle {
    pub rect: egui::Rect,
}

impl Element for Rectangle {
    fn render(&mut self, ctx: &ElementContext, ui: &mut egui::Ui) {
        let scaled_rect = (self.rect * ctx.scale).translate(ctx.current_delta_screen);
        ui.painter()
            .rect_filled(scaled_rect, 0.0, egui::Color32::from_rgb(200, 200, 200));
    }
}

pub struct ElementBuilder {
    pub name: &'static str,
    pub create: &'static dyn Fn() -> Box<BuilderFn>,
}

pub type BuilderFn = dyn FnMut(&mut egui::Ui, &ElementContext) -> Option<Box<dyn Element>>;

pub fn builders() -> Vec<ElementBuilder> {
    vec![ElementBuilder {
        name: "Rectangle",
        create: &|| {
            let mut start_pos = None;
            Box::new(move |ui, ctx| {
                if ctx.primary_clicked {
                    if let Some(start_pos) = start_pos {
                        return Some(Box::new(Rectangle {
                            rect: egui::Rect::from_two_pos(start_pos, ctx.hover_position_world),
                        }));
                    } else {
                        start_pos = Some(ctx.hover_position_world);
                    }
                } 
                if let Some(start_pos) = start_pos {
                    ui.painter().rect_filled(
                        egui::Rect::from_two_pos(
                            start_pos * ctx.scale + ctx.current_delta_screen,
                            ctx.hover_position_world * ctx.scale + ctx.current_delta_screen,
                        ),
                        0.0,
                        egui::Color32::from_rgb(200, 200, 200),
                    );
                }
                None
            })
        },
    }]
}
