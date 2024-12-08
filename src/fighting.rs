use std::f32::consts::FRAC_PI_2;
use bevy::prelude::*;
use std::time::Duration;
use bevy::color::palettes::css::{GRAY, LIGHT_GRAY, LIGHT_GREEN, RED};
use crate::{AttackTimer, Player, Monster, RightArm};
use crate::chracter_controller::MonsterAIState;
use crate::third_person_camera::ThirdPersonCamera;

#[derive(Component, Debug, Clone)]
pub struct Actor {
    pub max_hit_points: usize,
    pub hit_points: usize,
    pub defense: usize,
    pub power: usize
}

impl Actor {
    pub fn new(max_hit_points: usize, defense: usize, power: usize) -> Self {
        Self {
            max_hit_points,
            hit_points: max_hit_points,
            defense,
            power
        }
    }
}

#[derive(Component)]
struct Fading {
    fade_duration: Timer,
}

impl Fading {
    fn new() -> Self {
        Self {
            fade_duration: Timer::from_seconds(2.0, TimerMode::Once),
        }
    }
}


const HEALTHBAR_DISTANCE: f32 = 5.0;  // Distanz, ab der Healthbar sichtbar wird
const HEALTHBAR_HEIGHT: f32 = 1.5;    // Höhe über dem Monster
const HEALTHBAR_WIDTH: f32 = 1.0;     // Breite des Healthbars

#[derive(Component)]
pub struct MonsterHealthbar;

// Events
#[derive(Event)]
pub struct AttackEvent {
    pub attacker: Entity,
    pub direction: Vec3,
}

#[derive(Event)]
pub struct DamageEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub fixed_damage: usize
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct MyGizmos {}


pub struct FightingPlugin;

impl Plugin for FightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AttackEvent>()
            .add_event::<DamageEvent>()
            .init_gizmo_group::<MyGizmos>()
            .add_systems(Update, (
                handle_attacks,
                process_damage,
                sword_rotation,
                fade_out_monsters,
            ).chain())
            .add_systems(Update, (
                update_healthbar_visibility,
                render_healthbars,
                update_config_gizmo
            ).chain());;
    }
}

const ATTACK_TIME:f32=0.5;
const ATTACK_DISTANCE:f32=2.0;
fn handle_attacks(
    mut commands: Commands,
    mut attack_events: EventReader<AttackEvent>,
    mut damage_events: EventWriter<DamageEvent>,
    mut actors: Query<(Entity, &Transform)>,
    all_actors: Query<(Entity, &Transform), With<Actor>>,
    arm_query: Query<Entity, (With<RightArm>, Without<AttackTimer>)>,
    children_query: Query<&Children>
) {
    for event in attack_events.read() {
        let Ok((attacker, attacker_transform)) = actors.get_mut(event.attacker) else {continue};
        let Ok(player_children) = children_query.get(attacker) else {continue};
        for &child in player_children.iter() {
            let Ok(arm_entity) = arm_query.get(child) else {continue};
            //add animation
            commands.entity(arm_entity).insert(AttackTimer(Timer::new(
                Duration::from_secs_f32(ATTACK_TIME),
                TimerMode::Once
            )));

            //add damage
            for (actor, actor_transform) in all_actors.iter() {
                if actor != attacker {
                    if actor_transform.translation.distance(attacker_transform.translation) <= ATTACK_DISTANCE {
                        damage_events.send(DamageEvent {
                            attacker: attacker,
                            target: actor,
                            fixed_damage: 0
                        });
                        break;
                    }
                }
            }
        }
    }
}

fn sword_rotation(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut AttackTimer), With<RightArm>>,
) {
    for (entity, mut transform, mut attack_timer) in query.iter_mut() {
        attack_timer.0.tick(time.delta());

        let progress = attack_timer.0.elapsed_secs() / attack_timer.0.duration().as_secs_f32();

        // Up rotation in first half, down rotation in second half
        let rotation = if progress < 0.5 {
            // Interpolate from horizontal to 90 degrees up
            Quat::from_rotation_x(FRAC_PI_2 * (1.0 - progress * 2.0))
        } else {
            // Interpolate back from 90 degrees up to horizontal
            Quat::from_rotation_x(FRAC_PI_2 * (progress - 1.0) * 2.0)
        };

        transform.rotation = rotation;

        if attack_timer.0.finished() {
            commands.entity(entity).remove::<AttackTimer>();
        }
    }
}


