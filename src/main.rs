use bevy::color::palettes::css::{LIGHT_GRAY};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::common_conditions::input_toggle_active;
use bevy_egui::EguiPlugin;
use rand::Rng;
use std::f32::consts::PI;

use bimap::BiMap;

use orbitcamera::{OrbitCameraPlugin,OrbitCamera};
use third_person_camera::ThirdPersonCameraPlugin;
use dungeon_lighting::{DungeonLightingPlugin,place_torch_lights};
use crate::third_person_camera::ThirdPersonCamera;
use crate::create_dungeon::{StringMapGenerator, DungeonGeneratorStrategy, MapGeneratorStart};
use crate::fighting::{FightingPlugin, Actor, AttackEvent};
use crate::chracter_controller::{MonsterAIPlugin,MonsterAIState};
use crate::ui::{HeadUpDisplay, UiPlugin};

mod orbitcamera;
mod third_person_camera;
mod dungeon_lighting;
mod create_dungeon;
mod fighting;
mod chracter_controller;
mod ui;

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
enum TileType {
    Empty,
    Wall,
    Floor,
    Player
}

#[derive(Debug)]
struct TileMapping {
    mapping: BiMap<TileType, char>,
}

impl TileMapping {
    fn new() -> Self {
        let mut mapping = BiMap::new();
        mapping.insert(TileType::Wall, '#');
        mapping.insert(TileType::Floor, '.');
        mapping.insert(TileType::Player, '@');
        mapping.insert(TileType::Empty, ' ');

        TileMapping { mapping }
    }

    fn get_char(&self, tile_type: &TileType) -> char {
        self.mapping.get_by_left(tile_type).cloned().unwrap()
    }

    fn get_tile_type(&self, character: char) -> TileType {
        self.mapping.get_by_right(&character).cloned().unwrap()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tile {
    tile_type: TileType
}

impl Tile {
    fn new(tile_type: TileType) -> Self {
        Tile{
            tile_type
        }
    }

}

const TILE_SIZE: f32 = 4.0;

const ROOM_MAX_SIZE:usize = 10;
const ROOM_MIN_SIZE:usize = 6;
const MAX_ROOMS:usize = 30;
const MAX_MONSTERS_PER_ROOM:usize = 2;
const MAX_ITEMS_PER_ROOM:usize = 2;

#[derive(Clone, Debug)]
pub struct Grid {
    inner: Vec<Vec<Tile>>
}

impl Grid {
    pub fn new(width:usize, height:usize, tile_type: TileType) -> Self {
        let grid:Vec<Vec<Tile>> = vec![vec![Tile::new(tile_type); width]; height];
        Self { inner: grid }
    }

    pub fn width(&self) -> usize {
        self.inner.first().map_or(0, |row| row.len())
    }

    pub fn height(&self) -> usize {
        self.inner.len()
    }
    /// Safe access to tiles
    pub fn get(&self, col: usize, row: usize) -> Option<&Tile> {
        self.inner.get(row)?.get(col)
    }
    /// Mutable access to tiles
    pub fn get_mut(&mut self, col: usize, row: usize) -> Option<&mut Tile> {
        self.inner.get_mut(row)?.get_mut(col)
    }
    /// Checks if a position in the grid is valid
    pub fn is_valid_position(&self, col: usize, row: usize) -> bool {
        row < self.height() && col < self.width()
    }
}

// Indexing trait for direct access
impl std::ops::Index<(usize, usize)> for Grid {
    type Output = Tile;
    fn index(&self, (col, row): (usize, usize)) -> &Self::Output {
        &self.inner[row][col]
    }
}
impl std::ops::IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, ( col, row): (usize, usize)) -> &mut Self::Output {
        &mut self.inner[row][col]
    }
}

#[derive(Debug)]
enum ItemTyp {
    HealPotion
}

#[derive(Debug)]
struct ItemInMap{
    position: (usize, usize),
    item_type: ItemTyp
}

#[derive(Debug)]
enum MonsterType {
    Orc,
    Troll
}

#[derive(Debug)]
struct MonsterInMap{
    position: (usize, usize),
    monster_type: MonsterType
}

#[derive(Debug, Resource)]
struct GameMap {
    grid: Grid,
    tile_mapping: TileMapping,
    player_position: (usize, usize),
    monsters: Vec<MonsterInMap>,
    items: Vec<ItemInMap>,
    center: (usize, usize),
    width: usize,
    height: usize
}

