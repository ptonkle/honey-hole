# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-17

> the builder project is a good core. There are a lot of good lessons from ape race too, but do not port over excessive recommendations or influence from ape race which would make the game feel less unique. Good, general things to port over are things like the idea of a design journal, the menu structure, the unit testing visibility and tagging enhancements that were worked on today, ask me any questions about things you should or orshouldn't learn from

The person's answers to the follow-up questions:

- Port from Ape Race: the design journal, test tags and the quick check, check visibility (nextest, progress bars, watch window, check times, a fast mode for browser checks), and the generic menu machinery. Port no Ape Race screens or content.
- Standard from the start: What's New (version numbers and patch notes). Replays and online play are left out until the game's own design calls for them.
- Glossary: keep `GLOSSARY.md` and its copy tests, but show no hover definitions on screen (Ape Race style).
- Project notes: lean. Start with the template notebooks and add checklists or archives only when the game needs them.

The agent decided: port the general tooling now, before the game is described, and keep every Ape Race name, screen and tuning choice out of it.

## 2026-10-17 (online)

> Online play will be a part of this but I want more care since this game will likely need different interactions between players compared to ape race

The agent decided: online play is in scope, but neither Ape Race's netcode (each browser owns its own ape and sends its state 20 times a second; the host runs the AI and results) nor Mandible's (lockstep over a full mesh) is copied by default. The sync model is chosen in `PLAN.md` from the game's own player interactions once the game is described, and agreed with the person before any transport code. Only the transport layer that is truly general (WebRTC introduced by room code over pinned Nostr relays, with its BUILDER-SPEC exemptions and `check-online.sh`) may be reused. Meanwhile the core is kept deterministic and driven by per-player inputs, so every sync model stays open.
