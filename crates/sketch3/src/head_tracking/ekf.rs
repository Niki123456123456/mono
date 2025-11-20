use glam::{DMat3, DMat4, DQuat, DVec3, dvec3};
use crate::head_tracking::*;

pub const EPS: f64 = 1e-15;

#[derive(Clone, Copy, Debug)]
pub struct GyroscopeData {
    pub system_timestamp: u64,
    pub sensor_timestamp_ns: u64,
    pub data: DVec3,
}

#[derive(Clone, Copy, Debug)]
pub struct AccelerometerData {
    pub system_timestamp: u64,
    pub sensor_timestamp_ns: u64,
    pub data: DVec3,
}

#[derive(Clone)]
pub struct RotationState {
    pub timestamp: i64,
    pub sensor_from_start_rotation: DQuat,
    pub sensor_from_start_rotation_velocity: DVec3,
}

impl Default for RotationState {
    fn default() -> Self {
        Self {
            timestamp: 0,
            sensor_from_start_rotation: DQuat::IDENTITY,
            sensor_from_start_rotation_velocity: DVec3::ZERO,
        }
    }
}

pub struct SensorFusionEkf {
    current_state: RotationState,

    is_timestep_filter_initialized: bool,
    is_gyro_filter_valid: bool,
    is_aligned_with_gravity: bool,

    state_covariance: DMat3,
    process_covariance: DMat3,
    accel_measurement_covariance: DMat3,
    innovation_covariance: DMat3,
    accel_measurement_jacobian: DMat3,
    kalman_gain: DMat3,

    innovation: DVec3,
    accel_measurement: DVec3,
    prediction: DVec3,
    control_input: DVec3,
    state_update: DVec3,

    current_gyro_sensor_timestamp_ns: u64,
    current_accel_sensor_timestamp_ns: u64,

    filtered_gyro_timestep_s: f64,
    num_gyro_timestep_samples: u32,

    previous_accel_norm: f64,
    moving_avg_accel_norm_change: f64,

    execute_reset_with_next_accel_sample: bool,

    gyro_bias_estimator: GyroscopeBiasEstimator,
    gyro_bias_estimate: DVec3,

    velocity_filter: Option<LowpassFilter>,
}

impl SensorFusionEkf {
    pub fn new() -> Self {
        let mut s = Self {
            current_state: RotationState::default(),
            is_timestep_filter_initialized: false,
            is_gyro_filter_valid: false,
            is_aligned_with_gravity: false,
            state_covariance: DMat3::IDENTITY,
            process_covariance: DMat3::IDENTITY,
            accel_measurement_covariance: DMat3::IDENTITY,
            innovation_covariance: DMat3::IDENTITY,
            accel_measurement_jacobian: DMat3::ZERO,
            kalman_gain: DMat3::ZERO,
            innovation: DVec3::ZERO,
            accel_measurement: DVec3::ZERO,
            prediction: DVec3::ZERO,
            control_input: DVec3::ZERO,
            state_update: DVec3::ZERO,
            current_gyro_sensor_timestamp_ns: 0,
            current_accel_sensor_timestamp_ns: 0,
            filtered_gyro_timestep_s: 0.0,
            num_gyro_timestep_samples: 0,
            previous_accel_norm: 0.0,
            moving_avg_accel_norm_change: 0.0,
            execute_reset_with_next_accel_sample: false,
            gyro_bias_estimator: GyroscopeBiasEstimator::new(),
            gyro_bias_estimate: DVec3::ZERO,
            velocity_filter: None,
        };
        s.reset_state();
        s
    }

    pub fn set_low_pass_filter(&mut self, cutoff_hz: i32) {
        self.velocity_filter = Some(LowpassFilter::new(cutoff_hz as f64));
        self.reset_state();
    }

    pub fn reset(&mut self) {
        self.execute_reset_with_next_accel_sample = true;
    }

