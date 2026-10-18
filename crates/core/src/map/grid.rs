//! Grid tools: distances over the cells and connected regions.
//!
//! A grid is `n` by `n` cells, indexed `y * n + x`, row 0 in the south.
//! Distances are in millimetres between cell centres, by a 16-neighbour
//! chamfer (steps of 1, the diagonal and the knight's move), which keeps a
//! straight-line distance within about 2% of the true one.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Unreached.
pub const FAR: i32 = i32::MAX;

/// The 16 steps and their lengths in thousandths of a cell.
const STEPS: [(i32, i32, i32); 16] = [
    (1, 0, 1000),
    (-1, 0, 1000),
    (0, 1, 1000),
    (0, -1, 1000),
    (1, 1, 1414),
    (1, -1, 1414),
    (-1, 1, 1414),
    (-1, -1, 1414),
    (2, 1, 2236),
    (2, -1, 2236),
    (-2, 1, 2236),
    (-2, -1, 2236),
    (1, 2, 2236),
    (-1, 2, 2236),
    (1, -2, 2236),
    (-1, -2, 2236),
];

/// The cells a step passes beside, which must be open too, so a path does
/// not cut a corner of land.
fn beside(dx: i32, dy: i32) -> [(i32, i32); 2] {
    match (dx.abs(), dy.abs()) {
        (1, 1) => [(dx, 0), (0, dy)],
        (2, 1) => [(dx.signum(), 0), (dx.signum(), dy)],
        (1, 2) => [(0, dy.signum()), (dx, dy.signum())],
        _ => [(0, 0), (0, 0)],
    }
}

/// Distance from the nearest source cell to every cell, moving only
/// through `open` cells (sources count as open). Unreached cells are [`FAR`].
pub fn distances(n: usize, cell_mm: i32, sources: &[usize], open: &[bool]) -> Vec<i32> {
    distances_within(n, cell_mm, sources, open, FAR)
}

/// [`distances`], but cells farther than `limit` may be left [`FAR`].
pub fn distances_within(n: usize, cell_mm: i32, sources: &[usize], open: &[bool], limit: i32) -> Vec<i32> {
    let mut dist = vec![FAR; n * n];
    let mut heap = BinaryHeap::new();
    for &s in sources {
        dist[s] = 0;
        heap.push(Reverse((0i32, s as u32)));
    }
    let step_mm: Vec<i32> = STEPS
        .iter()
        .map(|&(_, _, k)| (cell_mm as i64 * k as i64 / 1000) as i32)
        .collect();
    let ni = n as i32;
    let ok = |x: i32, y: i32| x >= 0 && y >= 0 && x < ni && y < ni && open[(y * ni + x) as usize];
    while let Some(Reverse((d, i))) = heap.pop() {
        let i = i as usize;
        if d > dist[i] || d > limit {
            continue;
        }
        let (x, y) = ((i % n) as i32, (i / n) as i32);
        for (s, &(dx, dy, _)) in STEPS.iter().enumerate() {
            let (tx, ty) = (x + dx, y + dy);
            if !ok(tx, ty) {
                continue;
            }
            if beside(dx, dy).iter().any(|&(bx, by)| !ok(x + bx, y + by)) {
                continue;
            }
            let j = (ty * ni + tx) as usize;
            let nd = d + step_mm[s];
            if nd < dist[j] {
                dist[j] = nd;
                heap.push(Reverse((nd, j as u32)));
            }
        }
    }
    dist
}

/// Labels each connected region of `mask` cells 1, 2, ... (0 outside the
/// mask), joining across edges, or across edges and corners when
/// `corners`. Returns the labels and the number of regions.
pub fn regions(n: usize, mask: &[bool], corners: bool) -> (Vec<u32>, u32) {
    let mut label = vec![0u32; n * n];
    let mut count = 0;
    let ni = n as i32;
    let mut stack = Vec::new();
    for start in 0..n * n {
        if !mask[start] || label[start] != 0 {
            continue;
        }
        count += 1;
        label[start] = count;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % n) as i32, (i / n) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if (dx == 0 && dy == 0) || (!corners && dx != 0 && dy != 0) {
                        continue;
                    }
                    let (tx, ty) = (x + dx, y + dy);
                    if tx < 0 || ty < 0 || tx >= ni || ty >= ni {
                        continue;
                    }
                    let j = (ty * ni + tx) as usize;
                    if mask[j] && label[j] == 0 {
                        label[j] = count;
                        stack.push(j);
                    }
                }
            }
        }
    }
    (label, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_go_around_walls_and_regions_count_pockets() {
        // 5 x 5, a wall at x = 2 for y = 0..=3, open at y = 4
        let n = 5;
        let mut open = vec![true; n * n];
        for y in 0..4 {
            open[y * n + 2] = false;
        }
        let d = distances(n, 1000, &[0], &open);
        // (4,0) is reached over the top of the wall, through (2,4). Worked by
        // hand: (0,0) -> (0,1) -> (0,2) -> (1,4) is 1000 + 1000 + 2236 (a
        // knight's move), then 1000 to (2,4): 5236. Cutting the wall's corner
        // from (1,3) is not allowed. The way down is the mirror: 10472 in all.
        assert_eq!(d[4], 10472);
        assert_eq!(d[2], FAR);
        assert_eq!(d[1], 1000);
        // knight's move on open ground: (0,0) to (2,1) is 2236
        let all = vec![true; n * n];
        assert_eq!(distances(n, 1000, &[0], &all)[n + 2], 2236);

        // two pockets touching only at a corner: one region with corners, two without
        let mut mask = vec![false; n * n];
        mask[0] = true;
        mask[n + 1] = true;
        assert_eq!(regions(n, &mask, false).1, 2);
        assert_eq!(regions(n, &mask, true).1, 1);
    }
}
