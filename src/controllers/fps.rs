use crate::{LookAngles, LookTransform, LookTransformBundle, Smoother};

use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    input::{mouse::MouseMotion, prelude::*},
    math::prelude::*,
    transform::components::Transform,
};
use bevy::math::{DVec2, DVec3};
//use bevy::reflect::TypeData;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct FpsCameraPlugin {
    pub override_input_system: bool,
}

impl FpsCameraPlugin {
    pub fn new(override_input_system: bool) -> Self {
        Self {
            override_input_system,
        }
    }
}

impl Plugin for FpsCameraPlugin {
    fn build(&self, app: &mut App) {
        let app = app
            .add_system_to_stage(CoreStage::PreUpdate, on_controller_enabled_changed)
            .add_system(control_system)
            .add_event::<ControlEvent>();

        if !self.override_input_system {
            app.add_system(default_input_map);
        }
    }
}

#[derive(Bundle)]
pub struct FpsCameraBundle {
    controller: FpsCameraController,
    //#[bundle]
    look_transform: LookTransformBundle,
    transform: Transform,
}

impl FpsCameraBundle {
    pub fn new(
        controller: FpsCameraController,
        eye: DVec3,
        target: DVec3,
    ) -> Self {
        // Make sure the transform is consistent with the controller to start.
        let transform = Transform::from_translation(eye).looking_at(target, DVec3::Y);

        Self {
            controller,
            look_transform: LookTransformBundle {
                transform: LookTransform::new(eye, target),
                smoother: Smoother::new(controller.smoothing_weight),
            },
            transform,
        }
    }
}

/// Your typical first-person camera controller.
#[derive(Clone, Component, Copy, Debug, Deserialize, Serialize)]
pub struct FpsCameraController {
    pub enabled: bool,
    pub mouse_rotate_sensitivity: DVec2,
    pub translate_sensitivity: f64,
    pub smoothing_weight: f64,
}

impl Default for FpsCameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            mouse_rotate_sensitivity: DVec2::splat(0.002),
            translate_sensitivity: 0.5,
            smoothing_weight: 0.9,
        }
    }
}

pub enum ControlEvent {
    Rotate(DVec2),
    TranslateEye(DVec3),
}

define_on_controller_enabled_changed!(FpsCameraController);

pub fn default_input_map(
    mut events: EventWriter<ControlEvent>,
    keyboard: Res<Input<KeyCode>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    controllers: Query<&FpsCameraController>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().find(|c| {
        c.enabled
    }) {
        controller
    } else {
        return;
    };
    let FpsCameraController {
        translate_sensitivity,
        mouse_rotate_sensitivity,
        ..
    } = *controller;

    let mut cursor_delta = DVec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += DVec2::new(event.delta.x as f64, event.delta.y as f64);
    }

    events.send(ControlEvent::Rotate(
        mouse_rotate_sensitivity * cursor_delta,
    ));

    for (key, dir) in [
        (KeyCode::W, DVec3::Z),
        (KeyCode::A, DVec3::X),
        (KeyCode::S, -DVec3::Z),
        (KeyCode::D, -DVec3::X),
        (KeyCode::LShift, -DVec3::Y),
        (KeyCode::Space, DVec3::Y),
    ]
    .iter()
    .cloned()
    {
        if keyboard.pressed(key) {
            events.send(ControlEvent::TranslateEye(translate_sensitivity * dir));
        }
    }
}

pub fn control_system(
    mut events: EventReader<ControlEvent>,
    mut cameras: Query<(&FpsCameraController, &mut LookTransform)>,
) {
    // Can only control one camera at a time.
    let mut transform =
        if let Some((_, transform)) = cameras.iter_mut().find(|c| {
            c.0.enabled
        }) {
            transform
        } else {
            return;
        };

        let look_vector = transform.look_direction().unwrap();
        let mut look_angles = LookAngles::from_vector(look_vector);

        let yaw_rot = DQuat::from_axis_angle(DVec3::Y, look_angles.get_yaw());
        let rot_x = yaw_rot * DVec3::X;
        let rot_y = yaw_rot * DVec3::Y;
        let rot_z = yaw_rot * DVec3::Z;

        for event in events.iter() {
            match event {
                ControlEvent::Rotate(delta) => {
                    // Rotates with pitch and yaw.
                    look_angles.add_yaw(-delta.x);
                    look_angles.add_pitch(-delta.y);
                }
                ControlEvent::TranslateEye(delta) => {
                    // Translates up/down (Y) left/right (X) and forward/back (Z).
                    transform.eye += delta.x * rot_x + delta.y * rot_y + delta.z * rot_z;
                }
            }
        }

        look_angles.assert_not_looking_up();

        transform.target = transform.eye + transform.radius() * look_angles.unit_vector();
}