    pub fn rotate_sensor_space_to_start_space_transformation(
        &mut self,
        rotation: DQuat,
    ) {
        self.current_state.sensor_from_start_rotation *= rotation;
    }

    fn reset_state(&mut self) {
        const MIN_ACCEL_NOISE_SIGMA: f64 = 0.75;
        const INITIAL_STATE_COV: f64 = 25.0;
        const INITIAL_PROCESS_COV: f64 = 1.0;

        self.current_state.sensor_from_start_rotation = DQuat::IDENTITY;
        self.current_state.sensor_from_start_rotation_velocity = DVec3::ZERO;

        self.current_gyro_sensor_timestamp_ns = 0;
        self.current_accel_sensor_timestamp_ns = 0;

        self.state_covariance = DMat3::IDENTITY * INITIAL_STATE_COV;
        self.process_covariance = DMat3::IDENTITY * INITIAL_PROCESS_COV;
        self.accel_measurement_covariance =
            DMat3::IDENTITY * MIN_ACCEL_NOISE_SIGMA * MIN_ACCEL_NOISE_SIGMA;
        self.innovation_covariance = DMat3::IDENTITY;
        self.accel_measurement_jacobian = DMat3::ZERO;
        self.kalman_gain = DMat3::ZERO;

        self.innovation = DVec3::ZERO;
        self.accel_measurement = DVec3::ZERO;
        self.prediction = DVec3::ZERO;
        self.control_input = DVec3::ZERO;
        self.state_update = DVec3::ZERO;

        self.moving_avg_accel_norm_change = 0.0;
        self.is_timestep_filter_initialized = false;
        self.is_gyro_filter_valid = false;
        self.is_aligned_with_gravity = false;

        self.gyro_bias_estimator.reset();
        self.gyro_bias_estimate = DVec3::ZERO;

        if let Some(ref mut v) = self.velocity_filter {
            v.reset();
        }
    }

    pub fn latest_rotation_state(&self) -> RotationState {
        self.current_state.clone()
    }

    pub fn predict_rotation(&self, requested_timestamp: i64) -> DQuat {
        if requested_timestamp == 0 {
            return self.current_state.sensor_from_start_rotation;
        }

        let dt_s = compute_time_diff_seconds(
            requested_timestamp,
            self.current_state.timestamp,
        );

        let update = get_rotation_from_gyro(
            self.current_state.sensor_from_start_rotation_velocity,
            dt_s,
        );
        update * self.current_state.sensor_from_start_rotation
    }

    pub fn process_gyroscope_sample(&mut self, sample: GyroscopeData) {
        const MAX_GYRO_SAMPLE_DELAY_S: f64 = 0.04;
        const DEFAULT_GYRO_TIMESTEP_S: f64 = 0.01;
        if self.execute_reset_with_next_accel_sample {
            return;
        }

        if self.current_gyro_sensor_timestamp_ns >= sample.sensor_timestamp_ns {
            return;
        }

        if self.current_gyro_sensor_timestamp_ns != 0 {
            let mut dt_s = (sample.sensor_timestamp_ns
                - self.current_gyro_sensor_timestamp_ns) as f64
                * 1e-9;

            if dt_s > MAX_GYRO_SAMPLE_DELAY_S {
                dt_s = if self.is_gyro_filter_valid {
                    self.filtered_gyro_timestep_s
                } else {
                    DEFAULT_GYRO_TIMESTEP_S
                };
            } else {
                self.filter_gyro_timestep(dt_s);
            }

            // Bias estimation
            self.gyro_bias_estimator
                .process_gyroscope(sample.data, sample.sensor_timestamp_ns);
            if self.gyro_bias_estimator.is_current_estimate_valid() {
                self.gyro_bias_estimate = self.gyro_bias_estimator.gyro_bias();
            }

            if self.is_aligned_with_gravity {
                let corrected = sample.data - self.gyro_bias_estimate;
                let rot_from_gyro = get_rotation_from_gyro(corrected, dt_s);
                self.current_state.sensor_from_start_rotation =
                    rot_from_gyro * self.current_state.sensor_from_start_rotation;
                let motion_update = DMat3::from_quat(rot_from_gyro);
                self.update_state_covariance(motion_update);
                self.state_covariance = self.state_covariance
                    + (dt_s * dt_s) * self.process_covariance;
            }
        }

        self.current_state.timestamp = sample.system_timestamp as i64;
        self.current_gyro_sensor_timestamp_ns = sample.sensor_timestamp_ns;

        if let Some(ref mut vfilter) = self.velocity_filter {
            vfilter.add_sample(
                sample.data - self.gyro_bias_estimate,
                self.current_gyro_sensor_timestamp_ns,
            );
            if vfilter.is_initialized() {
                let filtered_velocity = vfilter.filtered();
                self.current_state.sensor_from_start_rotation_velocity= dvec3(
                    filtered_velocity[0],
                    filtered_velocity[1],
                    filtered_velocity[2],
                );
            }
        } else {
            self.current_state.sensor_from_start_rotation_velocity = dvec3(
                sample.data[0] - self.gyro_bias_estimate[0],
                sample.data[1] - self.gyro_bias_estimate[1],
                sample.data[2] - self.gyro_bias_estimate[2],
            );
        }
    }

