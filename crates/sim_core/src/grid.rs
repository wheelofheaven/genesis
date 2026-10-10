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
    /// Prescribed regional rainfall proxy; zero preserves the legacy climate.
    #[serde(default)]
    pub rainfall: f32,
    /// Generated upstream rainfall accumulation, in abstract catchment units.
    #[serde(default)]
    pub drainage: f32,
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
            rainfall: 0.,
            drainage: 0.,
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

/// Boundary conditions for a local field. Earth regions are bounded windows;
/// only legacy fixtures and synthetic islands use toroidal wrapping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boundary {
    #[default]
    Wrap,
    Clamp,
}

/// Environmental samples with explicit boundary conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub boundary: Boundary,
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
            boundary: Boundary::Wrap,
        }
    }

    /// Convert (x, y) using the configured wrapping or bounded-edge policy.
    #[inline]
    pub fn index(&self, x: i32, y: i32) -> usize {
        let coordinate = |v: i32, size: u32| match self.boundary {
            Boundary::Wrap => v.rem_euclid(size as i32),
            Boundary::Clamp => v.clamp(0, size as i32 - 1),
        } as u32;
        let wx = coordinate(x, self.width);
        let wy = coordinate(y, self.height);
        (wy * self.width + wx) as usize
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
    }
    /// Get a cell reference using this field's boundary conditions.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> &Cell {
        &self.cells[self.index(x, y)]
    }

    /// Get a mutable cell using the configured boundary policy.
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

    /// Get cardinal neighbour coordinates; access applies the boundary policy.
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
    fn bounded_regions_do_not_connect_opposite_edges() {
        let mut grid = Grid::new(10, 10);
        grid.boundary = Boundary::Clamp;
        grid.get_mut(0, 5).elevation = -10.;
        grid.get_mut(9, 5).elevation = 20.;
        assert_eq!(grid.get(-1, 5).elevation, -10.);
        assert_eq!(grid.get(10, 5).elevation, 20.);
        let h = crate::spatial::elevation(&grid, crate::spatial::Position::new(10., 5.));
        assert!(h > 10.);
        crate::rules::apply_terraform(&mut grid, 0, 5, 2, 1.);
        assert_eq!(grid.get(0, 5).elevation, -9.);
        assert_eq!(grid.get(9, 5).elevation, 20.);
    }
    #[test]
    fn test_grid_size() {
        let grid = Grid::new(16, 8);
        assert_eq!(grid.cells().len(), 128);
    }
}
