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
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MEvents {
    pub a_x: Vec<f64>,
    pub a_y: Vec<f64>,
    pub a_z: Vec<f64>,
    pub r_x: Vec<f64>,
    pub r_y: Vec<f64>,
    pub r_z: Vec<f64>,
    pub o_x: Vec<f64>,
    pub o_y: Vec<f64>,
    pub o_z: Vec<f64>,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PushupWorkout {
    pub raw: MEvents,
    pub i: f64,
    pub start: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SurrealDBResult<T> {
    pub result: T,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PushupWorkoutRequest {
    pub id: String,
    pub i: f64,
    pub start: f64,
}

#[derive(serde::Serialize)]
pub struct PushupWorkoutPartRequest<'a> {
    pub workout_id: String,
    pub i: usize,
    pub a_x: &'a [f64],
    pub a_y: &'a [f64],
    pub a_z: &'a [f64],
    pub r_x: &'a [f64],
    pub r_y: &'a [f64],
    pub r_z: &'a [f64],
    pub o_x: &'a [f64],
    pub o_y: &'a [f64],
    pub o_z: &'a [f64],
}

pub fn generate_random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(length);

    let window = window().expect("no global `window` exists");
    let crypto = window.crypto().expect("should have crypto");

    let mut random_bytes = vec![0u8; length];
    crypto
        .get_random_values_with_u8_array(&mut random_bytes)
        .unwrap();

    for byte in random_bytes {
        let idx = (byte as usize) % CHARSET.len();
        result.push(CHARSET[idx] as char);
    }

    result
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

fn main() {
    common::app::run("push-up tracker", |cc| {
        let window = window().unwrap();

        let mut e = Arc::new(Mutex::new(None::<PushupWorkout>));
        let mut o = Arc::new(Mutex::new(Orientation::default()));

        let mut motion_closure: Option<Closure<dyn FnMut(_)>> = None;
        let mut orientation_closure: Option<Closure<dyn FnMut(_)>> = None;

        let start = get_navigation_start();

        let mut promise = get_workouts_p(&cc.egui_ctx);

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            if ui.button("start").clicked() {
                request_device_motion_permission();
                request_orientation_motion_permission();
                let e2 = e.clone();
                let o2 = o.clone();
                let ctx2 = ui.ctx().clone();
                let ctx3 = ui.ctx().clone();
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
                        let mut events = e2.lock();
                        let mut raw = &mut events
                            .get_or_insert_with(|| PushupWorkout {
                                raw: MEvents::default(),
                                i,
                                start: start + t,
                            })
                            .raw;

                        raw.a_x.push(a_x);
                        raw.a_y.push(a_y);
                        raw.a_z.push(a_z);
                        raw.r_x.push(r_x);
                        raw.r_y.push(r_y);
                        raw.r_z.push(r_z);
                        let mut o = o2.lock();
                        raw.o_x.push(o.alpha);
                        raw.o_y.push(o.beta);
                        raw.o_z.push(o.gamma);
                    }
                }) as Box<dyn FnMut(_)>));

                let o2 = o.clone();
                orientation_closure = Some(Closure::wrap(Box::new(
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
                )as Box<dyn FnMut(_)>));

                window
                    .add_event_listener_with_callback(
                        "deviceorientation",
                        orientation_closure
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unchecked_ref(),
                    )
                    .unwrap();
                window
                    .add_event_listener_with_callback(
                        "devicemotion",
                        motion_closure.as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }

            {
                let e = e.lock();
                if let Some(e) = e.as_ref() {
                    let mut i = e.raw.a_x.len();
                    if i > 0 {
                        i -= 1; // last element
                        ui.label(format!("i: {:?}", e.i));
                        ui.label(format!("x: {:?}", e.raw.a_x[i]));
                        ui.label(format!("y: {:?}", e.raw.a_y[i]));
                        ui.label(format!("z: {:?}", e.raw.a_z[i]));
                        ui.label(format!("x: {:?}", e.raw.r_x[i]));
                        ui.label(format!("y: {:?}", e.raw.r_y[i]));
                        ui.label(format!("z: {:?}", e.raw.r_z[i]));
                        ui.label(format!("x: {:?}", e.raw.o_x[i]));
                        ui.label(format!("y: {:?}", e.raw.o_y[i]));
                        ui.label(format!("z: {:?}", e.raw.o_z[i]));
                        ui.label(format!("count: {}", i + 1));
                    }
                    if ui.button("send").clicked() {
                        create_pushup_workout(&e, ui.ctx());
                    }
                } else {
                    if ui.button("send fake").clicked() {
                        let w = PushupWorkout {
                            raw: MEvents {
                                a_x: vec![0.02699037132151425f64; 200],
                                a_y: vec![0.08210610934384167f64; 200],
                                a_z: vec![0.07051679383702576f64; 200],
                                r_x: vec![0.017808628413081166f64; 200],
                                r_y: vec![0.02699037132151425f64; 200],
                                r_z: vec![0.0484881365761160f64; 200],
                                o_x: vec![0.017808628413081166f64; 200],
                                o_y: vec![0.02699037132151425f64; 200],
                                o_z: vec![0.0484881365761160f64; 200],
                            },
                            i: 0.0,
                            start: start,
                        };
                        create_pushup_workout(&w, ui.ctx());
                    }
                }
            }

            if let Some(workouts) = promise.ready_mut() {
                match workouts {
                    Ok(workouts) => {
                        ui.label(format!("got workouts: {}", workouts.len()));
                        if !workouts.is_empty() {
                            let workout = &workouts[0];
                            egui_plot::Plot::new("plot")
                                .legend(egui_plot::Legend::default().follow_insertion_order(true))
                                .show(ui, |plot_ui| {
                                    plot_ui.line(egui_plot::Line::new(
                                        "a_x",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.a_x),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "a_y",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.a_y),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "a_z",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.a_z),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "r_x",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.r_x),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "r_y",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.r_y),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "r_z",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.r_z),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "o_x",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.o_x),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "o_y",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.o_y),
                                    ));
                                    plot_ui.line(egui_plot::Line::new(
                                        "o_z",
                                        egui_plot::PlotPoints::from_ys_f64(&workout.raw.o_z),
                                    ));
                                });
                        }
                    }
                    Err(err) => {
                        ui.label(format!("Error: {}", err));
                    }
                }
            } else {
                ui.label("loading workouts...");
            }
        });
    });
}

