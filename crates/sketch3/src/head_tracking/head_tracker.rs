use glam::{DQuat, DVec3, dvec3};

use crate::head_tracking::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ViewportOrientation {
    LandscapeLeft = 0,
    LandscapeRight = 1,
    Portrait = 2,
    PortraitUpsideDown = 3,
}

impl ViewportOrientation {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => ViewportOrientation::LandscapeLeft,
            1 => ViewportOrientation::LandscapeRight,
            2 => ViewportOrientation::Portrait,
            3 => ViewportOrientation::PortraitUpsideDown,
            _ => ViewportOrientation::LandscapeLeft,
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: DVec3,
    pub orientation: DQuat,
}

pub struct HeadTracker {
    is_tracking: bool,
    sensor_fusion: SensorFusionEkf,
    latest_gyro_data: GyroscopeData,

    viewport_orientation: ViewportOrientation,
    viewport_orientation_initialized: bool,
    pub count : usize,
}

impl HeadTracker {
    pub fn new() -> Self {
        Self {
            is_tracking: true,
            sensor_fusion: SensorFusionEkf::new(),
            latest_gyro_data: GyroscopeData {
                system_timestamp: 0,
                sensor_timestamp_ns: 0,
                data: dvec3(0.0, 0.0, 0.0),
            },
            viewport_orientation: ViewportOrientation::LandscapeLeft,
            viewport_orientation_initialized: false,
            count: 0,
        }
    }

    pub fn pause(&mut self) {
        if !self.is_tracking {
            return;
        }

        // Stop prediction by feeding zero angular velocity. :contentReference[oaicite:3]{index=3}
        let mut event = self.latest_gyro_data;
        event.data = dvec3(0.0, 0.0, 0.0);
        self.process_gyroscope_sample(event);
        self.is_tracking = false;
    }

    pub fn resume(&mut self) {
        self.is_tracking = true;
    }

    pub fn recenter(&mut self) {
        self.sensor_fusion.reset();
    }

    pub fn set_low_pass_filter(&mut self, cutoff_frequency_hz: f64) {
        self.sensor_fusion.set_low_pass_filter(cutoff_frequency_hz as i32);
    }

    pub fn process_accelerometer_sample(&mut self, event: AccelerometerData) {
        if !self.is_tracking {
            return;
        }
        self.sensor_fusion.process_accelerometer_sample(event);
       
    }

    pub fn process_gyroscope_sample(&mut self, event: GyroscopeData) {
        if !self.is_tracking {
            return;
        }
        self.latest_gyro_data = event;
        self.sensor_fusion.process_gyroscope_sample(event);
         self.count+=1;
    }

    pub fn get_pose(
        &mut self,
        timestamp_ns: i64,
        viewport_orientation: ViewportOrientation,
    ) -> Pose {
        let rotation = self.get_rotation(viewport_orientation, timestamp_ns);

        if self.viewport_orientation_initialized
            && viewport_orientation != self.viewport_orientation
        {
            let compensation =
                viewport_change_rotation_compensation()[self.viewport_orientation.idx()]
                                                       [viewport_orientation.idx()];
            self.sensor_fusion
                .rotate_sensor_space_to_start_space_transformation(compensation);
        }

        self.viewport_orientation = viewport_orientation;
        self.viewport_orientation_initialized = true;

        let orientation_xyzw = rotation;
        let position = apply_neck_model(orientation_xyzw, 1.0);

        Pose {
            position,
            orientation: orientation_xyzw,
        }
    }

    fn get_rotation(
        &self,
        viewport_orientation: ViewportOrientation,
        timestamp_ns: i64,
    ) -> DQuat {
        let predicted = self.sensor_fusion.predict_rotation(timestamp_ns);
        let sensor_to_display =
            sensor_to_display_rotations()[viewport_orientation.idx()];
        let ekf_to_head =
            ekf_to_head_tracker_rotations()[viewport_orientation.idx()];
        sensor_to_display * predicted * ekf_to_head
    }
}

pub fn map_with_orientation(predicted : DQuat, viewport_orientation: ViewportOrientation,) -> DQuat{
    let sensor_to_display =
            sensor_to_display_rotations()[viewport_orientation.idx()];
        let ekf_to_head =
            ekf_to_head_tracker_rotations()[viewport_orientation.idx()];
        sensor_to_display * predicted * ekf_to_head
}

// === Static rotations ported from head_tracker.cc. :contentReference[oaicite:4]{index=4} ===

pub fn sensor_to_display_rotations() -> [DQuat; 4] {
    // Directly use the same quaternions as SensorToDisplayRotations() in C++. :contentReference[oaicite:5]{index=5}
    [

        
        DQuat::from_array([0.0, 0.0, 0.7071067811865476_f64, 0.7071067811865476_f64]),  // LL
        DQuat::from_array([0.0, 0.0, -0.7071067811865476_f64, 0.7071067811865476_f64]), // LR
        DQuat::from_array([0.0, 0.0, 0.0, 1.0]),                                         // P
        DQuat::from_array([0.0, 0.0, 1.0, 0.0]),                                         // PUD
    ]
}

pub  fn ekf_to_head_tracker_rotations() -> [DQuat; 4] {
    // Port of EkfToHeadTrackerRotations(). :contentReference[oaicite:6]{index=6}
    [
        DQuat::from_array([0.5, -0.5, -0.5, 0.5]), // LL
        DQuat::from_array([0.5, 0.5, 0.5, 0.5]),   // LR
        DQuat::from_array([
            0.7071067811865476_f64,
            0.0,
            0.0,
            0.7071067811865476_f64,
        ]), // P
        DQuat::from_array([
            0.0,
            -0.7071067811865476_f64,
            -0.7071067811865476_f64,
            0.0,
        ]), // PUD
    ]
}

fn rotation_z(angle: f64) -> DQuat {
    DQuat::from_axis_angle(dvec3(0.0, 0.0, 1.0), angle)
}

fn viewport_change_rotation_compensation() -> [[DQuat; 4]; 4] {
    let half_pi = std::f64::consts::FRAC_PI_2;
    let pi = std::f64::consts::PI;

    [
        // Current: LandscapeLeft
        [
            rotation_z(0.0),
            rotation_z(pi),
            rotation_z(-half_pi),
            rotation_z(half_pi),
        ],
        // Current: LandscapeRight
        [
            rotation_z(pi),
            rotation_z(0.0),
            rotation_z(half_pi),
            rotation_z(-half_pi),
        ],
        // Current: Portrait
        [
            rotation_z(half_pi),
            rotation_z(-half_pi),
            rotation_z(0.0),
            rotation_z(pi),
        ],
        // Current: PortraitUpsideDown
        [
            rotation_z(-half_pi),
            rotation_z(half_pi),
            rotation_z(pi),
            rotation_z(0.0),
        ],
    ]
}
