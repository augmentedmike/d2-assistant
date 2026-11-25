//! Inventory and stash grid occupancy helpers.

/// Mark the cells covered by an item, clipping at grid bounds.
pub(crate) fn mark_grid(
    grid: &mut [bool],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    item_width: usize,
    item_height: usize,
) {
    if width == 0 || grid.len() < width.saturating_mul(height) || x >= width || y >= height {
        return;
    }
    for row in y..y.saturating_add(item_height).min(height) {
        for column in x..x.saturating_add(item_width).min(width) {
            grid[row * width + column] = true;
        }
    }
}

/// Find and reserve the first free rectangle in row-major order.
pub(crate) fn find_grid_space(
    grid: &mut [bool],
    width: usize,
    height: usize,
    item_width: usize,
    item_height: usize,
) -> Option<(usize, usize)> {
    if width == 0
        || height == 0
        || item_width == 0
        || item_height == 0
        || grid.len() < width.checked_mul(height)?
        || item_width > width
        || item_height > height
    {
        return None;
    }
    for y in 0..=height - item_height {
        for x in 0..=width - item_width {
            let empty = (y..y + item_height)
                .all(|row| (x..x + item_width).all(|column| !grid[row * width + column]));
            if empty {
                mark_grid(grid, width, height, x, y, item_width, item_height);
                return Some((x, y));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserves_first_available_rectangle_in_row_major_order() {
        let mut grid = vec![false; 12];
        assert_eq!(find_grid_space(&mut grid, 4, 3, 2, 2), Some((0, 0)));
        assert_eq!(find_grid_space(&mut grid, 4, 3, 2, 2), Some((2, 0)));
        assert_eq!(find_grid_space(&mut grid, 4, 3, 2, 2), None);
    }

    #[test]
    fn marks_only_cells_inside_the_grid() {
        let mut grid = vec![false; 6];
        mark_grid(&mut grid, 3, 2, 2, 1, 3, 2);
        assert_eq!(grid, vec![false, false, false, false, false, true]);
    }

    #[test]
    fn rejects_zero_sized_items_and_malformed_grid_buffers() {
        let mut grid = vec![false; 3];
        assert_eq!(find_grid_space(&mut grid, 3, 1, 0, 1), None);
        assert_eq!(find_grid_space(&mut grid, 3, 2, 1, 1), None);
        assert_eq!(find_grid_space(&mut grid, 3, 1, 1, 1), Some((0, 0)));
    }

    #[test]
    fn refuses_items_larger_than_the_target_grid() {
        let mut grid = vec![false; 8];
        assert_eq!(find_grid_space(&mut grid, 4, 2, 5, 1), None);
        assert_eq!(find_grid_space(&mut grid, 4, 2, 2, 3), None);
    }
}
