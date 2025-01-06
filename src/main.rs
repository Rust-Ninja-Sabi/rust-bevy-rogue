use std::collections::HashMap;
use bevy::color::palettes::css::{LIGHT_GRAY};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin};
use bevy_egui::EguiPlugin;
use rand::Rng;
use std::f32::consts::PI;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use serde::{Serialize, Deserialize};

use orbitcamera::{OrbitCameraPlugin,OrbitCamera};
use third_person_camera::ThirdPersonCameraPlugin;
use dungeon_lighting::{DungeonLightingPlugin,place_torch_lights};
use crate::third_person_camera::ThirdPersonCamera;
use crate::create_dungeon::{StringMapGenerator, DungeonGeneratorStrategy,
                            MapGeneratorStart, BresenhamLine, DungeonWriter};
use crate::fighting::{FightingPlugin, Actor, AttackEvent, DamageEvent};
use crate::chracter_controller::{MonsterAIPlugin,MonsterAIState};
use crate::ui::{HeadUpDisplay, UiPlugin};

mod orbitcamera;
mod third_person_camera;
mod dungeon_lighting;
mod create_dungeon;
mod fighting;
mod chracter_controller;
mod ui;

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    MainMenu,
    InGame
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
// And we need to add an attribute to let us know what the source state is
// and what value it needs to have. This will ensure that unless we're
// in [`AppState::InGame`], the [`IsPaused`] state resource
// will not exist.
#[source(GameState = GameState::InGame)]
enum TransitionState {
    #[default]
    Running,
    StairsDown,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash,Copy)]
enum TileType {
    Empty,
    Wall,
    Floor,
    StaircaseDown,
    Player,
    Potion,
    Lightning,
    Orc,
    Troll
}

#[derive(Clone, Debug)]
struct TileRow {
    character: char,
    tile_type: TileType,
    item_type: Option<ItemType>,
    monster_type: Option<MonsterType>
}

#[derive(Debug)]
struct TileMapping {
    rows: Vec<TileRow>
}

impl TileMapping {
    fn new() -> Self {
        let mut rows:Vec<TileRow > = Vec::new();

        rows.push(TileRow{character: '#', tile_type: TileType::Wall, item_type: None, monster_type: None});
        rows.push(TileRow{character: '.', tile_type: TileType::Floor, item_type: None, monster_type: None});
        rows.push(TileRow{character: '>', tile_type: TileType::StaircaseDown, item_type: None, monster_type: None});
        rows.push(TileRow{character: '@', tile_type: TileType::Player, item_type: None, monster_type: None});
        rows.push(TileRow{character: '!', tile_type: TileType::Potion, item_type: Some(ItemType::HealPotion), monster_type: None});
        rows.push(TileRow{character: '?', tile_type: TileType::Lightning, item_type: Some(ItemType::Lightning), monster_type: None});
        rows.push(TileRow{character: 'o', tile_type: TileType::Orc, item_type: None, monster_type: Some(MonsterType::Orc)});
        rows.push(TileRow{character: 'T', tile_type: TileType::Troll, item_type: None, monster_type: Some(MonsterType::Troll)});
        rows.push(TileRow{character: ' ', tile_type: TileType::Empty, item_type: None, monster_type: None});

        /*
        ^   A trap (known)
        ;   A glyph of warding
        '   An open door
        <   A staircase up
        +   A closed door
        %   A mineral vein
        *   A mineral vein with treasure
        :   A pile of rubble
        ,   A mushroom (or food)
        -   A wand or rod
        _   A staff
        =   A ring
        "   An amulet
        $   Gold or gems
        ~   Lights, Tools, Chests, etc
        &   Multiple items
        /   A pole-arm
        |   An edged weapon
        \   A hafted weapon
        }   A sling, bow, or x-bow
        {   A shot, arrow, or bolt
        (   Soft armour
        [   Hard armour
        ]   Misc. armour
        )   A shield
        a..z, A..Z  Monster
        */

        TileMapping { rows }
    }

    fn get_char(&self, tile_type: &TileType) -> char {
        self.rows.iter().find(|&row| row.tile_type == *tile_type).unwrap().character
    }

    fn get_tile_row(&self, character: char) -> TileRow {
        self.rows.iter().find(|&row| row.character == character).unwrap().clone()
    }

