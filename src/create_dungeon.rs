use bevy::math::Vec3;
use petgraph::graph::{Graph, NodeIndex};
use rand::Rng;

use crate::{GameMap, TileMapping, Tile, TileType, Grid, MonsterInMap, MonsterType, ItemInMap, ItemType, Item};



#[derive(Debug)]
pub struct BresenhamLine {
    /*
        Bresenham algorithm https://de.wikipedia.org/wiki/Bresenham-Algorithmus
        No diagonal allow https://stackoverflow.com/questions/4381269/line-rasterisation-cover-all-pixels-regardless-of-line-gradient
    */
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    dx: i32,
    sx: i32,
    dy: i32,
    sy: i32,
    err: i32,
    diagonal_allow: bool,
    finished: bool,
}

impl BresenhamLine {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32, diagonal_allow: bool) -> Self {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let err = dx + dy;

        BresenhamLine {
            x0,
            y0,
            x1,
            y1,
            dx,
            sx,
            dy,
            sy,
            err,
            diagonal_allow,
            finished: false,
        }
    }
}

impl Iterator for BresenhamLine {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let current_point = (self.x0, self.y0);

        if self.x0 == self.x1 && self.y0 == self.y1 {
            self.finished = true;
            return Some(current_point);
        }

        let e2 = 2 * self.err;
        let mut y_walk = false;

        if e2 > self.dy {
            self.err += self.dy;
            self.x0 += self.sx;
            y_walk = true;
        }

        if self.diagonal_allow || (!y_walk && !self.diagonal_allow) {
            if e2 < self.dx {
                self.err += self.dx;
                self.y0 += self.sy;
            }
        }

        Some(current_point)
    }
}

#[derive(Clone)]
struct Room{
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    center: (usize, usize)
}

impl Room{
    fn new(x: usize, y: usize, width: usize, height: usize) -> Self {

        let x1 = x;
        let y1 = y;
        let x2 = x + width;
        let y2 = y + height;
        let center = ((x1 + x2) / 2, (y1 + y2) / 2);

        Room{
            x1,
            y1,
            x2,
            y2,
            center
        }
    }

    fn inner(&self) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        // Returns the inner area of this room as 2D array indices
        (self.x1 + 1..self.x2, self.y1 + 1..self.y2)
    }

    fn width(&self) -> usize {
        self.x2.abs_diff(self.x1)
    }

    fn height(&self) -> usize {
        self.y2.abs_diff(self.y1)
    }

    fn fill_grid(&self,grid: &mut Grid){
        let (x_range, y_range) = self.inner();
        for x in x_range {
            for y in y_range.clone() {
                grid[(x,y)].tile_type =  TileType::Floor;
            }
        }
    }

    fn create_tunnel(&self,grid: &mut Grid,other:&Room){
        //create an L-shaped tunnel between these two rooms
        // Move vertically, then horizontally.
        let center1 = self.center;
        let center2 = other.center;
        let mut corner_x = center1.0;
        let mut corner_y = center2.1;

        let horizontal: bool = rand::thread_rng().gen();
        if horizontal {
            // Move horizontally, then vertically.
            corner_x = center2.0;
            corner_y = center1.1;
        }

        //create tunnel
        let line = BresenhamLine::new(center1.0 as i32, center1.1 as i32,
                                      corner_x as i32, corner_y as i32, true);

        for point in line {
            grid[(point.0 as usize, point.1 as usize)].tile_type =  TileType::Floor;
        };

        let line = BresenhamLine::new(corner_x as i32, corner_y as i32,
                                      center2.0 as i32, center2.1 as i32, true);

        for point in line {
            grid[(point.0 as usize, point.1 as usize)].tile_type =  TileType::Floor;
        };

    }

    /*fn create_in_area(area: &Area) -> Room {
        let mut rng = rand::thread_rng();

        let width = rng.gen_range(ROOM_MIN_SIZE..=ROOM_MAX_SIZE.min(area.width - 2));
        let height = rng.gen_range(ROOM_MIN_SIZE..=ROOM_MAX_SIZE.min(area.height - 2));

        let x_offset = rng.gen_range(0..=(area.width - width));
        let y_offset = rng.gen_range(0..=(area.height - height));

        Room::new(
            area.x + x_offset,
            area.y + y_offset,
            width,
            height
        )
    }*/

    fn intersects(&self, other: &Room) -> bool{
        //Return True if this room overlaps with another RectangularRoom.

        !(self.x2 <= other.x1 ||       // Self is left from other
            other.x2 <= self.x1 ||       // Other is left from self
            self.y2 <= other.y1 ||       // self is over other
            other.y2 <= self.y1)
    }

}

