use bevy::prelude::*;
use smooth_bevy_cameras::{LookTransform, LookTransformBundle, LookTransformPlugin, Smoother};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(LookTransformPlugin)
        .add_startup_system(setup)
        .run();
}

/// setup a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });

    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    commands
        .spawn(LookTransformBundle {
            transform: LookTransform {
                eye: DVec3::new(-2.0, 2.5, 5.0),
                target: DVec3::new(0.0, 0.5, 0.0),
            },
            smoother: Smoother::new(0.9),
        })
        .insert(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0)
                .looking_at(DVec3::new(0.0, 0.5, 0.0), DVec3::Y),
            ..default()
        });
}
