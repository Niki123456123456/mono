use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use three_d::*;

pub mod bricks;

fn main() {
    run("sketch3", |c| {

        let mut camera = Camera::new_perspective(
            Viewport::new_at_origo(512, 512),
            vec3(5.0, 2.0, 2.5),
            vec3(0.0, 0.0, -0.5),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.1,
            1000.0,
        );

        let mut control = OrbitControl::new(camera.target(), 1.0, 100.0);

        let mut cube = Gm::new(
            Mesh::new(&c.ctx, &CpuMesh::cube()),
            PhysicalMaterial::new_transparent(
                &c.ctx,
                &CpuMaterial {
                    albedo: Srgba {
                        r: 0,
                        g: 0,
                        b: 255,
                        a: 100,
                    },
                    ..Default::default()
                },
            ),
        );
        let light0 = DirectionalLight::new(&c.ctx, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&c.ctx, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

        return Box::new(move |mut ctx| {
            let mut panel_width = 0.0;

            ctx.update_ui(|ctx| {
                use three_d::egui::*;
                SidePanel::left("side_panel").show(ctx, |ui| {
                    ui.heading("Hello World");
                });
                panel_width = ctx.used_rect().width();
            });

            let viewport = Viewport {
                x: (panel_width * ctx.frame_input.device_pixel_ratio) as i32,
                y: 0,
                width: ctx.frame_input.viewport.width
                    - (panel_width * ctx.frame_input.device_pixel_ratio) as u32,
                height: ctx.frame_input.viewport.height,
            };

            camera.set_viewport(viewport);
            control.handle_events(&mut camera, &mut ctx.frame_input.events);

            ctx.frame_input
                .screen()
                .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
                .render(&camera, cube.into_iter(), &[&light0, &light1])
                .write(|| {
                    return ctx.gui.render();
                });
        });
    });
}

pub fn run(app_name: &str, f: impl Fn(CreateContext3d) -> Box<dyn FnMut(Context3d)>) {
    let window = Window::new(WindowSettings {
        title: app_name.to_string(),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut gui = three_d::GUI::new(&context);

    let ctx = CreateContext3d {
        ctx: context.clone(),
    };

    let mut update = (f)(ctx);

    window.render_loop(move |mut frame_input| {
        let ctx = Context3d {
            frame_input,
            gui: &mut gui,
        };

        (update)(ctx);
        return FrameOutput::default();
    });
}

pub struct CreateContext3d {
    pub ctx: Context,
}

pub struct Context3d<'a> {
    pub frame_input: FrameInput,
    pub gui: &'a mut GUI,
}

impl<'a> Context3d<'a> {
    pub fn update_ui(&mut self, callback: impl FnOnce(&egui::Context)) {
        self.gui.update(
            &mut self.frame_input.events,
            self.frame_input.accumulated_time,
            self.frame_input.viewport,
            self.frame_input.device_pixel_ratio,
            callback,
        );
    }
}
