use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use three_d::*;

pub mod bricks;

pub struct SelectedPart {
    pub part: bricks::Part,
    pub model: Model<PhysicalMaterial>,
}

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

        let mut control = OrbitControl::new(camera.target(), 1.0, 100000.0);

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

        let material = PhysicalMaterial::new(
            &c.ctx,
            &CpuMaterial {
                albedo: Srgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                ..Default::default()
            },
        );

        let light0 = DirectionalLight::new(&c.ctx, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&c.ctx, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

        let mut include_pr = false;
        let mut resolver = bricks::get_ldraw_lib();
        let mut source_map = weldr::SourceMap::new();

        let mut selected_part: Option<SelectedPart> = None;

        return Box::new(move |mut ctx| {
            let mut panel_width = 0.0;

            let ctx3d = ctx.frame_input.context.clone();

            ctx.update_ui(|egui_ctx| {
                use three_d::egui::*;
                SidePanel::left("side_panel").show(egui_ctx, |ui| {
                    ui.heading("Part Categories");
                    if let Some(selected_part) = &selected_part {
                        ui.label(format!(
                            "Selected part: {}: {}",
                            selected_part.part.number, selected_part.part.name
                        ));
                    }
                    let mut scroll_height = ui.available_height();

                    ScrollArea::vertical().show(ui, |ui| {
                        for x in bricks::PART_CATEGORIES.iter() {
                            ui.collapsing(&x.name, |ui| {
                                for part in x.parts.iter() {
                                    if !include_pr && part.number.contains("pr") {
                                        continue;
                                    }
                                    if ui
                                        .label(format!("{}: {}", part.number, part.name))
                                        .clicked()
                                    {
                                        match weldr::parse(
                                            &format!("{}.dat", part.number),
                                            &resolver,
                                            &mut source_map,
                                        ) {
                                            Ok(x) => {
                                                let source_file = source_map.get(&x).unwrap();
                                                let mut cpu_model =
                                                    bricks::gltf_writer::write_gltf(
                                                        false,
                                                        source_file,
                                                        &source_map,
                                                    )
                                                    .unwrap();
                                                cpu_model
                                                    .geometries
                                                    .iter_mut()
                                                    .for_each(|m| m.compute_normals());
                                                let m = Model::<PhysicalMaterial>::new(
                                                    &ctx3d, &cpu_model,
                                                )
                                                .unwrap();
                                                
                                                selected_part = Some(SelectedPart {
                                                    part: part.clone(),
                                                    model: m,
                                                });
                                            }
                                            Err(err) => {
                                                println!(
                                                    "Error parsing part {}: {}",
                                                    part.number, err
                                                );
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    });
                });
                panel_width = egui_ctx.used_rect().width();
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

            let _ = ctx
                .frame_input
                .screen()
                .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
                .render(&camera, cube.into_iter(), &[&light0, &light1])
                .write(|| {
                    if let Some(selected_part) = &selected_part {
                        for x in selected_part.model.iter() {
                            x.render_with_material(&material, &camera, &[&light0, &light1]);
                        }
                    }
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
