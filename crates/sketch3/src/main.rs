use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use three_d::*;

pub mod controls;

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
        let light0 = DirectionalLight::new(&three_d, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&three_d, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

        let mut control = make_static_mut(controls::OrbitControl::new(camera.target(), 1.0, 100.0));

        let camera = make_static_mut(camera);
        let cube = make_static(cube);
        let light0 = make_static(light0);
        let light1 = make_static(light1);

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            test(ui, &three_d, ui.available_size(), |r| {
                let mut camera = camera.lock().unwrap();
                let mut control = control.lock().unwrap();
                control.handle_events(camera.deref_mut(), &r.ctx);
                camera.set_viewport(r.viewport);
                cube.render(camera.deref(), &[light0, light1]);
            });
        });
    });
}
pub struct RenderContext {
    pub viewport: Viewport,
    pub ctx: egui::Context,
}

fn make_static_mut<T>(value: T) -> &'static std::sync::Mutex<T> {
    Box::leak(Box::new(std::sync::Mutex::new(value)))
}

fn make_static<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn test(
    ui: &mut egui::Ui,
    three_d: &three_d::Context,
    desired_size: egui::Vec2,
    render: impl FnOnce(RenderContext) + Send + Sync + 'static,
) {
    let ctx = ui.ctx().clone();
    let render: PaintCallback<RenderContext> = render.into();
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());
    let three_d = three_d.clone();
    let callback = egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let ctx = ctx.clone();
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

            let render_context = RenderContext { viewport, ctx };

            (render.as_fn())(render_context);
        })),
    };
    ui.painter().add(callback);
}

struct PaintCallback<T> {
    inner: Arc<std::sync::Mutex<Option<Box<dyn FnOnce(T) + Send + Sync>>>>,
}

impl<T: 'static> Clone for PaintCallback<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static, X> From<X> for PaintCallback<T>
where
    X: FnOnce(T) + Send + Sync + 'static,
{
    fn from(value: X) -> Self {
        PaintCallback::new(value)
    }
}

impl<T: 'static> PaintCallback<T> {
    pub fn new(f: impl FnOnce(T) + Send + Sync + 'static) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(Some(Box::new(f)))),
        }
    }

    pub fn as_fn(&self) -> impl Fn(T) + Sync + Send + 'static {
        let inner = self.inner.clone();
        move |t| {
            if let Some(f) = inner.lock().unwrap().take() {
                f(t);
            }
        }
    }
}