fn process_damage(
    mut damage_events: EventReader<DamageEvent>,
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
    mut actors: Query<(Entity, &mut Actor, &Name, Option<&mut MonsterAIState>)>
) {
    for event in damage_events.read() {

        let attacker_power = if event.fixed_damage == 0 {
            let Ok((_attacker_entity, attacker, attacker_name, _)) = actors.get(event.attacker) else { continue };
            attacker.power
        } else {
            0
        };

        let Ok((target_entity,
                   mut target,
                   target_name,
                   mut monster_ai_state)) = actors.get_mut(event.target) else { continue };

        let Some(ai_state) = &mut monster_ai_state else { continue };
        if **ai_state != MonsterAIState::Fading {
            let damage = if event.fixed_damage > 0 {
                event.fixed_damage
            } else {
                if target.defense < attacker_power {
                    attacker_power - target.defense
                } else {
                    0
                }
            };

            let new_hit_points:i32 = target.hit_points as i32 - damage as i32;

            if new_hit_points <= 0 {
                let player = player_query.single();

                if player == target_entity {

                } else {
                    println!("added fading");
                    commands.entity(target_entity).insert(Fading::new());
                    **ai_state = MonsterAIState::Fading;
                }
            } else {
                target.hit_points = new_hit_points as usize;
            }
        }
    }
}

fn fade_out_monsters(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Fading, &mut Handle<StandardMaterial>), With<Monster>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut fading, material_handle) in query.iter_mut() {
        // Timer aktualisieren
        fading.fade_duration.tick(time.delta());

        // Aktuelle Materialien abrufen
        if let Some(material) = materials.get_mut(material_handle.id()) {
            // Alpha-Wert linear reduzieren
            let alpha = 1.0 - fading.fade_duration.fraction();

            // Neuen Farbwert mit reduziertem Alpha erstellen
            material.base_color.set_alpha(alpha);

            // Monster entfernen, wenn vollständig ausgeblendet
            if fading.fade_duration.finished() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn update_healthbar_visibility(
    player_query: Query<&Transform, (With<Player>,Without<Monster>)>,
    mut monsters_query: Query<(Entity, &Transform, &Actor, Option<&MonsterHealthbar>), With<Monster>>,
    mut commands: Commands,
) {
    let player_transform = player_query.single();

    for (monster_entity, monster_transform, actor, healthbar) in monsters_query.iter_mut() {
        let distance = player_transform.translation.distance(monster_transform.translation);

        if distance <= HEALTHBAR_DISTANCE && healthbar.is_none() {
            commands.entity(monster_entity)
                .insert(MonsterHealthbar);
        } else if distance > HEALTHBAR_DISTANCE && healthbar.is_some() {
            commands.entity(monster_entity)
                .remove::<MonsterHealthbar>();
        }
    }
}

fn render_healthbars(
    mut my_gizmos: Gizmos<MyGizmos>,
    camera_query: Query<&Transform, With<ThirdPersonCamera>>,
    monsters_query: Query<(&Transform, &Actor), (With<Monster>, With<MonsterHealthbar>)>,
) {
    let camera_transform = camera_query.single();

    for (monster_transform, actor) in monsters_query.iter() {

        // Berechne Healthbar-Position über dem Monster
        let healthbar_pos = monster_transform.translation + Vec3::Y * HEALTHBAR_HEIGHT;

        // Berechne Blickrichtung der Kamera
        let look_direction = camera_transform.forward();

        // Rotiere in Richtung der Kamera
        let rotation = Quat::from_axis_angle(Vec3::Y, look_direction.y.atan2(look_direction.x));

        // Healthbar-Breite basierend auf Gesundheitszustand
        let health_percentage = actor.hit_points as f32 / actor.max_hit_points as f32;
        let current_width = HEALTHBAR_WIDTH * health_percentage;

        // Hintergrund-Linie (grau)
        let left_point = healthbar_pos - rotation * Vec3::X * (HEALTHBAR_WIDTH / 2.0);
        let right_point = healthbar_pos + rotation * Vec3::X * (HEALTHBAR_WIDTH / 2.0);

        // Gesundheitsbalken-Linie (rot)
        let health_right_point = left_point + rotation * Vec3::X * current_width;
        my_gizmos.line(
            left_point,
            health_right_point,
            Color::Srgba(LIGHT_GREEN)
        );
    }
}

fn update_config_gizmo(
    mut config_store: ResMut<GizmoConfigStore>,
) {

    let (my_config, _) = config_store.config_mut::<MyGizmos>();
    my_config.line_width = 20.0;

}