//! channel-persistence: the switch-channel table must behave like the game's `switch_channels`
//! (ACTION.ELF 0x26fc90) — a value holds until one of the game's own writers rewrites it, and every
//! per-tick rewrite the port does traces to a game writer:
//!   - the touch-clear pass `Trigger_Calc` 0x1b1900 (type-2 outs cleared, then `|=` the hit —
//!     `Trigger_Update` 0x1b1a78 case 2);
//!   - the mux recompute `Trigger_Update` cases 3/4/0xC (own OUT written every tick, 0 or 1);
//!   - the OUT copy case 5 (`switch_channels[input] = switch_channels[out]` for every nonzero
//!     input, every tick — this is what 750q saw as "ch 0x33 cleared within one tick" on
//!     07000001: the disc's OUT placement #296 copies out 0x32 → input 51 = 0x33 every tick
//!     [D 07000001 placements; the port's trigger dump]);
//!   - the sensors `Sensor_Update` 0x1af618 (state 0/2 clear their out every tick);
//!   - the flickers `Flicker_Update` 0x1ab960 and the emitters `Emitter_Update` 0x17b080 (both
//!     rewrite their channel every tick);
//!   - the doors `Door_SetState` 0x135618 (the channel mirrors the old state's motion bits on a
//!     motion transition — transition-only, not per tick);
//!   - the level latches `Trigger_Calc` 0x1b1900:21-26 (05: 0xa4 → 0x6f, 0b: 0x34 → 0x2d);
//!   - `Drone_InitComms` 0x139520:198-201 (alert → ch 0x97, unstamped).
//! Opt-in: NF_ISO.
#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use nf_game::triggers::kind;
use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load01() -> Walker {
    let mut w = Walker::new(nf_view::load(&iso(), "07000001.bin", false).expect("load 07000001"));
    w.top_up_health = true;
    w.wait_enabled(3000, "the castle load");
    w
}

/// The per-tick channel owners of a live sim, per the game's writer families [M — the header list].
/// Doors are a transition writer (not per tick) but their channel is a game owner either way; the
/// [S] stand-ins are listed separately so a census hit on them is never confused with a game owner.
fn owners(sim: &nf_view::scene::Sim, level: u32) -> std::collections::BTreeSet<u8> {
    let mut o = std::collections::BTreeSet::new();
    let d = &sim.drones;
    for t in &d.triggers {
        match t.kind {
            // The game rewrites every touch-family out per tick: the type-2 clear + `|= hit`
            // [M 0x1b1900's clear + 0x1b1a78 case 2], and the type-1 (TOUCH_ONCE) fall-through
            // into case 2 — `switch_channels[out] |= hit` is a STORE every tick while the channel
            // is off (a no-op 0-store when unhit; the port's hit-only write lands the same values).
            kind::TOUCH_ONCE | kind::TOUCH | kind::AND | kind::OR | kind::SEQ_AND => { o.insert(t.out); }
            // `Trigger_Update` case 5: the OUT copy's targets are its nonzero inputs.
            kind::OUT => { for &c in &t.list { if c != 0 { o.insert(c as u8); } } }
            _ => {}
        }
    }
    for dr in &d.doors.list { if dr.ch_open != 0 { o.insert(dr.ch_open as u8); } }
    for s in &d.sensors.list { if s.out != 0 { o.insert(s.out as u8); } }
    for f in &d.fx.flickers { if f.signal_channel != 0 { o.insert(f.signal_channel as u8); } }
    for e in &d.fx.emitters { if e.ch_out != 0 { o.insert(e.ch_out as u8); } }
    if level == 0x0700_0005 { o.insert(0x6f); }
    if level == 0x0700_000b { o.insert(0x2d); }
    o.insert(0x97); // Drone_InitComms 0x139520:198-201
    // Script-owned channels: the disc scripts' own event 0xD / 0x11 targets and the SP out writes
    // [M Script_EventHandler 0x1d94c0 — the game's script path writes switch_channels the same
    // way; D the level's script data]. Accumulated live by `Channels::script_channels`.
    o.extend(&d.channels.script_channels);
    o
}

