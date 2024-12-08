use bevy::prelude::*;
use bevy::render::view::RenderLayers;

#[derive(Component, Default, Clone)]
pub enum CameraMode {
    #[default]
    YawPitch,
    Orbit,
}

#[derive(Component)]
pub struct ThirdPersonCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target_distance: f32,
    settings: CameraSettings,
    mode: CameraMode,
}

#[derive(Clone)]
pub struct CameraSettings {
    min_zoom: f32,
    max_zoom: f32,
    zoom_speed: f32,
    smoothing_factor: f32,
    height_offset: f32,
    min_pitch: f32,
    max_pitch: f32,
    rotate_left_key: KeyCode,
    rotate_right_key: KeyCode,
    look_up_key: KeyCode,
    look_down_key: KeyCode
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            min_zoom: 2.0,
            max_zoom: 10.0,
            zoom_speed: 0.5,
            smoothing_factor: 0.5,
            height_offset: 10.0,
            min_pitch: -1.5,
            max_pitch: 1.5,
            rotate_left_key: KeyCode::KeyA,
            rotate_right_key: KeyCode::KeyD,
            look_up_key: KeyCode::KeyW,
            look_down_key: KeyCode::KeyS
        }
    }
}

impl ThirdPersonCamera {
    pub fn new(initial_distance: f32) -> Self {
        let settings = CameraSettings::default();
        Self {
            yaw: 0.0,
            pitch: 0.3,
            distance: initial_distance,
            target_distance: initial_distance,
            settings,
            mode: CameraMode::Orbit,
        }
    }

    pub fn with_settings(mut self, settings: CameraSettings) -> Self {
        self.settings = settings;
        self
    }
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self::new(5.0)
    }
}

pub struct ThirdPersonCameraPlugin;

impl Plugin for ThirdPersonCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_camera)
            .add_systems(Update, (
                update_camera_rotation_keyboard,
                update_camera_position
            ));
    }
}

fn setup_camera(
    mut commands: Commands,
    player_query: Query<&Transform, With<crate::Player>>
) {
    let player_transform = player_query.single();

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(
                player_transform.translation.x,
                player_transform.translation.y + 1.0,
                player_transform.translation.z + 10.0
            ).looking_at(player_transform.translation, Vec3::Y),
            ..default()
        },
        RenderLayers::layer(0),
        ThirdPersonCamera::new(10.0),
    ));
}

fn update_camera_rotation_keyboard(
    mut query: Query<&mut ThirdPersonCamera>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut camera = query.single_mut();
    let rotation_speed = 2.0;

    match camera.mode {
        CameraMode::YawPitch => {
            if keyboard_input.pressed(camera.settings.rotate_left_key) {
                camera.yaw += rotation_speed * time.delta_seconds();
            }
            if keyboard_input.pressed(camera.settings.rotate_right_key) {
                camera.yaw -= rotation_speed * time.delta_seconds();
            }
            if keyboard_input.pressed(camera.settings.look_up_key) {
                camera.pitch -= rotation_speed * time.delta_seconds();
            }
            if keyboard_input.pressed(camera.settings.look_down_key) {
                camera.pitch += rotation_speed * time.delta_seconds();
            }
        }
        CameraMode::Orbit => {
            if keyboard_input.pressed(camera.settings.rotate_left_key) {
                camera.yaw -= rotation_speed * time.delta_seconds();
            }
            if keyboard_input.pressed(camera.settings.rotate_right_key) {
                camera.yaw += rotation_speed * time.delta_seconds();
            }
            if keyboard_input.pressed(camera.settings.look_up_key) {
                camera.settings.height_offset += rotation_speed * time.delta_seconds() * 5.0;
            }
            if keyboard_input.pressed(camera.settings.look_down_key) {
                camera.settings.height_offset -= rotation_speed * time.delta_seconds() * 5.0;
            }
        }
    }

    // Pitch limit YawPitch Modus
    if matches!(camera.mode, CameraMode::YawPitch) {
        camera.pitch = camera.pitch.clamp(
            camera.settings.min_pitch,
            camera.settings.max_pitch
        );
    }
}

fn update_camera_position(
    mut query: Query<(&ThirdPersonCamera, &mut Transform)>,
    player_query: Query<&Transform, (With<crate::Player>, Without<ThirdPersonCamera>)>,
    time: Res<Time>,
) {
    let player_transform = match player_query.get_single() {
        Ok(transform) => transform,
        Err(_) => return,
    };

    let (camera, mut camera_transform) = query.single_mut();
    let target_pos = player_transform.translation;

    let offset = match camera.mode {
        CameraMode::YawPitch => Vec3::new(
            camera.distance * camera.yaw.cos() * camera.pitch.cos(),
            camera.distance * camera.pitch.sin() + camera.settings.height_offset,
            camera.distance * camera.yaw.sin() * camera.pitch.cos()
        ),
        CameraMode::Orbit => Vec3::new(
            camera.distance * camera.yaw.cos(),
            camera.settings.height_offset,
            camera.distance * camera.yaw.sin()
        ),
    };

    let current_pos = camera_transform.translation;
    let new_pos = current_pos.lerp(
        target_pos + offset,
        camera.settings.smoothing_factor * time.delta_seconds()
    );

    camera_transform.translation = new_pos;
    camera_transform.look_at(target_pos, Vec3::Y);
}