impl GameMap {
    fn from_string(map_string: &str) -> Result<Self, String> {
        StringMapGenerator{map_string: map_string.to_string()}.generate()
    }

    /*
    impl Trait in Rust means:

Generic function parameter
Concrete type determined by compiler at compile-time
Enables static dispatch
Type-safe and performant
Compiles to monomorphic code (specialized for each concrete type)

Benefits:

Flexibility in type selection
No runtime overhead
Compiler optimizations possible
     */

    fn create_dungeon(generator: impl DungeonGeneratorStrategy) -> Result<Self, String> {
        generator.generate()
    }


    fn print(&self) {
        for y in 0..self.grid.width() {
            for x in 0..self.grid.height() {
                let ch = self.tile_mapping.get_char(&self.grid[(x,y)].tile_type);
                print!("{}", ch);
            }
            println!();
        }
    }

    fn to_string(&self,position:(usize,usize),
                 player_position:(usize,usize),
                 width:usize,height:usize) -> String {
        let mut parts: Vec<char> = Vec::new();
        for y in position.1..(height+position.1) {
            for x in position.0..(width+position.0) {
                if player_position == (x, y) {
                    parts.push(self.tile_mapping.get_char(&TileType::Player));
                } else {
                    parts.push(self.tile_mapping.get_char(&self.grid[(x,y)].tile_type));
                }
            }
            parts.push('\n');
        }
        parts.remove(parts.len() - 1);
        parts.into_iter().collect()
    }

    fn grid_to_world(&self, x:usize, y:usize) -> Vec3 {
        Vec3::new((x as f32 - self.center.0 as f32) * TILE_SIZE,
                  0.0,
                  (y as f32 - self.center.1 as f32) * TILE_SIZE)

    }

    fn world_to_grid(&self, position: Vec3) -> (usize, usize) {
        let x = ((position.x+0.5*TILE_SIZE) / TILE_SIZE + self.center.0 as f32) as usize;
        let y = ((position.z+0.5*TILE_SIZE) / TILE_SIZE + self.center.1 as f32) as usize;
        (x, y)
    }