    fn get_tile_type(&self, character: char) -> TileType {
        self.rows.iter().find(|&row| row.character == character).unwrap().tile_type.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tile {
    tile_type: TileType,
    render_hint: RenderHint
}

impl Tile {
    fn new(tile_type: TileType) -> Self {
        Tile{
            tile_type,
            render_hint: RenderHint::Empty
        }
    }

}

const TILE_SIZE: f32 = 4.0;

const ROOM_MAX_SIZE:usize = 10;
const ROOM_MIN_SIZE:usize = 6;
const MAX_ROOMS:usize = 30;

#[derive(Debug)]
struct FloorParameterItem{
    max_monsters_per_room: usize,
    max_items_per_room: usize
}

#[derive(Debug, Resource)]
struct FloorParameters{
    items:Vec<FloorParameterItem>
}

impl FloorParameters {
    fn new() -> Self {
        let mut items: Vec<FloorParameterItem> = Vec::new();
        items.push(FloorParameterItem { max_monsters_per_room: 2, max_items_per_room: 10 });
        items.push(FloorParameterItem { max_monsters_per_room: 2, max_items_per_room: 1 });
        items.push(FloorParameterItem { max_monsters_per_room: 2, max_items_per_room: 1 });
        items.push(FloorParameterItem { max_monsters_per_room: 3, max_items_per_room: 2 });
        items.push(FloorParameterItem { max_monsters_per_room: 3, max_items_per_room: 2 });
        items.push(FloorParameterItem { max_monsters_per_room: 5, max_items_per_room: 2 });
        items.push(FloorParameterItem { max_monsters_per_room: 5, max_items_per_room: 2 });
        items.push(FloorParameterItem { max_monsters_per_room: 5, max_items_per_room: 2 });

        Self {
            items
        }
    }
}
#[derive(Clone, Debug)]
struct ItemAndMonsterParameterItem {
    items: Vec<(ItemType,f32)>,
    monsters: Vec<(MonsterType,f32)>
}

#[derive(Resource)]
struct ItemAndMonsterParameters {
    parameters: Vec<ItemAndMonsterParameterItem>
}

impl ItemAndMonsterParameters {
    fn new()->Self{
        let mut parameters:Vec<ItemAndMonsterParameterItem> = Vec::new();
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::Lightning,1.0)],
            monsters: vec![(MonsterType::Orc,1.0)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.8),(ItemType::Lightning,0.2)],
            monsters: vec![(MonsterType::Orc,0.8),(MonsterType::Troll,0.2)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.8),(ItemType::Lightning,0.2)],
            monsters: vec![(MonsterType::Orc,0.8),(MonsterType::Troll,0.2)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.5),(ItemType::Lightning,0.5)],
            monsters: vec![(MonsterType::Orc,0.5),(MonsterType::Troll,0.5)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.5),(ItemType::Lightning,0.5)],
            monsters: vec![(MonsterType::Orc,0.5),(MonsterType::Troll,0.5)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.5),(ItemType::Lightning,0.5)],
            monsters: vec![(MonsterType::Orc,0.5),(MonsterType::Troll,0.5)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.5),(ItemType::Lightning,0.5)],
            monsters: vec![(MonsterType::Orc,0.5),(MonsterType::Troll,0.5)]
        });
        parameters.push(ItemAndMonsterParameterItem{
            items: vec![(ItemType::HealPotion,0.5),(ItemType::Lightning,0.5)],
            monsters: vec![(MonsterType::Orc,0.5),(MonsterType::Troll,0.5)]
        });

        Self {
            parameters
        }

    }
}

#[derive(Debug, PartialEq, Clone)]
enum RenderHint {
    Empty,
    RoomFloor
}

#[derive(Clone, Debug)]
pub struct Grid {
    inner: Vec<Vec<Tile>>
}

