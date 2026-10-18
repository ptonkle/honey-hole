# Design questions, round 1 (2026-10-17)

Answer by number; a short answer is fine. Each question names the agent's suggestion, which stands if you have no preference.

## Balance and anti-fun risks

B1. **Ramming costs the rammer nothing.** The victim loses the fish on the line and up to 30 s of travel back from the spawn point; the rammer loses nothing. A player who only hunts could out-score a player who fishes. *Suggestion:* a ram also stalls the rammer's motor for about 3 s, and rams that wreck count toward nothing.

B2. **The angler can't see the attacker.** While the line is in the water the screen is the underwater camera, so a fishing player sees an attack coming only through the small picture-in-picture and the motor sound. *Suggestion:* keep that tension, but put a red edge pulse on the picture-in-picture when a rival is within about 5 s at their speed.

B3. **Spawn camping.** Everyone respawns at one spawn point, so a rammer can wait there. *Suggestion:* 3 s of ghosting after a respawn (can't wreck or be wrecked), and a wide spawn area.

B4. **Big fish may not be worth it.** With values of 1×/1.5×/2×/3× and reel times of 3/5/10/20 s, commons earn about 0.33 value per second of reeling and legendaries 0.15, and a legendary also gets announced and hunted for 20 s. Players would ignore legendaries. *Suggestion:* base values that rise faster than reel time, e.g. common 100, uncommon 250, rare 700, legendary 2,500 (×length factor), so one legendary is worth about 25 commons.

B5. **Teleporter plus the hook popup.** A Teleporter can drop someone right on top of a legendary hook, and the legendary is then almost always lost. *Suggestion:* the teleport lands no closer than about a third of the map to any player, or the 5 s charge is visible on the radar.

B6. **Fish running out.** 65 fish across 4 players in 8 minutes is about 16 each, if a full cycle (find, cast, bites, reel) takes about 30 s. The lake may run dry before the end, and late-match scarcity pushes everyone to hunt. *Suggestion:* let AI playtests measure this first, then set counts; keep the stated spread for now.

B7. **Grounding in fog.** Wrecking on land above 50% speed in fog that hides the shoreline will feel unfair to new players. *Suggestion:* the shoreline is always visible a little beyond the fog (a darker silhouette), and shallows (reeds) slow the boat before the land.

B8. **Leaders aren't visible.** Hidden scores protect the leader from being ganged up on, which is good, but hook popups let a careful player keep count. *Suggestion:* keep it as it is; just noting that it's a choice.

## Fishing

F1. "Bringing in a legendary fish should take 20 minutes": we read this as **20 seconds** (then 10, 5, 3). Right?
F2. **How do you find fish?** Visible shadows in the water (Animal Crossing), only the radar (Enhanced Radar), or blind casting near likely spots? *Suggestion:* faint shadows visible from the boat within a short range, and nothing on the normal radar.
F3. **Does the bobber's depth matter?** "Hit space to stop it dropping" suggests fish live at depths. *Suggestion:* yes: commons are shallow and legendaries are deep, so a deep cast takes longer.
F4. **Mode switching while reeling:** does pressing Q to drive away drop the fish? *Suggestion:* yes. You choose between fleeing and landing it.
F5. Does a fish that slips the line go back into the lake (and the Fishpedia count stay the same)? *Suggestion:* yes.

## Fish roster

R1. **Only real northern Michigan freshwater fish, or a few silly ones too?** Your examples were a shark, a clownfish and a tuna. *Suggestion:* real lake fish, plus one legendary "lake monster" per map for fun.
R2. The draft roster the model work has started on: bluegill and yellow perch (common), smallmouth bass and walleye (uncommon), northern pike (rare), muskellunge and lake sturgeon (legendary). How many kinds per tier? *Suggestion:* 4 common, 4 uncommon, 3 rare, 2 legendary, as in your wireframe.

## Items and the Fish-Boy

I1. Does Cloaking Tech also hide you from Enhanced Radar? *Suggestion:* yes.
I2. Does a normal radar ping show fish at all? *Suggestion:* no; players and the buoy only.
I3. Can you open the Fish-Boy while driving? *Suggestion:* yes, but the boat keeps going: a risk you choose to take.
I4. Is the hook popup's location shown anywhere (e.g. a flash on the radar)? *Suggestion:* no. Rivals must ping to find you, which gives the radar its job.

## Online and AI

O1. **What other interactions are planned besides ramming?** For example: tangling lines, stealing a catch, shared catches, chat or emotes. This decides how players are synced.
O2. Do AI players fill empty seats in online lobbies too, or only in offline matches? *Suggestion:* both, with the host choosing how many.
O3. Should AI difficulty levels exist? *Suggestion:* Easy / Normal / Hard.

## Models, project, publishing

P1. **Model files:** should you be able to edit them in Blender (we'd import .glb files), or are hand-readable data files that I write enough? *Suggestion:* data files now; a Blender import if you want to model some yourself.
P2. Rename the folder to `honey-hole/`? *Suggestion:* yes, now, before there's much history.
P3. Should settings be remembered between visits (R1.23)? *Suggestion:* yes, in browser storage.
P4. May I publish, and where (repository name, GitHub Pages)?
