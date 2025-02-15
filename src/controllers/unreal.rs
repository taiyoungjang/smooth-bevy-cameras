use crate::{LookAngles, LookTransform, LookTransformBundle, Smoother};

use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    input::{
        mouse::{MouseMotion, MouseWheel},
        prelude::*,
    },
    math::prelude::*,
    transform::components::Transform,
};
use bevy::math::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct UnrealCameraPlugin {
    pub override_input_system: bool,
}

impl UnrealCameraPlugin {
    pub fn new(override_input_system: bool) -> Self {
        Self {
            override_input_system,
        }
    }
}

impl Plugin for UnrealCameraPlugin {
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
pub struct UnrealCameraBundle {
    controller: UnrealCameraController,
    //#[bundle]
    look_transform: LookTransformBundle,
    transform: Transform,
}

impl UnrealCameraBundle {
    pub fn new(controller: UnrealCameraController, eye: DVec3, target: DVec3) -> Self {
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

/// A camera controlled with the mouse in the same way as Unreal Engine's viewport controller.
#[derive(Clone, Component, Copy, Debug, Deserialize, Serialize)]
pub struct UnrealCameraController {
    /// Whether to process input or ignore it
    pub enabled: bool,

    /// How many radians per frame for each rotation axis (yaw, pitch) when rotating with the mouse
    pub rotate_sensitivity: DVec2,

    /// How many units per frame for each direction when translating using Middle or L+R panning
    pub mouse_translate_sensitivity: DVec2,

    /// How many units per frame when translating using scroll wheel
    pub wheel_translate_sensitivity: f64,

    /// How many units per frame when translating using W/S/Q/E
    /// Updated with scroll wheel while dragging with any mouse button
    pub keyboard_mvmt_sensitivity: f64,

    /// Wheel sensitivity for modulating keyboard movement speed
    pub keyboard_mvmt_wheel_sensitivity: f64,

    /// The greater, the slower to follow input
    pub smoothing_weight: f64,
}

impl Default for UnrealCameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            rotate_sensitivity: DVec2::splat(0.002),
            mouse_translate_sensitivity: DVec2::splat(0.02),
            wheel_translate_sensitivity: 1.0,
            keyboard_mvmt_sensitivity: 0.1,
            keyboard_mvmt_wheel_sensitivity: 0.1,
            smoothing_weight: 0.7,
        }
    }
}

pub enum ControlEvent {
    Locomotion(DVec2),
    Rotate(DVec2),
    TranslateEye(DVec2),
}

define_on_controller_enabled_changed!(UnrealCameraController);

pub fn default_input_map(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    keyboard: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut controllers: Query<&mut UnrealCameraController>,
) {
    // Can only control one camera at a time.
    let mut controller = if let Some(controller) = controllers.iter_mut().find(|c| c.enabled) {
        controller
    } else {
        return;
    };
    let UnrealCameraController {
        rotate_sensitivity: mouse_rotate_sensitivity,
        mouse_translate_sensitivity,
        wheel_translate_sensitivity,
        mut keyboard_mvmt_sensitivity,
        keyboard_mvmt_wheel_sensitivity,
        ..
    } = *controller;

    let left_pressed = mouse_buttons.pressed(MouseButton::Left);
    let right_pressed = mouse_buttons.pressed(MouseButton::Right);
    let middle_pressed = mouse_buttons.pressed(MouseButton::Middle);

    let mut cursor_delta = DVec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += DVec2::new(event.delta.x as f64, event.delta.y as f64);
    }

    let mut wheel_delta = 0.0f64;
    for event in mouse_wheel_reader.iter() {
        wheel_delta += event.x as f64 + event.y as f64;
    }

    let mut panning_dir = DVec2::ZERO;
    let mut translation_dir = DVec2::ZERO; // y is forward/backward axis, x is rotation around Z

    for key in keyboard.get_pressed() {
        match key {
            KeyCode::E => {
                panning_dir.y += 1.0;
            }

            KeyCode::Q => {
                panning_dir.y -= 1.0;
            }

            KeyCode::A => {
                panning_dir.x -= 1.0;
            }

            KeyCode::D => {
                panning_dir.x += 1.0;
            }

            KeyCode::S => {
                translation_dir.y -= 1.0;
            }

            KeyCode::W => {
                translation_dir.y += 1.0;
            }

            _ => {}
        }
    }

    let mut panning = DVec2::ZERO;
    let mut locomotion = DVec2::ZERO;

    // If any of the mouse button are pressed; read additional signals from the keyboard for panning
    // and locomotion along camera view axis
    if left_pressed || middle_pressed || right_pressed {
        panning += keyboard_mvmt_sensitivity * panning_dir;

        if translation_dir.y != 0.0 {
            locomotion.y += keyboard_mvmt_sensitivity * translation_dir.y;
        }

        keyboard_mvmt_sensitivity += keyboard_mvmt_wheel_sensitivity * wheel_delta;
        controller.keyboard_mvmt_sensitivity = keyboard_mvmt_sensitivity.max(0.01);
    }
    // Otherwise, if any scrolling is happening, do locomotion along camera view axis
    else if wheel_delta != 0.0 {
        locomotion.y += wheel_translate_sensitivity * wheel_delta;
    }

    // You can also pan using the mouse only; add those signals to existing panning
    if middle_pressed || (left_pressed && right_pressed) {
        panning += mouse_translate_sensitivity * cursor_delta;
    }

    // When left only is pressed, mouse movements add up to the "unreal locomotion" scheme
    if left_pressed && !middle_pressed && !right_pressed {
        locomotion.x = mouse_rotate_sensitivity.x * cursor_delta.x;
        locomotion.y -= mouse_translate_sensitivity.y * cursor_delta.y;
    }

    if !left_pressed && !middle_pressed && right_pressed {
        events.send(ControlEvent::Rotate(
            mouse_rotate_sensitivity * cursor_delta,
        ));
    }

    if panning.length_squared() > 0.0 {
        events.send(ControlEvent::TranslateEye(panning));
    }

    if locomotion.length_squared() > 0.0 {
        events.send(ControlEvent::Locomotion(locomotion));
    }
}

pub fn control_system(
    mut events: EventReader<ControlEvent>,
    mut cameras: Query<(&UnrealCameraController, &mut LookTransform)>,
) {
    // Can only control one camera at a time.
    let mut transform = if let Some((_, transform)) = cameras.iter_mut().find(|c| c.0.enabled) {
        transform
    } else {
        return;
    };

    let look_vector;
    match transform.look_direction() {
        Some(safe_look_vector) => look_vector = safe_look_vector,
        None => return,
    }
    let mut look_angles = LookAngles::from_vector(look_vector);

    for event in events.iter() {
        match event {
            ControlEvent::Locomotion(delta) => {
                // Translates forward/backward and rotates about the Y axis.
                look_angles.add_yaw(-delta.x);
                transform.eye += delta.y * look_vector;
            }
            ControlEvent::Rotate(delta) => {
                // Rotates with pitch and yaw.
                look_angles.add_yaw(-delta.x);
                look_angles.add_pitch(-delta.y);
            }
            ControlEvent::TranslateEye(delta) => {
                let yaw_rot = DQuat::from_axis_angle(DVec3::Y, look_angles.get_yaw());
                let rot_x = yaw_rot * DVec3::X;

                // Translates up/down (Y) and left/right (X).
                transform.eye -= delta.x * rot_x - DVec3::new(0.0, delta.y, 0.0);
            }
        }
    }

    look_angles.assert_not_looking_up();

    transform.target = transform.eye + transform.radius() * look_angles.unit_vector();
}