    pub fn process_accelerometer_sample(&mut self, sample: AccelerometerData) {
        if self.current_accel_sensor_timestamp_ns >= sample.sensor_timestamp_ns {
            return;
        }

        if self
            .execute_reset_with_next_accel_sample
            .then_some(())
            .is_some()
        {
            self.execute_reset_with_next_accel_sample = false;
            self.reset_state();
        }

        self.accel_measurement = dvec3(sample.data[0], sample.data[1], sample.data[2]);
        self.current_accel_sensor_timestamp_ns = sample.sensor_timestamp_ns;

        self.gyro_bias_estimator
            .process_accelerometer(sample.data, sample.sensor_timestamp_ns);

        const CANONICAL_Z: DVec3 = DVec3::new(0.0, 0.0, 1.0);

        if !self.is_aligned_with_gravity {
            self.current_state.sensor_from_start_rotation =
                DQuat::from_rotation_arc(CANONICAL_Z, self.accel_measurement.normalize());
            self.is_aligned_with_gravity = true;
            self.previous_accel_norm = self.accel_measurement.length();
            return;
        }

        self.update_measurement_covariance();

        self.innovation =
            self.compute_innovation(self.current_state.sensor_from_start_rotation);
        self.compute_measurement_jacobian();

        // S = H P H^T + R
        self.innovation_covariance = self.accel_measurement_jacobian
            * self.state_covariance
            * self.accel_measurement_jacobian.transpose()
            + self.accel_measurement_covariance;

        // K = P H^T S^-1
        
        if let Some(inv_s) = Some(self.innovation_covariance.inverse()) {
            self.kalman_gain =
                self.state_covariance * self.accel_measurement_jacobian.transpose() * inv_s;
        } else {
            return;
        }

        self.state_update = self.kalman_gain * self.innovation;

        // P = (I - K H) P
        let i = DMat3::IDENTITY;
        self.state_covariance =
            (i - self.kalman_gain * self.accel_measurement_jacobian)
                * self.state_covariance;

        let rot_from_state_update = rotation_from_vector(self.state_update);
        self.current_state.sensor_from_start_rotation =
            rot_from_state_update * self.current_state.sensor_from_start_rotation;
        let motion_update = DMat3::from_quat(rot_from_state_update);
        self.update_state_covariance(motion_update);
    }

    fn update_state_covariance(&mut self, motion_update: DMat3) {
        self.state_covariance =
            motion_update * self.state_covariance * motion_update.transpose();
    }

