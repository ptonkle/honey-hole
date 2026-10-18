//! Model files: every fish, boat, figure, item, prop and device is a JSON
//! file under `data/models/`, named by its id (`bluegill.json`). The file
//! is the source of truth; no Rust code builds a model. The core parses and
//! checks each file and poses it into triangles for the renderer.
//!
//! The format, in brief (SECOND-ORDER R1.40):
//!
//! ```text
//! {
//!   "_note":  "for people: what the model is and where its markings come from",
//!   "name":   "Bluegill",                 shown in the gallery
//!   "kind":   "fish",                     fish, boat, figure, item, prop, device
//!   "unit":   "mm",                       every length is whole millimetres
//!   "view":   { "yaw": 30, "pitch": 12 }, the gallery's opening angle, degrees
//!   "colours": {                          flat colours by name, 0-255 each,
//!     "back": [52, 84, 60],               or a slot the game fills at draw time:
//!     "hull": { "slot": "player", "shade": 80 },  (shade: percent, default 100)
//!     "lamp": { "rgb": [255, 220, 120], "glow": true }  unlit: see below
//!   },
//!   "parts": [                            rigid pieces, parents before children
//!     { "name": "body", "pivot": [0, 0, 0], "vertices": [[x, y, z], ...] },
//!     { "name": "tail", "parent": "body", "pivot": [0, 0, -90], "vertices": [...] }
//!   ],
//!   "faces": [                            flat faces over all parts' vertices,
//!     { "colour": "back", "v": [0, 1, 2] } numbered in part order; 3 or more
//!   ],                                    corners, convex, either winding
//!   "anchors": { "drive": { "part": "hull", "at": [x, y, z] } },
//!   "animations": {                       keyframed part poses, looping;
//!     "idle": { "ms": 1400, "ease": "linear", "keys": [   ease: smooth or linear
//!       { "at": 0,   "parts": { "tail": { "turn": [0, 18, 0] } } },
//!       { "at": 700, "parts": { "tail": { "turn": [0, -18, 0] } } } ] }
//!   }
//! }
//! ```
//!
//! Axes follow glTF: +y up, +z forward (a fish's nose, a boat's bow), +x
//! to the model's left. A vertex belongs to the part that lists it, and
//! moves with it; a face may join vertices of two parts and then stretches
//! between them (a fish's body bends without a gap). A part's pose turns
//! it about its pivot (degrees: about z, then x, then y), moves it
//! (`move`, mm) and scales it (`scale`, percent; 0 hides it), on top of its
//! parent's pose. A part a key leaves out is at rest in that key. Between
//! keys the pose eases in and out (`"ease": "smooth"`, the default: right
//! for a swing between two keys) or moves straight (`"linear"`: right for
//! a wave sampled at many keys); after the last key it goes back to the
//! first, which is at 0. A colour marked `"glow": true` (written as
//! `{ "rgb": [...], "glow": true }`, or on a slot) is unlit: it is drawn
//! at full brightness whatever the sun and shade, and the fog takes it
//! less (`glow_fog_pct` in the light, `data/look.json`), so a lamp reads
//! through the fog. A plain `[r, g, b]` colour is lit, as before. An anchor is a point on a part where another
//! model stands: a boat's `drive` and `fish` anchors are where the angler
//! goes in those poses.
//!
//! Each of these maps onto glTF (nodes with pivots, a skin with one joint
//! per vertex, animation channels), so an importer could be added later
//! (SECOND-ORDER R1.41).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::gfx::render::{self, Rgb};

/// The one length unit model files use.
pub const UNIT: &str = "mm";
/// No coordinate may be further than this from the origin (30 m): the
/// core's metres hold about 32 km, and a model stays well inside.
pub const MAX_MM: i32 = 30_000;
/// The longest animation, in milliseconds.
pub const MAX_MS: u32 = 60_000;
/// The colour slots a model may name, filled when it is drawn.
pub const SLOTS: [&str; 1] = ["player"];

