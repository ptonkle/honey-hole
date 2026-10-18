// The settings kept between visits: the one file of the page that may
// touch browser storage (BUILDER-SPEC, the D17 change; the person's choice,
// brief/questions.md P3; tests/no_storage.rs holds it to this).
//
// It keeps one key, holding the settings exactly as the core wrote them
// (`ui.settings`). It reads nothing into the page: the stored text goes back
// to the core through `act("settings.load", ...)`, which checks every value
// and refuses a bad save whole. Every access is wrapped, so a private
// window or blocked storage only means the settings last for this visit.

const KEY = "honey-hole.settings";

// The last text written (or read), so an unchanged view model writes
// nothing.
let kept = null;

// Hands the stored settings to the core; returns the core's answer, or
// null when nothing is stored, storage is blocked or the core refused it.
export function loadSettings(game) {
  let text = null;
  try {
    text = window.localStorage.getItem(KEY);
  } catch (e) {
    return null;
  }
  if (text === null) return null;
  const result = JSON.parse(game.act("settings.load", text));
  if (result.error) return null;
  kept = text;
  return result;
}

// Stores the settings the core wrote, when they changed.
export function saveSettings(text) {
  if (!text || text === kept) return;
  kept = text;
  try {
    window.localStorage.setItem(KEY, text);
  } catch (e) {
    // storage full or blocked: the settings last for this visit
  }
}
