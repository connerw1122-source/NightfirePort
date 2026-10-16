# net-e2e-loopback — report

Task 706, branch `s/net-e2e-loopback`. Host and join an online match end to end over loopback:
two FrontEnds plus two Sims in one test, over 127.0.0.1 through nf-net.

## What the test does

`crates/nf-view/tests/net_e2e_loopback_live.rs` (NF_ISO-gated, both green):

- `host_and_join_an_online_match_end_to_end_over_loopback` — the full flow, **every menu step
  through FrontEnd actions (no direct page writes)**: the host FE walks Main → Multiplayer →
  Host Online → clears the default port (Backspace) → types `27950` → Start Listening (`mp_host_begin`
  → `sess.listen`) → the host lobby (the game's Join page); the client FE walks Join Online → types
  `127.0.0.1:27950` → Connect (`mp_net_dial` → `sess.connect`) → the lobby mirror. Both sides'
  panels fill from the session's agents (the codename rode the handshake). The host picks Arena
  (Deathmatch, scenario row 1 — row 0 is Quick Game) → Skyrail (map row 0) → the setup defaults →
  Game Rules (frag limit 10 → 2 through the real ◀ stepping) → Continue (the start checks pass on
  2 humans) → the confirm page → `mp_confirm_start`. Both machines hand over — the host locally,
  the client through the tunnelled match payload (`mp_client_start`) — and load the arena through
  the real `load_mp_carry` with the same tunnelled seed, each swapping its own slot's bond in.
  Then **1200 lockstep ticks** of scripted input through the real session (`local_input` →
  `ready_inputs` → `step_net_remotes` + the local `tick_ctx` → `set_slot` → the MP update), the
  **per-tick state hash asserted equal at every 15-tick exchange**, a scripted kill at tick 400
  through the game's own scoring path (`MP_PlayerKilled` 0x188f70, both machines, the same tick —
  the replay contract's cross-sim event) asserts slot 0's kill/score and slot 1's death on both,
  the second kill at 900 reaches the frag limit, the MP end machine runs 1 → 3 → 4 → 5
  [M MP_Update 0x184bd0] with `request_debriefing` on both, and both front ends open the
  debriefing (`open_mp_debriefing` ← GameState+0x1C = 0x40000033 [M 0x184e80]) with boards that
  agree on every number. Runs in ~6 s.
- `the_same_match_survives_5_percent_loss_and_reordering` — the SAME flow over a link that drops
  5% of the packets (50‰) and transposes 8% (80‰), seeded per side: the lockstep layer's ack-window
  redundancy absorbs it and every assertion holds unchanged. ~6 s.

## What was fixed along the way

1. **The lobby's re-bind left every other handle stale** (`crates/nf-view/src/mpnet.rs`):
   `SessionProxy::listen`/`connect` (the lobby's Start Listening / Connect) used to build a fresh
   `NetCore` and **replace the `Rc` inside the proxy** — the app's `net` handle, and any other
   holder of the proxy's core (a test's clone, a second FE), kept pointing at the stale placeholder
   socket. The e2e hit it immediately: the lockstep's `local_input` went to a core that was never
   listening. Fix: `NetCore::rebind_host(port)` / `rebind_client(addr)` swap a fresh `Session` **in
   place on the same core** (resetting the session state exactly like the old fresh core), and the
   proxy's listen/connect use them — every handle sees the new session. The codename carry
   (`set_codename` before listen, the old code's `display_name()` read) is preserved.
2. **Test-only fault injection in the transport** (`crates/nf-net/src/net.rs`,
   `src/session.rs`): `UdpTransport::inject_fault(seed, drop‰, reorder‰)` drops and one-slot
   transposes the SEND path from a seeded SplitMix64 (the same shape the nf-net loss tests use;
   the receive path and the protocol untouched); `Session::inject_link_fault` forwards it
   (concrete `Session<UdpTransport>` impl); `SessionProxy` carries the recipe as a second field so
   a lossy proxy stays lossy across the lobby's re-bind (`host_faulty`/`connect_faulty`
   constructors; production passes `None`).
3. **nf-net's `loopback_e2e` underflow** (`crates/nf-net/tests/loopback_e2e.rs:206`):
   `corrupt_client == Some(slot - 1) && slot > 0` evaluated `slot - 1` before the guard — slot 0 is
   the host, so both tests panicked in a debug run ("attempt to subtract with overflow"). The
   `slot > 0` guard now comes first. `cargo test -p nf-net`: 25/25 green (11 unit + 8 lockstep +
   2 loopback_e2e + 4 ported_session).

## Verification

- `cargo test -p nf-net` (offline): **25 passed**.
- The two new e2e tests (above): **both green** against the slim ISO.
- Full suite on the final tree (fast runner `python3 tools/test/suite.py`): **1896 passed, 6 failed, 72 ignored (47 min)** — the 6 are the 3 allowed `disc_golden` + the 3 pre-existing pylon render probes (the standing baseline); zero new failures. The build was 436 test binaries in 402 s.

## Suite total

`1896 passed, 6 failed, 72 ignored (47 min)` on the final tree (origin/master @ c10b530f already merged — master did not move during the task; the branch was cut from it). FAILED lines: `disc_golden::bigf_header_fields_behave_as_documented`, `disc_golden::bigf_tables_match_the_golden_manifest`, `disc_golden::directory_tree_matches_the_golden_manifest`, `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`, `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot`, `probe_pylon_render::probe_render_pylon_only` — all pre-existing, none from this task's changes. The 2 new e2e tests are green in the totals.

## What's left

- The codename wire gap (observation, not fixed): the tunnelled 208-byte `MatchSettings` has no
  name bytes, so the client's own `MpSettings.players[].codename` copies arrive empty — the
  debriefing boards agree on every number but the client's name cells are blank and its
  `MpEndStatus::Wins` argument is empty. `net_state_hash` folds no names, so lockstep is
  unaffected. A follow-up could carry the names in the DATA tunnel (the game's own MPSettings
  holds them at +slot·0x30 [M C_RBMPCNAME]).
- The menu start's seed stays the deterministic default (`MpSettings::deathmatch`'s 0 →
  `Rng::new`'s 0x1234_5678 [M mod.rs:166 + scene.rs:6063]); the app's `--net-host` path generates
  one, the menu path never did. Two consecutive menu-started matches on the same map share the
  level-build stream — harmless for determinism, listed as an observation.
- Live checks for the main agent: none required — the whole flow is verified headless. If the
  main agent wants one visual: the host lobby mirror and the debriefing board on a real
  2-machine run should show the codenames in the name cells.
