use serde::{Deserialize, Serialize};

/// Biome classification derived from climate proxies and elevation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Biome {
    #[default]
    Ocean,
    Beach,
    Grassland,
    Forest,
    Tundra,
    Desert,
    Mountain,
}

/// A single cell in the world grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Elevation relative to sea level. Negative = submerged.
    pub elevation: f32,
    /// Derived water depth (sea_level - elevation, clamped >= 0).
    pub water_depth: f32,
    /// Temperature proxy.
    pub temp: f32,
    /// Moisture proxy.
    pub moisture: f32,
    /// Fertility proxy.
    pub fertility: f32,
    /// Toxicity proxy (reserved for future use, 0.0 in MVP).
    pub toxicity: f32,
    /// Derived biome classification.
    pub biome: Biome,
    /// Fungus coverage [0.0, 1.0].
    pub fungus: f32,
    /// Plant coverage [0.0, 1.0].
    pub plants: f32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            elevation: -1.0,
            water_depth: 1.0,
            temp: 0.5,
            moisture: 0.5,
            fertility: 0.0,
            toxicity: 0.0,
            biome: Biome::Ocean,
            fungus: 0.0,
            plants: 0.0,
        }
    }
}

/// The world grid. Toroidal (wrapping) 2D grid of cells.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    cells: Vec<Cell>,
}

impl Grid {
    /// Create a new grid with default cells.
    pub fn new(width: u32, height: u32) -> Self {
        let cells = vec![Cell::default(); (width * height) as usize];
        Self {
            width,
            height,
            cells,
        }
    }

    /// Convert (x, y) to a flat index with toroidal wrapping.
    #[inline]
    pub fn index(&self, x: i32, y: i32) -> usize {
        let wx = x.rem_euclid(self.width as i32) as u32;
        let wy = y.rem_euclid(self.height as i32) as u32;
        (wy * self.width + wx) as usize
    }

    /// Get a cell reference by grid coordinates (wrapping).
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    /// Get a mutable cell reference by grid coordinates (wrapping).
    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut Cell {
        let idx = self.index(x, y);
        &mut self.cells[idx]
    }

    /// Direct access to the cell slice (for bulk reads).
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Direct mutable access to the cell slice.
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    /// Iterate over all cells with their (x, y) coordinates.
    pub fn iter_coords(&self) -> impl Iterator<Item = (u32, u32, &Cell)> {
        self.cells.iter().enumerate().map(move |(i, cell)| {
            let x = i as u32 % self.width;
            let y = i as u32 / self.width;
            (x, y, cell)
        })
    }

    /// Get the 4 cardinal neighbor coordinates (with wrapping).
    pub fn neighbors4(&self, x: i32, y: i32) -> [(i32, i32); 4] {
        [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toroidal_wrapping() {
        let grid = Grid::new(10, 10);
        assert_eq!(grid.index(0, 0), grid.index(10, 10));
        assert_eq!(grid.index(-1, 0), grid.index(9, 0));
        assert_eq!(grid.index(0, -1), grid.index(0, 9));
    }

    #[test]
    fn test_grid_size() {
        let grid = Grid::new(16, 8);
        assert_eq!(grid.cells().len(), 128);
    }
}
