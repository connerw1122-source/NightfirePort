# Merge log

One line per finished queue task: merged into master or not, and why. Written by the hourly check-in.

- 2026-10-12 **fix-npcs**: merged. Drones are hidden and undamageable during every cut-scene (`Drone_EnableAll(0)`); the searchlight spot ray now hits Bond's sphere, so standing in a beam raises the alarm; 07000006 combat/stealth music dispatch. The fix-hud "team gate" was correctly not ported. Suite 374/374 after all three merges.
- 2026-10-12 **fix-wire**: merged. The 07000005 cable (and 07000002's) is crossed hand-over-hand (`Player_CollWire` / `Player_Wire` / `Player_Zipline` ported); no more falling to death on E. Open: the third-person wire camera.
- 2026-10-12 **fix-player4**: merged. Fist punches knock guards out (state 0x42, logged as disabled, not killed) instead of killing them as bullets; left/right punch alternation from the shared RNG.