    fn generate(
        &mut self,
        commands: &mut Commands,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        // By default AssetServer will load assets from inside the "assets" folder.
        // For example, the next line will load GltfAssetLabel::Primitive{mesh:0,primitive:0}.from_asset("ROOT/assets/models/cube/cube.gltf"),
        // where "ROOT" is the directory of the Application.
        //
        // This can be overridden by setting [`AssetPlugin.file_path`].
        let abstract_mesh = false;
        let wall_size:f32 = 1.0;
        let mut rng = rand::thread_rng();

        let wall_handle:Handle<Scene> = asset_server.load("models/wall.gltf#Scene0");

        let floor_handle:Handle<Scene> = asset_server.load("models/floor_dirt_large.gltf#Scene0");


        for y in 0..self.height {
            for x in 0..self.width {
                match self.grid[(x,y)].tile_type {
                    TileType::Wall => {
                        let position = self.grid_to_world(x,y);
                        if abstract_mesh {
                            /*let entity = commands.spawn(PbrBundle {
                           mesh: meshes.add(Mesh::from(Cuboid::new(TILE_SIZE,TILE_SIZE,TILE_SIZE))),
                           material: materials.add(Color::Srgba(DARK_GRAY)),
                           transform: Transform::from_xyz(position.x,TILE_SIZE/2.0,position.z),
                           ..default()
                       })
                           .id();*/
                        } else {
                            //right
                            if x != self.width-1 && self.grid[(x+1,y)].tile_type == TileType::Floor {
                                commands.spawn(SceneBundle {
                                    scene: wall_handle.clone(),
                                    transform:Transform {
                                        translation:  Vec3::new(position.x+TILE_SIZE*0.5-wall_size*0.5,0.0,position.z),
                                        rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    ..Default::default()
                                });
                            }
                            //left
                            if x != 0 && self.grid[(x-1,y)].tile_type == TileType::Floor {
                                commands.spawn(SceneBundle {
                                    scene: wall_handle.clone(),
                                    transform:Transform {
                                        translation:  Vec3::new(position.x-TILE_SIZE*0.5+wall_size*0.5,0.0,position.z),
                                        rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    ..Default::default()
                                });
                            }
                            //up
                            if y != 0 && self.grid[(x,y-1)].tile_type == TileType::Floor {
                                commands.spawn(SceneBundle {
                                    scene: wall_handle.clone(),
                                    transform:Transform {
                                        translation:  Vec3::new(position.x,0.0,position.z-TILE_SIZE*0.5+wall_size*0.5),
                                        //rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    ..Default::default()
                                });
                            }
                            //down
                            if y != self.height-1 && self.grid[(x,y+1)].tile_type == TileType::Floor {
                                commands.spawn(SceneBundle {
                                    scene: wall_handle.clone(),
                                    transform:Transform {
                                        translation:  Vec3::new(position.x,0.0,position.z+TILE_SIZE*0.5-wall_size*0.5),
                                        //rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    ..Default::default()
                                });
                            }
                        }
                    },
                    TileType::Floor => {
                        let position = self.grid_to_world(x,y);
                        if abstract_mesh {
                            commands.spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(Cuboid::new(TILE_SIZE,0.1,TILE_SIZE))),
                                material: materials.add(Color::Srgba(LIGHT_GRAY)),
                                transform: Transform{
                                    translation: Vec3::new(position.x,-0.05,position.z),
                                    rotation: Quat::from_rotation_y(PI*0.5*rng.gen_range(1..=3)as f32),
                                    ..default()
                                },
                                ..default()
                            });
                        } else {
                            let entity = commands.spawn(SceneBundle {
                                scene: floor_handle.clone(),
                                transform: Transform::from_xyz(position.x,-0.05,position.z),
                                ..Default::default()
                            }).id();
                        }
                        //self.grid[j][i].entities.push(entity);
                    },
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct RightArm;

#[derive(Component)]
struct AttackTimer(Timer);

#[derive(Component)]
struct Monster;

#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Yet Another Roguelike Tutorial in Rust with Bevy".to_string(),
                resolution: WindowResolution::new(920.0, 640.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ThirdPersonCameraPlugin)
        .add_plugins((OrbitCameraPlugin,DungeonLightingPlugin, EguiPlugin))
        .add_plugins(UiPlugin)
        .add_plugins(MonsterAIPlugin)
        .add_plugins(FightingPlugin)
        .add_plugins((
            // Adds frame time diagnostics
            FrameTimeDiagnosticsPlugin,
            // Adds a system that prints diagnostics to the console
            LogDiagnosticsPlugin::default(),
            // Any plugin can register diagnostics. Uncomment this to add an entity count diagnostics:
            // bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Uncomment this to add an asset count diagnostics:
            // bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default(),
            // Uncomment this to add system info diagnostics:
            // bevy::diagnostic::SystemInformationDiagnosticsPlugin::default()
        ))
        .add_systems(Startup, (setup_orbitcamera, setup))
        //.add_systems(Startup, place_torch_lights)
        .add_systems(Update, debug)
        .add_systems(Update, move_player)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

    // gamemap
    let map_string = "....................
.........####.......
....................
..........@.........
....................
....................";

   // let game_map = GameMap::from_string(map_string).expect("Failed to parse level");
    let mut game_map = GameMap::create_dungeon( MapGeneratorStart::new(80,45,
                                                                       MAX_ROOMS,
                                                                       ROOM_MIN_SIZE,
                                                                       ROOM_MAX_SIZE,
                                                                       MAX_MONSTERS_PER_ROOM,
                                                                       MAX_ITEMS_PER_ROOM) )
                                                .expect("Failed to create level");

    //game_map.print();

    println!("Player position: ({}, {})", game_map.player_position.0, game_map.player_position.1);

    setup_character(&mut commands, &mut meshes, &mut materials, &mut game_map);

    // monster
    setup_monster(&mut commands, &mut meshes, &mut materials, &mut game_map);
    // item
    setup_item(&mut commands, &asset_server,&mut game_map);
    // ground
    game_map.generate(&mut commands, asset_server, meshes, materials);

    commands.insert_resource(game_map);
}

fn setup_monster(commands: &mut Commands, mut meshes: &mut ResMut<Assets<Mesh>>, mut materials: &mut ResMut<Assets<StandardMaterial>>, game_map: &mut GameMap) {
    for i in game_map.monsters.iter() {

        let position = game_map.grid_to_world(i.position.0, i.position.1);
        match i.monster_type {
            MonsterType::Troll => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(Capsule3d::new(0.8, 1.5))), // Large, bulky troll
                        material: materials.add(StandardMaterial { // Earthy, stone-like color
                            base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                        transform: Transform::from_xyz(position.x, 0.8, position.z),
                        ..default()
                    },
                    Monster, // Component to identify the Troll
                    Actor::new(16, 1, 4),
                    MonsterAIState::Idle
                )).insert(Name::new("troll")).with_children(|parent| {
                    // Front (rough, rocky appearance)
                    parent.spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(Cuboid::new(0.4, 0.4, 0.4))),
                            material: materials.add(StandardMaterial { // Earthy, stone-like color
                                base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            }),
                            transform: Transform::from_xyz(0.0, 0.7, -0.7),
                            ..default()
                        },
                    )).insert(Name::new("troll-front"));

                    // Left Arm (massive, club-like)
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Mesh::from(Cuboid::new(0.5, 1.2, 0.5))), // Even larger arm
                        material: materials.add(StandardMaterial { // Earthy, stone-like color
                            base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                        transform: Transform::from_xyz(-1.0, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.3)),
                        ..default()
                    }).insert(Name::new("troll-left-arm"));

                    // Right Arm with Giant Club
                    parent.spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(Cuboid::new(0.5, 1.2, 0.5))),
                            material: materials.add(StandardMaterial { // Earthy, stone-like color
                                base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            }),
                            transform: Transform::from_xyz(1.0, 0.2, 0.0)
                                .with_rotation(Quat::from_rotation_x(0.3)),
                            ..default()
                        },
                        RightArm,
                    )).insert(Name::new("troll-right-arm")).with_children(|arm| {
                        // Giant Club
                        arm.spawn((
                            PbrBundle {
                                mesh: meshes.add(Mesh::from(Cuboid::new(0.3, 1.5, 0.3))),
                                material: materials.add(StandardMaterial { // Earthy, stone-like color
                                    base_color: Color::srgba(0.4, 0.3, 0.2, 1.0),
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                }),
                                transform: Transform::from_xyz(0.0, -1.0, -0.3)
                                    .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                                ..default()
                            }
                        )).insert(Name::new("troll-sword"));
                    });
                });
            }
            _ => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(Capsule3d::new(0.6, 1.2))), // Slightly larger and bulkier
                        material: materials.add(StandardMaterial { // Greenish skin tone
                            base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                        transform: Transform::from_xyz(position.x, 0.8, position.z),
                        ..default()
                    },
                    Monster,
                    Actor::new(10, 0, 3),
                    MonsterAIState::Idle
                )).insert(Name::new("orc")).with_children(|parent| {
                    // Front (more brutish look)
                    parent.spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(Cuboid::new(0.3, 0.3, 0.3))), // Slightly larger
                            material: materials.add(StandardMaterial { // Greenish skin tone
                                base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            }),
                            transform: Transform::from_xyz(0.0, 0.6, -0.6), // Adjusted position
                            ..default()
                        },
                    )).insert(Name::new("orc-front"));

                    // Left Arm (more muscular)
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Mesh::from(Cuboid::new(0.4, 1.0, 0.4))), // Thicker arm
                        material: materials.add(StandardMaterial { // Greenish skin tone
                            base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        }),
                        transform: Transform::from_xyz(-0.8, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.2)), // Slight angle
                        ..default()
                    }).insert(Name::new("orc-left-arm"));

                    // Right Arm with Battle Axe
                    parent.spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(Cuboid::new(0.4, 1.0, 0.3))), // Muscular arm
                            material: materials.add(StandardMaterial { // Greenish skin tone
                                base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            }),
                            transform: Transform::from_xyz(0.8, 0.2, 0.0)
                                .with_rotation(Quat::from_rotation_x(0.2)), // Slight angle
                            ..default()
                        },
                        RightArm,
                    )).insert(Name::new("orc-right-arm")).with_children(|arm| {
                        // Battle Axe replacing the sword
                        arm.spawn((
                            PbrBundle {
                                mesh: meshes.add(Mesh::from(Cuboid::new(0.2, 1.2, 0.3))), // Larger, brutal weapon
                                material: materials.add(StandardMaterial {
                                    base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                }),
                                transform: Transform::from_xyz(0.0, -0.7, -0.3)
                                    .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                                ..default()
                            },
                        )).insert(Name::new("orc-sword"));
                    });
                });
            }
        }
    }
}

