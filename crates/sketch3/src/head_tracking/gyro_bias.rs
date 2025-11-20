use glam::{DQuat, DVec3, dvec3, vec3};

use crate::head_tracking::*;



pub struct GyroscopeBiasEstimator {
    accel_lp: LowpassFilter,
    sim_gyro_from_accel_lp: LowpassFilter,
    gyro_lp: LowpassFilter,
    gyro_bias_lp: LowpassFilter,

    accel_static_counter: IsStaticCounter,
    gyro_static_counter: IsStaticCounter,

    current_accumulated_weights_gyro_bias: f64,

    mean_filter: MeanFilter,
    median_filter: MedianFilter,

    last_mean_filtered_accel: DVec3,
}

struct IsStaticCounter {
    min_static_frames: i32,
    consecutive_static_frames: i32,
}

impl IsStaticCounter {
    fn new(min_static_frames: i32) -> Self {
        Self {
            min_static_frames,
            consecutive_static_frames: 0,
        }
    }

    fn append_frame(&mut self, is_static: bool) {
        if is_static {
            self.consecutive_static_frames += 1;
        } else {
            self.consecutive_static_frames = 0;
        }
    }

    fn is_recently_static(&self) -> bool {
        self.consecutive_static_frames >= self.min_static_frames
    }

    fn reset(&mut self) {
        self.consecutive_static_frames = 0;
    }
}

impl GyroscopeBiasEstimator {
    pub fn new() -> Self {
        // Constants from original C++ code. :contentReference[oaicite:3]{index=3}
        const ACCEL_LP_CUTOFF_HZ: f32 = 1.0;
        const ROTVEL_ACCEL_LP_CUTOFF_HZ: f32 = 0.15;
        const GYRO_LP_CUTOFF_HZ: f32 = 1.0;
        const GYRO_BIAS_LP_CUTOFF_HZ: f32 = 0.15;
        const STATIC_FRAMES: i32 = 50;

        Self {
            accel_lp: LowpassFilter::new(ACCEL_LP_CUTOFF_HZ as f64),
            sim_gyro_from_accel_lp: LowpassFilter::new(ROTVEL_ACCEL_LP_CUTOFF_HZ as f64),
            gyro_lp: LowpassFilter::new(GYRO_LP_CUTOFF_HZ as f64),
            gyro_bias_lp: LowpassFilter::new(GYRO_BIAS_LP_CUTOFF_HZ as f64),

            accel_static_counter: IsStaticCounter::new(STATIC_FRAMES),
            gyro_static_counter: IsStaticCounter::new(STATIC_FRAMES),

            current_accumulated_weights_gyro_bias: 0.0,

            mean_filter: MeanFilter::new(5),
            median_filter: MedianFilter::new(5),

            last_mean_filtered_accel: DVec3::ZERO,
        }
    }

    pub fn reset(&mut self) {
        self.accel_lp.reset();
        self.gyro_lp.reset();
        self.gyro_bias_lp.reset();
        self.accel_static_counter.reset();
        self.gyro_static_counter.reset();
        self.current_accumulated_weights_gyro_bias = 0.0;
    }

    pub fn process_gyroscope(&mut self, gyro_sample: DVec3, timestamp_ns: u64) {
        // Update gyro LP and gyro delta.
        self.gyro_lp.add_sample(gyro_sample, timestamp_ns);
        let smoothed_gyro_delta = gyro_sample - self.gyro_lp.filtered();

        const GYRO_DELTA_STATIC_THRESHOLD: f64 = 0.03;
        self.gyro_static_counter.append_frame(
            smoothed_gyro_delta.length() < GYRO_DELTA_STATIC_THRESHOLD as f64,
        );

        if self.gyro_static_counter.is_recently_static()
            && self.accel_static_counter.is_recently_static()
        {
            if !self.update_gyro_bias(gyro_sample, timestamp_ns) {
                // Motion too large, reset static counter.
                self.gyro_static_counter.append_frame(false);
            }
        } else {
            self.current_accumulated_weights_gyro_bias = 0.0;
        }
    }

