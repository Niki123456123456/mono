use std::collections::VecDeque;

use glam::{DVec3};

pub struct MeanFilter {
    filter_size: usize,
    buffer: VecDeque<DVec3>,
}

impl MeanFilter {
    pub fn new(filter_size: usize) -> Self {
        Self {
            filter_size,
            buffer: VecDeque::with_capacity(filter_size),
        }
    }

    pub fn add_sample(&mut self, sample: DVec3) {
        self.buffer.push_back(sample);
        if self.buffer.len() > self.filter_size {
            self.buffer.pop_front();
        }
    }

    pub fn is_valid(&self) -> bool {
        self.buffer.len() == self.filter_size
    }

    pub fn filtered(&self) -> DVec3 {
        if self.buffer.is_empty() {
            return DVec3::ZERO;
        }
        let mut sum = DVec3::ZERO;
        for s in &self.buffer {
            sum += s;
        }
        sum / (self.filter_size as f64)
    }
}

pub struct MedianFilter {
    filter_size: usize,
    buffer: VecDeque<DVec3>,
    norms: VecDeque<f64>,
}

impl MedianFilter {
    pub fn new(filter_size: usize) -> Self {
        Self {
            filter_size,
            buffer: VecDeque::with_capacity(filter_size),
            norms: VecDeque::with_capacity(filter_size),
        }
    }

    pub fn add_sample(&mut self, sample: DVec3) {
        self.norms.push_back(sample.length());
        self.buffer.push_back(sample);
        if self.buffer.len() > self.filter_size {
            self.buffer.pop_front();
            self.norms.pop_front();
        }
    }

    pub fn is_valid(&self) -> bool {
        self.buffer.len() == self.filter_size
    }

    pub fn filtered(&self) -> DVec3 {
        if self.buffer.is_empty() {
            return DVec3::ZERO;
        }

        let mut norms_vec: Vec<f64> = self.norms.iter().copied().collect();
        norms_vec.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let median_norm = norms_vec[self.filter_size / 2];
        // Find first sample whose stored norm matches the median norm.
        let mut it = self.buffer.iter();
        for n in &self.norms {
            let s = it.next().unwrap();
            if (*n - median_norm).abs() < 1e-6 {
                return *s;
            }
        }

        // Fallback.
        *self.buffer.back().unwrap()
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.norms.clear();
    }
}

pub struct LowpassFilter {
    cutoff_time_constant: f64,
    last_timestamp_ns: u64,
    initialized: bool,
    filtered_data: DVec3,
}

impl LowpassFilter {
    pub fn new(cutoff_freq_hz: f64) -> Self {
        let cutoff_time_constant = 1.0 / (2.0 * std::f64::consts::PI * cutoff_freq_hz);
        let mut s = Self {
            cutoff_time_constant,
            last_timestamp_ns: 0,
            initialized: false,
            filtered_data: DVec3::ZERO,
        };
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        self.initialized = false;
        self.filtered_data = DVec3::ZERO;
        self.last_timestamp_ns = 0;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn filtered(&self) -> DVec3 {
        self.filtered_data
    }

    pub fn last_timestamp_ns(&self) -> u64 {
        self.last_timestamp_ns
    }

    pub fn add_sample(&mut self, sample: DVec3, timestamp_ns: u64) {
        self.add_weighted_sample(sample, timestamp_ns, 1.0);
    }

    pub fn add_weighted_sample(&mut self, sample: DVec3, timestamp_ns: u64, weight: f64) {
        const SECONDS_FROM_NS: f64 = 1e-9;
        const MIN_TIMESTEP_S: f64 = 0.001; // 1000 Hz
        const MAX_TIMESTEP_S: f64 = 1.0;   // 1 Hz

        if !self.initialized {
            self.filtered_data = sample;
            self.last_timestamp_ns = timestamp_ns;
            self.initialized = true;
            return;
        }

        if timestamp_ns < self.last_timestamp_ns {
            self.last_timestamp_ns = timestamp_ns;
            return;
        }

        let delta_s =
            (timestamp_ns - self.last_timestamp_ns) as f64 * SECONDS_FROM_NS;
        if delta_s <= MIN_TIMESTEP_S || delta_s > MAX_TIMESTEP_S {
            self.last_timestamp_ns = timestamp_ns;
            return;
        }

        let weighted_delta_secs = weight * delta_s;
        let alpha =
            weighted_delta_secs / (self.cutoff_time_constant + weighted_delta_secs);

        self.filtered_data =
            (1.0 - alpha) * self.filtered_data + alpha * sample;
        self.last_timestamp_ns = timestamp_ns;
    }
}
