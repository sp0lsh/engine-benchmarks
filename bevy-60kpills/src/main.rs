// build: cargo run --release
// run: ./target/release/bevy-demo

use bevy::asset::AssetPlugin;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_obj::ObjPlugin;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Component)]
struct Spin {
    radians_per_sec: f32,
}

#[derive(Component)]
struct FpsText;

const NUM_PILLS: usize = 60 * 1000;

fn main() {
    let asset_folder = asset_dir_from_exe();
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: asset_folder,
                ..default()
            }),
            ObjPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (spin_system, update_fps_text))
        .run();
}

fn asset_dir_from_exe() -> String {
    use std::path::PathBuf;
    let exe = std::env::current_exe().ok();
    if let Some(exe) = exe {
        let assets: PathBuf = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|root| root.join("assets"))
            .unwrap_or_else(|| PathBuf::from("assets"));
        return assets.to_string_lossy().into_owned();
    }
    "assets".to_string()
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera (Bevy 0.17 uses components instead of bundles)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // UI camera for rendering UI text, render after 3D
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..Default::default()
        },
    ));

    // UI: FPS (top-left)
    commands
        .spawn((
            Text::new(format!("Bevy 0.17, {} pills, FPS: ", NUM_PILLS)),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            FpsText,
        ));

    // Load mesh + texture
    let pill_mesh: Handle<Mesh> = assets.load("models/pill.obj");
    let base_tex: Handle<Image> = assets.load("textures/pill_color.png");
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(base_tex),
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    // Spawn 1000 instances
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..NUM_PILLS {
        let pos = Vec3::new(
            rng.gen_range(-10.0..10.0),
            rng.gen_range(-10.0..10.0),
            rng.gen_range(-18.0..-4.0),
        );
        let rot_x = rng.gen_range(-std::f32::consts::PI..std::f32::consts::PI);
        commands.spawn((
            Mesh3d(pill_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_x(rot_x)),
            Spin {
                radians_per_sec: std::f32::consts::FRAC_PI_2,
            },
        ));
    }
}

fn spin_system(time: Res<Time>, mut q: Query<(&Spin, &mut Transform)>) {
    for (spin, mut tf) in &mut q {
        tf.rotate_y(spin.radians_per_sec * time.delta_secs());
    }
}

fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut q: Query<&mut TextSpan, With<FpsText>>) {
    if let Some(fps_diag) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps_diag.smoothed() {
            for mut span in &mut q {
                **span = format!("{value:.0}");
            }
        }
    }
}
