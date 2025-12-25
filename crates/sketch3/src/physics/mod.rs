use rapier3d::control::{DynamicRayCastVehicleController, WheelTuning};
use rapier3d::parry::query::DefaultQueryDispatcher;
use rapier3d::prelude::*;

pub struct Engine {
    vehicle: DynamicRayCastVehicleController,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    impulse_joint_set: ImpulseJointSet,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    integration_params: IntegrationParameters,
}

impl Engine {
    pub fn new() -> Self {
        let integration_params: IntegrationParameters = IntegrationParameters::default();
        let mut physics_pipeline: PhysicsPipeline = PhysicsPipeline::new();
        let mut island_manager: IslandManager = IslandManager::new();
        let mut rigid_body_set: RigidBodySet = RigidBodySet::new();
        let mut collider_set: ColliderSet = ColliderSet::new();
        let mut impulse_joint_set: ImpulseJointSet = ImpulseJointSet::new();
        let mut multibody_joint_set: MultibodyJointSet = MultibodyJointSet::new();
        let mut ccd_solver: CCDSolver = CCDSolver::new();
        let mut broad_phase: BroadPhaseBvh = DefaultBroadPhase::new();
        let mut narrow_phase: NarrowPhase = NarrowPhase::new();

        let plane_handle = rigid_body_set.insert(RigidBodyBuilder::fixed()
            .translation(vector![0.0, -0.1, 0.0])
            .build());
        collider_set.insert_with_parent(ColliderBuilder::cuboid(100., 0.1, 100.).build(), plane_handle, &mut rigid_body_set);



        let car_handle = rigid_body_set.insert(RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 2.0, 0.0])
            .build());
        collider_set.insert_with_parent(ColliderBuilder::cuboid(1.0, 0.25, 2.0).build(), car_handle, &mut rigid_body_set);

        


        let mut vehicle: DynamicRayCastVehicleController =
            DynamicRayCastVehicleController::new(car_handle);

        // front_x = 0.85 y_radius = 0.70 half_width_z = 0.6
        // back_x  = 1.03 y_radius = 0.54 half_width_z = 0.6

        let half_width_z = 0.6;
        let half_height = 0.15;
        let wheel_positions = [
            (point![0.85, -1. + 0.35, half_width_z], 0.70),
            (point![0.85, -1. + 0.35, -half_width_z], 0.70),
            (point![-1.03, -1. + 0.27, half_width_z], 0.54),
            (point![-1.03, -1. + 0.27, -half_width_z], 0.54),
        ];

        let tuning = WheelTuning {
            suspension_stiffness: 100.0,
            suspension_damping: 10.0,
            ..WheelTuning::default()
        };
        for (pos, radius) in wheel_positions {
            vehicle.add_wheel(pos, -Vector::y(), Vector::z(), 0.1, radius, &tuning);
        }

        Self {
            vehicle,
            physics_pipeline,
            island_manager,
            impulse_joint_set,
            rigid_body_set,
            collider_set,
            multibody_joint_set,
            ccd_solver,
            broad_phase,
            narrow_phase,
            integration_params,
        }
    }

    pub fn update_vehicle_inputs(
        &mut self,
        steering_input: f32,
        engine_force_input: f32,
        brake_input: f32,
    ) {
        // We assume wheel indices:
        // 0: front-right, 1: front-left, 2: rear-right, 3: rear-left
        let wheels = self.vehicle.wheels_mut();

        for (i, wheel) in wheels.iter_mut().enumerate() {
            // Reset everything each frame
            wheel.brake = brake_input;

            if i >= 2 {
                // Rear wheels: they steer + get engine force
                wheel.steering = steering_input; // <-- lenken
                wheel.engine_force = engine_force_input;
            } else {
                // Front wheels: no steering, maybe no drive
                wheel.steering = 0.0;
                wheel.engine_force = 0.0;
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        let gravity = vector![0.0, -9.81, 0.0];
        let queries = self.broad_phase.as_query_pipeline_mut(
            &DefaultQueryDispatcher {},
            &mut self.rigid_body_set,
            &mut self.collider_set,
            QueryFilter::default(),
        );

        self.vehicle.update_vehicle(dt, queries);

        self.physics_pipeline.step(
            &gravity,
            &self.integration_params,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    pub fn vehicle_transform(&self) -> three_d::Mat4 {
        let car_body = &self.rigid_body_set[self.vehicle.chassis];
        let iso = car_body.position();
        use glam::{Mat4, Quat, Vec3};
        let translation = Vec3::new(iso.translation.x, iso.translation.y, iso.translation.z);

        let rotation = Quat::from_xyzw(
            iso.rotation.i,
            iso.rotation.j,
            iso.rotation.k,
            iso.rotation.w,
        );
        let model_matrix: Mat4 = Mat4::from_rotation_translation(rotation, translation);
        return crate::maps::glam_to_three_d(&model_matrix);
    }
}