#[derive(Debug, Clone, PartialEq)]
struct Area {
    x: usize,
    y: usize,
    width: usize,
    height: usize
}

impl Area {
    fn split_area(
        &self,
        graph: &mut Graph<(Area, Option<Room>), ()>,
        node: NodeIndex,
        min_width: usize,
        min_height: usize
    ) {
        if self.width <= 20 || self.height <= 20 {
            return;
        }

        let mut rng = rand::thread_rng();
        let split_vertical = rng.gen_bool(0.5);

        if split_vertical {
            if self.width > min_width {
                let split = rng.gen_range(10..=(self.width - 10));
                let left = Area {
                    x: self.x,
                    y: self.y,
                    width: split,
                    height: self.height,
                };
                let right = Area {
                    x: self.x + split,
                    y: self.y,
                    width: self.width - split,
                    height: self.height,
                };
                let left_node = graph.add_node((left.clone(), None));
                let right_node = graph.add_node((right.clone(), None));
                graph.add_edge(node, left_node, ());
                graph.add_edge(node, right_node, ());
                left.split_area(graph, left_node, min_width, min_height);
                right.split_area(graph, right_node, min_width, min_height);
            }
        } else {
            if self.height > min_height {
                let split = rng.gen_range(10..=(self.height - 10));
                let top = Area {
                    x: self.x,
                    y: self.y,
                    width: self.width,
                    height: split,
                };
                let bottom = Area {
                    x: self.x,
                    y: self.y + split,
                    width: self.width,
                    height: self.height - split,
                };
                let top_node = graph.add_node((top.clone(), None));
                let bottom_node = graph.add_node((bottom.clone(), None));
                graph.add_edge(node, top_node, ());
                graph.add_edge(node, bottom_node, ());
                top.split_area(graph, top_node, min_width, min_height);
                bottom.split_area(graph, bottom_node, min_width, min_height);
            }
        }
    }
}

fn get_random_leaf_room_bsp(
    graph: &Graph<(Area, Option<Room>), ()>,
    node: NodeIndex
) -> Option<(NodeIndex, Room)> {
    let mut rng = rand::thread_rng();
    let mut leaf_rooms = Vec::new();

    if graph.neighbors(node).count() == 0 {
        if let Some(room) = &graph[node].1 {
            return Some((node, room.clone()));
        }
    } else {
        for neighbor in graph.neighbors(node) {
            if let Some((leaf_node, room)) = get_random_leaf_room_bsp(graph, neighbor) {
                leaf_rooms.push((leaf_node, room));
            }
        }
    }

    if leaf_rooms.is_empty() {
        None
    } else {
        Some(leaf_rooms[rng.gen_range(0..leaf_rooms.len())].clone())
    }
}

fn get_first_room_bsp(graph: &Graph<(Area, Option<Room>), ()>, root: NodeIndex) -> Option<Room> {
    if graph.neighbors(root).count() == 0 {
        return graph[root].1.clone();
    }

    for neighbor in graph.neighbors(root) {
        if let Some(room) = get_first_room_bsp(graph, neighbor) {
            return Some(room);
        }
    }

    None
}
pub trait DungeonGeneratorStrategy {
    fn generate(&self) -> Result<GameMap, String>;
}

