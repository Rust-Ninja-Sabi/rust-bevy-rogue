use bevy::prelude::*;
use crate::{GameMap, Player, Monster, AttackEvent, TileType, GameState};

#[derive(Component, PartialEq, Debug, Clone, Copy)]
pub enum MonsterAIState {
    Idle,
    Pursuing,
    Attacking,
    Fading
}

pub struct MonsterAIPlugin;

impl Plugin for MonsterAIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                update_monster_ai,
                monster_movement
            ).chain().run_if(in_state(GameState::InGame)));
    }
}

const MONSTER_SPEED: f32 = 1.0;
const ATTACK_RANGE: f32 = 2.0;
const VISION_RANGE: f32 = 10.0;

fn update_monster_ai(
    commands: Commands,
    player_query: Query<&Transform, (With<Player>,Without<Monster>)>,
    mut monster_query: Query<(Entity, &Transform, &mut MonsterAIState), (With<Monster>,Without<Player>)>,
    game_map: Res<GameMap>
) {
    let player_transform = player_query.single();

    for (monster_entity, monster_transform, mut ai_state) in monster_query.iter_mut() {
        if *ai_state != MonsterAIState::Fading {
            let distance = monster_transform.translation.distance(player_transform.translation);

            // First, check if player is within vision range
            if distance <= VISION_RANGE {
                // Perform line of sight check
                if has_line_of_sight(&game_map, monster_transform.translation, player_transform.translation) {
                    *ai_state = if distance <= ATTACK_RANGE {
                        MonsterAIState::Attacking
                    } else {
                        MonsterAIState::Pursuing
                    };
                } else {
                    *ai_state = MonsterAIState::Idle;
                }
            } else {
                *ai_state = MonsterAIState::Idle;
            }
        }
    }
}

/// Checks if there is a clear line of sight between two points in the game map
fn has_line_of_sight(game_map: &GameMap, start: Vec3, end: Vec3) -> bool {
    const STEP_SIZE: f32 = 0.5; // Smaller step size for more precise checking
    let direction = (end - start).normalize();
    let total_distance = start.distance(end);

    let mut current_pos = start;

    while current_pos.distance(start) < total_distance {
        let grid_pos = game_map.world_to_grid(current_pos);

        // Check if the current grid position is a wall
        if game_map.grid[grid_pos].tile_type == TileType::Wall {
            return false;
        }

        // Move to next position along the line
        current_pos += direction * STEP_SIZE;
    }

    true
}

fn monster_movement(
    mut monster_query: Query<(Entity, &mut Transform, &MonsterAIState), With<Monster>>,
    player_query: Query<&Transform, (With<Player>,Without<Monster>)>,
    mut attack_events: EventWriter<AttackEvent>,
    game_map: Res<GameMap>,
    time: Res<Time>
) {
    let player_transform = player_query.single();

    for (monster_entity, mut monster_transform, ai_state) in monster_query.iter_mut() {
        match ai_state {
            MonsterAIState::Attacking => {
                // Send attack event if close enough
                attack_events.send(AttackEvent {
                    attacker: monster_entity,
                    direction: (player_transform.translation - monster_transform.translation).normalize()
                });
            },
            MonsterAIState::Pursuing => {
                // Move towards player
                let direction = (player_transform.translation - monster_transform.translation).normalize();
                let movement = direction * MONSTER_SPEED * time.delta_secs();

                let new_position = monster_without_colliding(
                    &game_map,
                    monster_transform.translation,
                    movement
                );

                monster_transform.translation = new_position;
                monster_transform.look_at(player_transform.translation, Vec3::Y);
            },
            MonsterAIState::Idle => {
                // Optionally add idle wandering behavior
            },
            MonsterAIState::Fading => {}
        }
    }
}

fn monster_without_colliding(
    game_map: &GameMap,
    position: Vec3,
    move_vector: Vec3
) -> Vec3 {
    const MONSTER_DISTANCE: f32 = 0.5;
    let new_position = position + move_vector;

    // Check collision points around the monster
    let check_points = [
        Vec3::new(0.0, 0.0, -MONSTER_DISTANCE),
        Vec3::new(0.0, 0.0, MONSTER_DISTANCE),
        Vec3::new(-MONSTER_DISTANCE, 0.0, 0.0),
        Vec3::new(MONSTER_DISTANCE, 0.0, 0.0)
    ];

    for point in check_points.iter() {
        let check_pos = *point + new_position;
        let map_pos = game_map.world_to_grid(check_pos);

        // Check tile type and reject movement if it's a wall
        if game_map.grid[map_pos].tile_type == crate::TileType::Wall {
            return position;
        }
    }

    new_position
}