impl Grid {
    pub fn new(width:usize, height:usize, tile_type: TileType) -> Self {
        let grid:Vec<Vec<Tile>> = vec![vec![Tile::new(tile_type); width]; height];
        Self {
            inner: grid
        }
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

    fn is_wall_between(&self, pos_0:(usize,usize), pos_1:(usize,usize)) -> bool {
        let line = BresenhamLine::new(
                                        pos_0.0 as i32,
                                        pos_0.1 as i32,
                                        pos_1.0 as i32,
                                        pos_1.1 as i32,false);
        for i in line {
            if self[(i.0 as usize, i.1 as usize)].tile_type == TileType::Wall {
                return true;
            }
        }
        false
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

#[derive(Resource)]
struct LoadMapAndItems(bool);

#[derive(Resource,Copy, Clone)]
struct CurrentFloor(usize);

impl CurrentFloor {
    fn load(file_name: &str) -> Self {
        let input = fs::read_to_string(file_name).expect("Unable to read file");
        let current_floor:usize = input.parse().expect("Unable to parse floor");
        CurrentFloor(current_floor)
    }

    fn next(&mut self) {
        self.0 += 1;
    }

    fn save(&self) {
        let mut file = File::create(FLOOR_JSON_FILE).expect("Unable to create file");
        file.write_all(self.0.to_string().as_bytes()).expect("Unable to write data");
    }
}

#[derive(Debug,PartialEq,Eq, Hash, Copy, Clone,Serialize, Deserialize)]
enum ItemType {
    HealPotion,
    Lightning
}

impl ItemType {
    fn to_string(&self) -> String {
        match self {
            ItemType::HealPotion => String::from("HealPotion"),
            ItemType::Lightning => String::from("Lightning")
        }
    }

    fn to_tile_type(&self) -> TileType {
        match self {
            ItemType::HealPotion => TileType::Potion,
            ItemType::Lightning => TileType::Lightning
        }
    }
}

#[derive(Debug)]
struct ItemInMap{
    position: (usize, usize),
    item_type: ItemType
}

#[derive(Debug, Clone)]
enum MonsterType {
    Orc,
    Troll
}

impl MonsterType {
    fn to_tile_type(&self) -> TileType {
        match self {
            MonsterType::Orc => TileType::Orc,
            MonsterType::Troll => TileType::Troll
        }
    }
}

#[derive(Debug)]
struct MonsterInMap{
    position: (usize, usize),
    monster_type: MonsterType
}

#[derive(Debug, Resource)]
struct ShowFps(bool);

#[derive(Debug, Resource)]
struct ShowPlayerValuesAndInventar(bool);

#[derive(Debug, Resource, Serialize, Deserialize)]
struct Inventory{
    heal_potion: usize,
    items:HashMap<ItemType, usize>,
    activ_item:Option<ItemType>
}

impl Inventory {
    fn new() -> Self {
        Inventory{
            heal_potion: 0,
            items:HashMap::new(),
            activ_item: None
        }
    }

    fn add_item(&mut self, item_type: ItemType) {
        println!("Item added: {:?}", item_type);
        if item_type == ItemType::HealPotion {
            self.heal_potion += 1;
        } else {
            *self.items.entry(item_type).or_insert(0) += 1;
            if self.activ_item == None {
                self.activ_item = Some(item_type);
            }
        }
    }

    fn remove_item(&mut self, item_type: ItemType) {
        if self.heal_potion > 0 {
            self.heal_potion -=1;
        }
        else {
            if let Some(value) = self.items.get_mut(&item_type) {
                if *value > 1 {
                    *value -= 1;
                } else {
                    self.items.remove(&item_type);
                    if let Some(active_type) = self.activ_item {
                        if active_type == item_type {
                            if self.items.len() == 0 {
                                self.activ_item = None;
                            } else {
                                if let Some((item_type,_)) = self.items.iter().next() {
                                    self.activ_item = Some(*item_type);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn get_active_item_name(&self) -> String {
        match self.activ_item {
            Some(item_type) => {
                let sum =  self.items[&item_type];
                format!("<X> {} {}", item_type.to_string(), sum)
            },
            None => "nothing active".to_string()
        }
    }

    fn save(&self) {
        let mut file = File::create(INVENTORY_JSON_FILE).expect("Unable to create file");
        let inventory = serde_json::to_string(self).expect("Unable to serialize inventory");
        file.write_all(inventory.as_bytes()).expect("Unable to write data");
    }

    fn load(file_name: &str) -> Self {
        let input = fs::read_to_string(file_name).expect("Unable to read file");
        let inventory:Inventory = serde_json::from_str(&input).expect("Unable to parse inventory");
        inventory
    }
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

const MAP_TEXT_FILE: &'static str = "dungeon.map";
const INVENTORY_JSON_FILE: &'static str = "inventory.json";
const ACTOR_JSON_FILE: &'static str = "actor.json";
const FLOOR_JSON_FILE: &'static str = "floor.json";

impl GameMap {
    fn from_string(map_string: &str) -> Result<Self, String> {
        StringMapGenerator{map_string: map_string.to_string()}.generate()
    }

    fn load(file_name: &str) -> Self{
        let input = fs::read_to_string(file_name).expect("Unable to read file");
        let game_map = GameMap::from_string(&input).expect("Failed to parse level");
        game_map
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


    fn print(
        &self,
        player: Vec3,
        items: Vec<(Vec3, ItemType)>,
        monsters:Vec<(Vec3,MonsterType)>
    ) {
        let writer = DungeonWriter::default();
        println!("{}", writer.write(self,player,items,monsters));
    }

    fn save(
        &self,
        player: Vec3,
        items: Vec<(Vec3, ItemType)>,
        monsters:Vec<(Vec3,MonsterType)>
    ) {
        let writer = DungeonWriter::default();

        let map_text = writer.write(self, player, items, monsters);

        let mut file = File::create(MAP_TEXT_FILE).expect("Unable to create file");

        file.write_all(map_text.as_bytes()).expect("Unable to write data");
    }

    fn to_string(
        &self,
        position:(i32,i32),
        player_position:(usize,usize),
        width:usize,height:usize
    ) -> String {

        let mut parts: Vec<char> = Vec::new();

        for y in position.1 as i32..(height as i32+position.1) as i32 {
            for x in position.0 as i32..(width as i32+position.0) as i32 {
                if 0<=x && x < self.width as i32 &&
                    0<=y && y < self.height as i32 {
                    if player_position == (x as usize, y as usize) {
                        parts.push(self.tile_mapping.get_char(&TileType::Player));
                    } else {
                        parts.push(self.tile_mapping.get_char(&self.grid[(x as usize, y as usize)].tile_type));
                    }
                }else{
                    parts.push(' ');
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

    fn collide_with_wall(&self, position:Vec3, distance:f32)->bool{
        let directions = vec![
            Vec3::new(0.0,0.0,-distance),
            Vec3::new(0.0,0.0,distance),
            Vec3::new(distance,0.0,0.0),
            Vec3::new(-distance,0.0,0.0),
        ];

        for i in directions {
            let new_position = position + i;
            let map_up =  self.world_to_grid(new_position);
            if self.grid[map_up].tile_type == TileType::Wall {
                return true
            }
        }

        false
    }

    fn generate(
        &mut self,
        commands: &mut Commands,
        current_floor: &mut ResMut<CurrentFloor>,
        asset_server: &Res<AssetServer>,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
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

        let floor_room_handle:Handle<Scene> = asset_server.load("models/floor_tile_large.gltf#Scene0");


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
                                commands.spawn((
                                    SceneRoot( wall_handle.clone()),
                                    Transform {
                                        translation:  Vec3::new(position.x+TILE_SIZE*0.5-wall_size*0.5,0.0,position.z),
                                        rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    Floor(current_floor.0)
                                ));
                            }
                            //left
                            if x != 0 && self.grid[(x-1,y)].tile_type == TileType::Floor {
                                commands.spawn((
                                    SceneRoot( wall_handle.clone()),
                                    Transform {
                                        translation:  Vec3::new(position.x-TILE_SIZE*0.5+wall_size*0.5,0.0,position.z),
                                        rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    Floor(current_floor.0)
                                ));
                            }
                            //up
                            if y != 0 && self.grid[(x,y-1)].tile_type == TileType::Floor {
                                commands.spawn((
                                    SceneRoot(wall_handle.clone()),
                                    Transform {
                                        translation:  Vec3::new(position.x,0.0,position.z-TILE_SIZE*0.5+wall_size*0.5),
                                        //rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    Floor(current_floor.0)
                                ));
                            }
                            //down
                            if y != self.height-1 && self.grid[(x,y+1)].tile_type == TileType::Floor {
                                commands.spawn((
                                    SceneRoot( wall_handle.clone()),
                                    Transform {
                                        translation:  Vec3::new(position.x,0.0,position.z+TILE_SIZE*0.5-wall_size*0.5),
                                        //rotation: Quat::from_rotation_y(PI/2.0),
                                        ..default()
                                    },
                                    Floor(current_floor.0)
                                ));
                            }
                        }
                    },
                    TileType::Floor => {
                        let position = self.grid_to_world(x,y);
                        if abstract_mesh {
                            commands.spawn((
                                Mesh3d( meshes.add(Mesh::from(Cuboid::new(TILE_SIZE,0.1,TILE_SIZE)))),
                                MeshMaterial3d(materials.add(Color::Srgba(LIGHT_GRAY))),
                                Transform{
                                    translation: Vec3::new(position.x,-0.05,position.z),
                                    rotation: Quat::from_rotation_y(PI*0.5*rng.gen_range(1..=3)as f32),
                                    ..default()
                                },
                                Floor(current_floor.0)
                            ));
                        } else {
                            let new_handle = if self.grid[(x,y)].render_hint == RenderHint::RoomFloor{
                                floor_room_handle.clone()
                            }else{
                                floor_handle.clone()
                            };
                            commands.spawn((
                                SceneRoot(new_handle),
                                Transform::from_xyz(position.x,-0.05,position.z),
                                Floor(current_floor.0)
                            ));
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component)]
struct Player;

enum TransitionStep {
    StairDownStart,
    StairDownEnd
}
#[derive(Component)]
struct PlayerTransition{
    step: TransitionStep,
    timer: Timer
}

#[derive(Component)]
struct RightArm;

#[derive(Component)]
struct AttackTimer(Timer);

#[derive(Component,Clone)]
struct Monster{
    monster_type: MonsterType
}

#[derive(Component)]
struct Floor(usize);

#[derive(Component)]
struct Item{
    item_type: ItemType
}

#[derive(Component)]
struct ThrowableBall;


#[derive(Component)]
struct ThrownBall {
    velocity: Vec3,
    lifetime: Timer,
    item_type: ItemType
}

#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(LoadMapAndItems(false))
        .insert_resource(CurrentFloor(0))
        .insert_resource(FloorParameters::new())
        .insert_resource(ItemAndMonsterParameters::new())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Yet Another Roguelike Tutorial in Rust with Bevy".to_string(),
                resolution: WindowResolution::new(920.0, 640.0),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_sub_state::<TransitionState>()
        .add_plugins(ThirdPersonCameraPlugin)
        .add_plugins((
            OrbitCameraPlugin,
            DungeonLightingPlugin,
            EguiPlugin))
        .add_plugins((
            UiPlugin,
            MonsterAIPlugin,
            FightingPlugin))
        .add_plugins((
            // Adds frame time diagnostics
            FrameTimeDiagnosticsPlugin,
            // Adds a system that prints diagnostics to the console
            //LogDiagnosticsPlugin::default(),
            // Any plugin can register diagnostics. Uncomment this to add an entity count diagnostics:
            // bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Uncomment this to add an asset count diagnostics:
            // bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default(),
            // Uncomment this to add system info diagnostics:
            // bevy::diagnostic::SystemInformationDiagnosticsPlugin::default()
        ))
        .add_systems(OnEnter(GameState::InGame), (setup_orbitcamera, setup))
        .insert_resource(ShowFps(false))
        .insert_resource(ShowPlayerValuesAndInventar(false))
        //.add_systems(Startup, place_torch_lights)
        .add_systems(Update, do_transition_stairsdown.run_if(in_state(TransitionState::StairsDown)))
        .add_systems(Update, debug.run_if(in_state(GameState::InGame)))
        .add_systems(Update,(
            move_player,
            player_item_colliding,
            player_use_item,
            throw_ball,
            update_thrown_ball,
            quit).run_if(in_state(GameState::InGame)))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    load_map_and_items: Res<LoadMapAndItems>,
    mut current_floor: ResMut<CurrentFloor>,
    floor_parameters: Res<FloorParameters>,
    item_and_monster_parameters: Res<ItemAndMonsterParameters>
) {
   // let game_map = GameMap::from_string(map_string).expect("Failed to parse level");
    let mut game_map = if load_map_and_items.0 {
        GameMap::load(MAP_TEXT_FILE)
    } else {
        GameMap::create_dungeon(MapGeneratorStart::new(80, 45,
                                                         current_floor.0,
                                                         MAX_ROOMS,
                                                         ROOM_MIN_SIZE,
                                                         ROOM_MAX_SIZE,
                                                         floor_parameters.items[current_floor.0].max_monsters_per_room,
                                                         floor_parameters.items[current_floor.0].max_items_per_room,
                                                         item_and_monster_parameters.parameters[current_floor.0].clone(),
                                                         None))
            .expect("Failed to create level")
    };

    //game_map.print();

    println!("Player position: ({}, {})", game_map.player_position.0, game_map.player_position.1);

    if load_map_and_items.0 {
        commands.insert_resource(Inventory::load(INVENTORY_JSON_FILE));
        current_floor.0 = CurrentFloor::load(FLOOR_JSON_FILE).0;
    } else {
        commands.insert_resource(Inventory::new());
    };

    setup_character(&mut commands, &mut meshes, &mut materials, &mut game_map, &load_map_and_items);

    // monster
    setup_monster(&mut commands, &current_floor, &mut meshes, &mut materials, &mut game_map);
    // item
    setup_item(&mut commands, &asset_server, &current_floor, &mut game_map);
    // ground
    game_map.generate(&mut commands, &mut current_floor, &asset_server,  &mut meshes, &mut materials);

    commands.insert_resource(game_map);
}

fn setup_monster(
    commands: &mut Commands,
    current_floor: &ResMut<CurrentFloor>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_map: &mut GameMap
) {
    for i in game_map.monsters.iter() {

        let position = game_map.grid_to_world(i.position.0, i.position.1);
        match i.monster_type {
            MonsterType::Troll => {
                commands.spawn((
                    Mesh3d(meshes.add(Mesh::from(Capsule3d::new(0.8, 1.5)))), // Large, bulky troll
                    MeshMaterial3d( materials.add(StandardMaterial { // Earthy, stone-like color
                        base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_xyz(position.x, 0.8, position.z),
                    Monster{monster_type:MonsterType::Troll}, // Component to identify the Troll
                    Actor::new(16, 16,1, 4,100),
                    MonsterAIState::Idle,
                    Floor(current_floor.0)
                )).insert(Name::new("troll")).with_children(|parent| {
                    // Front (rough, rocky appearance)
                    parent.spawn((
                                     Mesh3d( meshes.add(Mesh::from(Cuboid::new(0.4, 0.4, 0.4)))),
                         MeshMaterial3d(materials.add(StandardMaterial { // Earthy, stone-like color
                                base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            })),
                                     Transform::from_xyz(0.0, 0.7, -0.7)
                    )).insert(Name::new("troll-front"));

                    // Left Arm (massive, club-like)
                    parent.spawn((
                        Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.5, 1.2, 0.5)))), // Even larger arm
                        MeshMaterial3d( materials.add(StandardMaterial { // Earthy, stone-like color
                            base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_xyz(-1.0, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.3)),
                        Floor(current_floor.0)
                    )).insert(Name::new("troll-left-arm"));

                    // Right Arm with Giant Club
                    parent.spawn((
                            Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.5, 1.2, 0.5)))),
                            MeshMaterial3d( materials.add(StandardMaterial { // Earthy, stone-like color
                                base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            })),
                            Transform::from_xyz(1.0, 0.2, 0.0)
                                .with_rotation(Quat::from_rotation_x(0.3)),
                        RightArm,
                        Floor(current_floor.0)
                    )).insert(Name::new("troll-right-arm")).with_children(|arm| {
                        // Giant Club
                        arm.spawn(
                            (
                                Mesh3d( meshes.add(Mesh::from(Cuboid::new(0.3, 1.5, 0.3)))),
                                MeshMaterial3d( materials.add(StandardMaterial { // Earthy, stone-like color
                                    base_color: Color::srgba(0.4, 0.3, 0.2, 1.0),
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                })),
                                Transform::from_xyz(0.0, -1.0, -0.3)
                                    .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                                Floor(current_floor.0)
                            )).insert(Name::new("troll-sword"));
                    });
                });
            }
            _ => {
                commands.spawn((
                        Mesh3d( meshes.add(Mesh::from(Capsule3d::new(0.6, 1.2)))), // Slightly larger and bulkier
                        MeshMaterial3d( materials.add(StandardMaterial { // Greenish skin tone
                            base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_xyz(position.x, 0.8, position.z),
                    Monster{monster_type: MonsterType::Troll},
                    Actor::new(10,10, 0, 3,100),
                    MonsterAIState::Idle,
                    Floor(current_floor.0)
                )).insert(Name::new("orc")).with_children(|parent| {
                    // Front (more brutish look)
                    parent.spawn((
                            Mesh3d( meshes.add(Mesh::from(Cuboid::new(0.3, 0.3, 0.3)))), // Slightly larger
                            MeshMaterial3d( materials.add(StandardMaterial { // Greenish skin tone
                                base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            })),
                            Transform::from_xyz(0.0, 0.6, -0.6), // Adjusted position,
                        Floor(current_floor.0)
                    )).insert(Name::new("orc-front"));

                    // Left Arm (more muscular)
                    parent.spawn((
                        Mesh3d( meshes.add(Mesh::from(Cuboid::new(0.4, 1.0, 0.4)))), // Thicker arm
                        MeshMaterial3d( materials.add(StandardMaterial { // Greenish skin tone
                            base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_xyz(-0.8, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.2)), // Slight angle
                        Floor(current_floor.0)
                    )).insert(Name::new("orc-left-arm"));

                    // Right Arm with Battle Axe
                    parent.spawn((
                            Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.4, 1.0, 0.3)))), // Muscular arm
                            MeshMaterial3d(materials.add(StandardMaterial { // Greenish skin tone
                                base_color: Color::srgba(0.4, 0.6, 0.3, 1.0),
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            })),
                            Transform::from_xyz(0.8, 0.2, 0.0)
                                .with_rotation(Quat::from_rotation_x(0.2)), // Slight angle
                        RightArm,
                        Floor(current_floor.0)
                    )).insert(Name::new("orc-right-arm")).with_children(|arm| {
                        // Battle Axe replacing the sword
                        arm.spawn((
                                Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.2, 1.2, 0.3)))), // Larger, brutal weapon
                                MeshMaterial3d( materials.add(StandardMaterial {
                                    base_color: Color::srgba(0.5, 0.4, 0.3, 1.0),
                                    alpha_mode: AlphaMode::Blend,
                                    ..default()
                                })),
                               Transform::from_xyz(0.0, -0.7, -0.3)
                                    .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                            Floor(current_floor.0)
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
    hit_points: usize,
    defense: usize,
    power: usize,
}

const PLAYER_BODY_RADIUS: f32 = 0.5;
const PLAYER_BODY_LENGTH: f32 = 1.0;
fn setup_character(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_map: &mut GameMap,
    load_map_and_items: &Res<LoadMapAndItems>
) {
    let mut player_position = game_map.grid_to_world(game_map.player_position.0,
                                                     game_map.player_position.1);
    player_position.y = 0.9;

    let mut max_hit_points = 30;
    let mut hit_points = 30;
    let mut defense = 2;
    let mut power=5;

    if load_map_and_items.0 {
        let actor = Actor::load();
        max_hit_points = actor.max_hit_points;
        hit_points = actor.hit_points;
        defense = actor.defense;
        power = actor.power;
    }

    let character = Character{
        name: String::from("player"),
        body_radius : PLAYER_BODY_RADIUS,
        body_length: PLAYER_BODY_LENGTH,
        position: player_position,
        color: Color::srgb(0.2, 0.4, 0.8),
        max_hit_points,
        hit_points,
        defense,
        power
    };

    commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Capsule3d::new(character.body_radius,
                                                       character.body_length)))),
            MeshMaterial3d(materials.add(character.color)),
            Transform::from_translation(character.position),
        Player,
        HeadUpDisplay::new(),
        Actor::new (character.max_hit_points, character.hit_points, character.defense, character.power,0),
        Name::new(character.name)
    )).with_children(|parent| {
        //front
        parent.spawn((
                Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.25, 0.25, 0.25)))),
                MeshMaterial3d(materials.add(character.color)),
                Transform::from_xyz(0.0, 0.5, -0.5),
            Name::new("player-front")
        ));

        // Left Arm
        parent.spawn((
                Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.3, 0.8, 0.3)))),
                MeshMaterial3d(materials.add(character.color)),
                Transform::from_xyz(-0.7, 0.2, 0.0)
                    .with_rotation(Quat::from_rotation_x(0.0)),
            Name::new("player-left-arm")
        )).with_children(|arm| {
            // Wurfkugel an der linken Hand
            arm.spawn((
                Mesh3d( meshes.add(Mesh::from(Sphere::new(0.2)))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))), // Rote Kugel
                Transform::from_xyz(0.0, -0.5, 0.2),
                ThrowableBall,
                Visibility::Hidden,
                Name::new("player-throwball")
            ));
        });

        // Right Arm with Sword (two components)
        parent.spawn((
                Mesh3d(meshes.add(Mesh::from(Cuboid::new(0.3, 0.8, 0.3)))),
                MeshMaterial3d(materials.add(character.color)),
                Transform::from_xyz(0.7, 0.2, 0.0)
                    .with_rotation(Quat::from_rotation_x(0.0)),
            RightArm,
            Name::new("player-right-arm")
        )).with_children(|arm| {
            // Sword as a long cuboid attached to right arm
            arm.spawn((
                  Mesh3d( meshes.add(Mesh::from(Cuboid::new(0.1, 0.8, 0.1)))),
                    MeshMaterial3d(materials.add(Color::srgb(0.6, 0.6, 0.6))),
                    Transform::from_xyz(0.0, -0.5, -0.2)
                        .with_rotation(Quat::from_rotation_x(PI * 0.5)),
                Name::new("player-sword")
            ));
        });
    });
}

fn setup_item(
    commands: &mut Commands,
    asset_server:  &Res<AssetServer>,
    current_floor: &ResMut<CurrentFloor>,
    game_map: &mut GameMap
) {
    let heal_portion_handle:Handle<Scene> = asset_server.load("models/bottle_A_brown.gltf#Scene0");
    let trunk_handle:Handle<Scene> = asset_server.load("models/trunk_small_A.gltf#Scene0");

    for i in game_map.items.iter() {

        let position = game_map.grid_to_world(i.position.0, i.position.1);
        match i.item_type {
            ItemType::HealPotion => {
               commands.spawn((
                   SceneRoot(heal_portion_handle.clone()),
                   Transform {
                       translation:  Vec3::new(position.x,0.0,position.z),
                       //rotation: Quat::from_rotation_y(PI/2.0),
                       ..default()
                   },
                   Item{item_type: ItemType::HealPotion},
                     Floor(current_floor.0)
               ));
            }
            ItemType::Lightning => {
                commands.spawn((
                    SceneRoot(trunk_handle.clone()),
                    Transform {
                        translation:  Vec3::new(position.x,0.0,position.z),
                    //rotation: Quat::from_rotation_y(PI/2.0),
                    ..default()
                    },
                    Item{item_type: ItemType::Lightning},
                    Floor(current_floor.0)
                ));
            }
        }
    }
}

fn setup_orbitcamera(
    mut commands: Commands
){
    commands.spawn((
        Camera3d::default(),
        Camera{
            is_active:false,
            ..default()
            }
    ))
        .insert(OrbitCamera{
            distance : 28.0,
            ..default()
        })
        .insert(Name::new("OrbitCamera"));
}

fn debug(
    keyboard_input:Res<ButtonInput<KeyCode>>,
    mut show_fps: ResMut<ShowFps>,
    mut show_player_values_and_inventar: ResMut<ShowPlayerValuesAndInventar>,
    mut query: Query<&mut Camera>
)
{
    if keyboard_input.just_pressed(KeyCode::KeyO) {
        for mut camera in query.iter_mut() {
            camera.is_active = ! camera.is_active
        }
    } else if keyboard_input.just_pressed(KeyCode::KeyF) {
        show_fps.0 = !show_fps.0;
    } else if keyboard_input.just_pressed(KeyCode::KeyI) {
        show_player_values_and_inventar.0 = !show_player_values_and_inventar.0;
    }
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
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(Entity, &mut Transform), (With<Player>, Without<Monster>)>,
    mut attack_events: EventWriter<AttackEvent>,
    camera_query: Query<&Transform, (With<ThirdPersonCamera>, Without<Player>,Without<Monster>)>,
    monster_query: Query<&mut Transform, (With<Monster>, Without<Player>)>,
    game_map: Res<GameMap>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<TransitionState>>,
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

                        // Store the original forward directionmu
                        let original_forward = player_transform.forward().as_vec3();

                        // Update player position
                        player_transform.translation = player_without_colliding(
                            &mut commands,
                            &player_entity,
                            &game_map,
                            &mut next_state,
                            &monster_query,
                            player_transform.translation,
                            move_vector * time.delta_secs() * SPEED
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
    commands: &mut Commands,
    player: &Entity,
    game_map: &GameMap,
    next_state: &mut ResMut<NextState<TransitionState>>,
    monster_query: &Query<&mut Transform, (With<Monster>, Without<Player>)>,
    position:Vec3,
    move_vector:Vec3
)->Vec3{

    let new_position = position + move_vector;

    //stairs down
    let stairs_down = new_position;
    let map_stairs_down =  game_map.world_to_grid(stairs_down);
    if game_map.grid[map_stairs_down].tile_type == TileType::StaircaseDown {
        if game_map.grid_to_world(map_stairs_down.0,map_stairs_down.1).distance(stairs_down) <= PLAYER_DISTANCE * 2.0 {
            next_state.set(TransitionState::StairsDown);
            commands.entity(*player).insert(PlayerTransition {
                step: TransitionStep::StairDownStart,
                timer: Timer::new(Duration::from_secs_f32(1.0), TimerMode::Once)
            });
            return position;
        } else {
            return new_position;
        }

    }
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

fn player_item_colliding(
    mut commands: Commands,
    mut inventory: ResMut<Inventory>,
    player_query: Query<&Transform, (With<Player>, Changed<Transform>)>,
    mut throwball_query: Query<&mut Visibility, (With<ThrowableBall>, Without<ThrownBall>)>,
    mut item_query: Query<(Entity, &Item, &Transform), (With<Item>, Without<Player>)>
) {
    for player_transform in player_query.iter() {
        for (item_entity, item, item_transform) in item_query.iter_mut() {
            if player_transform.translation.distance(item_transform.translation) <= PLAYER_DISTANCE *2.0 {
                inventory.add_item(item.item_type.clone());
                if item.item_type == ItemType::Lightning {
                    if let Ok(mut ball_visibility) = throwball_query.get_single_mut() {
                        *ball_visibility = Visibility::Visible;
                    }
                }
                commands.entity(item_entity).despawn_recursive();
            }
        }
    }
}

fn player_use_item(
    keyboard_input:Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Actor, With<Player>>,
    mut inventory: ResMut<Inventory>,
)
{
    //Portion
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        if inventory.heal_potion > 0 {
            for mut actor in query.iter_mut() {
                if actor.hit_points < actor.max_hit_points {
                    inventory.remove_item(ItemType::HealPotion);
                    actor.hit_points = actor.max_hit_points.min(actor.hit_points+20);
                }
            }
        }
    };
}

const BALL_TEMPO:f32=8.0;
const BALL_RADIUS:f32=0.2;

fn throw_ball(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, (With<Player>, Without<ThrownBall>)>,
    mut throwball_query: Query<(&mut Visibility, &GlobalTransform), (With<ThrowableBall>, Without<ThrownBall>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inventory: ResMut<Inventory>
) {
    if keyboard_input.just_pressed(KeyCode::KeyX) {
        if let Ok(player_transform) = player_query.get_single() {
            if let Ok((mut ball_visibility, ball_global_transform)) = throwball_query.get_single_mut() {

                // Determine throw direction based on player orientation
                let mut throw_direction = player_transform.forward().as_vec3().normalize();
                throw_direction.y = 0.0;

                // Calculate start position using the global transformation of the ball
                let start_position = ball_global_transform.translation();

                // Remove the ball from the player
                *ball_visibility = Visibility::Hidden;

                // Spawn a new independent ball at the saved global position
                if let Some(active_item) = inventory.activ_item {
                    commands.spawn((
                        Mesh3d(meshes.add(Mesh::from(Sphere::new(BALL_RADIUS)))),
                        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))), // Rote Kugel
                        Transform::from_translation(start_position),
                        ThrownBall {
                            velocity: throw_direction * BALL_TEMPO,  // Throw velocity
                            lifetime: Timer::from_seconds(2.0, TimerMode::Once),
                            item_type: active_item
                        }
                    ));
                    inventory.remove_item(active_item);
                }
            }
        }
    }
}


const BALL_GRAVITY: f32 = 2.40665;

fn update_thrown_ball(
    mut commands: Commands,
    time: Res<Time>,
    mut ball_query: Query<(Entity, &mut Transform, &mut ThrownBall),Without<Monster>>,
    monster_query: Query<(Entity, &Transform), With<Monster>>,
    mut damage_events: EventWriter<DamageEvent>,
    game_map: Res<GameMap>, // Game world information for collision detection
) {
    let delta_time = time.delta_secs();

    for (entity, mut transform, mut ball) in ball_query.iter_mut() {
        // Update position: The ball moves in its direction with a given speed
        ball.velocity.y -= BALL_GRAVITY * delta_time; // Gravity pulls the ball downward
        transform.translation += ball.velocity * delta_time;

        // Reduce the lifetime of the ball
        ball.lifetime.tick(time.delta());
        if ball.lifetime.finished() {
            remove_ball(&mut commands, entity);
            continue;
        }

        //floor or wall
        if transform.translation.y < 0.0  ||
            game_map.collide_with_wall(transform.translation, BALL_RADIUS) {
            commands.entity(entity).despawn_recursive();
        } else if let Some(monster) = collide_with_monster(transform.translation,BALL_RADIUS,
                                                           &monster_query) {
            damage_events.send(DamageEvent {
                attacker: entity,
                target: monster,
                fixed_damage: 10
            });
            remove_ball(&mut commands, entity);
        }
    }
}

fn remove_ball(
    commands: &mut Commands,
    entity: Entity
) {
    commands.entity(entity).despawn();
}

fn collide_with_monster(
    position:Vec3,
    distance:f32,
    monster_query: &Query<(Entity, &Transform), With<Monster>>
)->Option<Entity>{

    for (monster,i) in monster_query.iter() {
        if position.distance(i.translation) <= distance {
            return Some(monster);
        }
    }
    None
}

fn quit(
    keyboard_input:Res<ButtonInput<KeyCode>>,
    game_map: Res<GameMap>,
    query_player: Query<(&Transform, &Actor), With<Player>>,
    query_item: Query<(&Item, &Transform), (With<Item>, Without<Player>)>,
    query_monster: Query<(&Monster, &Transform), (With<Monster>, Without<Player>)>,
    inventory: Res<Inventory>,
    current_floor: Res<CurrentFloor>
)
{
    if keyboard_input.just_pressed(KeyCode::KeyQ) {
        for (player,player_actor) in query_player.iter() {
            let mut items: Vec<(Vec3,ItemType)> = Vec::new();
            for (item, item_transform) in query_item.iter() {
                items.push((item_transform.translation.clone(),item.item_type.clone()));
            }
            let mut monsters:Vec<(Vec3,MonsterType)> = Vec::new();
            for (monster, monster_transform) in query_monster.iter() {
                monsters.push((monster_transform.translation.clone(),monster.monster_type.clone()))
            }
            game_map.save(
                player.translation.clone(),
                items,
                monsters
            );

            inventory.save();

            player_actor.save();

            current_floor.save();

            std::process::exit(0);
        }
    };
}

const TRANSITION_SPEED:f32=2.0;

fn do_transition_stairsdown(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut current_floor: ResMut<CurrentFloor>,
    floor_parameters: Res<FloorParameters>,
    item_and_monster_parameters: Res<ItemAndMonsterParameters>,
    mut game_map: ResMut<GameMap>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<TransitionState>>,
    mut player_query: Query<(Entity, &mut Transform, &mut PlayerTransition), With<Player>>,
    despawn_query: Query<Entity, (With<Floor>, Without<Player>)>,
){

    for(player_entity, mut player_transform, mut player_transition) in player_query.iter_mut() {
        match player_transition.step {
            TransitionStep::StairDownStart => {
                if player_transition.timer.finished() {
                    player_transition.step = TransitionStep::StairDownEnd;
                    player_transition.timer.set_duration(Duration::from_secs_f32(2.0));
                    //respawn current floor
                    despawn_current_floor(
                        &mut commands,
                         &despawn_query);
                    //generate next floor
                    current_floor.0 += 1;
                    setup_next_floor(
                        &mut commands,
                        &asset_server,
                        &mut meshes,
                        &mut materials,
                        &mut current_floor,
                        &floor_parameters,
                        &item_and_monster_parameters,
                        &mut game_map,
                        &mut player_transform);
                    player_transform.translation.y = 4.0 * PLAYER_DISTANCE;
                } else {
                    player_transition.timer.tick(time.delta());
                    let mut new_position = player_transform.translation.clone();
                    new_position.y -= time.delta_secs()*TRANSITION_SPEED;
                    player_transform.translation = new_position;
                }
            },
            TransitionStep::StairDownEnd => {
                if player_transition.timer.finished() {
                    //despawn player_transition
                    commands.entity(player_entity).remove::<PlayerTransition>();
                    next_state.set(TransitionState::Running);
                } else {
                    player_transition.timer.tick(time.delta());
                    let mut new_position = player_transform.translation.clone();
                    new_position.y -= time.delta_secs()*TRANSITION_SPEED;
                    player_transform.translation = new_position;
                    if player_transform.translation.y < 0.0 {
                        player_transform.translation.y = 0.0;
                    }
                }
            }
        }
    }
}

fn setup_next_floor(
    mut commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
    current_floor: &mut ResMut<CurrentFloor>,
    floor_parameters: &Res<FloorParameters>,
    item_and_monster_parameters: &Res<ItemAndMonsterParameters>,
    mut game_map: &mut ResMut<GameMap>,
    player: &mut Transform
) {

    let player_position = game_map.world_to_grid(player.translation.clone());
    **game_map = GameMap::create_dungeon(MapGeneratorStart::new(80, 45,
                                                                current_floor.0,
                                                       MAX_ROOMS,
                                                       ROOM_MIN_SIZE,
                                                       ROOM_MAX_SIZE,
                                                         floor_parameters.items[current_floor.0].max_monsters_per_room,
                                                            floor_parameters.items[current_floor.0].max_items_per_room,
                                                                item_and_monster_parameters.parameters[current_floor.0].clone(),
                                                                Some(player_position)
    ))
            .expect("Failed to create level");

    // monster
    setup_monster(&mut commands, &current_floor, &mut meshes, &mut materials, &mut game_map);
    // item
    setup_item(&mut commands, &asset_server, &current_floor, &mut game_map);

    // ground
    game_map.generate(&mut commands, current_floor, asset_server,  meshes, materials);
}

fn despawn_current_floor(
    commands: &mut Commands,
    query: &Query<Entity, (With<Floor>,Without<Player>)>
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}