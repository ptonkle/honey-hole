//! Cloaking Tech, on the Fish-Boy: an overhead picture of a boat, rubbed
//! out with the pointer like Paint's eraser. The picture is a coarse grid
//! of cells (`cols` x `rows`), each a part of the boat or nothing; holding
//! the button erases every cell whose middle the eraser's square covers,
//! along the whole stroke. Done when `done_pct` of the boat's cells are
//! gone (the brief: about 98%, not all, which could frustrate).

use super::shape::{inside, Pt};
use super::tuning::CloakTuning;
use super::{cue, Ctx, Input, Paint};
use crate::ui2d::theme::c;
use crate::ui2d::{Rect, Rgba};

/// The parts, by cell value.
const NONE: u8 = 0;
const RIM: u8 = 1;
const FLOOR: u8 = 2;
const BENCH: u8 = 3;
const MOTOR: u8 = 4;
const DECK: u8 = 5;

/// The boat from above, bow to the right, in thousandths of the picture.
const HULL: [Pt; 9] = [
    (70, 190),
    (560, 110),
    (760, 150),
    (900, 280),
    (975, 500),
    (900, 720),
    (760, 850),
    (560, 890),
    (70, 810),
];
/// The floor inside the gunwale: the hull drawn in to this many
/// thousandths about its middle.
const INNER_PERMILLE: i32 = 850;
const HULL_MIDDLE: Pt = (520, 500);
/// The casting deck starts this far along.
const DECK_FROM: i32 = 760;
/// The benches' spans along the boat, and the motor's box at the stern.
const BENCHES: [(i32, i32); 2] = [(300, 370), (540, 610)];
const MOTOR_BOX: (i32, i32, i32, i32) = (0, 400, 120, 600);

fn part(nx: i32, ny: i32) -> u8 {
    let (mx0, my0, mx1, my1) = MOTOR_BOX;
    if nx >= mx0 && nx < mx1 && ny >= my0 && ny < my1 {
        return MOTOR;
    }
    if !inside((nx, ny), &HULL) {
        return NONE;
    }
    let inner: Vec<Pt> = HULL
        .iter()
        .map(|p| {
            (
                HULL_MIDDLE.0 + (p.0 - HULL_MIDDLE.0) * INNER_PERMILLE / 1000,
                HULL_MIDDLE.1 + (p.1 - HULL_MIDDLE.1) * INNER_PERMILLE / 1000,
            )
        })
        .collect();
    if !inside((nx, ny), &inner) {
        return RIM;
    }
    if nx >= DECK_FROM {
        return DECK;
    }
    if BENCHES.iter().any(|(a, b)| nx >= *a && nx < *b) {
        return BENCH;
    }
    FLOOR
}

pub struct Cloak {
    pub t: CloakTuning,
    /// the picture's place on the canvas
    pub pic: Rect,
    /// each cell's part, row by row; erased cells are NONE
    pub cells: Vec<u8>,
    pub total: i32,
    pub left: i32,
    /// the stroke's last point, while the button is held
    last: Option<Pt>,
    next_tick_pct: i32,
}

impl Cloak {
    pub fn new(t: &CloakTuning, canvas: [i32; 2]) -> Cloak {
        let [pw, ph] = t.picture;
        let pic = Rect::new((canvas[0] - pw) / 2, (canvas[1] - ph) / 2, pw, ph);
        let mut cells = Vec::with_capacity((t.cols * t.rows) as usize);
        for row in 0..t.rows {
            for col in 0..t.cols {
                let nx = (col * 1000 + 500) / t.cols;
                let ny = (row * 1000 + 500) / t.rows;
                cells.push(part(nx, ny));
            }
        }
        let total = cells.iter().filter(|v| **v != NONE).count() as i32;
        Cloak {
            t: t.clone(),
            pic,
            cells,
            total,
            left: total,
            last: None,
            next_tick_pct: t.tick_every_pct,
        }
    }

    fn cell_w(&self) -> i32 {
        self.pic.w / self.t.cols
    }

    fn cell_h(&self) -> i32 {
        self.pic.h / self.t.rows
    }

    /// The middle of cell (col, row) on the canvas.
    pub fn cell_mid(&self, col: i32, row: i32) -> Pt {
        (
            self.pic.x + col * self.cell_w() + self.cell_w() / 2,
            self.pic.y + row * self.cell_h() + self.cell_h() / 2,
        )
    }

    pub fn erased_pct(&self) -> i32 {
        (self.total - self.left) * 100 / self.total.max(1)
    }