/// The documented port-only [S] stand-in writers (never game owners).
fn standins(level: u32) -> std::collections::BTreeSet<u8> {
    let mut o = std::collections::BTreeSet::new();
    if level == 0x0700_004a { o.insert(0x22); o.insert(0x4a); } // docs/levels/0700004a.md §6
    o
}

/// A channel NOBODY owns (per the census above) holds its value across 600 real-input ticks: the
/// game's `switch_channels` has no per-tick recompute [M — the writer families are the header
/// list; every one of them writes only its own channel]. 750q's repro channel 0x33 turned out to
/// be OWNED on 07000001 (the OUT #296 copy, see the sister test), so this test pins the actual
/// claim on a channel the level's writer census leaves untouched. The channel is pinned (not
/// computed) so the assertion is stable: 0x3c carries no owner on 07000001 — no trigger out or
/// copy target, no door/sensor/flicker/emitter channel, no latch, not the 0x97 comms byte [D
/// census in `the_port_rewrites_only_game_owned_channels`].
#[test]
fn a_channel_nobody_owns_holds_600_ticks_on_07000001() {
    let mut w = load01();
    let level = 0x0700_0001;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let own = owners(sim, level);
        assert!(!own.contains(&0x3c), "the census owns ch 0x3c on 07000001 ({:?}) — pick another pin channel",
            own.iter().map(|c| format!("0x{c:02x}")).collect::<Vec<_>>());
    }
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let f = sim.drones.frame;
        sim.drones.channels.set(0x3c, 1, f);
    }
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x3c), "the set did not land");
    for i in 0..600 {
        w.tick(Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x3c),
            "ch 0x3c was cleared at census tick {i} (frame {}) — a non-owner rewrote it", sim.drones.frame);
    }
    // Sister (fails when the writer hunt breaks the mux semantics): an OWNED channel still tracks
    // its owner — ch 0x33 must NOT hold (the OUT #296 copy rewrites it from ch 0x32 every tick).
    {
        let sim = w.scene.sim.as_mut().unwrap();
        let f = sim.drones.frame;
        sim.drones.channels.set(0x3c, 0, f); // drop the pin; 0x3c must now read 0
        assert!(!sim.drones.channels.on(0x3c), "an explicit 0 write did not land");
    }
}

/// The 750q misdiagnosis, pinned as the game's own behaviour: 07000001's OUT trigger #296 (params
/// p0=0x32 out, p1=51=0x33 input [D]) copies `switch_channels[0x32]` into `switch_channels[0x33]`
/// EVERY TICK — `Trigger_Update` case 5 [M 0x1b1a78:147-161]: a channel value written from outside
/// its writer set is rewritten by the copy within one tick, exactly what the game would do. The
/// port must keep doing this (and 0x3c must keep holding — the sister pair).
#[test]
fn the_0x33_clear_is_the_discs_out_copy_on_07000001() {
    let mut w = load01();
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let t = sim.drones.triggers.iter()
            .find(|t| t.kind == kind::OUT && t.out == 0x32 && t.list.contains(&51))
            .expect("the disc's OUT #296 (out 0x32 → input 0x33) [D]");
        assert!(t.script.is_none(), "the copy is a placement trigger, not a script entity");
    }
    for want in [1u8, 0] {
        {
            let sim = w.scene.sim.as_mut().unwrap();
            let f = sim.drones.frame;
            sim.drones.channels.set(0x32, want, f);
            sim.drones.channels.set(0x33, want ^ 1, f); // a stale opposite value in the copy target
        }
        w.tick(Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.channels.v[0x33], want,
            "ch 0x33 must track ch 0x32 within one tick (the game's OUT copy, case 5)");
    }
}