pub struct StringMapGenerator {
    pub map_string: String,
}

impl StringMapGenerator {
    pub fn new(map_string: &str) -> Self {
        StringMapGenerator {
            map_string: map_string.to_string(),
        }
    }
}

impl DungeonGeneratorStrategy for StringMapGenerator {
     fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();
        let lines: Vec<&str> = self.map_string.trim().split('\n').collect();

        if lines.is_empty() {
            return Err("Empty map".to_string());
        }

        let height = lines.len();
        let width = lines[0].len();

        let mut grid = Grid::new(height, width, TileType::Empty);
        let mut player_position: (usize, usize) = (0,0);

        for (y, line) in lines.iter().enumerate() {
            let row: Vec<Tile> = line
                .chars()
                .enumerate()
                .map(|(x, ch)| {
                    let tile_type = tile_mapping.get_tile_type(ch);
                    let tile = Tile::new(tile_type);

                    if tile.tile_type == TileType::Player {
                        player_position = (x, y);
                    }
                    tile
                })
                .collect();

            for (x,tile) in row.iter().enumerate(){
                grid[(x,y)] = tile.clone();
            }

        }

        grid[player_position] = Tile::new(TileType::Floor);

        let x_center = width / 2;
        let y_center = height / 2;

        let center = (x_center, y_center);

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters: Vec::new(),
            items: Vec::new(),
            center,
            width,
            height
        })
    }
}



pub struct MapGeneratorFirst {
    width: usize,
    height: usize
}

impl MapGeneratorFirst {
    pub fn new(width:usize, height:usize) -> Self {
        MapGeneratorFirst{
            width,
            height
        }
    }
}

impl DungeonGeneratorStrategy for MapGeneratorFirst {
    fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();

        let grid = Grid::new(self.width,self.height,TileType::Floor);

        let x_center = self.width / 2;
        let y_center = self.height / 2;

        let center = (x_center, y_center);

        let player_position: (usize, usize) = (x_center,y_center);

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters: Vec::new(),
            items: Vec::new(),
            center,
            width: self.width,
            height: self.height
        })
    }
}


pub struct MapGeneratorSecond {
    width: usize,
    height: usize
}

impl MapGeneratorSecond {
    pub fn new(width:usize, height:usize) -> Self {
        MapGeneratorSecond {
            width,
            height
        }
    }
}

impl DungeonGeneratorStrategy for MapGeneratorSecond {
    fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();

        let mut grid = Grid::new(self.width,self.height,TileType::Wall);

        let x_center = self.width / 2;
        let y_center = self.width / 2;

        let center = (x_center, y_center);

        let room_1 = Room::new(20, 15, 10, 15);
        let room_2 = Room::new(35, 15, 10, 15);

        room_1.fill_grid(&mut grid);
        room_2.fill_grid(&mut grid);

        let player_position: (usize, usize) = room_1.center.clone();

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters: Vec::new(),
            items: Vec::new(),

            center,
            width: self.width,
            height: self.height
        })
    }
}


pub struct MapGeneratorThird {
    width: usize,
    height: usize
}

impl MapGeneratorThird {
    pub fn new(width:usize, height:usize) -> Self {
        MapGeneratorThird {
            width,
            height
        }
    }
}

impl DungeonGeneratorStrategy for MapGeneratorThird {
    fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();

        let mut grid = Grid::new(self.width,self.height,TileType::Wall);

        let x_center = self.width / 2;
        let y_center = self.height / 2;

        let center = (x_center, y_center);

        let room_1 = Room::new(20, 15, 10, 15);
        let room_2 = Room::new(35, 15, 10, 15);

        room_1.fill_grid(&mut grid);
        room_2.fill_grid(&mut grid);

        room_1.create_tunnel(&mut grid, &room_2);

        let player_position: (usize, usize) = room_1.center.clone();

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters: Vec::new(),
            items: Vec::new(),
            center,
            width: self.width,
            height: self.height
        })
    }
}

