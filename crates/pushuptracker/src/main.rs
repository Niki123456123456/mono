use std::sync::Arc;

use egui::mutex::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, DeviceMotionEvent, DeviceOrientationEvent};
use js_sys::{Reflect, Function};
use web_sys::console;

#[derive(Default)]
pub struct MEvent{
    pub interval : Option<f64>,
    pub x : Option<f64>,
    pub y : Option<f64>,
    pub z : Option<f64>,
}

pub fn request_device_motion_permission() {
    let global = js_sys::global();

    // Check if 'DeviceMotionEvent' exists
    if let Ok(device_motion_event) = Reflect::get(&global, &JsValue::from_str("DeviceMotionEvent")) {
        // Check if 'requestPermission' is a function
        if let Ok(request_permission) = Reflect::get(&device_motion_event, &JsValue::from_str("requestPermission")) {
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

        let mut e = Arc::new( Mutex::new(MEvent::default()));

        let mut motion_closure: Option<Closure<dyn FnMut(_)>> = None;

        return Box::new(move |ctx| {

            let ui = ctx.ui;

           
            if ui.button("start").clicked(){
                request_device_motion_permission();
                let e2 = e.clone();
                let ctx2 = ui.ctx().clone();
                motion_closure = Some( Closure::wrap(Box::new(move |event: DeviceMotionEvent| {

                    ctx2.request_repaint();

                    let mut e  = e2.lock();
                    
                    if let Some(acc_g) = event.acceleration_including_gravity() {
                        log_f64("Accelerometer_gx", acc_g.x());
                        log_f64("Accelerometer_gy", acc_g.y());
                        log_f64("Accelerometer_gz", acc_g.z());
                    }
            
                    if let Some(acc) = event.acceleration() {
                        log_f64("Accelerometer_x", acc.x());
                        log_f64("Accelerometer_y", acc.y());
                        log_f64("Accelerometer_z", acc.z());
                        e.x = acc.x();
                        e.y = acc.y();
                        e.z = acc.z();
                    }
        
                    e.interval = event.interval();
            
                    log_f64("Accelerometer_i", event.interval());
            
                    if let Some(rot) = event.rotation_rate() {
                        log_f64("Gyroscope_z", rot.alpha());
                        log_f64("Gyroscope_x", rot.beta());
                        log_f64("Gyroscope_y", rot.gamma());
                    }
            
                    log("Motion event triggered");
                }) as Box<dyn FnMut(_)>));
                
                
                window
                    .add_event_listener_with_callback("devicemotion", motion_closure.as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
                // motion_closure.forget();
            
                // let orientation_closure = Closure::wrap(Box::new(move |event: DeviceOrientationEvent| {
                //     log_f64("Orientation_a", event.alpha());
                //     log_f64("Orientation_b", event.beta());
                //     log_f64("Orientation_g", event.gamma());
            
                //     log("Orientation event triggered");
                // }) as Box<dyn FnMut(_)>);
                // window.add_event_listener_with_callback(
                //     "deviceorientation",
                //     orientation_closure.as_ref().unchecked_ref(),
                // ).unwrap();
                // orientation_closure.forget();
            }

            {
                let e = e.lock();
                ui.label(format!("interval: {:?}", e.interval));
                ui.label(format!("x: {:?}", e.x));
                ui.label(format!("y: {:?}", e.y));
                ui.label(format!("z: {:?}", e.z));
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
