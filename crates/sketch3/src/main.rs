use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bricks::line_writer::LineMesh;
use three_d::*;

use crate::maps::TileCache;

pub mod bricks;
pub mod export;
pub mod maps;

pub struct SelectedPart {
    pub part: bricks::Part,
    pub lines: CpuMesh,
    pub triangles: CpuMesh,
    pub lines_gpu: bricks::line_writer::LineMesh,
    pub triangles_gpu: Mesh,
    pub tex: Option<three_d::Texture2DRef>,
}

pub fn remove_transformation(p: &mut three_d_asset::Primitive) {
    if let three_d_asset::Geometry::Triangles(g) = &mut p.geometry {
        match &mut g.positions {
            Positions::F32(vector3s) => {
                for pos in vector3s.iter_mut() {
                    let transformed = p.transformation * Vec4::new(pos.x, pos.y, pos.z, 1.0);
                    pos.x = transformed.x;
                    pos.y = transformed.y;
                    pos.z = transformed.z;
                }
            }
            Positions::F64(vector3s) => {
                // todo
            }
        }
    }
    p.transformation = Mat4::identity();
}

pub fn merge_geos(cpu_model: &mut three_d::CpuModel) {
    let mut merged_positions = Vec::new();
    let mut merged_indices = Vec::new();
    let mut x = 0;
    for geo in cpu_model.geometries.iter_mut() {
        if let three_d_asset::Geometry::Triangles(mesh) = &mut geo.geometry {
            match (&mut mesh.positions, &mut mesh.indices) {
                (Positions::F32(positions), Indices::U32(indices)) => {
                    for i in indices.iter_mut() {
                        *i += x;
                        //println!("{} {}", i, x);
                    }
                    x += positions.len() as u32;
                    merged_indices.append(indices);
                    merged_positions.append(positions);
                }
                _ => {
                    println!("Unsupported positions or indices format");
                }
            }
        }
    }
    cpu_model.geometries = vec![three_d_asset::Primitive {
        name: Default::default(),
        transformation: Mat4::identity(),
        animations: Vec::new(),
        geometry: three_d_asset::Geometry::Triangles(three_d_asset::TriMesh {
            positions: Positions::F32(merged_positions),
            indices: Indices::U32(merged_indices),
            ..Default::default()
        }),
        material_index: None,
    }];
}

pub fn color_picker(ui: &mut egui::Ui, c: &mut Srgba) {
    let mut color = egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a);
    egui::color_picker::color_edit_button_srgba(
        ui,
        &mut color,
        egui::color_picker::Alpha::OnlyBlend,
    );

    c.r = color.r();
    c.g = color.g();
    c.b = color.b();
    c.a = color.a();
}

// https://rebrickable.com/downloads/

fn bricks_ui(
    ui: &mut egui::Ui,
    selected_part: &mut Option<SelectedPart>,
    source_map: &mut weldr::SourceMap,
    ctx3d: &Context,
    resolver: &bricks::ZipResolver,
) {
    ui.heading("Part Categories");

    if let Some(selected_part) = selected_part.as_mut() {
        if ui.button("Export obj").clicked() {
            export::obj::export(
                &selected_part.lines,
                &selected_part.triangles,
                &selected_part.part.name,
            );
        }
        if ui.button("Export stl").clicked() {
            export::stl::export(&selected_part.lines, &selected_part.part.name);
        }
        if ui.button("Export gltf").clicked() {
            export::gltf::export(&selected_part.triangles, &selected_part.part.name);
        }
    }

    // color_picker(ui, &mut background_color);
    // color_picker(ui, &mut face_color);
    // color_picker(ui, &mut line_color);

    if let Some(selected_part) = &selected_part {
        ui.label(format!(
            "Selected part: {}: {}",
            selected_part.part.number, selected_part.part.name
        ));
    }
    let mut scroll_height = ui.available_height();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for x in bricks::PART_CATEGORIES.iter() {
            ui.collapsing(&x.name, |ui| {
                for part in x.parts.iter() {
                    if true && part.number.contains("pr") {
                        continue;
                    }
                    if ui
                        .label(format!("{}: {}", part.number, part.name))
                        .clicked()
                    {
                        match weldr::parse(&format!("{}.dat", part.number), resolver, source_map) {
                            Ok(x) => {
                                let source_file = source_map.get(&x).unwrap();

                                let lines =
                                    bricks::line_writer::get_line_mesh(source_file, &source_map);
                                let triangles = bricks::line_writer::get_triangle_mesh(
                                    source_file,
                                    &source_map,
                                );

                                let lines_gpu = bricks::line_writer::LineMesh::new(&ctx3d, &lines);
                                let triangles_gpu = three_d::Mesh::new(&ctx3d, &triangles);

                                *selected_part = Some(SelectedPart {
                                    part: part.clone(),
                                    lines,
                                    triangles,
                                    lines_gpu,
                                    triangles_gpu,
                                    tex: None,
                                });
                            }
                            Err(err) => {
                                println!("Error parsing part {}: {}", part.number, err);
                            }
                        }
                    }
                }
            });
        }
    });
}

