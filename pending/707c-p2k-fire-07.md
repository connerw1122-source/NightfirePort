## Your task: `p2k-fire-07`, the P2K never fires on 07000007 after the disguise lock

Found by level-07-interactables (707a, interact_07000007_live `the_party_vase_breaks_on_a_p2k_round`, left
`#[ignore]`): after the hall walk-in releases the weapon lock (ch 0x48, the P2K auto-drawn — weapon 2 out),
holding fire through `Sim::step_bond` for 100 ticks never spawns a shot (`last_shot` stays `None`). If the
real player can't shoot on 07 after the lock lifts, that is a game-breaking bug on the level. Find the fire gate
that holds it (suspects from the worker: the weapon-lock auto-draw's switch state vs the fire path's lockout
[M Player_Weapon 0x1a24c8:242-275]), compare with the game, fix it, and un-ignore the vase test (the vase,
statue #109 and the window #298 must break on the real path). Sister: the shot is still refused while the lock
channel holds.