/// What a model is for. The gallery lists them in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Fish,
    Boat,
    Figure,
    Item,
    Device,
    Prop,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Fish,
        Kind::Boat,
        Kind::Figure,
        Kind::Item,
        Kind::Device,
        Kind::Prop,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Fish => "fish",
            Kind::Boat => "boat",
            Kind::Figure => "figure",
            Kind::Item => "item",
            Kind::Device => "device",
            Kind::Prop => "prop",
        }
    }

    /// The animations every model of this kind must have: the game plays
    /// them by name.
    pub fn needs(self) -> &'static [Anim] {
        match self {
            Kind::Fish => &[Anim::Idle, Anim::Panic],
            Kind::Figure => &[Anim::Drive, Anim::Fish],
            _ => &[],
        }
    }
}

/// The animations the game knows by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anim {
    /// a fish swimming at ease
    Idle,
    /// a hooked fish's fast, panicked swim
    Panic,
    /// the angler on the back bench, by the outboard
    Drive,
    /// the angler standing at the bow
    Fish,
    /// a floating thing on the swell
    Bob,
    /// a plant in the wind
    Sway,
}

impl Anim {
    pub const ALL: [Anim; 6] = [
        Anim::Idle,
        Anim::Panic,
        Anim::Drive,
        Anim::Fish,
        Anim::Bob,
        Anim::Sway,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Anim::Idle => "idle",
            Anim::Panic => "panic",
            Anim::Drive => "drive",
            Anim::Fish => "fish",
            Anim::Bob => "bob",
            Anim::Sway => "sway",
        }
    }

    pub fn from_key(key: &str) -> Option<Anim> {
        Anim::ALL.into_iter().find(|a| a.key() == key)
    }
}

// ---- the file as written ----

