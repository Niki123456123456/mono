mod filters;
pub use filters::*;
mod gyro_bias;
use glam::dvec3;
pub use gyro_bias::*;
mod ekf;
pub use ekf::*;
mod neck_model;
pub use neck_model::*;
mod head_tracker;
pub use head_tracker::*;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::{window, DeviceMotionEvent, EventTarget, Performance};
use js_sys::{Function, Reflect};

thread_local! {
    static GLOBAL_TRACKER: RefCell<Option<HeadTracker>> = RefCell::new(None);
    static MOTION_HANDLER: RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>> = RefCell::new(None);
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
                    web_sys::console::error_1(&e);
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
                    web_sys::console::error_1(&e);
                }
            }
        }
    }
}


pub fn start_head_tracking()  {
    GLOBAL_TRACKER.with(|t| {
        *t.borrow_mut() = Some(HeadTracker::new());
    });

    let win = window().ok_or_else(|| JsValue::from_str("no window")).unwrap();
    let target: EventTarget = win.clone().into();

    // Closure that handles DeviceMotionEvent and feeds samples to the tracker.
    let handler = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
        let motion: DeviceMotionEvent = match event.dyn_into() {
            Ok(m) => m,
            Err(_) => return,
        };

        let perf: Performance = match window().and_then(|w| w.performance()) {
            Some(p) => p,
            None => return,
        };
        let now_ms = perf.now(); // f64 milliseconds
        let ts_ns = (now_ms * 1e6) as u64; // convert to nanoseconds

        let accel_opt = motion.acceleration_including_gravity();
        let rot_opt = motion.rotation_rate();

        GLOBAL_TRACKER.with(|cell| {
            let mut borrowed = cell.borrow_mut();
            let tracker = match borrowed.as_mut() {
                Some(t) => t,
                None => return,
            };

            // Accelerometer: m/s^2, including gravity.
            if let Some(accel) = accel_opt.as_ref() {
                let ax = accel.x().unwrap_or(0.0);
                let ay = accel.y().unwrap_or(0.0);
                let az = accel.z().unwrap_or(0.0);

                let sample = AccelerometerData {
                    system_timestamp: ts_ns,
                    sensor_timestamp_ns: ts_ns,
                    data: dvec3(ax as f64, ay as f64, az as f64),
                };
                tracker.process_accelerometer_sample(sample);
                 web_sys::console::log_1(&format!("{:?}", sample).into());
            }

            // Gyroscope: usually deg/s -> convert to rad/s.
            if let Some(rot) = rot_opt.as_ref() {
                let deg_to_rad = std::f64::consts::PI / 180.0;

                // Browser axis mapping: alpha=z, beta=x, gamma=y in most specs.
                let gz = rot.alpha().unwrap_or(0.0) as f64 * deg_to_rad;
                let gx = rot.beta().unwrap_or(0.0) as f64 * deg_to_rad;
                let gy = rot.gamma().unwrap_or(0.0) as f64 * deg_to_rad;

                let sample = GyroscopeData {
                    system_timestamp: ts_ns,
                    sensor_timestamp_ns: ts_ns,
                    data: dvec3(gx, gy, gz),
                };
                tracker.process_gyroscope_sample(sample);
                web_sys::console::log_1(&format!("{:?}", sample).into());
            }
        });
    }));

    target.add_event_listener_with_callback("devicemotion", handler.as_ref().unchecked_ref()).unwrap();

    // Store closure so it isn't dropped.
    MOTION_HANDLER.with(|slot| {
        *slot.borrow_mut() = Some(handler);
    });

}

pub fn pause_head_tracking() {
    GLOBAL_TRACKER.with(|cell| {
        if let Some(ref mut tracker) = *cell.borrow_mut() {
            tracker.pause();
        }
    });
}

pub fn resume_head_tracking() {
    GLOBAL_TRACKER.with(|cell| {
        if let Some(ref mut tracker) = *cell.borrow_mut() {
            tracker.resume();
        }
    });
}

pub fn get_pose_now(orientation: ViewportOrientation) ->  Option<(Pose, usize)> {
    let perf = match window().and_then(|w| w.performance()) {
        Some(p) => p,
        None => return None,
    };
    let now_ms = perf.now();
    let ts_ns = (now_ms * 1e6) as i64;

    let mut pose_opt: Option<(Pose, usize)> = None;


    GLOBAL_TRACKER.with(|cell| {
        if let Some(ref mut tracker) = *cell.borrow_mut() {
            pose_opt = Some((tracker.get_pose(ts_ns, orientation.into()),  tracker.count));
        }
    });

    return pose_opt;
}