/// The census across every on-foot level: idle through the real input path, record EVERY channel
/// write per tick (`Channels::writes`), and require each written channel to be a game owner (the
/// writer families in the header) or a documented [S] stand-in. A write outside both sets is a
/// port-only writer — the bug class 750q hunted.
#[test]
fn the_port_rewrites_only_game_owned_channels() {
    let levels: [(&str, u32); 23] = [
        ("07000001.bin", 0x0700_0001), ("07000002.bin", 0x0700_0002), ("07000003.bin", 0x0700_0003),
        ("07000004.bin", 0x0700_0004), ("07000005.bin", 0x0700_0005), ("07000006.bin", 0x0700_0006),
        ("07000007.bin", 0x0700_0007), ("07000008.bin", 0x0700_0008), ("07000009.bin", 0x0700_0009),
        ("0700000a.bin", 0x0700_000a), ("0700000b.bin", 0x0700_000b), ("0700000c.bin", 0x0700_000c),
        ("0700000d.bin", 0x0700_000d), ("07000011.bin", 0x0700_0011), ("07000012.bin", 0x0700_0012),
        ("07000013.bin", 0x0700_0013), ("07000014.bin", 0x0700_0014), ("07000015.bin", 0x0700_0015),
        ("07000016.bin", 0x0700_0016), ("0700001b.bin", 0x0700_001b), ("07000041.bin", 0x0700_0041),
        ("0700004a.bin", 0x0700_004a), ("0700004b.bin", 0x0700_004b),
    ];
    let mut any = false;
    for (name, level) in levels {
        let Ok(scene) = nf_view::load(&iso(), name, false) else { eprintln!("[census] {name}: load failed (skip)"); continue };
        let mut w = Walker::new(scene);
        w.top_up_health = true;
        // Soft enable wait (a level whose intro never ends is a census skip, not a failure — the
        // walkreal binaries cover those routes).
        let mut enabled = false;
        for _ in 0..3000 {
            w.tick(Input::default(), false);
            if !w.scene.sim.as_ref().unwrap().disabled { enabled = true; break; }
        }
        if !enabled { eprintln!("[census] {name}: never enabled in 3000 ticks (skip)"); continue; }
        let own = {
            let sim = w.scene.sim.as_ref().unwrap();
            owners(sim, level)
        };
        let st = standins(level);
        eprintln!("[census] {name}: {} owners at enable: {}", own.len(), own.iter().map(|c| format!("0x{c:02x}")).collect::<Vec<_>>().join(","));
        for tick in 0..120 {
            // `own` is the owner set AS OF THIS TICK's start (the port's tick order: the touch
            // clear + hits run first, the script pass that creates/deletes script triggers runs
            // later — so a script trigger created last tick is already an owner here, and one
            // deleted mid-tick still owned the writes its clear made this tick).
            let own = owners(w.scene.sim.as_ref().unwrap(), level);
            {
                let sim = w.scene.sim.as_mut().unwrap();
                sim.drones.channels.writes = Some(Vec::new());
            }
            w.tick(Input::default(), false);
            let writes = w.scene.sim.as_mut().unwrap().drones.channels.writes.take().unwrap_or_default();
            for &(c, v, f, src) in &writes {
                if src == 1 { continue; } // the script path's own writes [M 0x1d94c0; D script data]
                let sin = st.contains(&c);
                let mut game = own.contains(&c);
                if !game {
                    // A script-created touch trigger born mid-tick (between the snapshot and this
                    // tick's Trigger_Calc) owns its out from the tick it exists: re-check against
                    // the post-tick lists. Only a channel NO live object owns at either end of the
                    // tick is a port-only writer.
                    game = owners(w.scene.sim.as_ref().unwrap(), level).contains(&c);
                }
                assert!(game || sin,
                    "{name} tick {tick} frame {f}: ch 0x{c:02x} = {v} written by NO game owner (owners {:?}, stand-ins {:?}; triggers {:?})",
                    own.iter().map(|c| format!("0x{c:02x}")).collect::<Vec<_>>(),
                    st.iter().map(|c| format!("0x{c:02x}")).collect::<Vec<_>>(),
                    w.scene.sim.as_ref().unwrap().drones.triggers.iter().map(|t| format!("#{} k{} out 0x{:02x} script {:?} gone {}", t.placement, t.kind, t.out, t.script, t.gone)).collect::<Vec<_>>());
                if sin && !game {
                    eprintln!("[census] {name} tick {tick} frame {f}: ch 0x{c:02x} = {v} — [S] stand-in (documented)");
                }
            }
            any = true;
        }
    }
    assert!(any, "the census ran no level — NF_ISO broken?");
}
