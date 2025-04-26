use std::sync::Arc;

use three_d::*;

fn main() {
    common::app::run("sketch3", |cc| {
        let gl = cc.gl.as_ref().unwrap().clone();

        let three_d = three_d::Context::from_gl_context(gl).unwrap();

        let mut camera = Camera::new_perspective(
            Viewport::new_at_origo(512, 512),
            vec3(5.0, 2.0, 2.5),
            vec3(0.0, 0.0, -0.5),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.1,
            1000.0,
        );

        let mut cube = Gm::new(
            Mesh::new(&three_d, &CpuMesh::cube()),
            PhysicalMaterial::new_transparent(
                &three_d,
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
        cube.set_transformation(Mat4::from_translation(vec3(0.0, 0.0, 1.3)) * Mat4::from_scale(0.2));
    
        let light0 = DirectionalLight::new(&three_d, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&three_d, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));


        //camera.set_viewport(viewport)
        let mut control = OrbitControl::new(camera.target(), 1.0, 100.0);

        let render = Arc::new( move |camera| {
           cube.render(&camera, &[&light0, &light1]);
        });

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            paint_three_d(ui, &three_d, ui.available_size(), &mut camera, &render);
        });
    });
}

fn paint_three_d(ui: &mut egui::Ui, three_d: &three_d::Context, desired_size: egui::Vec2, camera : &mut Camera, render : &Arc<impl Fn(Camera) + Sync + Send + 'static>) {
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

    let events = ui.input(|i| i.events.clone());
    let three_d = three_d.clone();
    let mut camera = camera.clone();
    let render = render.clone();
    let callback = egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let viewport = info.viewport_in_pixels();
            let viewport = Viewport {
                x: viewport.left_px,
                y: viewport.from_bottom_px,
                width: viewport.width_px.abs() as u32,
                height: viewport.height_px.abs() as u32,
            };
        
            let clip_rect = info.clip_rect_in_pixels();
            three_d.set_scissor(ScissorBox {
                x: clip_rect.left_px,
                y: clip_rect.from_bottom_px,
                width: clip_rect.width_px.abs() as u32,
                height: clip_rect.height_px.abs() as u32,
            });
            let mut camera = camera.clone();
            camera.set_viewport(viewport);
            //paint_with_three_d(&three_d, &info, f);
            //(f)(viewport);
            (render)(camera.clone());
        })),
    };
    ui.painter().add(callback);
}

fn map_event(event: egui::Event) -> Option<three_d::Event> {
    match event {
        egui::Event::Copy => {}
        egui::Event::Cut => {}
        egui::Event::Paste(_) => {}
        egui::Event::Text(_) => {}
        egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
        } => {
            //three_d::Event::KeyPress { kind: (), modifiers: (), handled: () }
        }
        egui::Event::PointerMoved(pos2) => {}
        egui::Event::MouseMoved(vec2) => {}
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers,
        } => {}
        egui::Event::PointerGone => {}
        egui::Event::Zoom(_) => {}
        egui::Event::Ime(ime_event) => {}
        egui::Event::Touch {
            device_id,
            id,
            phase,
            pos,
            force,
        } => {}
        egui::Event::MouseWheel {
            unit,
            delta,
            modifiers,
        } => {
            //three_d::Event::MouseWheel { delta: (), position: (), modifiers: (), handled: false };
        }
        egui::Event::WindowFocused(_) => {}
        egui::Event::Screenshot {
            viewport_id,
            user_data,
            image,
        } => {}
    }
    return None;
}