fn hundred() -> i32 {
    100
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileModel {
    #[serde(rename = "_note", default)]
    #[allow(dead_code)]
    note: String,
    name: String,
    kind: String,
    unit: String,
    #[serde(default)]
    view: FileView,
    colours: BTreeMap<String, FileColour>,
    parts: Vec<FilePart>,
    faces: Vec<FileFace>,
    #[serde(default)]
    anchors: BTreeMap<String, FileAnchor>,
    #[serde(default)]
    animations: BTreeMap<String, FileAnimation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileView {
    yaw: i32,
    pitch: i32,
}

impl Default for FileView {
    fn default() -> FileView {
        FileView { yaw: 30, pitch: 15 }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FileColour {
    Rgb([i32; 3]),
    Lit(FileLit),
    Slot {
        slot: String,
        #[serde(default = "hundred")]
        shade: i32,
        #[serde(default)]
        glow: bool,
    },
}

/// A fixed colour written as an object, so it can carry `glow`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLit {
    rgb: [i32; 3],
    #[serde(default)]
    glow: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePart {
    name: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    pivot: [i32; 3],
    vertices: Vec<[i32; 3]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileFace {
    colour: String,
    v: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileAnchor {
    part: String,
    at: [i32; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileAnimation {
    ms: u32,
    #[serde(default = "smooth")]
    ease: String,
    keys: Vec<FileKey>,
}

fn smooth() -> String {
    "smooth".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileKey {
    at: u32,
    #[serde(default)]
    parts: BTreeMap<String, FilePose>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePose {
    #[serde(default)]
    turn: [i32; 3],
    #[serde(rename = "move", default)]
    shift: [i32; 3],
    #[serde(default = "hundred")]
    scale: i32,
}

// ---- the checked model ----

/// A face's colour: fixed, or a slot filled when drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    Rgb(Rgb),
    /// a slot of `SLOTS`, at `shade` percent
    Slot(usize, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub parent: Option<usize>,
    pub pivot: V3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub paint: usize,
    pub v: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub name: String,
    pub part: usize,
    pub at: V3,
}

/// One part's pose: turned (angle units about z, x, y), moved, scaled (Q16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub turn: [i32; 3],
    pub shift: V3,
    pub scale: i32,
}

impl Pose {
    pub const REST: Pose = Pose {
        turn: [0; 3],
        shift: V3::ZERO,
        scale: ONE,
    };

    fn blend(&self, o: &Pose, t: i32) -> Pose {
        Pose {
            turn: [0, 1, 2].map(|k| fx::lerp(self.turn[k], o.turn[k], t)),
            shift: self.shift.lerp(o.shift, t),
            scale: fx::lerp(self.scale, o.scale, t),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub at: u32,
    /// one pose per part, in part order
    pub poses: Vec<Pose>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub anim: Anim,
    pub ms: u32,
    /// eased in and out between keys (`"smooth"`), or straight (`"linear"`)
    pub smooth: bool,
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// the file's name without `.json`
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// the gallery's opening angle (angle units)
    pub yaw: i32,
    pub pitch: i32,
    pub paints: Vec<Paint>,
    /// by paint: drawn unlit, at full brightness (`"glow": true`)
    pub glows: Vec<bool>,
    pub parts: Vec<Part>,
    /// every vertex (metres, Q16) and the part it moves with
    pub verts: Vec<(V3, usize)>,
    pub faces: Vec<Face>,
    pub anchors: Vec<Anchor>,
    /// by `Anim` order
    pub animations: Vec<Animation>,
    /// the rest pose's bounds, in millimetres
    pub min_mm: [i32; 3],
    pub max_mm: [i32; 3],
}

/// Millimetres to the core's metres (Q16).
pub fn mm(p: [i32; 3]) -> V3 {
    let c = |a: i32| (a as i64 * ONE as i64 / 1000) as i32;
    v3(c(p[0]), c(p[1]), c(p[2]))
}

fn in_range(p: [i32; 3]) -> bool {
    p.iter().all(|c| c.abs() <= MAX_MM)
}

/// Reads and checks one model file. `id` is its file name without `.json`;
/// every refusal starts with it.
pub fn parse(id: &str, json: &str) -> Result<Model, String> {
    let file: FileModel = serde_json::from_str(json).map_err(|e| format!("{id}: {e}"))?;
    check(id, file).map_err(|e| format!("{id}: {e}"))
}

fn check(id: &str, f: FileModel) -> Result<Model, String> {
    if f.unit != UNIT {
        return Err(format!("the unit is \"{}\"; model files use \"{UNIT}\".", f.unit));
    }
    let kind = Kind::ALL
        .into_iter()
        .find(|k| k.key() == f.kind)
        .ok_or_else(|| format!("there is no kind \"{}\".", f.kind))?;
    if f.name.trim().is_empty() {
        return Err("the model has no name.".into());
    }

    // colours, by name
    let colour_names: Vec<&String> = f.colours.keys().collect();
    let mut paints = Vec::new();
    let mut glows = Vec::new();
    for (name, c) in &f.colours {
        let (rgb, glow) = match c {
            FileColour::Rgb(c) => (Some(*c), false),
            FileColour::Lit(l) => (Some(l.rgb), l.glow),
            FileColour::Slot { glow, .. } => (None, *glow),
        };
        glows.push(glow);
        paints.push(match (c, rgb) {
            (_, Some(c)) => {
                if c.iter().any(|v| !(0..=255).contains(v)) {
                    return Err(format!("the colour \"{name}\" is outside 0 to 255."));
                }
                Paint::Rgb(render::rgb(c))
            }
            (FileColour::Slot { slot, shade, .. }, None) => {
                let s = SLOTS
                    .iter()
                    .position(|k| k == slot)
                    .ok_or_else(|| format!("the colour \"{name}\" names no slot \"{slot}\"."))?;
                if !(0..=200).contains(shade) {
                    return Err(format!("the colour \"{name}\" has a shade outside 0 to 200."));
                }
                Paint::Slot(s, *shade)
            }
            (_, None) => unreachable!("only a slot has no rgb"),
        });
    }

    // parts and their vertices
    if f.parts.is_empty() {
        return Err("the model has no parts.".into());
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut verts = Vec::new();
    for p in &f.parts {
        if p.name.trim().is_empty() {
            return Err("a part has no name.".into());
        }
        if parts.iter().any(|q| q.name == p.name) {
            return Err(format!("there are two parts \"{}\".", p.name));
        }
        let parent = match &p.parent {
            None => None,
            Some(name) => Some(parts.iter().position(|q| &q.name == name).ok_or_else(|| {
                format!(
                    "the part \"{}\" has a parent \"{name}\" that is not listed before it.",
                    p.name
                )
            })?),
        };
        if !in_range(p.pivot) || !p.vertices.iter().all(|v| in_range(*v)) {
            return Err(format!("the part \"{}\" reaches past {MAX_MM} mm.", p.name));
        }
        for v in &p.vertices {
            verts.push((*v, parts.len()));
        }
        parts.push(Part {
            name: p.name.clone(),
            parent,
            pivot: mm(p.pivot),
        });
    }

    // faces
    if f.faces.is_empty() {
        return Err("the model has no faces.".into());
    }
    let mut used = vec![false; verts.len()];
    let mut painted = vec![false; paints.len()];
    let mut faces = Vec::new();
    for (k, face) in f.faces.iter().enumerate() {
        let paint = colour_names
            .iter()
            .position(|n| **n == face.colour)
            .ok_or_else(|| {
                format!(
                    "face {k} names the colour \"{}\", which is not listed.",
                    face.colour
                )
            })?;
        if face.v.len() < 3 {
            return Err(format!("face {k} has fewer than 3 corners."));
        }
        for (i, &v) in face.v.iter().enumerate() {
            if v >= verts.len() {
                return Err(format!("face {k} names vertex {v}; there are {}.", verts.len()));
            }
            if face.v[..i].contains(&v) {
                return Err(format!("face {k} names vertex {v} twice."));
            }
            used[v] = true;
        }
        painted[paint] = true;
        faces.push(Face {
            paint,
            v: face.v.clone(),
        });
    }
    if let Some(v) = used.iter().position(|u| !u) {
        return Err(format!("vertex {v} is in no face."));
    }
    if let Some(c) = painted.iter().position(|p| !p) {
        return Err(format!("the colour \"{}\" is on no face.", colour_names[c]));
    }

    let part_of = |name: &str| parts.iter().position(|p| p.name == name);

    // anchors
    let mut anchors = Vec::new();
    for (name, a) in &f.anchors {
        let part =
            part_of(&a.part).ok_or_else(|| format!("the anchor \"{name}\" names no part \"{}\".", a.part))?;
        if !in_range(a.at) {
            return Err(format!("the anchor \"{name}\" reaches past {MAX_MM} mm."));
        }
        anchors.push(Anchor {
            name: name.clone(),
            part,
            at: mm(a.at),
        });
    }

    // animations
    let mut animations = Vec::new();
    for (name, a) in &f.animations {
        let anim = Anim::from_key(name).ok_or_else(|| format!("there is no animation \"{name}\"."))?;
        if a.ms == 0 || a.ms > MAX_MS {
            return Err(format!(
                "the animation \"{name}\" lasts {} ms; it must be 1 to {MAX_MS}.",
                a.ms
            ));
        }
        if a.keys.first().map(|k| k.at) != Some(0) {
            return Err(format!("the animation \"{name}\" must start with a key at 0."));
        }
        let mut keys: Vec<Key> = Vec::new();
        for key in &a.keys {
            if keys.last().is_some_and(|k| k.at >= key.at) || key.at >= a.ms {
                return Err(format!(
                    "the animation \"{name}\" has a key at {} out of order or past its end.",
                    key.at
                ));
            }
            let mut poses = vec![Pose::REST; parts.len()];
            for (part, p) in &key.parts {
                let i = part_of(part)
                    .ok_or_else(|| format!("the animation \"{name}\" names no part \"{part}\"."))?;
                if p.turn.iter().any(|t| t.abs() > 720) || !in_range(p.shift) || !(0..=400).contains(&p.scale)
                {
                    return Err(format!("the animation \"{name}\" poses \"{part}\" out of range."));
                }
                poses[i] = Pose {
                    turn: p.turn.map(deg),
                    shift: mm(p.shift),
                    scale: p.scale * ONE / 100,
                };
            }
            keys.push(Key { at: key.at, poses });
        }
        let smooth = match a.ease.as_str() {
            "smooth" => true,
            "linear" => false,
            other => return Err(format!("the animation \"{name}\" has no ease \"{other}\".")),
        };
        animations.push(Animation {
            anim,
            ms: a.ms,
            smooth,
            keys,
        });
    }
    animations.sort_by_key(|a| a.anim);
    for need in kind.needs() {
        if !animations.iter().any(|a| a.anim == *need) {
            return Err(format!(
                "a {} model needs the animation \"{}\".",
                kind.key(),
                need.key()
            ));
        }
    }

    let mut min_mm = [i32::MAX; 3];
    let mut max_mm = [i32::MIN; 3];
    for (v, _) in &verts {
        for k in 0..3 {
            min_mm[k] = min_mm[k].min(v[k]);
            max_mm[k] = max_mm[k].max(v[k]);
        }
    }

    Ok(Model {
        id: id.to_string(),
        name: f.name,
        kind,
        yaw: deg(f.view.yaw),
        pitch: deg(f.view.pitch),
        paints,
        glows,
        parts,
        verts: verts.into_iter().map(|(v, p)| (mm(v), p)).collect(),
        faces,
        anchors,
        animations,
        min_mm,
        max_mm,
    })
}

/// Reads every model from one JSON object of `{id: model}` (the build
/// bundles `data/models/*.json` so). They come back in gallery order: by
/// kind, then shortest first, then by id.
pub fn parse_all(json: &str) -> Result<Vec<Model>, String> {
    let all: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(json).map_err(|e| format!("The models could not be read: {e}"))?;
    if all.is_empty() {
        return Err("There are no models.".into());
    }
    let mut models = Vec::new();
    for (id, value) in all {
        models.push(parse(&id, &value.to_string())?);
    }
    models.sort_by_key(|m| (m.kind, m.length_mm(), m.id.clone()));
    Ok(models)
}

/// Eases `t` (0..=ONE) in and out: 3t² − 2t³.
fn ease(t: i32) -> i32 {
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

impl Model {
    /// Front to back (along z), in millimetres.
    pub fn length_mm(&self) -> i32 {
        self.max_mm[2] - self.min_mm[2]
    }

    /// Bottom to top, in millimetres.
    pub fn height_mm(&self) -> i32 {
        self.max_mm[1] - self.min_mm[1]
    }

    /// The middle of the rest pose's bounds, and the radius of a ball
    /// round them.
    pub fn ball(&self) -> (V3, i32) {
        let lo = mm(self.min_mm);
        let hi = mm(self.max_mm);
        (lo.add(hi).scale(ONE / 2), hi.sub(lo).len() / 2)
    }

    pub fn animation(&self, anim: Anim) -> Option<&Animation> {
        self.animations.iter().find(|a| a.anim == anim)
    }

    pub fn anchor(&self, name: &str) -> Option<&Anchor> {
        self.anchors.iter().find(|a| a.name == name)
    }

    /// Whether any face takes the player's colour.
    pub fn has_slot(&self) -> bool {
        self.paints.iter().any(|p| matches!(p, Paint::Slot(..)))
    }

    /// Each part's pose `ms` into `anim` (looping), or the rest pose.
    pub fn poses(&self, anim: Option<Anim>, ms: u32) -> Vec<Pose> {
        let Some(a) = anim.and_then(|a| self.animation(a)) else {
            return vec![Pose::REST; self.parts.len()];
        };
        let t = ms % a.ms;
        let i = a.keys.iter().rposition(|k| k.at <= t).unwrap_or(0);
        let from = &a.keys[i];
        let (to, end) = match a.keys.get(i + 1) {
            Some(k) => (k, k.at),
            None => (&a.keys[0], a.ms),
        };
        let span = (end - from.at).max(1) as i32;
        let s = fx::ratio((t - from.at) as i32, span);
        let s = if a.smooth { ease(s) } else { s };
        from.poses
            .iter()
            .zip(&to.poses)
            .map(|(p, q)| p.blend(q, s))
            .collect()
    }

    /// Each part's transform from rest to `poses`, parents first.
    pub fn transforms(&self, poses: &[Pose]) -> Vec<Affine> {
        let mut out: Vec<Affine> = Vec::with_capacity(self.parts.len());
        for (part, pose) in self.parts.iter().zip(poses) {
            let local = Affine::translate(part.pivot.add(pose.shift))
                .then(&Affine::rot_y(pose.turn[1]))
                .then(&Affine::rot_x(pose.turn[0]))
                .then(&Affine::rot_z(pose.turn[2]))
                .then(&Affine::scale(v3(pose.scale, pose.scale, pose.scale)))
                .then(&Affine::translate(part.pivot.neg()));
            out.push(match part.parent {
                Some(p) => out[p].then(&local),
                None => local,
            });
        }
        out
    }

    /// Where an anchor is under these transforms, as a frame to place
    /// another model by: the anchor part's turn, at the anchor's point.
    pub fn anchor_frame(&self, anchor: &Anchor, transforms: &[Affine]) -> Affine {
        let t = transforms[anchor.part];
        t.at(t.apply(anchor.at))
    }

    /// The colour of each face's paint, the player's colour filling the
    /// slot.
    pub fn colours(&self, player: Rgb) -> Vec<Rgb> {
        self.paints
            .iter()
            .map(|p| match p {
                Paint::Rgb(c) => *c,
                Paint::Slot(_, shade) => player.map(|v| (v * shade / 100).clamp(0, ONE)),
            })
            .collect()
    }

    /// Appends the posed model's triangles, placed by `place`, to `out`.
    pub fn draw(&self, transforms: &[Affine], place: &Affine, colours: &[Rgb], out: &mut Vec<i32>) {
        let at: Vec<V3> = self
            .verts
            .iter()
            .map(|(v, part)| place.apply(transforms[*part].apply(*v)))
            .collect();
        for face in &self.faces {
            let p: Vec<V3> = face.v.iter().map(|&i| at[i]).collect();
            let n = newell(&p);
            for k in 1..p.len() - 1 {
                let glow = if self.glows[face.paint] { ONE } else { 0 };
                render::push_tri_glow(out, [p[0], p[k], p[k + 1]], n, colours[face.paint], glow);
            }
        }
    }
}

/// A polygon's unit normal by Newell's method (sound for any convex,
/// nearly flat polygon, in its winding's direction), in 64 bits.
pub fn newell(p: &[V3]) -> V3 {
    let mut n = [0i64; 3];
    for (i, a) in p.iter().enumerate() {
        let b = p[(i + 1) % p.len()];
        let (ax, ay, az) = (a.x as i64, a.y as i64, a.z as i64);
        let (bx, by, bz) = (b.x as i64, b.y as i64, b.z as i64);
        n[0] += (ay - by) * (az + bz);
        n[1] += (az - bz) * (ax + bx);
        n[2] += (ax - bx) * (ay + by);
    }
    let big = n.iter().map(|c| c.unsigned_abs()).max().unwrap_or(0);
    let mut shift = 0;
    while big >> shift >= 1 << 30 {
        shift += 1;
    }
    v3(
        (n[0] >> shift) as i32,
        (n[1] >> shift) as i32,
        (n[2] >> shift) as i32,
    )
    .norm()
}

#[cfg(test)]
mod tests;
