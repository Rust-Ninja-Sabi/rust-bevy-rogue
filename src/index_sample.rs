// Definition of a simple Grid structure for numerical data
struct Grid {
    // Two-dimensional vector as internal data storage structure
    data: Vec<Vec<i32>>
}

// Implementation of the Index trait for immutable access
// Enables syntax like: let value = grid[(x, y)]
impl std::ops::Index<(usize, usize)> for Grid {
    // Defines the return type: reference to an i32
    type Output = i32;

    // Method called when accessing an element with square brackets
    // - self: Reference to the Grid instance
    // - (row, col): Tuple with row and column index
    // - Returns reference to the element at the specified position
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        // Direct access to the element
        // WARNING: Panics if indices are out of bounds!
        &self.data[row][col]
    }
}

// Implementation of the IndexMut trait for mutable access
// Extends the Index trait and allows modifications
// Enables syntax like: grid[(x, y)] = new_value
impl std::ops::IndexMut<(usize, usize)> for Grid {
    // Method called when assigning a value with square brackets
    // - self: Mutable reference to the Grid instance
    // - (row, col): Tuple with row and column index
    // - Returns mutable reference to the element so it can be modified
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        // Direct mutable access to the element
        // WARNING: Panics if indices are out of bounds!
        &mut self.data[row][col]
    }
}

// Alternative implementation for row indexing
impl std::ops::Index<usize> for Grid {
    // Return type is now an entire vector
    type Output = Vec<i32>;

    // Allows access to entire rows: grid[0]
    fn index(&self, row: usize) -> &Self::Output {
        // Returns reference to the complete row
        &self.data[row]
    }
}

fn main() {
    // Initialization of a Grid with example data
    let mut grid = Grid {
        data: vec![
            vec![1, 2, 3],   // First row
            vec![4, 5, 6],   // Second row
            vec![7, 8, 9]    // Third row
        ]
    };

    // Immutable access with tuple indexing
    // Reads value at position (1, 1) - here: 5
    let value = grid[(1, 1)];
    println!("Value at position (1, 1): {}", value);

    // Mutable access: Changes value at position (1, 1) to 99
    grid[(1, 1)] = 99;

    // Access entire row
    let row = grid[1];
    println!("Entire second row: {:?}", row);
}