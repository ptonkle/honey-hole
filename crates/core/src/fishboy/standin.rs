//! A plain lake behind the Fish-Boy until the world renderer
//! (`crates/core/src/scene/`) draws Burt Lake: the water as one flat sheet,
//! the land near the eye as flat strips a little above it, and the map's
//! trees near the eye as their models, seen first person from your demo
//! boat. It exists so the raised view shows a live world round its edge;
//! the world renderer replaces it (SECOND-ORDER R2.66).

use crate::app::{App, Screen};
use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{facing, v3, Affine, V3};
use crate::gfx::render::{self, Camera};
use crate::map::{Cell, TreeKind, P};

/// How far round the eye the land and trees are drawn (mm).
const REACH_MM: i64 = 260_000;
/// The eye's height above the water.
const EYE: i32 = ONE * 3 / 2;
/// The land's flat top above the water.
const LAND: i32 = ONE / 2;

/// Millimetres on the map as the world's metres: x east, y up, z south.
fn world(p: P, y: i32) -> V3 {
    let m = |v: i32| (v as i64 * ONE as i64 / 1000) as i32;
    v3(m(p.x), y, -m(p.y))
}

fn quad(out: &mut Vec<i32>, a: V3, b: V3, c: V3, d: V3, colour: render::Rgb) {
    render::push_tri(out, [a, b, c], V3::UP, colour);
    render::push_tri(out, [a, c, d], V3::UP, colour);
}

impl App {
    /// The stand-in world's view values and triangles, in Play.
    pub fn standin_frame(&self) -> (Vec<i32>, Vec<i32>) {
        let (Some(theme), Some(demo)) = (self.fishboy.theme.as_ref(), self.fishboy.demo.as_ref()) else {
            return (Vec::new(), Vec::new());
        };
        if self.screen != Screen::Play {
            return (Vec::new(), Vec::new());
        }
        let look = &theme.standin;
        let map = &demo.map;
        let view = demo.view();
        let Some(me) = view.boats.get(view.me) else {
            return (Vec::new(), Vec::new());
        };
        let mut tris = Vec::new();

        // the water, past the map's edges
        let size = map.size_mm();
        let (lo, hi) = (P::new(-size / 2, -size / 2), P::new(size * 3 / 2, size * 3 / 2));
        quad(
            &mut tris,
            world(lo, 0),
            world(P::new(hi.x, lo.y), 0),
            world(hi, 0),
            world(P::new(lo.x, hi.y), 0),
            render::rgb(look.water),
        );

        // the land near the eye, a strip for each run of land in a row
        let n = map.n;
        let near = |p: P| p.dist2(me.at) <= REACH_MM * REACH_MM;
        let land = render::rgb(look.land);
        for row in 0..n {
            let mut col = 0;
            while col < n {
                let i = row * n + col;
                let solid = matches!(map.cells[i], Cell::Land | Cell::Island) && near(map.centre(i));
                if !solid {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < n
                    && matches!(map.cells[row * n + col], Cell::Land | Cell::Island)
                    && near(map.centre(row * n + col))
                {
                    col += 1;
                }
                let c = map.cell_mm;
                let (x0, x1) = (start as i32 * c, col as i32 * c);
                let (y0, y1) = (row as i32 * c, (row + 1) as i32 * c);
                quad(
                    &mut tris,
                    world(P::new(x0, y0), LAND),
                    world(P::new(x1, y0), LAND),
                    world(P::new(x1, y1), LAND),
                    world(P::new(x0, y1), LAND),
                    land,
                );
            }
        }

        // the trees near the eye
        let model = |id: &str| self.models.iter().find(|m| m.id == id);
        for t in &map.trees {
            if !near(t.at) {
                continue;
            }
            let id = match t.kind {
                TreeKind::Pine => "pine",
                TreeKind::Birch => "birch",
                TreeKind::Maple => "pine-small",
            };
            let Some(m) = model(id) else {
                continue;
            };
            let s = fx::ratio(t.scale, 1000);
            let place = Affine::translate(world(t.at, LAND)).then(&Affine::scale(v3(s, s, s)));
            let transforms = m.transforms(&m.poses(None, 0));
            m.draw(&transforms, &place, &m.colours([ONE; 3]), &mut tris);
        }

        // first person from your boat, looking along its heading
        let a = me.heading;
        let yaw = fx::atan2(fx::cos(a), -fx::sin(a));
        let fov = deg(theme.fishboy.fov_deg);
        let cam = Camera {
            eye: world(me.at, EYE),
            fwd: facing(yaw, -deg(4)),
            fov,
            near: ONE / 10,
            far: (REACH_MM as i32 / 1000 + 60) * ONE,
        };
        let [f0, f1] = look.fog_m.map(|m| m * ONE);
        (
            render::view_values(&cam, render::aspect(self.size), (f0, f1), &look.light),
            tris,
        )
    }
}