    fn filter_gyro_timestep(&mut self, timestep: f64) {
        const TIMESTEP_FILTER_COEFF: f64 = 0.95;
        const TIMESTEP_FILTER_MIN_SAMPLES: u32 = 10;

        if !self.is_timestep_filter_initialized {
            self.filtered_gyro_timestep_s = timestep;
            self.num_gyro_timestep_samples = 1;
            self.is_timestep_filter_initialized = true;
            return;
        }

        self.filtered_gyro_timestep_s = TIMESTEP_FILTER_COEFF
            * self.filtered_gyro_timestep_s
            + (1.0 - TIMESTEP_FILTER_COEFF) * timestep;
        self.num_gyro_timestep_samples += 1;

        if self.num_gyro_timestep_samples > TIMESTEP_FILTER_MIN_SAMPLES {
            self.is_gyro_filter_valid = true;
        }
    }

    fn compute_innovation(&self, rotation_in: DQuat) -> DVec3 {
        const CANONICAL_Z: DVec3 = DVec3::new(0.0, 0.0, 1.0);
        let predicted_down = rotation_in * CANONICAL_Z;
        let rot = DQuat::from_rotation_arc(predicted_down.normalize(), self.accel_measurement.normalize());
        let (axis, angle) = rot.to_axis_angle();
        axis * angle
    }

    fn compute_measurement_jacobian(&mut self) {
        const FINITE_DIFF_EPS: f64 = 1e-7;

        for dof in 0..3 {
            let mut delta = DVec3::ZERO;
            delta[dof] = FINITE_DIFF_EPS;

            let eps_rot = rotation_from_vector(delta);
            let delta_rot = self.compute_innovation(
                eps_rot * self.current_state.sensor_from_start_rotation,
            );

            let col = self.accel_measurement_jacobian.col_mut(dof);
            *col = (self.innovation - delta_rot) / FINITE_DIFF_EPS;
        }
    }

    fn update_measurement_covariance(&mut self) {
        const SMOOTHING_FACTOR: f64 = 0.5;
        const MAX_ACCEL_NORM_CHANGE: f64 = 0.15;
        const MIN_ACCEL_NOISE_SIGMA: f64 = 0.75;
        const MAX_ACCEL_NOISE_SIGMA: f64 = 7.0;

        let current_norm = self.accel_measurement.length();
        let norm_change = (current_norm - self.previous_accel_norm).abs();
        self.previous_accel_norm = current_norm;

        self.moving_avg_accel_norm_change = SMOOTHING_FACTOR * norm_change
            + (1.0 - SMOOTHING_FACTOR) * self.moving_avg_accel_norm_change;

        let ratio = self.moving_avg_accel_norm_change / MAX_ACCEL_NORM_CHANGE;
        let sigma = (MIN_ACCEL_NOISE_SIGMA
            + ratio * (MAX_ACCEL_NOISE_SIGMA - MIN_ACCEL_NOISE_SIGMA))
            .min(MAX_ACCEL_NOISE_SIGMA);

        self.accel_measurement_covariance =
            DMat3::IDENTITY * sigma * sigma;
    }
}

fn rotation_from_vector(a: DVec3) -> DQuat {
    let norm_a = a.length();
    if norm_a < EPS {
        DQuat::IDENTITY
    } else {
         DQuat::from_axis_angle(a / norm_a, norm_a)
    }
}

fn get_rotation_from_gyro(gyro: DVec3, timestep_s: f64) -> DQuat {
    let vel = gyro.length();
    if vel < EPS {
        return DQuat::IDENTITY;
    }
    // Invert sign to go from start->sensor to sensor->start. :contentReference[oaicite:5]{index=5}

    DQuat::from_axis_angle(gyro / vel, -timestep_s * vel)
}

fn compute_time_diff_seconds(a_ns: i64, b_ns: i64) -> f64 {
    (a_ns - b_ns) as f64 * 1e-9
}

fn set(m :&mut DMat3, index: (usize, usize), v : f64){
    let mut c =m.col_mut(index.0);
    c[index.1] = v;
}