//! `data/flow.json`: the look of the match's flow (the HUD, the popups,
//! the countdown, the end, the postgame, a wreck's burst and the ghosting)
//! and the popup table. Where things sit is the core's; the look, the
//! timings of what is only drawn, and which event makes which popup are
//! data.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::ui2d::Font;

pub type C = [i32; 3];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// "display" (big words and numbers) and "body" (labels)
    pub fonts: BTreeMap<String, Font>,
    pub card: CardLook,
    pub hud: HudLook,
    pub countdown: CountdownLook,
    pub popups: PopupLook,
    pub postgame: PostgameLook,
    pub burst: BurstLook,
    /// a ghosted boat's opacity, percent
    pub ghost_pct: i32,
}

/// The rounded, outlined card every HUD piece sits on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardLook {
    pub fill: C,
    pub fill_pct: i32,
    pub outline: C,
    pub outline_w: i32,
    pub radius: i32,
    /// the drop shadow: offset down and right, and its opacity
    pub shadow: i32,
    pub shadow_pct: i32,
    /// dark text on a card, and the softer text of labels
    pub ink: C,
    pub soft: C,
    /// light text straight on the world (with an outline)
    pub light: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HudLook {
    pub driving: C,
    pub fishing: C,
    pub gauge_face: C,
    pub gauge_tick: C,
    pub gauge_fill: C,
    /// the part of the dial past the plain top speed (nitrous)
    pub gauge_boost: C,
    pub needle: C,
    pub ping_ready: C,
    pub ping_wait: C,
    pub item_slot: C,
    pub key_cap: C,
    /// the key hints fade over this long once `flow.hints_ms` is up
    pub hint_fade_ms: i32,
    pub seed: C,
    /// your boat wrecked: a flash over the screen, this colour, fading
    /// over this long
    pub wreck_flash: C,
    pub wreck_flash_pct: i32,
    pub wreck_flash_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountdownLook {
    pub number: C,
    pub go: C,
    pub size: i32,
    /// each number pops in over this long
    pub pop_ms: i32,
    /// "Go" stays up this long after the start
    pub go_ms: i32,
    pub end: C,
}

/// One popup style.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopupStyle {
    pub fill: C,
    pub outline: C,
    pub ink: C,
    /// "fish", "burst", "bobber" or "none"
    pub icon: String,
    /// the outline and badge take the event's tier colour, and the badge
    /// names the tier (so it never shows by colour alone)
    #[serde(default)]
    pub tier: bool,
}

/// One row of the popup table: an event and the popup it makes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopupRow {
    /// the `MatchEvent`'s name, as its JSON `type`
    pub event: String,
    /// event fields that must match: a dotted path to a value, or to "*"
    /// for any value at that path
    #[serde(default)]
    pub when: BTreeMap<String, String>,
    /// the string key in `data/strings.json`
    pub text: String,
    /// a key of `popups.styles`
    pub style: String,
    /// each `{placeholder}` in the text and how to fill it:
    /// "name:<path>" a seat's player name, "fish:<path>" a roster id's
    /// fish name, "tier:<path>" a tier's name, "text:<path>" the value
    #[serde(default)]
    pub fill: BTreeMap<String, String>,
    /// the path of the event's tier, for a style with `tier`
    #[serde(default)]
    pub tier: String,
    /// "all" (the default), "mine" or "others": by the event's `seat`
    #[serde(default)]
    pub who: String,
    /// popups in one group replace each other where they stand (a
    /// countdown that counts on)
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopupLook {
    /// how many show at once; the rest wait their turn
    pub max_shown: usize,
    pub slide_ms: i32,
    pub fade_ms: i32,
    pub styles: BTreeMap<String, PopupStyle>,
    pub rows: Vec<PopupRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostgameLook {
    /// the first card after this long, then one every `card_ms`, a
    /// pause of `gap_ms` between players, and the ranking `rank_ms`
    /// after the last
    pub first_ms: i32,
    pub card_ms: i32,
    pub gap_ms: i32,
    pub rank_ms: i32,
    /// a card's points count up over this long
    pub count_ms: i32,
    /// the backdrop over the lake, and its opacity
    pub backdrop: C,
    pub backdrop_pct: i32,
    pub card: C,
    pub winner: C,
    /// the fish's box behind its model
    pub fish_box: [i32; 3],
    /// the camera turns once round the boat in this long
    pub orbit_ms: i32,
}

/// A wreck's burst in the world, and the victim's camera shake.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurstLook {
    pub ms: i32,
    pub shards: i32,
    pub shard_mm: i32,
    pub speed_mm_per_s: i32,
    pub up_mm_per_s: i32,
    pub fall_mm_per_s2: i32,
    /// the fireball's widest, mm, reached at `swell_pct` of the burst
    pub fireball_mm: i32,
    pub swell_pct: i32,
    /// hot to cool: the shards and the fireball shade through these
    pub colours: Vec<C>,
    pub shake_ms: i32,
    pub shake_mm: i32,
    /// the shake's wobbles a second
    pub shake_hz: i32,
}