pub struct MapGeneratorStart1 {
    width: usize,
    height: usize,
    max_rooms: usize,
    room_min_size: usize,
    room_max_size: usize
}

impl MapGeneratorStart1 {
    pub fn new(width:usize, height:usize,
               max_rooms: usize,
               room_min_size: usize,
               room_max_size: usize) -> Self {
        MapGeneratorStart1 {
            width,
            height,
            max_rooms,
            room_min_size,
            room_max_size
        }
    }
}

impl DungeonGeneratorStrategy for MapGeneratorStart1 {
    fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();

        let mut grid = Grid::new(self.width,self.height,TileType::Wall);

        let x_center = self.width / 2;
        let y_center = self.height / 2;

        let center = (x_center, y_center);

        let mut player_position: (usize, usize) = (0, 0);

        let mut rooms: Vec<Room> = Vec::new();

        let mut rng = rand::thread_rng();

        for _ in 0..self.max_rooms {
            let room_width = rng.gen_range(self.room_min_size..=self.room_max_size);
            let room_height = rng.gen_range(self.room_min_size..=self.room_max_size);

            let x = rng.gen_range(0..(self.width - room_width - 1));
            let y = rng.gen_range(0..(self.height - room_height - 1));

            let new_room = Room::new(x, y, room_width, room_height);

            // Run through the other rooms and see if they intersect with this one.
            let mut intersection_found = false;
            for room in rooms.iter() {
                if room.intersects(&new_room) {
                    intersection_found = true;
                }
            }

            if !intersection_found {
                new_room.fill_grid(&mut grid);

                if rooms.len() == 0 {
                    // The first room, where the player starts.
                    player_position = new_room.center.clone();
                } else {
                    // Dig out a tunnel between this room and the previous one.
                    new_room.create_tunnel(&mut grid, rooms.last().unwrap());
                }

                rooms.push(new_room);
            }
        }

        //remove walls
        remove_walls(self.width, self.height, &mut grid);

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters: Vec::new(),
            items: Vec::new(),
            center,
            width: self.width,
            height: self.height
        })
    }
}


pub struct MapGeneratorStart {
    width: usize,
    height: usize,
    max_rooms: usize,
    room_min_size: usize,
    room_max_size: usize,
    max_monsters_per_room: usize,
    max_items_per_room: usize
}

impl MapGeneratorStart {
    pub fn new(width:usize, height:usize,
               max_rooms: usize,
               room_min_size: usize,
               room_max_size: usize,
               max_monsters_per_room: usize,
               max_items_per_room: usize) -> Self {
        MapGeneratorStart {
            width,
            height,
            max_rooms,
            room_min_size,
            room_max_size,
            max_monsters_per_room,
            max_items_per_room
        }
    }
}

impl DungeonGeneratorStrategy for MapGeneratorStart {
    fn generate(&self) -> Result<GameMap, String> {
        let tile_mapping = TileMapping::new();

        let mut grid = Grid::new(self.width,self.height,TileType::Wall);

        let x_center = self.width / 2;
        let y_center = self.height / 2;

        let center = (x_center, y_center);

        let mut player_position: (usize, usize) = (0, 0);

        let mut rooms: Vec<Room> = Vec::new();

        let mut rng = rand::thread_rng();

        for _ in 0..self.max_rooms {
            let room_width = rng.gen_range(self.room_min_size..=self.room_max_size);
            let room_height = rng.gen_range(self.room_min_size..=self.room_max_size);

            let x = rng.gen_range(0..(self.width - room_width - 1));
            let y = rng.gen_range(0..(self.height - room_height - 1));

            let new_room = Room::new(x, y, room_width, room_height);

            // Run through the other rooms and see if they intersect with this one.
            let mut intersection_found = false;
            for room in rooms.iter() {
                if room.intersects(&new_room) {
                    intersection_found = true;
                }
            }

            if !intersection_found {
                new_room.fill_grid(&mut grid);

                if rooms.len() == 0 {
                    // The first room, where the player starts.
                    player_position = new_room.center.clone();
                } else {
                    // Dig out a tunnel between this room and the previous one.
                    new_room.create_tunnel(&mut grid, rooms.last().unwrap());
                }

                rooms.push(new_room);
            }
        }

        //add monsters
        let monsters = add_monsters(&grid, &rooms, self.max_monsters_per_room);

        //add items
        let items = add_items(&grid, &rooms, self.max_items_per_room);

        //remove walls
        remove_walls(self.width, self.height, &mut grid);

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            monsters,
            items,
            center,
            width: self.width,
            height: self.height
        })
    }
}

