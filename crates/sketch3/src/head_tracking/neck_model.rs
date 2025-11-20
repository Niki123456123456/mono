use glam::{DQuat, DVec3};



pub const DEFAULT_NECK_HORIZONTAL_OFFSET: f32 = -0.080; // meters in Z
pub const DEFAULT_NECK_VERTICAL_OFFSET: f32 = 0.075;    // meters in Y

/// Apply neck model to a given orientation quaternion (XYZW).
pub fn apply_neck_model(rotation: DQuat, factor: f64) -> DVec3 {
    let factor = factor.clamp(0.0, 1.0);

    let offset = rotation
        * DVec3::new(
            0.0,
            DEFAULT_NECK_VERTICAL_OFFSET as f64,
            DEFAULT_NECK_HORIZONTAL_OFFSET as f64,
        )
        - DVec3::new(0.0, DEFAULT_NECK_VERTICAL_OFFSET as f64, 0.0);

    let out = offset * factor;
    out
}
