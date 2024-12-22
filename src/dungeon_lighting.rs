use bevy::prelude::*;
use std::f32::consts::PI;
use crate::{GameMap, TileType, Player, GameState, setup};

// Lighting configuration constants
const AMBIENT_INTENSITY: f32 = 0.15;
const AMBIENT_COLOR: Color = Color::rgba(0.6, 0.7, 1.0, AMBIENT_INTENSITY);

const TORCH_BASE_INTENSITY: f32 = 0.5;
const TORCH_COLOR: Color = Color::rgba(1.0, 0.6, 0.2, 1.0);
const TORCH_RANGE: f32 = 8.0;

const PLAYER_LIGHT_BASE_INTENSITY: f32 = 0.3;
const PLAYER_LIGHT_MAX_INTENSITY: f32 = 0.7;
const PLAYER_LIGHT_COLOR: Color = Color::rgba(1.0, 1.0, 0.9, 1.0);

// Torch flickering parameters
const TORCH_FLICKER_SPEED: f32 = 5.0;
const TORCH_FLICKER_INTENSITY: f32 = 0.1;

/// Represents different types of lights in the dungeon
#[derive(Component)]
enum DungeonLightType {
    Ambient,
    Torch,
    SpotLight,
}

#[derive(Component)]
struct PlayerLight;

/// Plugin for managing dynamic dungeon lighting
pub struct DungeonLightingPlugin;

impl Plugin for DungeonLightingPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::InGame),
                         setup_ambient_lighting)
            .add_systems(OnEnter(GameState::InGame),
                         setup_player_light
                             .after(setup))
            //.add_systems(PostStartup, place_torch_lights)
            .add_systems(Update, (
                torch_flickering,
                dynamic_light_intensity).run_if(in_state(GameState::InGame)));
    }
}

/// Set up base ambient lighting with a low, blue-tinted intensity
fn setup_ambient_lighting(mut commands: Commands) {
    // Global ambient light with a soft, blue-gray tone
    commands.insert_resource(AmbientLight {
        color: AMBIENT_COLOR,
        brightness: AMBIENT_INTENSITY,
    });

    // Add a subtle global directional light for depth
    commands.spawn((
        DirectionalLight {
            illuminance: 500.0,
            color: Color::srgba(0.8, 0.8, 1.0, 0.2),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-2.0, 10.0, -2.0).looking_at(Vec3::ZERO, Vec3::Y)
    ));
}

fn setup_player_light(
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
) {
    if let Ok(player_entity) = player_query.get_single() {
        commands.entity(player_entity).with_children(|parent| {
            parent.spawn((
                             SpotLight {
                                     intensity: 240_000.0, // lumens
                                     color: Color::WHITE,
                                     shadows_enabled: false,
                                     radius: 20.0,
                                     range: 200.0,
                                     inner_angle: PI / 2.0 * 0.85,
                                     outer_angle: PI / 2.0,
                                     ..default()
                                 },
                         Transform::from_xyz(0.0, 4.0, 0.0)
                                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::X),
                             PlayerLight
            )).insert(Name::new("PlayerLight"));
        });
    }
}

/// Strategically place torch lights throughout the dungeon
pub fn place_torch_lights(
    mut commands: Commands,
    game_map: Res<GameMap>,
) {
    // Iterate through the game map to place torches near walls
    for y in 0..game_map.height {
        for x in 0..game_map.width {
            // Place torches near wall edges with some randomness
            if game_map.grid[(x,y)].tile_type == TileType::Wall {
                // Check adjacent tiles to ensure we're near a floor tile
                let adjacent_floor = [
                    (x as i32 - 1, y as i32),
                    (x as i32 + 1, y as i32),
                    (x as i32, y as i32 - 1),
                    (x as i32, y as i32 + 1),
                ];

                for (adj_x, adj_y) in adjacent_floor.iter() {
                    if *adj_x >= 0 && *adj_x < game_map.width as i32 &&
                        *adj_y >= 0 && *adj_y < game_map.height as i32 {
                        let adj_tile = &game_map.grid[(*adj_x as usize, *adj_y as usize)].tile_type;
                        if *adj_tile == TileType::Floor && rand::random::<f32>() < 0.3 {
                            let position = game_map.grid_to_world(*adj_x as usize, *adj_y as usize);

                            commands.spawn((
                                 PointLight {
                                        intensity: TORCH_BASE_INTENSITY,
                                        range: TORCH_RANGE,
                                        color: TORCH_COLOR,
                                        shadows_enabled: true,
                                        ..default()
                                    },
                                 Transform::from_translation(position + Vec3::Y * 2.0),
                                DungeonLightType::Torch,
                            ));
                        }
                    }
                }
            }
        }
    }
}


/// Create a realistic torch flickering effect
fn torch_flickering(
    time: Res<Time>,
    mut torch_lights: Query<&mut PointLight, With<DungeonLightType>>,
) {
    for mut light in torch_lights.iter_mut() {
        // Sine wave to create natural flickering
        let flicker = (time.elapsed_secs() * TORCH_FLICKER_SPEED).sin() * TORCH_FLICKER_INTENSITY;
        light.intensity = TORCH_BASE_INTENSITY + flicker;
    }
}

/// Adjust light intensity based on player movement and proximity
fn dynamic_light_intensity(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut lights: Query<(&mut PointLight, &Transform), With<DungeonLightType>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        for (mut light, light_transform) in lights.iter_mut() {
            // Calculate distance between light and player
            let distance = player_transform.translation.distance(light_transform.translation);

            // Adjust light intensity based on distance
            let base_intensity = match light.color {
                _ if light.color == TORCH_COLOR => TORCH_BASE_INTENSITY,
                _ if light.color == PLAYER_LIGHT_COLOR => PLAYER_LIGHT_BASE_INTENSITY,
                _ => 0.2
            };

            // Soft distance-based intensity falloff
            let intensity_factor = 1.0 - (distance / 20.0).min(1.0);
            light.intensity = base_intensity * intensity_factor;
        }
    }
}