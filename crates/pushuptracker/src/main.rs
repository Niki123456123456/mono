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

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MEvents {
    pub a_x: Vec<f64>,
    pub a_y: Vec<f64>,
    pub a_z: Vec<f64>,
    pub r_x: Vec<f64>,
    pub r_y: Vec<f64>,
    pub r_z: Vec<f64>,
    pub i: Vec<f64>,
    pub t: Vec<f64>,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PushupWorkout {
    pub raw: MEvents,
    pub start: f64,
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

pub fn get_navigation_start() -> f64 {
    let window = window().expect("no global `window` exists");
    let performance = window
        .performance()
        .expect("performance should be available");

    performance.timing().navigation_start()
}

fn main() {
    common::app::run("push-up tracker", |cc| {
        let window = window().unwrap();

        let mut e = Arc::new(Mutex::new(Vec::new()));

        let mut motion_closure: Option<Closure<dyn FnMut(_)>> = None;

        let start = get_navigation_start();

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
                if ui.button("send").clicked() {
                    let mut events = MEvents::default();
                    for event in e.iter() {
                        events.a_x.push(event.a_x);
                        events.a_y.push(event.a_y);
                        events.a_z.push(event.a_z);
                        events.r_x.push(event.r_x);
                        events.r_y.push(event.r_y);
                        events.r_z.push(event.r_z);
                        events.i.push(event.i);
                        events.t.push(event.t);
                    }
                    let workout = PushupWorkout { raw: events, start };
                    let s =
                        serde_json::to_string(&workout).expect("Failed to serialize workout data");
                    log(&s);

                    let mut r = ehttp::Request::post(
                        "https://delicate-weasel-06a8qi6eq5qs39dplfhab6frcc.aws-euw1.surreal.cloud/key/pushups",
                        s.as_bytes().to_vec(),
                    );

                    let body = wasm_bindgen::JsValue::from_serde(&workout).unwrap();
                    r.headers.insert("Content-Type", "application/json");
                    r.headers.insert("Authorization", "Basic YWRtaW46YWRtaW4=");
                    r.headers.insert("Accept", "application/json");
                    r.headers.insert("Surreal-DB", "sport");
                    r.headers.insert("Surreal-NS", "sport");

                    let ctx2 = ui.ctx().clone();
                    common::execute(async move {
                        let response = common::http::fetch(&r).await;
                        ctx2.request_repaint();
                    });
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

