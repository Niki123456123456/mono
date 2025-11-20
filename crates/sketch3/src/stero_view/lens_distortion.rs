use glam::*;

use crate::stero_view::*;

pub struct LensDistortion {
    pub device_params: DeviceParams,
    pub eye_left: LensDistortionEye,
    pub eye_right: LensDistortionEye,
}
pub struct LensDistortionEye {
    pub head_matrix: Mat4,
    pub mesh: DistortionMesh,
    pub fov: [f32; 4],
}

pub struct ViewportParams {
    pub size: Vec2,
    pub eye_offset: Vec2,
}

pub struct DeviceParams {
    pub screen_to_lens_distance: f32,
    pub inter_lens_distance: f32,
    pub tray_to_lens_distance: f32,
    pub vertical_alignment: AlignmentType,
    pub left_eye_field_of_view_angles: Vec<f32>,
    pub distortion: PolynomialRadialDistortion,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Eye {
    Left = 0,
    Right = 1,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AlignmentType {
    BOTTOM = 0,
    CENTER = 1,
    TOP = 2,
}

impl LensDistortion {
    pub fn default(screen_size_pixels: Vec2) -> Self {
        let device_params = DeviceParams {
            screen_to_lens_distance: 0.0681647,
            inter_lens_distance: 0.0681293,
            tray_to_lens_distance: 0.035,
            vertical_alignment: AlignmentType::CENTER,
            left_eye_field_of_view_angles: vec![33.3798, 33.3798, 33.3798, 33.3798],
            distortion: PolynomialRadialDistortion{
                coefficients: vec![0.282412, 0.249853],
            },
        };
         Self::new(device_params, 460., vec2(screen_size_pixels.x / 2., screen_size_pixels.y))
         // 1905, 1012,    460.  2532., 1170.
    }

    pub fn new(device_params: DeviceParams, ppi: f32, screen_size_pixels: Vec2) -> Self {
        let screen_size_meters = (screen_size_pixels / ppi) * kMetersPerInch;

        let fov_left = CalculateFov(&device_params, screen_size_meters);
        let fov_right = [fov_left[1], fov_left[0], fov_left[2], fov_left[3]];

        Self {
            eye_left: LensDistortionEye {
                head_matrix: Mat4::from_translation(vec3(
                    device_params.inter_lens_distance * 0.5,
                    0.,
                    0.,
                )),
                mesh: CreateDistortionMesh(
                    Eye::Left,
                    &fov_left,
                    &device_params,
                    screen_size_meters,
                ),
                fov: fov_left,
            },
            eye_right: LensDistortionEye {
                head_matrix: Mat4::from_translation(vec3(
                    -device_params.inter_lens_distance * 0.5,
                    0.,
                    0.,
                )),
                mesh: CreateDistortionMesh(
                    Eye::Left,
                    &fov_left,
                    &device_params,
                    screen_size_meters,
                ),
                fov: fov_left,
            },
            device_params,
        }
    }
}

const kDefaultBorderSizeMeters: f32 = 0.003;
const kMetersPerInch: f32 = 0.0254;

pub fn CreateDistortionMesh(
    eye: Eye,
    fov: &[f32; 4],
    device_params: &DeviceParams,
    screen_size_meters: Vec2,
) -> DistortionMesh {
    let screen = ViewportParams {
        size: screen_size_meters / device_params.screen_to_lens_distance,
        eye_offset: vec2(
            if eye == Eye::Left {
                (screen_size_meters.x - device_params.inter_lens_distance / 2.)
                    / device_params.screen_to_lens_distance
            } else {
                (screen_size_meters.x + device_params.inter_lens_distance / 2.)
                    / device_params.screen_to_lens_distance
            },
            GetYEyeOffsetMeters(device_params, screen_size_meters.y)
                / device_params.screen_to_lens_distance,
        ),
    };

    println!("screen size: {:?}", screen.size);

    let texture = ViewportParams {
        size: vec2(fov[0].tan() + fov[1].tan(), fov[2].tan() + fov[3].tan()),
        eye_offset: vec2(fov[0].tan(), fov[2].tan()),
    };

    return DistortionMesh::new(&device_params.distortion, screen, texture, eye);
}

pub fn GetYEyeOffsetMeters(device_params: &DeviceParams, screen_height_meters: f32) -> f32 {
    match device_params.vertical_alignment {
        AlignmentType::BOTTOM => device_params.tray_to_lens_distance - kDefaultBorderSizeMeters,
        AlignmentType::CENTER => screen_height_meters / 2.0,
        AlignmentType::TOP => {
            screen_height_meters - device_params.tray_to_lens_distance - kDefaultBorderSizeMeters
        }
    }
}

pub fn CalculateFov(device_params: &DeviceParams, screen_size_meters: Vec2) -> [f32; 4] {
    let eye_to_screen_distance = device_params.screen_to_lens_distance;
    let outer_distance = (screen_size_meters.x - device_params.inter_lens_distance) / 2.0;
    let inner_distance = device_params.inter_lens_distance / 2.0;
    let bottom_distance = GetYEyeOffsetMeters(device_params, screen_size_meters.y);
    let top_distance = screen_size_meters.y - bottom_distance;

    let outer_angle = device_params
        .distortion
        .distort(vec2(outer_distance / eye_to_screen_distance, 0.))[0]
        .atan();
    let inner_angle = device_params
        .distortion
        .distort(vec2(inner_distance / eye_to_screen_distance, 0.))[0]
        .atan();
    let bottom_angle = device_params
        .distortion
        .distort(vec2(0., bottom_distance / eye_to_screen_distance))[1]
        .atan();
    let top_angle = device_params
        .distortion
        .distort(vec2(0., top_distance / eye_to_screen_distance))[1]
        .atan();

    // FOV angles in device parameters are in degrees so they are converted
    // to radians for posterior use.
    let device_fov = [
        degrees_to_radians(device_params.left_eye_field_of_view_angles[0]),
        degrees_to_radians(device_params.left_eye_field_of_view_angles[1]),
        degrees_to_radians(device_params.left_eye_field_of_view_angles[2]),
        degrees_to_radians(device_params.left_eye_field_of_view_angles[3]),
    ];
    return [
        outer_angle.min(device_fov[0]),
        inner_angle.min(device_fov[1]),
        bottom_angle.min(device_fov[2]),
        top_angle.min(device_fov[3]),
    ];
}

const fn degrees_to_radians(angle: f32) -> f32 {
    angle * std::f32::consts::PI / 180.0
}