fn colour_ok(name: &str, c: C) -> Result<(), String> {
    if c.iter().any(|v| !(0..=255).contains(v)) {
        return Err(format!("The flow colour \"{name}\" is outside 0 to 255."));
    }
    Ok(())
}

pub const ICONS: [&str; 4] = ["fish", "burst", "bobber", "none"];
const FILLS: [&str; 4] = ["name", "fish", "tier", "text"];

impl FlowLook {
    pub fn parse(json: &str) -> Result<FlowLook, String> {
        let l: FlowLook =
            serde_json::from_str(json).map_err(|e| format!("The flow file could not be read: {e}"))?;
        l.check()?;
        Ok(l)
    }

    fn check(&self) -> Result<(), String> {
        for f in ["display", "body"] {
            if !self.fonts.contains_key(f) {
                return Err(format!("The flow file has no font \"{f}\"."));
            }
        }
        let (k, h, n, p) = (&self.card, &self.hud, &self.countdown, &self.postgame);
        let named = [
            ("card.fill", k.fill),
            ("card.outline", k.outline),
            ("card.ink", k.ink),
            ("card.soft", k.soft),
            ("card.light", k.light),
            ("hud.driving", h.driving),
            ("hud.fishing", h.fishing),
            ("hud.gauge_face", h.gauge_face),
            ("hud.gauge_tick", h.gauge_tick),
            ("hud.gauge_fill", h.gauge_fill),
            ("hud.gauge_boost", h.gauge_boost),
            ("hud.needle", h.needle),
            ("hud.ping_ready", h.ping_ready),
            ("hud.ping_wait", h.ping_wait),
            ("hud.item_slot", h.item_slot),
            ("hud.key_cap", h.key_cap),
            ("hud.seed", h.seed),
            ("hud.wreck_flash", h.wreck_flash),
            ("countdown.number", n.number),
            ("countdown.go", n.go),
            ("countdown.end", n.end),
            ("postgame.backdrop", p.backdrop),
            ("postgame.card", p.card),
            ("postgame.winner", p.winner),
            ("postgame.fish_box", p.fish_box),
        ];
        for (name, c) in named {
            colour_ok(name, c)?;
        }
        for (name, s) in &self.popups.styles {
            for c in [s.fill, s.outline, s.ink] {
                colour_ok(&format!("popups.styles.{name}"), c)?;
            }
            if !ICONS.contains(&s.icon.as_str()) {
                return Err(format!("The popup style \"{name}\" has no icon \"{}\".", s.icon));
            }
        }
        for (i, r) in self.popups.rows.iter().enumerate() {
            let at = format!("Popup row {} ({})", i + 1, r.event);
            if !self.popups.styles.contains_key(&r.style) {
                return Err(format!("{at} names the style \"{}\", which is not listed.", r.style));
            }
            if !["", "all", "mine", "others"].contains(&r.who.as_str()) {
                return Err(format!("{at} has who \"{}\"; who is all, mine or others.", r.who));
            }
            for (key, how) in &r.fill {
                let kind = how.split_once(':').map(|(k, _)| k).unwrap_or("");
                if !FILLS.contains(&kind) {
                    return Err(format!(
                        "{at} fills {{{key}}} by \"{how}\"; a fill is name:, fish:, tier: or text: and a field."
                    ));
                }
            }
            if self.popups.styles[&r.style].tier && r.tier.is_empty() {
                return Err(format!("{at} has a tier style but names no tier field."));
            }
        }
        if self.popups.max_shown == 0 {
            return Err("The popups need room for at least one.".into());
        }
        let b = &self.burst;
        if b.colours.is_empty() {
            return Err("The burst needs at least one colour.".into());
        }
        for c in &b.colours {
            colour_ok("burst.colours", *c)?;
        }
        let times = [
            h.hint_fade_ms,
            h.wreck_flash_ms,
            n.pop_ms,
            n.go_ms,
            self.popups.slide_ms,
            self.popups.fade_ms,
            p.card_ms,
            p.count_ms,
            p.orbit_ms,
            b.ms,
            b.shake_ms,
        ];
        if times.iter().any(|t| *t <= 0) || p.first_ms < 0 || p.gap_ms < 0 || p.rank_ms < 0 {
            return Err("The flow's times must be more than 0 ms.".into());
        }
        if !(0..=100).contains(&self.ghost_pct) || !(1..=99).contains(&b.swell_pct) {
            return Err("The ghost's opacity and the burst's swell are percents.".into());
        }
        Ok(())
    }

    pub fn display(&self) -> &Font {
        &self.fonts["display"]
    }

    pub fn body(&self) -> &Font {
        &self.fonts["body"]
    }
}
