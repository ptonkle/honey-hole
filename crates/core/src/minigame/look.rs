//! The minigames' look: `data/ui2d.json`'s `minigames` section. Colours
//! are 0-255; sizes are in the minigames' canvas units. Where a thing sits
//! is the minigame's layout; how it looks is here.

use serde::Deserialize;

use crate::ui2d::theme::C;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub banner: BannerLook,
    /// the instruction line and the progress readout during play
    pub text: C,
    pub text_size: i32,
    pub hint: C,
    pub hint_size: i32,
    pub progress_size: i32,
    pub bar: C,
    pub bar_back: C,
    /// the Fish-Boy's case round a minigame shown on its screen outside a
    /// match (the Minigames screen)
    pub bezel: C,
    /// the paper frame round a full-screen minigame (`minigame/frame.rs`)
    pub frame: FrameLook,
    pub cloak: CloakLook,
    pub power_reel: ReelLook,
    pub nitrous: AtomsLook,
    pub teleporter: BeatsLook,
    pub camera: CameraLook,
    pub enhanced_radar: FocusLook,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannerLook {
    pub band: C,
    pub edge: C,
    pub ink: C,
    pub shadow: C,
    /// the text's size once it has popped in, and the band's height
    pub size: i32,
    pub band_h: i32,
    /// the text grows to its size over this long
    pub pop_ms: i32,
    /// the scene under the banner is dimmed by this much, percent
    pub dim_pct: i32,
}

