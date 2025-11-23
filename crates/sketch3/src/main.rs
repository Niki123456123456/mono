use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bricks::line_writer::LineMesh;
use glam::vec2;
use three_d::{egui::viewport, *};
use three_d_asset::{ProjectionType, UvCoordinate};

use crate::{
    maps::{Place, TileCache},
    orientation::OrientationTracker,
};

pub mod bricks;
pub mod export;
pub mod head_tracking;
pub mod maps;
pub mod orientation;
pub mod stero_view;

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

fn main2() {
    //crate::bricks::creation::create();

    run("sketch3", |c| {
        let mut camera = Camera::new_perspective(
            Viewport::new_at_origo(512, 512),
            vec3(47702560.0, 0.0, -9691560.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0., 0., 1.),
            degrees(45.0),
            100.,        //0.1,
            1000000000., //1000.0,
        );

        let mut control =
            crate::maps::OrbitControl::new(camera.target(), 6_378_000.0 - 15_000., 50_000_000.0);

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

        let mut key = "AIzaSyBIXRsd8edAP6xU5LGwWVeqi6wVrt0et_4".to_string();
        let mut search = "".to_string();

        let mut search_promise = None;

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
                    let (lat, lon, ele) =
                        maps::xyz_to_latlonele(maps::three_d_vec3_to_glam_d(&camera_position));
                    let elef = if ele > 10_000. {
                        format!("{:.0} km", ele / 1000.)
                    } else if ele > 1_000. {
                        format!("{:.1} km", ele / 1000.)
                    } else {
                        format!("{:.0} m", ele)
                    };
                    ui.label(format!("lat {:.1}° lon {:.1}° ele {}", lat, lon, elef));
                    ui.label(format!("tiles {}", shown_tiles));

                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        egui::TextEdit::singleline(&mut search)
                            .return_key(Some(egui::KeyboardShortcut::new(
                                egui::Modifiers::NONE,
                                egui::Key::Enter,
                            )))
                            .hint_text("search")
                            .show(ui);
                        if ui.button("🔍").clicked() {
                            search_promise = Some(crate::maps::search(search.clone()))
                        }
                    });

                    if let Some(search_promise) = &search_promise {
                        if let Some(places) = search_promise.ready() {
                            for place in places {
                                ui.horizontal(|ui| {
                                    ui.label(&place.name);
                                    if ui.button("visit").clicked() {
                                        let coodinates =
                                            maps::latlon_to_xyz(place.lat, place.lon, 1000.);
                                        camera = Camera::new_perspective(
                                            Viewport::new_at_origo(512, 512),
                                            maps::glam_d_vec3_to_three_d(&coodinates),
                                            vec3(0.0, 0.0, 0.0),
                                            vec3(0.0, 0.0, 1.0),
                                            degrees(45.0),
                                            100.,        //0.1,
                                            1000000000., //1000.0,
                                        );
                                    }
                                });
                                ui.label(format!("{:.1}° {:.1}°", place.lat, place.lon));
                            }
                        }
                    }
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
        viewport: window.viewport(),
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
    pub viewport: Viewport,
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

fn get_camera(viewport: Viewport, alpfa: f32, gamma: f32) -> Camera2 {
    let mut camera = Camera2::new_perspective(
        viewport,
        vec3(0.0, 2.0, 0.0),
        vec3(0.0, 2.0, -0.5),
        vec3(0.0, 1.0, 0.0),
        degrees(33.3798),
        0.1,
        1000.0,
    );
    let a = (alpfa / 360.0) * std::f32::consts::TAU;
    let g = if gamma > 0. {
        ((90. - gamma) / 90.) * -std::f32::consts::PI
    } else if gamma < 0. {
        ((90. + gamma) / 90.) * std::f32::consts::PI
    } else {
        0.
    };
    // camera.rotate_around_with_fixed_up(camera.position, a, 0.);
    camera.rotate_around_with_fixed_up(camera.position, 0., g);
    return camera;
}
fn main() {
    head_tracking::request_device_motion_permission();
    head_tracking::request_orientation_motion_permission();

    run("sketch3", |c| {
        let mut tracker = OrientationTracker::new();
        let viewport: Viewport = c.viewport;
        let is_portrait = viewport.height > viewport.width;

        let mut started = false;

        let lens = stero_view::LensDistortion::default(vec2(
            viewport.width as f32,
            viewport.height as f32,
        ));

        let mut renderer = stero_view::Renderer::new(&c.ctx, lens, viewport);

        let light = AmbientLight::new(&c.ctx, 0.5, Srgba::WHITE);

        let mut assets = three_d_asset::io::RawAssets::new();
        assets.insert(
            "CubeRoom_BakedDiffuse.png",
            include_bytes!("assets/CubeRoom_BakedDiffuse.png").to_vec(),
        );
        assets.insert(
            "CubeRoom.obj",
            include_bytes!("assets/CubeRoom.obj").to_vec(),
        );
        let room_texture: CpuTexture = assets.deserialize("CubeRoom_BakedDiffuse.png").unwrap();
        let room_mesh: CpuMesh = assets.deserialize("CubeRoom.obj").unwrap();
        let mut room = Gm::new(
            Mesh::new(&c.ctx, &room_mesh),
            ColorMaterial::new_opaque(
                &c.ctx,
                &CpuMaterial {
                    albedo_texture: Some(room_texture),
                    ..Default::default()
                },
            ),
        );

        let mut camera = get_camera(viewport, 0., 0.);

        return Box::new(move |mut ctx| {
            let pose =
                head_tracking::get_pose_now(head_tracking::ViewportOrientation::LandscapeLeft);

            ctx.update_ui(|egui_ctx| {
                use three_d::egui::*;
                if !started || true {
                    SidePanel::left("side_panel").show(egui_ctx, |ui| {
                        // if let Some((pose, count)) = &pose {
                        //     ui.label(format!("{:?}", pose.orientation));
                        //     ui.label(format!("{:?}", pose.position));
                        //     ui.label(format!("{:?}", count));
                        // } else {
                        //     if ui.button("Test").clicked() {
                        //         head_tracking::request_device_motion_permission();
                        //         head_tracking::request_orientation_motion_permission();
                        //         // head_tracking::start_head_tracking();
                        //         tracker.start(ui.ctx().clone());
                        //     }
                        // }
                        if ui.button("Test").clicked() {
                            head_tracking::request_device_motion_permission();
                            head_tracking::request_orientation_motion_permission();
                            // head_tracking::start_head_tracking();
                            tracker.start(ui.ctx().clone());
                            started = true;
                        }
                        {
                            let o = tracker.o.lock();
                            let c = o.ahrs.quaternion().coords;
                            let q = glam::Quat::from_xyzw(c.x, c.y, c.z, c.w);
                            // ui.label(format!("{:.2} {:.2} {:.2} {:.2}", c.x, c.y, c.z, c.w));
                            ui.label(format!(
                                "alpa: {:.2}\nbeta: {:.2}\ngamma: {:.2} ",
                                o.alpha, o.beta, o.gamma
                            ));
                        }
                        ui.horizontal(|ui| {
                            if ui.button("-").clicked() {
                                // pi std::f32::consts::PI
                                // 2x pi =  std::f32::consts::TAU
                                camera.rotate_around_with_fixed_up(camera.position, -0.1, 0.);
                            }
                            if ui.button("+").clicked() {
                                camera.rotate_around_with_fixed_up(camera.position, 0.1, 0.);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("-").clicked() {
                                camera.rotate_around_with_fixed_up(camera.position, 0., -0.1);
                            }
                            if ui.button("+").clicked() {
                                camera.rotate_around_with_fixed_up(camera.position, 0., 0.1);
                            }
                        });
                    });
                }
            });

            // if let Some((pose, count)) = &pose {
            //     let o = glam::DMat4::from_quat(pose.orientation);
            //     // let view = camera.view();
            //     // let target = camera.target();
            //     // let target = (
            //     //     maps::dglam_to_three_d(&o)
            //     //     * target.extend(1.0))
            //     // .truncate();
            //     // let p = camera.position();
            //     // let up = camera.up();
            //     // camera.set_view(p, target, up);
            //     camera.view = maps::dglam_to_three_d(&o);
            // }

            {
                let o = tracker.o.lock();
                //let c = o.ahrs.quaternion().coords;
                // let q = glam::Quat::from_xyzw(c.x, c.y, c.z, c.w);
                // let m = glam::Mat4::from_quat(q);
                //let q = head_tracking::map_with_orientation(glam::DQuat::from_xyzw(c.x as f64, c.y as f64, c.z as f64, c.w as f64), head_tracking::ViewportOrientation::LandscapeLeft);
                // let q = quat_from_device_orientation(o.alpha, o.beta, o.gamma);
                // let m = glam::DMat4::from_quat(q);
                // camera.view = maps::dglam_to_three_d(&m);
                camera = get_camera(viewport, o.alpha as f32, o.gamma as f32);
            }

            let _ = ctx
                .frame_input
                .screen()
                .clear(ClearState::color_and_depth(0.1, 0.1, 0.1, 1.0, 1.0))
                .write(|| {
                    renderer.render(is_portrait, ctx.frame_input.viewport, |v| {
                        camera.set_viewport(v);
                        room.render(&camera, &[&light]);
                    });
                    return ctx.gui.render();
                });
        });
    })
}

fn quat_from_device_orientation(alpha_deg: f64, beta_deg: f64, gamma_deg: f64) -> glam::DQuat {
    let alpha = deg_to_rad(alpha_deg);
    let beta = deg_to_rad(beta_deg);
    let gamma = deg_to_rad(gamma_deg);

    // Axis vectors
    let z_axis = glam::DVec3::Z;
    let x_axis = glam::DVec3::X;
    let y_axis = glam::DVec3::Y;

    // Individual quaternions (intrinsic rotations):
    // Note: order is important – here: Z → X → Y
    let q_alpha = glam::DQuat::from_axis_angle(z_axis, alpha);
    let q_beta = glam::DQuat::from_axis_angle(x_axis, beta);
    let q_gamma = glam::DQuat::from_axis_angle(y_axis, gamma);

    // Combine in the same order as applied in the device frame.
    // For intrinsic rotations, the composition is q = q_z * q_x * q_y
    q_alpha * q_beta * q_gamma
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Camera2 {
    pub viewport: Viewport,
    pub projection_type: three_d_asset::ProjectionType,
    pub z_near: f32,
    pub z_far: f32,
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub view: Mat4,
    pub projection: Mat4,
}

impl Viewer for Camera2 {
    fn position(&self) -> Vec3 {
        self.position
    }

    fn view(&self) -> Mat4 {
        self.view
    }

    fn projection(&self) -> Mat4 {
        self.projection
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn z_near(&self) -> f32 {
        self.z_near
    }

    fn z_far(&self) -> f32 {
        self.z_far
    }

    fn color_mapping(&self) -> ColorMapping {
        ColorMapping::default()
    }

    fn tone_mapping(&self) -> ToneMapping {
        ToneMapping::default()
    }
}

impl Camera2 {
    ///
    /// New camera which projects the world with an orthographic projection.
    ///
    pub fn new_orthographic(
        viewport: Viewport,
        position: Vec3,
        target: Vec3,
        up: Vec3,
        height: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let mut camera = Self::new(viewport);
        camera.set_view(position, target, up);
        camera.set_orthographic_projection(height, z_near, z_far);
        camera
    }

    ///
    /// New camera which projects the world with a perspective projection.
    ///
    pub fn new_perspective(
        viewport: Viewport,
        position: Vec3,
        target: Vec3,
        up: Vec3,
        field_of_view_y: impl Into<Radians>,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let mut camera = Self::new(viewport);
        camera.set_view(position, target, up);
        camera.set_perspective_projection(field_of_view_y, z_near, z_far);
        camera
    }

    ///
    /// Specify the camera to use perspective projection with the given field of view in the y-direction and near and far plane.
    ///
    pub fn set_perspective_projection(
        &mut self,
        field_of_view_y: impl Into<Radians>,
        z_near: f32,
        z_far: f32,
    ) {
        self.z_near = z_near;
        self.z_far = z_far;
        let field_of_view_y = field_of_view_y.into();
        self.projection_type = ProjectionType::Perspective { field_of_view_y };
        self.projection = perspective(field_of_view_y, self.viewport.aspect(), z_near, z_far);
    }

    ///
    /// Specify the camera to use orthographic projection with the given dimensions.
    /// The view frustum height is `+/- height/2`.
    /// The view frustum width is calculated as `height * viewport.width / viewport.height`.
    /// The view frustum depth is `z_near` to `z_far`.
    /// All of the above values are scaled by the zoom factor which is one over the distance between the camera position and target.
    ///
    pub fn set_orthographic_projection(&mut self, height: f32, z_near: f32, z_far: f32) {
        self.projection_type = ProjectionType::Orthographic { height };
        self.z_near = z_near;
        self.z_far = z_far;
        let zoom = self.position.distance(self.target);
        let height = zoom * height;
        let width = height * self.viewport.aspect();
        self.projection = ortho(
            -0.5 * width,
            0.5 * width,
            -0.5 * height,
            0.5 * height,
            z_near,
            z_far,
        );
    }

    ///
    /// Set the current viewport.
    /// Returns whether or not the viewport actually changed.
    ///
    pub fn set_viewport(&mut self, viewport: Viewport) -> bool {
        if self.viewport != viewport {
            self.viewport = viewport;
            match self.projection_type {
                ProjectionType::Orthographic { height } => {
                    self.set_orthographic_projection(height, self.z_near, self.z_far);
                }
                ProjectionType::Perspective { field_of_view_y } => {
                    self.set_perspective_projection(field_of_view_y, self.z_near, self.z_far);
                }
            }
            true
        } else {
            false
        }
    }

    ///
    /// Change the view of the camera.
    /// The camera is placed at the given position, looking at the given target and with the given up direction.
    ///
    pub fn set_view(&mut self, position: Vec3, target: Vec3, up: Vec3) {
        self.position = position;
        self.target = target;
        self.up = up.normalize();
        self.view = Mat4::look_at_rh(
            Point3::from_vec(self.position),
            Point3::from_vec(self.target),
            self.up,
        );
        if let ProjectionType::Orthographic { height } = self.projection_type {
            self.set_orthographic_projection(height, self.z_near, self.z_far);
        }
    }

    /// Returns the [Frustum] for this camera.
    pub fn frustum(&self) -> Frustum {
        Frustum::new(self.projection() * self.view())
    }

    ///
    /// Returns the 3D position at the given uv coordinate of the viewport.
    ///
    pub fn position_at_uv_coordinates(&self, coords: impl Into<UvCoordinate>) -> Vec3 {
        match self.projection_type() {
            ProjectionType::Orthographic { .. } => {
                let coords = coords.into();
                let screen_pos = vec4(2. * coords.u - 1., 2. * coords.v - 1.0, 0.0, 1.);
                let p = (self.screen2ray() * screen_pos).truncate();
                p + (self.position - p).project_on(self.view_direction()) // Project onto the image plane
            }
            ProjectionType::Perspective { .. } => self.position,
        }
    }

    ///
    /// Returns the 3D view direction at the given uv coordinate of the viewport.
    ///
    pub fn view_direction_at_uv_coordinates(&self, coords: impl Into<UvCoordinate>) -> Vec3 {
        match self.projection_type() {
            ProjectionType::Orthographic { .. } => self.view_direction(),
            ProjectionType::Perspective { .. } => {
                let coords = coords.into();
                let screen_pos = vec4(2. * coords.u - 1., 2. * coords.v - 1.0, 0., 1.);
                (self.screen2ray() * screen_pos).truncate().normalize()
            }
        }
    }

    ///
    /// Returns the uv coordinate for the given world position.
    ///
    pub fn uv_coordinates_at_position(&self, position: Vec3) -> UvCoordinate {
        let proj = self.projection() * self.view() * position.extend(1.0);
        (
            0.5 * (proj.x / proj.w.abs() + 1.0),
            0.5 * (proj.y / proj.w.abs() + 1.0),
        )
            .into()
    }

    ///
    /// Returns the type of projection (orthographic or perspective) including parameters.
    ///
    pub fn projection_type(&self) -> &ProjectionType {
        &self.projection_type
    }

    ///
    /// Returns the view matrix, ie. the matrix that transforms objects from world space (as placed in the world) to view space (as seen from this camera).
    ///
    pub fn view(&self) -> Mat4 {
        self.view
    }

    ///
    /// Returns the projection matrix, ie. the matrix that projects objects in view space onto this cameras image plane.
    ///
    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    ///
    /// Returns the viewport.
    ///
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    ///
    /// Returns the distance to the near plane of the camera frustum.
    ///
    pub fn z_near(&self) -> f32 {
        self.z_near
    }

    ///
    /// Returns the distance to the far plane of the camera frustum.
    ///
    pub fn z_far(&self) -> f32 {
        self.z_far
    }

    ///
    /// Returns the position of this camera.
    ///
    pub fn position(&self) -> Vec3 {
        self.position
    }

    ///
    /// Returns the target of this camera, ie the point that this camera looks towards.
    ///
    pub fn target(&self) -> Vec3 {
        self.target
    }

    ///
    /// Returns the up direction of this camera.
    /// This will probably not be orthogonal to the view direction, use [up_orthogonal](Camera::up_orthogonal) instead if that is needed.
    ///
    pub fn up(&self) -> Vec3 {
        self.up
    }

    ///
    /// Returns the up direction of this camera that is orthogonal to the view direction.
    ///
    pub fn up_orthogonal(&self) -> Vec3 {
        self.right_direction().cross(self.view_direction())
    }

    ///
    /// Returns the view direction of this camera, ie. the direction the camera is looking.
    ///
    pub fn view_direction(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    ///
    /// Returns the right direction of this camera.
    ///
    pub fn right_direction(&self) -> Vec3 {
        self.view_direction().cross(self.up)
    }

    fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            projection_type: ProjectionType::Orthographic { height: 1.0 },
            z_near: 0.0,
            z_far: 0.0,
            position: vec3(0.0, 0.0, 5.0),
            target: vec3(0.0, 0.0, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            view: Mat4::identity(),
            projection: Mat4::identity(),
        }
    }

    fn screen2ray(&self) -> Mat4 {
        let mut v = self.view;
        if let ProjectionType::Perspective { .. } = self.projection_type {
            v[3] = vec4(0.0, 0.0, 0.0, 1.0);
        }
        (self.projection * v)
            .invert()
            .unwrap_or_else(|| Mat4::identity())
    }

    ///
    /// Translate the camera by the given change while keeping the same view and up directions.
    ///
    pub fn translate(&mut self, change: Vec3) {
        self.set_view(self.position + change, self.target + change, self.up);
    }

    ///
    /// Rotates the camera by the angle delta around the 'right' direction.
    ///
    pub fn pitch(&mut self, delta: impl Into<Radians>) {
        let target = (self.view.invert().unwrap()
            * Mat4::from_angle_x(delta)
            * self.view
            * self.target.extend(1.0))
        .truncate();
        if (target - self.position).normalize().dot(self.up).abs() < 0.999 {
            self.set_view(self.position, target, self.up);
        }
    }

    ///
    /// Rotates the camera by the angle delta around the 'up' direction.
    ///
    pub fn yaw(&mut self, delta: impl Into<Radians>) {
        let target = (self.view.invert().unwrap()
            * Mat4::from_angle_y(delta)
            * self.view
            * self.target.extend(1.0))
        .truncate();
        self.set_view(self.position, target, self.up);
    }

    ///
    /// Rotates the camera by the angle delta around the 'view' direction.
    ///
    pub fn roll(&mut self, delta: impl Into<Radians>) {
        let up = (self.view.invert().unwrap()
            * Mat4::from_angle_z(delta)
            * self.view
            * (self.up + self.position).extend(1.0))
        .truncate()
            - self.position;
        self.set_view(self.position, self.target, up.normalize());
    }

    ///
    /// Rotate the camera around the given point while keeping the same distance to the point.
    /// The input `x` specifies the amount of rotation in the left direction and `y` specifies the amount of rotation in the up direction.
    /// If you want the camera up direction to stay fixed, use the [rotate_around_with_fixed_up](Camera::rotate_around_with_fixed_up) function instead.
    ///
    pub fn rotate_around(&mut self, point: Vec3, x: f32, y: f32) {
        let dir = (point - self.position()).normalize();
        let right = dir.cross(self.up);
        let up = right.cross(dir);
        let new_dir = (point - self.position() + right * x - up * y).normalize();
        let rotation = rotation_matrix_from_dir_to_dir(dir, new_dir);
        let new_position = (rotation * (self.position() - point).extend(1.0)).truncate() + point;
        let new_target = (rotation * (self.target() - point).extend(1.0)).truncate() + point;
        self.set_view(new_position, new_target, up);
    }

    ///
    /// Rotate the camera around the given point while keeping the same distance to the point and the same up direction.
    /// The input `x` specifies the amount of rotation in the left direction and `y` specifies the amount of rotation in the up direction.
    ///
    pub fn rotate_around_with_fixed_up(&mut self, point: Vec3, x: f32, y: f32) {
        // Since rotations in linear algebra always describe rotations about the origin, we
        // subtract the point, do all rotations, and add the point again
        let position = self.position() - point;
        let target = self.target() - point;
        let up = self.up.normalize();
        // We use Rodrigues' rotation formula to rotate around the fixed `up` vector and around the
        // horizon which is calculated from the camera's view direction and `up`
        // https://en.wikipedia.org/wiki/Rodrigues%27_rotation_formula
        let k_x = up;
        let k_y = (target - position).cross(up).normalize();
        // Prepare cos and sin terms, inverted because the method rotates left and up while
        // rotations follow the right hand rule
        let cos_x = (-x).cos();
        let sin_x = (-x).sin();
        let cos_y = (-y).cos();
        let sin_y = (-y).sin();
        // Do the rotations following the rotation formula
        let rodrigues =
            |v, k: Vec3, cos, sin| v * cos + k.cross(v) * sin + k * k.dot(v) * (1.0 - cos);
        let position_x = rodrigues(position, k_x, cos_x, sin_x);
        let target_x = rodrigues(target, k_x, cos_x, sin_x);
        let position_y = rodrigues(position_x, k_y, cos_y, sin_y);
        let target_y = rodrigues(target_x, k_y, cos_y, sin_y);
        // Forbid to face the camera exactly up or down, fall back to just rotate in x direction
        let new_dir = (target_y - position_y).normalize();
        if new_dir.dot(up).abs() < 0.999 {
            self.set_view(position_y + point, target_y + point, self.up);
        } else {
            self.set_view(position_x + point, target_x + point, self.up);
        }
    }

    ///
    /// Moves the camera towards the camera target by the amount delta while keeping the given minimum and maximum distance to the target.
    ///
    pub fn zoom(&mut self, delta: f32, minimum_distance: f32, maximum_distance: f32) {
        self.zoom_towards(self.target, delta, minimum_distance, maximum_distance);
    }

    ///
    /// Moves the camera towards the given point by the amount delta while keeping the given minimum and maximum distance to the camera target.
    /// Note that the camera target is also updated so that the view direction is the same.
    ///
    pub fn zoom_towards(
        &mut self,
        point: Vec3,
        delta: f32,
        minimum_distance: f32,
        maximum_distance: f32,
    ) {
        let view = self.view_direction();
        let towards = (point - self.position).normalize();
        let cos_angle = view.dot(towards);
        if cos_angle.abs() > std::f32::EPSILON {
            let distance = self.target.distance(self.position);
            let minimum_distance = minimum_distance.max(std::f32::EPSILON);
            let maximum_distance = maximum_distance.max(minimum_distance);
            let delta_clamped =
                distance - (distance - delta).clamp(minimum_distance, maximum_distance);
            let a = view * delta_clamped;
            let b = towards * delta_clamped / cos_angle;
            self.set_view(self.position + b, self.target + b - a, self.up);
        }
    }

    ///
    /// Sets the zoom factor of this camera, ie. the distance to the camera will be `1/zoom_factor`.
    ///
    pub fn set_zoom_factor(&mut self, zoom_factor: f32) {
        let zoom_factor = zoom_factor.max(std::f32::EPSILON);
        let position = self.target - self.view_direction() / zoom_factor;
        self.set_view(position, self.target, self.up);
    }

    ///
    /// The zoom factor for this camera, which is the same as one over the distance between the camera position and target.
    ///
    pub fn zoom_factor(&self) -> f32 {
        let distance = self.target.distance(self.position);
        if distance > f32::EPSILON {
            1.0 / distance
        } else {
            0.0
        }
    }
}
