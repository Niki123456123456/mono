use std::sync::Arc;

use egui::mutex::Mutex;
use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::{DeviceMotionEvent, DeviceOrientationEvent, window};

#[derive(Default)]
pub struct MEvent {
    pub a_x: f64,
    pub a_y: f64,
    pub a_z: f64,
    pub r_x: f64,
    pub r_y: f64,
    pub r_z: f64,
    pub i: f64,
    pub t: f64,
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

fn main() {
    common::app::run("push-up tracker", |cc| {
        let window = window().unwrap();

        let mut e = Arc::new(Mutex::new(Vec::new()));

        let mut motion_closure: Option<Closure<dyn FnMut(_)>> = None;

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            if ui.button("start").clicked() {
                request_device_motion_permission();
                let e2 = e.clone();
                let ctx2 = ui.ctx().clone();
                motion_closure = Some(Closure::wrap(Box::new(move |event: DeviceMotionEvent| {
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
                        let e = MEvent {
                            a_x,
                            a_y,
                            a_z,
                            r_x,
                            r_y,
                            r_z,
                            i,
                            t,
                        };
                        let mut events = e2.lock();
                        events.push(e);
                    }
                }) as Box<dyn FnMut(_)>));

                window
                    .add_event_listener_with_callback(
                        "devicemotion",
                        motion_closure.as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }

            {
                let e = e.lock();
                if let Some(e) = e.last() {
                    ui.label(format!("interval: {:?}", e.i));
                    ui.label(format!("x: {:?}", e.a_x));
                    ui.label(format!("y: {:?}", e.a_y));
                    ui.label(format!("z: {:?}", e.a_z));
                    ui.label(format!("x: {:?}", e.r_x));
                    ui.label(format!("y: {:?}", e.r_y));
                    ui.label(format!("z: {:?}", e.r_z));
                }
            }
        });
    });
}

fn log_f64(label: &str, value: Option<f64>) {
    if let Some(v) = value {
        log(&format!("{}: {}", label, v));
    }
}

fn log(message: &str) {
    web_sys::console::log_1(&message.into());
}
