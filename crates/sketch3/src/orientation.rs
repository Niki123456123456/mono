use std::sync::Arc;

use egui::mutex::Mutex;
use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::{DeviceMotionEvent, DeviceOrientationEvent, window};

#[derive(Default)]
pub struct Orientation {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub ahrs: fusion_ahrs::Ahrs,
}
pub fn request_device_motion_permission() {
    let global = js_sys::global();

    // Check if 'DeviceMotionEvent' exists
    if let Ok(device_motion_event) = Reflect::get(&global, &JsValue::from_str("DeviceMotionEvent"))
    {
        // Check if 'requestPermission' is a function
        if let Ok(request_permission) = Reflect::get(
            &device_motion_event,
            &JsValue::from_str("requestPermission"),
        ) {
            if request_permission.is_function() {
                let func: &Function = request_permission.unchecked_ref();
                // Call DeviceMotionEvent.requestPermission()
                if let Err(e) = func.call0(&device_motion_event) {
                    console::error_1(&e);
                }
            }
        }
    }
}

pub fn request_orientation_motion_permission() {
    let global = js_sys::global();

    // Check if 'DeviceMotionEvent' exists
    if let Ok(device_motion_event) =
        Reflect::get(&global, &JsValue::from_str("DeviceOrientationEvent"))
    {
        // Check if 'requestPermission' is a function
        if let Ok(request_permission) = Reflect::get(
            &device_motion_event,
            &JsValue::from_str("requestPermission"),
        ) {
            if request_permission.is_function() {
                let func: &Function = request_permission.unchecked_ref();
                // Call DeviceMotionEvent.requestPermission()
                if let Err(e) = func.call0(&device_motion_event) {
                    console::error_1(&e);
                }
            }
        }
    }
}

pub fn get_navigation_start() -> f64 {
    let window = window().expect("no global `window` exists");
    let performance = window
        .performance()
        .expect("performance should be available");

    performance.timing().navigation_start()
}

pub struct OrientationTracker {
    pub o: Arc<Mutex<Orientation>>,
    pub motion_closure: Option<Closure<dyn FnMut(DeviceMotionEvent)>>,
    pub orientation_closure: Option<Closure<dyn FnMut(DeviceOrientationEvent)>>,
}

impl OrientationTracker {
    pub fn new() -> Self {
        Self {
            o: Arc::new(Mutex::new(Orientation::default())),
            motion_closure: None,
            orientation_closure: None,
        }
    }

    pub fn start(&mut self, ctx: three_d::egui::Context) {
        request_device_motion_permission();
        request_orientation_motion_permission();
        let window = window().unwrap();
        let o2 = self.o.clone();
        let ctx2 = ctx.clone();
        let ctx3 = ctx.clone();
        self.motion_closure = Some(Closure::wrap(Box::new(move |event: DeviceMotionEvent| {
            ctx2.request_repaint();

            let acceleration = event
                .acceleration()
                .and_then(|acc| acc.x().zip(acc.y().zip(acc.z())));
            let rotation = event
                .rotation_rate()
                .and_then(|acc| acc.alpha().zip(acc.beta().zip(acc.gamma())));
            let interval = event.interval();
            let t = event.time_stamp();

            if let Some((((a_x, (a_y, a_z)), (r_x, (r_y, r_z))), i)) =
                acceleration.zip(rotation).zip(interval)
            {
                let mut o = o2.lock();
                o.ahrs.update_no_magnetometer(
                    nalgebra::Vector3::new(r_x as f32, r_y as f32, r_z as f32),
                    nalgebra::Vector3::new(r_x as f32, r_y as f32, r_z as f32),
                    i as f32,
                );
                
            }
        }) as Box<dyn FnMut(_)>));

        let o2 = self.o.clone();
        self.orientation_closure = Some(Closure::wrap(Box::new(
            move |event: DeviceOrientationEvent| {
                ctx3.request_repaint();

                let alpha = event.alpha().unwrap_or(0.0);
                let beta = event.beta().unwrap_or(0.0);
                let gamma = event.gamma().unwrap_or(0.0);
                {
                    let mut o = o2.lock();
                    o.alpha = alpha;
                    o.beta = beta;
                    o.gamma = gamma;
                }
            },
        ) as Box<dyn FnMut(_)>));

        window
            .add_event_listener_with_callback(
                "deviceorientation",
                self.orientation_closure
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
        window
            .add_event_listener_with_callback(
                "devicemotion",
                self.motion_closure
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
    }
}