fn create_pushup_workout(w: &PushupWorkout, ctx: &egui::Context) {
    let id = generate_random_string(20);
    let s = serde_json::to_string(&PushupWorkoutRequest {
        id: id.clone(),
        i: w.i,
        start: w.start,
    })
    .expect("Failed to serialize workout data");

    let mut r = ehttp::Request::post(
        "https://delicate-weasel-06a8qi6eq5qs39dplfhab6frcc.aws-euw1.surreal.cloud/key/pushups",
        s.as_bytes().to_vec(),
    );
    r.headers.insert("Content-Type", "application/json");
    r.headers.insert("Authorization", "Basic YWRtaW46YWRtaW4=");
    r.headers.insert("Accept", "application/json");
    r.headers.insert("Surreal-DB", "sport");
    r.headers.insert("Surreal-NS", "sport");

    let ctx2 = ctx.clone();
    common::execute(async move {
        let response = common::http::fetch(&r).await;
        ctx2.request_repaint();
    });
    let mut i = 0;
    let n = 50;
    loop {
        if (i * n) >= w.raw.a_x.len() {
            break;
        }
        let r = (i * n)..(((i + 1) * n).min(w.raw.a_x.len()));
        let a_x = &w.raw.a_x[r.clone()];
        let a_y = &w.raw.a_y[r.clone()];
        let a_z = &w.raw.a_z[r.clone()];
        let r_x = &w.raw.r_x[r.clone()];
        let r_y = &w.raw.r_y[r.clone()];
        let r_z = &w.raw.r_z[r.clone()];

        let part_request = PushupWorkoutPartRequest {
            workout_id: id.clone(),
            i,
            a_x,
            a_y,
            a_z,
            r_x,
            r_y,
            r_z,
            o_x: &w.raw.o_x[r.clone()],
            o_y: &w.raw.o_y[r.clone()],
            o_z: &w.raw.o_z[r.clone()],
        };

        let s =
            serde_json::to_string(&part_request).expect("Failed to serialize workout part data");

        let mut r2 = ehttp::Request::post(
            "https://delicate-weasel-06a8qi6eq5qs39dplfhab6frcc.aws-euw1.surreal.cloud/key/pushups_part",
            s.as_bytes().to_vec(),
        );
        r2.headers.insert("Content-Type", "application/json");
        r2.headers.insert("Authorization", "Basic YWRtaW46YWRtaW4=");
        r2.headers.insert("Accept", "application/json");
        r2.headers.insert("Surreal-DB", "sport");
        r2.headers.insert("Surreal-NS", "sport");
        let ctx2 = ctx.clone();
        common::execute(async move {
            let response = common::http::fetch(&r2).await;
            ctx2.request_repaint();
        });

        i += 1;
    }
}

fn get_workouts_p(
    ctx: &egui::Context,
) -> poll_promise::Promise<Result<Vec<PushupWorkout>, std::string::String>> {
    let (sender, p) = poll_promise::Promise::new();
    let ctx = ctx.clone();
    common::execute(async move {
        let mut workouts = get_workouts().await;
        sender.send(workouts);
        ctx.request_repaint();
    });
    return p;
}

async fn get_workouts() -> Result<Vec<PushupWorkout>, String> {
    // return Err("Not implemented".to_string());
    let mut r = ehttp::Request::post(
        "https://delicate-weasel-06a8qi6eq5qs39dplfhab6frcc.aws-euw1.surreal.cloud/sql",
        r#"SELECT 
array::flatten(a_x) AS raw.a_x, 
array::flatten(a_y) AS raw.a_y, 
array::flatten(a_z) AS raw.a_z, 
array::flatten(r_x) AS raw.r_x, 
array::flatten(r_y) AS raw.r_y, 
array::flatten(r_z) AS raw.r_z,
array::flatten(o_x) AS raw.o_x, 
array::flatten(o_y) AS raw.o_y, 
array::flatten(o_z) AS raw.o_z, 
workout_id,
workout.i as i,
workout.start as start
FROM (
  SELECT * FROM pushups_part ORDER BY i
) GROUP BY workout_id;"#
            .as_bytes()
            .to_vec(),
    );
    r.headers.insert("Content-Type", "application/json");
    r.headers.insert("Authorization", "Basic YWRtaW46YWRtaW4=");
    r.headers.insert("Accept", "application/json");
    r.headers.insert("Surreal-DB", "sport");
    r.headers.insert("Surreal-NS", "sport");
    let response = common::http::fetch(&r).await?;
    let mut result =
        serde_json::from_slice::<Vec<SurrealDBResult<Vec<PushupWorkout>>>>(&response.bytes)
            .map_err(|e| e.to_string())?
            .into_iter()
            .last()
            .unwrap();

    return Ok(result.result);
}

fn log(message: &str) {
    web_sys::console::log_1(&message.into());
}