fn main() {
    //crate::bricks::creation::create();

    run("sketch3", |c| {
        let mut camera = Camera::new_perspective(
            Viewport::new_at_origo(512, 512),
            vec3(47702560.0, 0.0, -9691560.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            100.,        //0.1,
            1000000000., //1000.0,
        );

        let mut control =
            crate::maps::OrbitControl::new(camera.target(), 6_378_000.0, 50_000_000.0);

        let mut background_color = Srgba::new_opaque(27, 27, 27);
        let mut face_color = Srgba::new_opaque(74, 74, 74);
        let mut line_color = Srgba::new_opaque(159, 159, 159);

        let mut material = ColorMaterial::new(
            &c.ctx,
            &CpuMaterial {
                albedo: Srgba::WHITE,
                ..Default::default()
            },
        );

        let mut red = ColorMaterial::new(
            &c.ctx,
            &CpuMaterial {
                albedo: Srgba::RED,
                ..Default::default()
            },
        );

        let mut green: ColorMaterial = ColorMaterial::new(
            &c.ctx,
            &CpuMaterial {
                albedo: Srgba::GREEN,
                ..Default::default()
            },
        );

        let mut blue: ColorMaterial = ColorMaterial::new(
            &c.ctx,
            &CpuMaterial {
                albedo: Srgba::BLUE,
                ..Default::default()
            },
        );
        let max_distane = 50_000_000.;

        let x_line = LineMesh::from_vector(
            &c.ctx,
            vec![
                -Vec3::unit_x() * max_distane,
                Vec3::zero(),
                Vec3::zero(),
                Vec3::unit_x() * max_distane,
            ],
        );
        let y_line = LineMesh::from_vector(
            &c.ctx,
            vec![
                -Vec3::unit_y() * max_distane,
                Vec3::zero(),
                Vec3::zero(),
                Vec3::unit_y() * max_distane,
            ],
        );
        let z_line = LineMesh::from_vector(
            &c.ctx,
            vec![
                -Vec3::unit_z() * max_distane,
                Vec3::zero(),
                Vec3::zero(),
                Vec3::unit_z() * max_distane,
            ],
        );

        let light = AmbientLight::new(&c.ctx, 0.5, Srgba::WHITE);

        let mut include_pr = false;
        let mut resolver = bricks::get_ldraw_lib();
        let mut source_map = weldr::SourceMap::new();

        let mut selected_part: Option<SelectedPart> = None;

         let mut key = "".to_string();


        let mut tile_cache = TileCache::new(&c.ctx, key.clone());

        let mut shown_tiles = 0;

       
        return Box::new(move |mut ctx| {
            let mut panel_width = 0.0;

            let ctx3d = ctx.frame_input.context.clone();

            tile_cache.load(&ctx3d);

            ctx.update_ui(|egui_ctx| {
                use three_d::egui::*;
                SidePanel::left("side_panel").show(egui_ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("api key");
                        egui::TextEdit::singleline(&mut key).show(ui);
                        if ui.button("run").clicked() {
                            // tile_cache.set_client(key.clone());
                        }
                    });

                    let camera_position = camera.position();
                    ui.label(format!(
                        "p {} {} {}",
                        camera_position.x, camera_position.y, camera_position.z
                    ));
                    let target = camera.target();
                    ui.label(format!("t {} {} {}", target.x, target.y, target.z));
                    let (longlat, ele) =
                        maps::xyz_to_longlat(maps::three_d_vec3_to_glam_d(&camera_position));
                    let elef = if ele > 10_000. {
                        format!("{:.0} km", ele / 1000.)
                    } else if ele > 1_000. {
                        format!("{:.1} km", ele / 1000.)
                    } else {
                        format!("{:.0} m", ele)
                    };
                    ui.label(format!("{:.1}° {:.1}° ele {}", longlat.x, longlat.y, elef));
                    ui.label(format!("tiles {}", shown_tiles));
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

            let ctx3d = ctx.frame_input.context.clone();

            let _ = ctx
                .frame_input
                .screen()
                .clear(ClearState::color_and_depth(
                    background_color.r as f32 / 255.0,
                    background_color.g as f32 / 255.0,
                    background_color.b as f32 / 255.0,
                    background_color.a as f32 / 255.0,
                    1.0,
                ))
                .write(|| {
                    x_line.render_with_material(&red, &camera, &[&light]);
                    y_line.render_with_material(&green, &camera, &[&light]);
                    z_line.render_with_material(&blue, &camera, &[&light]);

                    shown_tiles = tile_cache.render(&camera, &[&light]);

                    if let Some(selected_part) = &selected_part {
                        unsafe {
                            ctx3d.enable(crate::context::POLYGON_OFFSET_FILL);
                            ctx3d.polygon_offset(1.0, 1.0);
                        }

                        material.color = face_color;
                        material.texture = selected_part.tex.clone();
                        if material.texture.is_some() {
                            material.color = line_color;
                        }
                        selected_part.triangles_gpu.render_with_material(
                            &material,
                            &camera,
                            &[&light],
                        );

                        unsafe {
                            ctx3d.disable(crate::context::POLYGON_OFFSET_FILL);
                        }

                        material.color = line_color;
                        material.texture = None;
                        selected_part
                            .lines_gpu
                            .render_with_material(&material, &camera, &[&light]);
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