struct Character{
    name : String,
    body_radius: f32,
    body_length: f32,
    position: Vec3,
    color: Color,
    max_hit_points: usize,
    defense: usize,
    power: usize,
}

fn setup_character(
    commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
    mut game_map: &mut GameMap
) {
    let mut player_position = game_map.grid_to_world(game_map.player_position.0,
                                                     game_map.player_position.1);
    player_position.y = 0.9;

    let character = Character{
        name: String::from("player"),
        body_radius : 0.5,
        body_length: 1.0,
        position: player_position,
        color: Color::srgb(0.2, 0.4, 0.8),
        max_hit_points:30,
        defense:2,
        power:5
    };

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Capsule3d::new(character.body_radius,
                                                       character.body_length))),
            material: materials.add(character.color),
            transform: Transform::from_translation(character.position),
            ..default()
        },
        Player,
        HeadUpDisplay::new(),
        Actor::new (character.max_hit_points, character.defense, character.power),
        Name::new(character.name)
    )).with_children(|parent| {
        //front
        parent.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Cuboid::new(0.25, 0.25, 0.25))),
                material: materials.add(character.color),
                transform: Transform::from_xyz(0.0, 0.5, -0.5),
                ..default()
            },
            Name::new("player-front")
        ));

        // Left Arm
        parent.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(0.3, 0.8, 0.3))),
            material: materials.add(character.color),
            transform: Transform::from_xyz(-0.7, 0.2, 0.0)
                .with_rotation(Quat::from_rotation_x(0.0)),
            ..default()
        }).insert(Name::new("player-left-arm"));

        // Right Arm with Sword (two components)
        parent.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Cuboid::new(0.3, 0.8, 0.3))),
                material: materials.add(character.color),
                transform: Transform::from_xyz(0.7, 0.2, 0.0)
                    .with_rotation(Quat::from_rotation_x(0.0)),
                ..default()
            },
            RightArm,
            Name::new("player-right-arm")
        )).with_children(|arm| {
            // Sword as a long cuboid attached to right arm
            arm.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(Cuboid::new(0.1, 0.8, 0.1))),
                    material: materials.add(Color::srgb(0.6, 0.6, 0.6)),
                    transform: Transform::from_xyz(0.0, -0.5, -0.2)
                        .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                    ..default()
                },
                Name::new("player-sword")
            ));
        });
    });
}