    pub fn process_accelerometer(&mut self, accel_sample: DVec3, timestamp_ns: u64) {
        let previous_accel_timestamp_ns = self.accel_lp.last_timestamp_ns();
        let is_lp_init = self.accel_lp.is_initialized();

        self.accel_lp.add_sample(accel_sample, timestamp_ns);
        let smoothed_accel_delta = accel_sample - self.accel_lp.filtered();

        const ACCEL_DELTA_STATIC_THRESHOLD: f64 = 0.5;
        self.accel_static_counter.append_frame(
            smoothed_accel_delta.length() < ACCEL_DELTA_STATIC_THRESHOLD as f64,
        );

        // Cannot compute rotation from accel with only one sample.
        if !is_lp_init {
            self.sim_gyro_from_accel_lp
                .add_sample(DVec3::ZERO, timestamp_ns);
            return;
        }

        if !self.accel_static_counter.is_recently_static() {
            return;
        }

        self.median_filter
            .add_sample(self.accel_lp.filtered());

        if !self.median_filter.is_valid() {
            self.mean_filter
                .add_sample(self.accel_lp.filtered());
            self.last_mean_filtered_accel = self.accel_lp.filtered();
            return;
        }

        self.mean_filter
            .add_sample(self.median_filter.filtered());

        let diff_ns = (timestamp_ns as i64 - previous_accel_timestamp_ns as i64) as f64;
        let mock_gyro = self.compute_angular_velocity_from_latest_accel(diff_ns);
        self.sim_gyro_from_accel_lp
            .add_sample(mock_gyro, timestamp_ns);

        self.last_mean_filtered_accel = self.mean_filter.filtered();
    }

    fn compute_angular_velocity_from_latest_accel(&self, timestep_ns: f64) -> DVec3 {
        const MIN_TIMESTEP: f64 = 1.0; // ns
        if timestep_ns < MIN_TIMESTEP {
            return DVec3::ZERO;
        }

        let mean_of_median = self.mean_filter.filtered();

        let from = dvec3(
            self.last_mean_filtered_accel[0],
            self.last_mean_filtered_accel[1],
            self.last_mean_filtered_accel[2],
        );
        let to = dvec3(mean_of_median[0], mean_of_median[1], mean_of_median[2]);

        let incremental_rotation = DQuat::from_rotation_arc(from.normalize(), to.normalize());
        let (axis, angle) = incremental_rotation.to_axis_angle();

        let angular_velocity = axis * (angle / timestep_ns);
        dvec3(
            angular_velocity[0] as f64,
            angular_velocity[1] as f64,
            angular_velocity[2] as f64,
        )
    }

    fn update_gyro_bias(&mut self, gyro_sample: DVec3, timestamp_ns: u64) -> bool {
        const GYRO_FOR_BIAS_THRESHOLD: f64 = 0.30;

        let gyro_norm = gyro_sample.length();
        if gyro_norm >= GYRO_FOR_BIAS_THRESHOLD {
            return false;
        }

        let mut update_weight = 1.0f64 - gyro_norm / GYRO_FOR_BIAS_THRESHOLD;
        update_weight *= update_weight;

        self.gyro_bias_lp.add_weighted_sample(
            self.gyro_lp.filtered(),
            timestamp_ns,
            update_weight as f64,
        );

        self.current_accumulated_weights_gyro_bias += update_weight;
        true
    }

    pub fn gyro_bias(&self) -> DVec3 {
        self.gyro_bias_lp.filtered()
    }

    pub fn is_current_estimate_valid(&self) -> bool {
        const RATIO_GYRO_BIAS_ACCEL: f64 = 1.5;
        const MIN_SUM_WEIGHTS_GYRO_BIAS: f64 = 25.0;

        let current_gravity_dir = self.last_mean_filtered_accel.normalize();
        let gyro_bias = self.gyro_bias_lp.filtered();

        let off_gravity_gyro_bias =
            gyro_bias - current_gravity_dir * gyro_bias.dot(current_gravity_dir);

        let gyro_from_accel = self.sim_gyro_from_accel_lp.filtered();

        let correlated = gyro_from_accel.length() * RATIO_GYRO_BIAS_ACCEL as f64
            > off_gravity_gyro_bias.length() + 1e-8;

        let enough_samples =
            self.current_accumulated_weights_gyro_bias > MIN_SUM_WEIGHTS_GYRO_BIAS;

        let counters_static = self.gyro_static_counter.is_recently_static()
            && self.accel_static_counter.is_recently_static();

        enough_samples && counters_static && !correlated
    }
}
