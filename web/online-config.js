// Online transport settings (PLAN.md decision 11, copied from Ape Race and
// Mandible), read by web/online.js, which loads only after a player presses
// Host or Join, and by web/net.js for how often it pumps the core.
//
// Builder exemptions for this one file, named in
// crates/core/tests/boundary.rs (`page_policy`): it may hold numbers, and it
// may name absolute `wss://` URLs. Everything else the page scanner bans is
// still banned here. The relay list below is also the list the
// Content-Security-Policy's connect-src may add to 'self'; the boundary test
// reads it from this file, so a relay added here and not to a page's CSP is
// blocked by the browser, and a relay in a CSP but not here fails the test.
// None of these numbers is a game rule; each is a transport timing or id.

export const ONLINE = {
  // The vendored Trystero Nostr bundle, pinned by sha256 in
  // web/vendor/SHA256SUMS and checked by scripts/build.sh. Imported only when
  // a player hosts or joins by room code.
  bundle: "./vendor/trystero-nostr-0.25.4.js",

  // Namespaces this game's rooms away from other Trystero apps on the same
  // public relays.
  appId: "honey-hole-p2p",

  // Encrypts session descriptions as they cross relays this project does not
  // run, so a relay cannot tamper with a handshake. Not a secret: it ships in
  // this file. Game traffic is DTLS-encrypted end to end regardless.
  password: "honey-hole",

  // The Nostr relays, pinned so the CSP can name them. All five come from
  // Trystero 0.25.4's own default list. Every page that loads web/online.js
  // must carry exactly these in connect-src (see BUILDER-SPEC.md, D4).
  relayUrls: [
    "wss://nos.lol",
    "wss://relay.mostr.pub",
    "wss://purplerelay.com",
    "wss://nostr.data.haus",
    "wss://relay.agorist.space",
  ],

  // STUN only: public servers that tell a browser its own public address.
  // Without TURN, two players both behind strict NATs cannot connect
  // (SECOND-ORDER R3.44, Ape Race's N2).
  stunServers: [
    { urls: "stun:stun.l.google.com:19302" },
    { urls: "stun:stun.cloudflare.com:3478" },
  ],

  // The negotiated data channel's stream id. Both ends create it with this id,
  // so there is no ondatachannel event and no race over who listened first.
  // Not 0 or 1: Trystero opens its own in-band channel, and in-band ids are
  // handed out from zero upwards.
  channelId: 40,

  // How long to wait for ICE gathering before a pasted code is shown anyway,
  // so an unreachable STUN server does not look like a hang.
  gatherTimeoutMs: 6000,

  // How often a link repeats its hello until the other end acknowledges it.
  // A message sent before the other end has created its side of a negotiated
  // channel is lost (vagrancy rtc.js), so the hello is repeated.
  helloRepeatMs: 250,

  // How long a room code waits for any relay to answer before reporting that
  // the relays look blocked.
  relayTimeoutMs: 15000,

  // How often web/net.js hands the clock to the core and sends what it
  // queued, besides every animation frame: a tab in the background gets no
  // frames, and the browser slows this timer to about once a second there
  // (web/net-clock.js keeps a background host going).
  pumpMs: 50,
};

// `?ice=none` drops the STUN servers, so tabs on one machine connect on host
// candidates alone and nothing leaves the origin. tests/browser/online.py
// uses it; a player never needs it.
export function iceServers() {
  const ice = new URLSearchParams(location.search).get("ice");
  return ice === "none" ? [] : ONLINE.stunServers;
}
