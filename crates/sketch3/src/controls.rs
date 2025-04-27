use three_d::*;

#[derive(Clone, Copy, Debug)]
pub struct OrbitControl {
    pub target: Vec3,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl OrbitControl {
    pub fn new(target: Vec3, min_distance: f32, max_distance: f32) -> Self {
        Self {
            target,
            min_distance,
            max_distance,
        }
    }

    pub fn handle_events(&mut self, camera: &mut Camera, ctx: &egui::Context) {
        let delta = ctx.input(|i| i.raw_scroll_delta.y);
        let speed = 0.001 * self.target.distance(camera.position()) * delta;
        camera.zoom_towards(self.target, speed, self.min_distance, self.max_distance);

        if ctx.input(|i| i.pointer.secondary_down()) {
            let delta = ctx.input(|i| i.pointer.delta());
            let speed = 0.01;
            camera.rotate_around_with_fixed_up(self.target, speed * delta.x, speed * delta.y);
        }
    }
}