fn add_items(grid: &Grid, rooms: &Vec<Room>,items_per_room:usize) -> Vec<ItemInMap> {
    let mut items:Vec<ItemInMap> = Vec::new();

    let mut rng = rand::thread_rng();

    //For each room 0 and a maximum items
    for room in rooms {
        let items_per_room = rng.gen_range(0..=items_per_room);
        for _ in 0..items_per_room {
            let position = (rng.gen_range(room.x1+1..room.x2),
                            rng.gen_range(room.y1+1..room.y2));
            let item_type = if rng.gen::<f32>() < 0.6 {
                ItemType::HealPotion
            } else {
                ItemType::Lightning
            };

            items.push(ItemInMap{
                item_type: ItemType::HealPotion,
                position
            })
        }
    }
    items
}

fn add_monsters(grid: &Grid, rooms: &Vec<Room>,max_monsters_per_room:usize) -> Vec<MonsterInMap> {
    let mut monsters:Vec<MonsterInMap> = Vec::new();

    let mut rng = rand::thread_rng();

    //For each room 0 and a maximum monsters
    for room in rooms {
        let monsters_per_room = rng.gen_range(0..=max_monsters_per_room);
        for _ in 0..monsters_per_room {
            // 80% Orc (a weaker enemy)
            // 20%  a Troll
            let monster_type: MonsterType = if rng.gen::<f32>() < 0.8 {
                MonsterType::Orc
            } else {
                MonsterType::Troll
            };
            let position = (rng.gen_range(room.x1+1..room.x2),
                                         rng.gen_range(room.y1+1..room.y2));
            monsters.push(MonsterInMap{
                monster_type,
                position
            })
        }
    }
    monsters
}

fn remove_walls(width:usize, height:usize, grid: &mut Grid){
    for x in 0..width {
        for y in 0..height {
            if y != 0 { //up
                if grid[(x, y - 1)].tile_type == TileType::Floor {
                    continue;
                }
            }
            if y != height - 1 {  //down
                if grid[(x, y + 1)].tile_type == TileType::Floor {
                    continue;
                }
            }
            if x != 0 { //left
                if grid[(x - 1, y)].tile_type == TileType::Floor {
                    continue;
                }
            }
            if x != width - 1 {  //right
                if grid[(x + 1, y)].tile_type == TileType::Floor {
                    continue;
                }
            }
            grid[(x, y)].tile_type = TileType::Empty
        }
    }
}

