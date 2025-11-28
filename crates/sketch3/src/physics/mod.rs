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
    fn new() -> Self {
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

        let car_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 1.0, 0.0])
            .build();
        let car_handle = rigid_body_set.insert(car_body);

        let chassis_collider = ColliderBuilder::cuboid(1.0, 0.25, 2.0).build();
        collider_set.insert_with_parent(chassis_collider, car_handle, &mut rigid_body_set);

        let mut vehicle: DynamicRayCastVehicleController =
            DynamicRayCastVehicleController::new(car_handle);

        let hw = 0.3;
        let hh = 0.15;
        let wheel_positions = [
            point![hw * 1.5, -hh, hw],
            point![hw * 1.5, -hh, -hw],
            point![-hw * 1.5, -hh, hw],
            point![-hw * 1.5, -hh, -hw],
        ];

         let tuning = WheelTuning {
        suspension_stiffness: 100.0,
        suspension_damping: 10.0,
        ..WheelTuning::default()
    };
        for pos in wheel_positions {
            vehicle.add_wheel(pos, -Vector::y(), Vector::z(), hh, hh / 4.0, &tuning);
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

    fn update_vehicle_inputs(
        vehicle: &mut DynamicRayCastVehicleController,
        steering_input: f32,
        engine_force_input: f32,
        brake_input: f32,
    ) {
        // We assume wheel indices:
        // 0: front-right, 1: front-left, 2: rear-right, 3: rear-left
        let wheels = vehicle.wheels_mut();

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

    fn step(&mut self, dt: f32) {
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
}
