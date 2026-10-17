# 744 mp-map-objects — done (partial, the sim core; wiring listed)

Branch s/mp-map-objects @ 5b24ab01. Report: docs/reports/mp-map-objects.md (full census +
addresses + live checks).

- The gondolas move: root cause was nis::Player::prime skipping mask-0 script players; the game
  primes every mode-A>=2 SP at create regardless of mask [SP_CreateScriptPlayer 0x1db9a8:72-76,
  SP_LoadScript 0x1dbc08; SP_Update 0x1dbda8:57-59 stops only on the +0x36 disabled byte whose
  sole writer SP_HideObj 0x1dcbf0 is copter-only]. Fixed + modelled the byte (Player.hidden).
  Skyrail's two carriages now ride the loop through the real script machinery (test pins it).
- Census of all 8 arenas [D]: the "remote tanks" are 4x 0xDA Car_Create on every arena except
  Sub Pen; no 0x2F copter anywhere — the "helicopters" are the Car's Enviro-Mods option-2
  variant (model 0x2000194). Ravine's 0xDA placements never create (GameState check 0x1305b0:19).
- The Car (remote tank/helicopter) state machine ported to car.rs with every constant cited
  [Car_Create 0x1305b0 / Car_Activate 0x131c48 / Car_Deactivate 0x131d88 / Car_Update 0x1307d8
  state-99 + heat + heli input conditioning + shell/overheat]; 5 tests.
- NOT done yet: wiring Car into the placement walk + Player_Activate case 0x28 + control walk
  (the machine exists, the sim hooks are the next session); Car_Update's tank drive branch +
  matrix composition + MG arm; the renderer's carriage draw on MP arenas (main agent's side).
- Verification: nf-game release suite 475 passed / 0 failed (incl. 9 new tests); all four
  crates compile. The full 4-crate suite runner could not fit the sandbox window (background
  processes are killed between tool calls here; foreground runs of nf-game done twice instead).
- Live checks in the report: 07000005's hidden-pair setter; Skyrail carriage render spots
  (--bond 12,32,-50,0,-0.5); Car_Create/Deactivate Mat_Copy arg order (disasm).