    fn done(&self) -> bool {
        (self.total - self.left) as i64 * 100 >= self.t.done_pct as i64 * self.total as i64
    }

    /// Erases every cell whose middle the eraser's square at `p` covers.
    fn erase_at(&mut self, p: Pt) {
        let half = self.t.eraser / 2;
        let (cw, ch) = (self.cell_w().max(1), self.cell_h().max(1));
        let col_lo = ((p.0 - half - self.pic.x) / cw - 1).max(0);
        let col_hi = ((p.0 + half - self.pic.x) / cw + 1).min(self.t.cols - 1);
        let row_lo = ((p.1 - half - self.pic.y) / ch - 1).max(0);
        let row_hi = ((p.1 + half - self.pic.y) / ch + 1).min(self.t.rows - 1);
        for row in row_lo..=row_hi {
            for col in col_lo..=col_hi {
                let m = self.cell_mid(col, row);
                if (m.0 - p.0).abs() <= half && (m.1 - p.1).abs() <= half {
                    let k = (row * self.t.cols + col) as usize;
                    if self.cells[k] != NONE {
                        self.cells[k] = NONE;
                        self.left -= 1;
                    }
                }
            }
        }
    }

    /// The eraser moved to `p` with the button held: every point along
    /// the way is erased.
    pub fn stroke(&mut self, ctx: &mut Ctx, p: Pt) {
        let from = self.last.unwrap_or(p);
        let (dx, dy) = (p.0 - from.0, p.1 - from.1);
        let len = crate::map::trig::hypot(dx as i64, dy as i64) as i32;
        let step = (self.t.eraser / 3).max(1);
        let n = len / step + 1;
        for k in 1..=n {
            self.erase_at((from.0 + dx * k / n, from.1 + dy * k / n));
        }
        self.last = Some(p);
        while self.erased_pct() >= self.next_tick_pct && self.next_tick_pct < 100 {
            ctx.cues.push(cue::TICK);
            self.next_tick_pct += self.t.tick_every_pct;
        }
    }

    /// The button came up: the next stroke starts afresh.
    pub fn lift(&mut self) {
        self.last = None;
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        match input.pointer {
            Some(p) if input.held || input.pressed => self.stroke(ctx, p),
            _ => self.lift(),
        }
        if input.released && !input.held {
            self.lift();
        }
        self.done()
    }

    pub fn draw(&self, p: &mut Paint) {
        let look = &p.look.cloak;
        let [w, h] = p.canvas;
        p.list.fill_rect(c(look.background), Rect::new(0, 0, w, h));
        // a faint grid over the picture, the Fish-Boy's look
        let grid = c(look.grid);
        let every = 5;
        for col in (0..=self.t.cols).step_by(every) {
            let x = self.pic.x + col * self.cell_w();
            p.list.line(grid, 1, (x, self.pic.y), (x, self.pic.bottom()));
        }
        for row in (0..=self.t.rows).step_by(every) {
            let y = self.pic.y + row * self.cell_h();
            p.list.line(grid, 1, (self.pic.x, y), (self.pic.right(), y));
        }
        // the boat's cells, a row's runs of one part as one rectangle
        let colour = |v: u8| match v {
            RIM => look.rim,
            FLOOR => look.floor,
            BENCH => look.bench,
            MOTOR => look.motor,
            _ => look.deck,
        };
        let (cw, ch) = (self.cell_w(), self.cell_h());
        for row in 0..self.t.rows {
            let mut col = 0;
            while col < self.t.cols {
                let v = self.cells[(row * self.t.cols + col) as usize];
                let start = col;
                while col < self.t.cols && self.cells[(row * self.t.cols + col) as usize] == v {
                    col += 1;
                }
                if v != NONE {
                    let r = Rect::new(
                        self.pic.x + start * cw,
                        self.pic.y + row * ch,
                        (col - start) * cw,
                        ch,
                    );
                    p.list.fill_rect(c(colour(v)), r);
                }
            }
        }
        // the eraser
        if let Some(at) = p.pointer {
            let e = self.t.eraser;
            let r = Rect::new(at.0 - e / 2, at.1 - e / 2, e, e);
            let fill = if p.held { 85 } else { 45 };
            p.list
                .round_rect(c(look.eraser).alpha(fill), Rgba::NONE, 0, r, e / 8);
            p.list.round_rect(Rgba::NONE, c(look.eraser_edge), 4, r, e / 8);
        }
    }
}
