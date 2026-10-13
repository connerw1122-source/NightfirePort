# The flicker lamps (placement type 0xe3, class 0x26)

`Flicker_Create` (0x1acb00, 208 B) / `Flicker_Update` (0x1ab960, 808 B). Ported in
`crates/nf-game/src/world_fx.rs` (fix-world-fx). The mesh itself already drew (the MISC_KINDS pass,
nf-view/src/scene.rs — `Flicker_Create` 0x1acb00:49 attaches the placement's glist); what was
missing was the tint ramp, and the ramp turns out to be nearly nothing on this build.

## Create (0x1acb00)

data+0 = the map light pointer (0 until found), data+4 (u16) = param 0 (the spark gate),
data+0x20 = param 1 (the signal channel) [M 0x1acb00:40-47]. The tint bytes are obj+0x103..105.

## Update (0x1ab960) — and the stub that guts it

`Light_FindMapLight` (0x1cc3d0, **8 bytes**) returns 0 unconditionally on this build [M] — the
found-light pointer stays 0, so every tick takes the not-found path:

- the tint target is WHITE (255,255,255); obj+0x103..105 move 10 % per tick toward it (min ±1) [M
  0x1ab960:101-118];
- the signal channel (param 1) is stamped **0** every tick; the light-off path (black target, the
  channel stamped 1, the spark puffs) is unreachable [M 0x1ab960:60-99];
- the spark branch (`Gas_Init` with a `Rand_Rand(5)` mesh pick from the table at 0x2fcca0 =
  meshes 0x020002d1..0x020002d5 [D elf dump], then `Particles_Create` 0x1cf478) would never fire —
  and `Particles_Update` 0x1cf528 self-kills through the stubbed `psiParticleUpdate` 0x111290
  anyway [M]: the psi particle sim is dummied out of the PS2 build.

The port keeps exactly the live behaviour: the tint fades black → white over ~1 s and the channel
stamps 0. Rendering the live tint on the MISC instance is a listed follow-up (the static draw is
the steady state — full colour); the first second's ramp is the only visible difference.