/// The magazine frame: a paper border `width` wide with an ink keyline
/// (`keyline` wide, `radius` round) inside it, the triple stripe in its top
/// (`stripes` top to bottom, each `stripe_h`), and dots on a `dot_step`
/// grid over `dots_w` of each bottom corner. The banner's band takes the
/// same stripes for its edges.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameLook {
    pub paper: C,
    pub ink: C,
    pub width: i32,
    pub keyline: i32,
    pub radius: i32,
    pub stripes: Vec<C>,
    pub stripe_h: i32,
    pub dots: C,
    pub dot_step: i32,
    pub dots_w: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloakLook {
    pub background: C,
    pub grid: C,
    /// the boat's parts: the gunwale, the floor, the benches, the motor,
    /// the casting deck
    pub rim: C,
    pub floor: C,
    pub bench: C,
    pub motor: C,
    pub deck: C,
    pub eraser: C,
    pub eraser_edge: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelLook {
    pub background: C,
    pub spool: C,
    pub line: C,
    pub rim: C,
    pub hub: C,
    pub handle: C,
    pub arrow: C,
    pub progress: C,
    pub pip_off: C,
    pub pip_on: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomsLook {
    pub background: C,
    pub arena: C,
    pub arena_line: C,
    pub nitrogen: C,
    pub oxygen: C,
    pub atom_ink: C,
    pub bin: C,
    pub bin_line: C,
    /// a wrong bin flashes this colour
    pub wrong: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatsLook {
    pub background: C,
    pub glow: C,
    pub circle: C,
    pub edge: C,
    pub ring: C,
    /// the ring inside the hit window
    pub ring_ready: C,
    pub number: C,
    pub hit: C,
    pub miss: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraLook {
    pub sky: C,
    pub sky_low: C,
    pub water: C,
    pub water_line: C,
    /// the flying fish's colours, picked in turn
    pub fish: Vec<C>,
    pub belly: C,
    pub eye: C,
    pub flash: C,
    pub photo: C,
    pub viewfinder: C,
    /// the viewfinder's width and height, and its brackets' length
    pub viewfinder_size: [i32; 2],
    pub bracket: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FocusLook {
    pub background: C,
    pub frame: C,
    pub fish: C,
    pub fish_line: C,
    pub eye: C,
    pub track: C,
    pub knob: C,
    pub knob_line: C,
    pub snap: C,
}

impl Look {
    /// Every colour, by its key, for the range check.
    fn colours(&self) -> Vec<(String, C)> {
        let b = &self.banner;
        let k = &self.cloak;
        let r = &self.power_reel;
        let a = &self.nitrous;
        let t = &self.teleporter;
        let m = &self.camera;
        let f = &self.enhanced_radar;
        let mut out: Vec<(&str, C)> = vec![
            ("banner.band", b.band),
            ("banner.edge", b.edge),
            ("banner.ink", b.ink),
            ("banner.shadow", b.shadow),
            ("text", self.text),
            ("hint", self.hint),
            ("bar", self.bar),
            ("bar_back", self.bar_back),
            ("bezel", self.bezel),
            ("frame.paper", self.frame.paper),
            ("frame.ink", self.frame.ink),
            ("frame.dots", self.frame.dots),
            ("cloak.background", k.background),
            ("cloak.grid", k.grid),
            ("cloak.rim", k.rim),
            ("cloak.floor", k.floor),
            ("cloak.bench", k.bench),
            ("cloak.motor", k.motor),
            ("cloak.deck", k.deck),
            ("cloak.eraser", k.eraser),
            ("cloak.eraser_edge", k.eraser_edge),
            ("power_reel.background", r.background),
            ("power_reel.spool", r.spool),
            ("power_reel.line", r.line),
            ("power_reel.rim", r.rim),
            ("power_reel.hub", r.hub),
            ("power_reel.handle", r.handle),
            ("power_reel.arrow", r.arrow),
            ("power_reel.progress", r.progress),
            ("power_reel.pip_off", r.pip_off),
            ("power_reel.pip_on", r.pip_on),
            ("nitrous.background", a.background),
            ("nitrous.arena", a.arena),
            ("nitrous.arena_line", a.arena_line),
            ("nitrous.nitrogen", a.nitrogen),
            ("nitrous.oxygen", a.oxygen),
            ("nitrous.atom_ink", a.atom_ink),
            ("nitrous.bin", a.bin),
            ("nitrous.bin_line", a.bin_line),
            ("nitrous.wrong", a.wrong),
            ("teleporter.background", t.background),
            ("teleporter.glow", t.glow),
            ("teleporter.circle", t.circle),
            ("teleporter.edge", t.edge),
            ("teleporter.ring", t.ring),
            ("teleporter.ring_ready", t.ring_ready),
            ("teleporter.number", t.number),
            ("teleporter.hit", t.hit),
            ("teleporter.miss", t.miss),
            ("camera.sky", m.sky),
            ("camera.sky_low", m.sky_low),
            ("camera.water", m.water),
            ("camera.water_line", m.water_line),
            ("camera.belly", m.belly),
            ("camera.eye", m.eye),
            ("camera.flash", m.flash),
            ("camera.photo", m.photo),
            ("camera.viewfinder", m.viewfinder),
            ("enhanced_radar.background", f.background),
            ("enhanced_radar.frame", f.frame),
            ("enhanced_radar.fish", f.fish),
            ("enhanced_radar.fish_line", f.fish_line),
            ("enhanced_radar.eye", f.eye),
            ("enhanced_radar.track", f.track),
            ("enhanced_radar.knob", f.knob),
            ("enhanced_radar.knob_line", f.knob_line),
            ("enhanced_radar.snap", f.snap),
        ];
        out.extend(m.fish.iter().map(|c| ("camera.fish", *c)));
        out.extend(self.frame.stripes.iter().map(|c| ("frame.stripes", *c)));
        out.into_iter().map(|(k, c)| (k.to_string(), c)).collect()
    }

    pub fn check(&self) -> Result<(), String> {
        for (name, c) in self.colours() {
            if c.iter().any(|v| !(0..=255).contains(v)) {
                return Err(format!(
                    "The ui2d colour \"minigames.{name}\" is outside 0 to 255."
                ));
            }
        }
        if self.camera.fish.is_empty() {
            return Err("The ui2d file's minigames.camera.fish lists no colour.".into());
        }
        let sizes = [
            self.text_size,
            self.hint_size,
            self.progress_size,
            self.banner.size,
            self.banner.band_h,
            self.camera.viewfinder_size[0],
            self.camera.viewfinder_size[1],
            self.camera.bracket,
            self.frame.width,
            self.frame.dot_step,
        ];
        if sizes.iter().any(|s| *s <= 0) || !(0..=100).contains(&self.banner.dim_pct) {
            return Err("The minigames' text, banner and viewfinder need a size.".into());
        }
        Ok(())
    }
}
