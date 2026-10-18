//! Two apps joined by hand through their page doors (`net_event`,
//! `net_pump`), as `web/net.js` joins them: the Play choice, the Online
//! screen, the Lobby, the start and a boat driven in one moving in the other.

use super::*;
use crate::app::play::tests::lobby;

/// Runs both apps for `ms`, 20 ms a frame, carrying each one's text to the
/// other (A is "a" to B, B is "b" to A).
pub(crate) fn run(a: &mut App, b: &mut App, now: &mut i64, ms: i64) {
    for _ in 0..ms / 20 {
        *now += 20;
        let pa = a.net_pump(*now * 1000);
        let pb = b.net_pump(*now * 1000);
        for o in pa.out {
            if o.to.as_deref().is_none_or(|t| t == "b") {
                b.net_event("message", "a", &o.text, false);
            }
        }
        for o in pb.out {
            if o.to.as_deref().is_none_or(|t| t == "a") {
                a.net_event("message", "b", &o.text, false);
            }
        }
    }
}

fn ids(a: &App) -> Vec<String> {
    let v = serde_json::to_value(a.ui()).unwrap();
    let mut out = Vec::new();
    for p in v["panels"].as_array().unwrap() {
        for item in p["items"].as_array().unwrap() {
            if let Some(id) = item["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    out
}

fn disabled(a: &App, id: &str) -> bool {
    let v = serde_json::to_value(a.ui()).unwrap();
    v["panels"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["items"].as_array().unwrap().iter())
        .find(|i| i["id"] == id)
        .map(|i| i["disabled"] == true)
        .unwrap_or(true)
}

/// Host A and joiner B in A's Lobby by room code.
pub(crate) fn meet() -> (App, App, i64) {
    let (mut a, mut b) = (lobby(), lobby());
    let mut now = 1000;
    a.advance(now * 1000);
    b.advance(now * 1000);
    for app in [&mut a, &mut b] {
        app.act("menu.back", "").unwrap();
        app.act("menu.play", "").unwrap();
        assert_eq!(app.screen, Screen::PlayMenu);
        app.act("menu.online", "").unwrap();
        assert_eq!(app.screen, Screen::Online);
    }
    a.act("online.name", "Hana").unwrap();
    b.act("online.name", "Bea").unwrap();
    a.act("online.host", "").unwrap();
    assert_eq!(a.screen, Screen::OnlineLobby);
    let code = a.online.room.clone();
    assert_eq!(code.len(), online::CODE_LEN);
    let req = a.net_pump(now * 1000).requests;
    assert_eq!(req[0]["op"], "host_room");
    assert_eq!(req[0]["room"], online::room_name(&code));
    b.act("online.room", &code.to_lowercase()).unwrap();
    b.act("online.join", "").unwrap();
    let req = b.net_pump(now * 1000).requests;
    assert_eq!(req[0]["op"], "join_room");
    assert_eq!(req[0]["room"], online::room_name(&code));
    a.net_event("connected", "b", "", false);
    b.net_event("connected", "a", "", true);
    run(&mut a, &mut b, &mut now, 1000);
    (a, b, now)
}

/// A and B in a match together (no AI players).
pub(crate) fn playing() -> (App, App, i64) {
    let (mut a, mut b, mut now) = meet();
    b.act("online.ready", "").unwrap();
    run(&mut a, &mut b, &mut now, 1000);
    a.act("online.start", "").unwrap();
    run(&mut a, &mut b, &mut now, 2000);
    assert_eq!((a.screen, b.screen), (Screen::Play, Screen::Play));
    (a, b, now)
}

#[test]
fn play_offers_offline_and_online() {
    let mut a = lobby();
    a.act("menu.back", "").unwrap();
    a.act("menu.play", "").unwrap();
    assert_eq!(ids(&a), ["menu.offline", "menu.online", "menu.back"]);
    a.act("menu.offline", "").unwrap();
    assert_eq!(a.screen, Screen::Lobby, "Offline is New match");
    a.act("menu.back", "").unwrap();
    a.act("menu.play", "").unwrap();
    a.act("menu.online", "").unwrap();
    assert_eq!(
        ids(&a),
        [
            "online.name",
            "online.host",
            "online.room",
            "online.join",
            "menu.back"
        ]
    );
    a.act("online.room", "AB").unwrap();
    a.act("online.join", "").unwrap();
    assert_eq!(a.screen, Screen::Online, "a short code joins nothing");
    let sub = serde_json::to_value(a.ui()).unwrap()["panels"][0]["sub"].clone();
    assert_eq!(sub, "ONLINE.BAD_ROOM", "and says why");
    a.act("flag.codes", "on").unwrap();
    assert!(
        ids(&a).contains(&"online.join_code".to_string()),
        "?codes offers pasted codes"
    );
}

#[test]
fn two_apps_meet_in_a_lobby_and_start_a_match_together() {
    let (mut a, mut b, mut now) = meet();
    let roster = a.online.session.as_ref().unwrap().roster().clone();
    assert_eq!(roster.seats[0].name, "Hana");
    assert_eq!(roster.seats[1].name, "Bea");
    assert!(roster.seats[1].ping_ms.is_some(), "the host shows B's ping");
    assert_eq!(b.screen, Screen::OnlineLobby);
    assert!(
        ids(&b).contains(&"online.ready".to_string()),
        "B can mark itself Ready"
    );
    assert!(
        !ids(&b).contains(&"online.start".to_string()),
        "only the host starts"
    );
    assert!(disabled(&a, "online.start"), "Start waits for B's Ready");
    // the host's setup: one AI player, which B's lobby shows
    a.act("lobby.ai", "1").unwrap();
    b.act("online.ready", "").unwrap();
    run(&mut a, &mut b, &mut now, 1000);
    let r = b.online.session.as_ref().unwrap().roster().clone();
    assert!(matches!(r.seats[2].who, online::Who::Ai { .. }), "{:?}", r.seats);
    assert!(!disabled(&a, "online.start"), "Start comes on once B is Ready");
    a.act("online.start", "").unwrap();
    run(&mut a, &mut b, &mut now, 2000);
    assert_eq!(a.screen, Screen::Play);
    assert_eq!(b.screen, Screen::Play);
    let (ga, gb) = (&a.play.as_ref().unwrap().game, &b.play.as_ref().unwrap().game);
    assert_eq!(ga.setup, gb.setup, "the same seats");
    assert_eq!(ga.map.seed, gb.map.seed, "the same lake");
    assert_eq!(b.play.as_ref().unwrap().seat, 1);
    assert!(!ga.setup.offline, "an online match does not pause");
    // B drives; A sees B's boat leave the spawn
    let spawn = ga.spawn_slot(1).0;
    b.act("key", "KeyW").unwrap();
    run(&mut a, &mut b, &mut now, 2000);
    let seen = a.play.as_ref().unwrap().game.boat(1).unwrap().at();
    let moved = crate::map::trig::hypot((seen.x - spawn.x) as i64, (seen.y - spawn.y) as i64);
    assert!(moved > 3000, "A sees B's boat move ({moved} mm)");
    assert_eq!(
        a.net_pump(now * 1000).remote_moved,
        Some(true),
        "and tells the page"
    );
}

#[test]
fn the_host_leaving_ends_the_joiners_match_and_says_so() {
    let (mut a, mut b, mut now) = playing();
    a.act("menu.quit", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
    run(&mut a, &mut b, &mut now, 200);
    assert!(b.play.as_ref().unwrap().game.ended);
    let v = serde_json::to_value(b.ui()).unwrap();
    let text = v["panels"].to_string();
    assert!(text.contains("ONLINE.HOST_LEFT"), "{text}");
}

#[test]
fn a_joiner_leaving_the_lobby_frees_its_seat() {
    let (mut a, mut b, mut now) = meet();
    b.act("online.leave", "").unwrap();
    assert_eq!(b.screen, Screen::Online);
    let p = b.net_pump(now * 1000);
    assert_eq!(
        p.requests[0]["op"], "leave",
        "the links close after the parting word"
    );
    for o in p.out {
        a.net_event("message", "b", &o.text, false);
    }
    run(&mut a, &mut b, &mut now, 200);
    let r = a.online.session.as_ref().unwrap().roster();
    assert_eq!(r.seats[1].who, online::Who::Open);
    assert!(b.online.session.is_none());
}