fn setup_item(
    commands: &mut Commands,
    asset_server:  &Res<AssetServer>,
    game_map: &mut GameMap
) {
    let heal_portion_handle:Handle<Scene> = asset_server.load("models/bottle_A_brown.gltf#Scene0");

    for i in game_map.items.iter() {

        let position = game_map.grid_to_world(i.position.0, i.position.1);
        match i.item_type {
            ItemTyp::HealPotion => {
               commands.spawn(SceneBundle {
                   scene: heal_portion_handle.clone(),
                   transform:Transform {
                       translation:  Vec3::new(position.x,0.0,position.z),
                       //rotation: Quat::from_rotation_y(PI/2.0),
                       ..default()
                   },
                   ..Default::default()
               });
            }
        }
    }
}

fn setup_orbitcamera(
    mut commands: Commands
){
    commands.spawn(Camera3dBundle{
        camera: Camera{
            is_active:false,
            order:5,
            ..default()
        },
        ..default()
    }
    )
        .insert(OrbitCamera{
            distance : 28.0,
            ..default()
        })
        .insert(Name::new("OrbitCamera"));
}

fn debug(
    keyboard_input:Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Camera>
)
{
    if keyboard_input.just_pressed(KeyCode::KeyO) {
        for mut camera in query.iter_mut() {
            camera.is_active = ! camera.is_active
        }
    };
}

const SPEED:f32 = 2.0;

