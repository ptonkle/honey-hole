//! Items shared online (`crate::online`, "Adding a claim"; SECOND-ORDER
//! R4.27). Offline none of this runs: the match decides everything itself.
//!
//! - **The buoy is the host's.** Every browser keeps the buoy's clock (the
//!   warning and the five beats come on the shared clock), but only the
//!   host places a buoy, at its fair point, and tells everyone
//!   (`WorldEvent::BuoySpawned`, with the buoy's number).
//! - **A pickup is a claim** (`Claim::PickBuoy`). A browser whose own boat
//!   (or, on the host, an AI player's) reaches the buoy claims it once; the
//!   host grants the first claim made ([`claims::pickup`]'s rule, settled
//!   by the claims' order) and every browser takes the buoy away. The
//!   taker's browser draws its item itself: it stays hidden, and the
//!   others' `ItemPicked` carries a stand-in item that no popup reads (the
//!   others' popup names only the player).
//! - **A use is a claim with its effect** (`Claim::UseItem`), always
//!   granted: the item works at once on its owner's browser, and the
//!   others learn of it. Nitrous travels in the boat's state, the cloak in
//!   `online.hidden` (and the others wipe its blips now); a teleport's
//!   destination and charge go to every browser, which draws its shimmer
//!   and raises `Teleported`; Power Reel, the camera and Enhanced Radar are
//!   the owner's alone.

use serde_json::json;

use super::{claims, Charge, Item};
use crate::map::P;
use crate::sim::remote::{Claim, Role, WorldEvent};
use crate::sim::{Match, MatchEvent};

/// The item a remote seat's `ItemPicked` names: the others never see the
/// real one.
const STAND_IN: Item = Item::Cloak;

impl Match {
    /// Online, the boats this browser may claim the buoy for: its own, and
    /// on the host the AI players'.
    pub(super) fn buoy_takers(&self) -> Vec<(usize, P)> {
        self.boats
            .iter()
            .flatten()
            .filter(|b| !b.wrecked() && !self.is_remote(b.seat))
            .map(|b| (b.seat, b.at()))
            .collect()
    }

    /// Online pickups: claim the buoy once for a boat of ours that reaches
    /// it. True when online (the offline pickup does not run).
    pub(super) fn online_pickups(&mut self) -> bool {
        if self.online.role == Role::Offline {
            return false;
        }
        let boats = self.buoy_takers();
        let Some(seat) = claims::pickup(self.items.buoy, &boats, self.items.tuning.pickup_radius_mm) else {
            return true;
        };
        let buoy = self.items.spawned;
        if self.items.claimed != Some(buoy) {
            self.items.claimed = Some(buoy);
            self.claim(seat, Claim::PickBuoy { buoy });
        }
        true
    }

    /// The host places the buoy and tells everyone; a joiner waits for its
    /// word. True when this browser places it.
    pub(super) fn places_buoys(&self) -> bool {
        self.online.role != Role::Client
    }

    /// After the host placed a buoy: everyone hears of it.
    pub(super) fn share_buoy(&mut self, at: P) {
        let buoy = self.items.spawned;
        self.broadcast(WorldEvent::BuoySpawned {
            buoy,
            at: [at.x, at.y],
        });
    }

    /// Online, a use's effect goes to the others (always granted).
    pub(super) fn share_use(&mut self, seat: usize, item: Item) {
        if self.online.role == Role::Offline || self.is_remote(seat) {
            return;
        }
        let effect = match (item, self.items.seats[seat].charge) {
            (Item::Teleporter, Some(c)) => json!({ "to": [c.to.x, c.to.y], "ticks": c.until - c.from }),
            _ => json!({}),
        };
        self.claim(
            seat,
            Claim::UseItem {
                item: item.key().into(),
                effect,
            },
        );
    }

    /// Before the items' tick: the host's buoys, pickups and the others'
    /// uses.
    pub(super) fn items_shared_before(&mut self) {
        if self.online.role == Role::Offline {
            return;
        }
        for e in self.online.world.clone() {
            if let WorldEvent::BuoySpawned { buoy, at } = e {
                if self.online.role == Role::Client {
                    self.place_buoy(P::new(at[0], at[1]));
                    self.items.spawned = buoy;
                }
            }
        }
        for v in self.online.verdicts.clone() {
            let local = !self.is_remote(v.seat);
            match v.claim {
                Claim::PickBuoy { buoy } if v.granted => {
                    if buoy == self.items.spawned {
                        self.items.buoy = None;
                    }
                    if local {
                        let item = self.draw_item();
                        let dropped = self.items.seats[v.seat].held.replace(item);
                        self.events.push(MatchEvent::ItemPicked {
                            seat: v.seat,
                            item,
                            dropped,
                        });
                    } else {
                        self.events.push(MatchEvent::ItemPicked {
                            seat: v.seat,
                            item: STAND_IN,
                            dropped: None,
                        });
                    }
                }
                Claim::UseItem { item, effect } if !local => {
                    let Some(item) = Item::from_key(&item) else {
                        continue;
                    };
                    match item {
                        Item::Cloak => self.wipe_blips(v.seat),
                        Item::Teleporter => {
                            let to = effect["to"].as_array().and_then(|a| {
                                Some(P::new(a.first()?.as_i64()? as i32, a.get(1)?.as_i64()? as i32))
                            });
                            let span = effect["ticks"].as_u64().unwrap_or(1).max(1) as u32;
                            if let Some(to) = to {
                                let tick = self.tick;
                                self.items.seats[v.seat].charge = Some(Charge {
                                    to,
                                    from: tick,
                                    until: tick + span,
                                });
                            }
                        }
                        _ => {}
                    }
                    self.events.push(MatchEvent::ItemUsed { seat: v.seat, item });
                }
                _ => {}
            }
        }
    }
}