/*

    fn create_dungeon_bsp(width:usize,
                      height:usize,
                      max_rooms: usize,
                      room_min_size: usize,
                      room_max_size: usize
    ) -> Result<Self, String> {
        let tile_mapping = TileMapping::new();

        let mut grid: Vec<Vec<Tile>> = vec![vec![Tile::new(TileType::Wall); width]; height];

        let x_center = width / 2;
        let y_center = height / 2;

        let center = (x_center, y_center);

        // Initialize a directed graph with nodes of type Area
        let mut graph = Graph::<(Area, Option<Room>), ()>::new();

        //start with the entire dungeon area // root node of the BSP tree
        let root_area = Area {
            x: 0,
            y: 0,
            width: width,
            height: height,
        };

        let root = graph.add_node((root_area.clone(), None));

        // Start the recursive splitting process
        root_area.split_area(&mut graph, root,ROOM_MIN_SIZE, ROOM_MAX_SIZE);

        // create a room within the cell by randomly choosing two points ("top leftand bottom right ") within its boundaries
        for node in graph.node_indices() {
            let area = &graph[node].0;
            let room = Room::create_in_area(area);
            room.fill_grid(&mut grid);
            graph[node].1 = Some(room);
        }

        //starting from the lowest layers , draw corridors to connect rooms in the nodes of the BSP tree with children of the same parent
        //until the children of the root node are connected
        let mut nodes_to_process = vec![root];

        while let Some(current_node) = nodes_to_process.pop() {
            let children: Vec<_> = graph.neighbors(current_node).collect();

            if children.len() == 2 {
                if graph.neighbors(children[0]).count() == 0 && graph.neighbors(children[1]).count() == 0 {
                    // both are leafs
                    if let (Some(room1), Some(room2)) = (&graph[children[0]].1, &graph[children[1]].1) {
                        room1.create_tunnel(&mut grid, room2);
                    }
                } else {
                    // there are more nodes
                    if let (Some((_, room1)), Some((_, room2))) = (
                        get_random_leaf_room_bsp(&graph, children[0]),
                        get_random_leaf_room_bsp(&graph, children[1])
                    ) {
                        room1.create_tunnel(&mut grid, &room2);
                    }
                }
                nodes_to_process.extend(children);
            }
        }

        let mut player_position: (usize, usize) =(0,0);

        if let Some(first_room) = get_first_room_bsp(&graph, root) {
            player_position  = first_room.center;
        }

        //remove walls
        for x in 0..width {
            for y in 0..height  {
                if y != 0 { //up
                    if grid[y-1][x].tile_type == TileType::Floor {
                        continue;
                    }
                }
                if y!=height {  //down
                    if grid[y+1][x].tile_type == TileType::Floor {
                        continue;
                    }
                }
                if x!=0 { //left
                    if grid[y][x-1].tile_type == TileType::Floor {
                        continue;
                    }
                }
                if x!=width{  //right
                    if grid[y][x+1].tile_type == TileType::Floor {
                        continue;
                    }
                }
                grid[y][x].tile_type = TileType::Empty

            }
        }

        Ok(GameMap {
            grid,
            tile_mapping,
            player_position,
            center,
            width,
            height
        })
    }


*/

pub struct DungeonWriter{}

impl Default for DungeonWriter {
    fn default() -> Self {
        DungeonWriter{}
    }
}
impl DungeonWriter {
    pub fn write(
        &self, game_map: &GameMap,
        player:Vec3,
        items:Vec<(Vec3,ItemType)>,
        monsters:Vec<(Vec3,MonsterType)>
    )->String {

        let mut map:Vec<Vec<char>> = vec![];

        //grid
        for y in 0..game_map.grid.height() {
            let mut row: Vec<char> = vec![];
            for x in 0..game_map.grid.width() {
                let ch = game_map.tile_mapping.get_char(&game_map.grid[(x,y)].tile_type);
                row.push(ch)
            };
            map.push(row);
        }

        //item
        for (item_position, item_type) in items {
            let item_grid = game_map.world_to_grid(item_position);
            map[item_grid.1][item_grid.0] = game_map.tile_mapping.get_char(&item_type.to_tile_type());
        };

        //monster
        for (monster_position, monster_type) in monsters {
            let monster_grid = game_map.world_to_grid(monster_position);
            map[monster_grid.1][monster_grid.0] = game_map.tile_mapping.get_char(&monster_type.to_tile_type());
        }

        //player
        let player_grid = game_map.world_to_grid(player);
        map[player_grid.1][player_grid.0] = game_map.tile_mapping.get_char(&TileType::Player);

        map.iter()
            .map(|row| row.iter().collect())
            .collect::<Vec<String>>()
            .join("\n")
    }
}