fn move_camera(
    mut query_camera: Query<&mut Transform, (With<MainCamera>,Without<Player>)>,
    query_player: Query<&Transform, (With<Player>,Without<MainCamera>)>
){

    let player_transfrom = query_player.single();

    for mut camera_transform in  query_camera.iter_mut() {
        let player_position = player_transfrom.translation.clone();
        let transform = Transform::from_translation(player_position + Vec3::new(0.0, 4.0, 10.0)).looking_at(player_position, Vec3::Y);
        camera_transform.translation = transform.translation;
        camera_transform.rotation = transform.rotation
    }
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(Entity, &mut Transform), (With<Player>, Without<Monster>)>,
    mut attack_events: EventWriter<AttackEvent>,
    camera_query: Query<&Transform, (With<ThirdPersonCamera>, Without<Player>,Without<Monster>)>,
    monster_query: Query<&mut Transform, (With<Monster>, Without<Player>)>,
    game_map: Res<GameMap>,
    time: Res<Time>
) {
        for (player_entity, mut player_transform) in player_query.iter_mut() {
            if keyboard_input.just_pressed(KeyCode::Space) {
                attack_events.send(AttackEvent {
                    attacker: player_entity,
                    direction: player_transform.forward().as_vec3()
                });
            } else {
            for camera_transform in camera_query.iter() {
                let camera_forward = camera_transform.forward().as_vec3();
                let camera_right = camera_transform.right().as_vec3();

                let mut move_vector = Vec3::ZERO;
                if keyboard_input.pressed(KeyCode::ArrowLeft) {
                    move_vector = Vec3::new(-camera_right.x, 0.0, -camera_right.z);
                }
                if keyboard_input.pressed(KeyCode::ArrowRight) {
                    move_vector = Vec3::new(camera_right.x, 0.0, camera_right.z);
                }
                if keyboard_input.pressed(KeyCode::ArrowUp) {
                    move_vector = Vec3::new(camera_forward.x, 0.0, camera_forward.z);
                }
                if keyboard_input.pressed(KeyCode::ArrowDown) {
                    move_vector = Vec3::new(-camera_forward.x, 0.0, -camera_forward.z);
                }

                if move_vector != Vec3::ZERO {
                    move_vector = move_vector.normalize_or_zero();

                    // Store the original forward direction
                    let original_forward = player_transform.forward().as_vec3();

                    // Update player position
                    player_transform.translation = player_without_colliding(
                        &game_map,
                        &monster_query,
                        player_transform.translation,
                        move_vector * time.delta_seconds() * SPEED
                    );

                    // Only rotate if the move vector is significantly different from current forward
                    let angle = move_vector.angle_between(original_forward);
                    if angle.abs() > 0.1 {
                        // Calculate the rotation angle, but only rotate around Y axis
                        let rotation = Quat::from_rotation_y(
                            original_forward.cross(move_vector).y * angle
                        );
                        player_transform.rotation *= rotation;
                    }
                }
            }
        }
    }
}

const PLAYER_DISTANCE:f32=0.5;

fn player_without_colliding(
    game_map: &GameMap,
    monster_query: &Query<&mut Transform, (With<Monster>, Without<Player>)>,
    position:Vec3,
    move_vector:Vec3
)->Vec3{

    let new_position = position + move_vector;

    //up
    let up = Vec3::new(0.0,0.0,-PLAYER_DISTANCE) + new_position;
    let map_up =  game_map.world_to_grid(up);
    if game_map.grid[map_up].tile_type == TileType::Wall {
        return position;
    }

    //down
    let down = Vec3::new(0.0,0.0,PLAYER_DISTANCE) + new_position;
    let map_down =  game_map.world_to_grid(down);
    if game_map.grid[map_down].tile_type == TileType::Wall {
        return position;
    }

    //left
    let left = Vec3::new(-PLAYER_DISTANCE,0.0,0.0) + new_position;
    let map_left =  game_map.world_to_grid(left);
    if game_map.grid[map_left].tile_type == TileType::Wall {
        return position;
    }

    //right
    let right = Vec3::new(PLAYER_DISTANCE,0.0,0.0) + new_position;
    let map_right =  game_map.world_to_grid(right);
    if game_map.grid[map_right].tile_type == TileType::Wall {
        return position;
    }

    //monster
    for i in monster_query.iter() {
        if new_position.distance(i.translation) <= PLAYER_DISTANCE * 2.0 {
            return position;
        }
    }

    new_position
}