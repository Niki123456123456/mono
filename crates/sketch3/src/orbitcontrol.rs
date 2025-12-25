use three_d::*;

pub fn handle_events(
    camera: &mut Camera,
    ctx: &egui::Context,
    target: Vec3,
    min_distance: f32,
    max_distance: f32,
) {
    let mut pointer_down = false;
    let mut delta = egui::Vec2::ZERO;
    let mut zoom_delta = 0.;
    ctx.input(|i| {
        zoom_delta = i.smooth_scroll_delta.y;
        pointer_down = i.pointer.primary_down();
        delta = i.pointer.delta();
    });

    if zoom_delta != 0. {
        let speed = 0.01 * (target.distance(camera.position) - min_distance) + 0.001;
        camera.zoom_towards(target, speed * zoom_delta, min_distance, max_distance);
    }

    if pointer_down {
        let delta = delta;
        let speed = -1.
            * ((target.distance(camera.position) - min_distance) / (max_distance - min_distance));
        camera.rotate_around_with_fixed_up(target, speed * delta.x, speed * delta.y);
    }
}
