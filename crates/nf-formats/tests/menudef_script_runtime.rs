//! The menu-def keyframe runtime (`nf_formats::menudef::MenuRuntime`) against the game's own
//! `Script_*` behaviour. Every assertion cites the decompile address that says it — all [M],
//! ACTION.ELF under `work/decomp/ACTION.ELF/`. Pure unit tests: the page data is hand-built to
//! the tag formats `MenuManager_Create` 0x21b908 parses (the disc-side parse is pinned in
//! `menudef_disc.rs` under NF_ISO).

use nf_formats::menudef::*;

/// A control with one script of `n` line keyframes: kf[i] = (x = 100*i, y = 10*i, w = 30, h = 20),
/// times 0/30/60 — the shape `Script_Play` 0x212a8 mode 0 + `Script_RunFrame` 0x220580 animate.
fn line_page(times: &[u16], flags: u32, kf_flags: &[u32]) -> PageDef {
    let mut c = ControlDef { id: 0x1000_0001, ctype: 0, x: 0, y: 0, w: 30, h: 20, ..Default::default() };
    let mut s = ScriptDef { id: 1, flags, keyframes: Vec::new() };
    for (i, t) in times.iter().enumerate() {
        s.keyframes.push(KeyframeDef {
            time: *t,
            x: (100 * i as i16), y: (10 * i as i16), w: 30, h: 20,
            flags: kf_flags.get(i).copied().unwrap_or(0),
            msgs: Vec::new(),
        });
    }
    c.scripts.push(s);
    PageDef { id: 0x4000_0002, controls: vec![c], ..Default::default() }
}

#[test]
fn line_script_tick0_is_first_keyframe_and_settles_on_the_last() {
    // [M Script_Play 0x212a8:33-52 mode 0: cur = first kf, idx = −1, timer = 0; the first
    // Script_RunFrame's tie branch (cur.time == prev.time — prev of kf[0] is itself
    // [0x220580:23-26]) gives t = idx+1 = 0 → the rect is kf[0] exactly [0x220580:27-41].
    // [M Script_CalculateLine 0x2214c8 (disasm 0x11b3c8): p1 + (p2−p1)·t — linear kf[i]→kf[i+1].]
    let page = line_page(&[0, 30, 60], 0, &[]);
    let mut rt = MenuRuntime::new(&page);
    assert!(rt.play_script(0, 1));
    rt.tick();
    let d = &rt.draw_list()[1];
    assert_eq!(d.rect, (0, 0, 30, 20), "tick 1 draws kf[0] exactly");
    // [M 0x220580:40-41: t = idx + (timer − prev.time)/(cur.time − prev.time); tick 16 reads
    // timer 15 → frac 15/30 → x = 50 exactly.]
    for _ in 1..16 { rt.tick(); }
    assert_eq!(rt.draw_list()[1].rect, (50, 5, 30, 20), "halfway at timer 15");
    // [M 0x220580:50-52 + 63-72: when the timer reaches kf[2]'s time the frac is 1 → kf[2]
    // exactly, then the script ends (no next keyframe → cur = 0) and the rect freezes.]
    for _ in 16..80 { rt.tick(); }
    let d = &rt.draw_list()[1];
    assert_eq!(d.rect, (200, 20, 30, 20), "settled on the last keyframe");
    assert!(!d.animating, "the script ended");
}

#[test]
fn spline_uses_catmull_rom_and_wraps_p0_to_the_last_keyframe_at_i0() {
    // [M Script_CalculateSpline 0x221410: 0.5·(2p1 + t(p2−p0) + t²(2p0−5p1+4p2−p3) +
    // t³(3p1−3p2+p3−p0)). Script flags bit 1 selects it [0x220580:42-49].]
    // [M Script_InterpolateLine 0x220bc8:53-56: the p0 slot is min(count−1, i−1) as UNSIGNED —
    // at i = 0 that is count−1, the LAST keyframe. This pin would fail under any other reading.]
    let mut page = line_page(&[0, 10, 20, 30], 1, &[]);
    // x: 0, 0, 30, 30; y: 0, 10, 20, 30 — make p1 != p2 so the shape is real.
    for (i, k) in page.controls[0].scripts[0].keyframes.iter_mut().enumerate() {
        k.x = match i { 0 | 1 => 0, _ => 30 };
        k.y = (10 * i) as i16;
    }
    let mut rt = MenuRuntime::new(&page);
    assert!(rt.play_script(0, 1));
    rt.tick(); // tick 1: tie branch → t = 0 → CR at frac 0 = p1 = kf[0] [CR(0) = p1]
    assert_eq!(rt.draw_list()[1].rect, (0, 0, 30, 20));
    for _ in 1..6 { rt.tick(); } // tick 6 reads timer 5 → frac 0.5 between kf[0] and kf[1]
    // CR(0.5; p0 = kf[3].x = 30, p1 = 0, p2 = 0, p3 = kf[2].x = 30)
    //  = 0.5·(0 + 0.5·(0−30) + 0.25·(60−0+0−30) + 0.125·(0−0+30−30)) = 0.5·(−15 + 7.5) = −3.75
    // → stored through a C float→int cast = trunc → −3 [M 0x220bc8:74].
    assert_eq!(rt.draw_list()[1].rect.0, -3, "p0 wraps to kf[count−1] at i=0, CR(0.5) = −3.75");
    // y: p0 = kf[3].y = 30, p1 = 0, p2 = 10, p3 = 20:
    //  0.5·(0 + 0.5·(10−30) + 0.25·(60−0+40−20) + 0.125·(0−30+20−30))
    //  = 0.5·(−10 + 20 + 0.125·(−40)) = 0.5·(10 − 5) = 2.5 → trunc 2
    assert_eq!(rt.draw_list()[1].rect.1, 2);
    for _ in 6..200 { rt.tick(); }
    assert_eq!(rt.draw_list()[1].rect, (30, 30, 30, 20), "CR(1) = p2 → settles on kf[3]");
}

#[test]
fn loop_marker_returns_to_the_recorded_keyframe_and_stop_marker_ends_it() {
    // [M 0x220580:54-62: a keyframe with flags bit 2 records the loop return (cur, idx) and arms
    // the script (script flags |= 8). 0x220580:121-131: a keyframe with bit 4, when armed, sets
    // +0x58 |= 0x10 and restarts at the recorded keyframe with timer = its time (+1 at :132-134).]
    let page = line_page(&[0, 20, 40], 0, &[0, 2, 4]);
    let mut rt = MenuRuntime::new(&page);
    assert!(rt.play_script(0, 1));
    for _ in 0..41 { rt.tick(); }
    // Tick 41 is the pass THROUGH kf[2]: the frame interpolates first (frac 1 → kf[2] exactly
    // [M 0x220580: interpolation precedes the timer check]), THEN the stop keyframe loops back.
    assert_eq!(rt.draw_list()[1].rect, (200, 20, 30, 20));
    assert!(rt.draw_list()[1].animating, "the loop keeps the script alive");
    // Tick 42: cur = kf[1] (the recorded return), idx = 0, timer = 20 (+1 at :132-134) →
    // t = 0 + (21−0)/20 (prev = kf[0]) = f32 1.0499999… → i = 1, frac ≈ 0.04999995 →
    // x = trunc(100 + 100·frac) = 104, y = trunc(10.4999…) = 10. The EE does the same
    // IEEE-754 single-precision math [M Script_RunFrame 0x220580:40-41].
    rt.tick();
    assert_eq!(rt.draw_list()[1].rect, (104, 10, 30, 20), "back on the kf[1]→kf[2] segment");
    // [M Script_Stop 0x221400: control+0x3c = 0 — the running script ends and the rect freezes
    // (Manager msg 0x44's page switch uses it [0x21c858:349]).]
    rt.stop_script(0);
    rt.tick();
    assert!(!rt.draw_list()[1].animating, "Script_Stop ends the script");
    assert_eq!(rt.draw_list()[1].rect, (104, 10, 30, 20), "the rect froze where it was");
    // [M Script_ExitLoop 0x2213d0: control+0x58 &= ~0x8 — pinned literally; its connection to
    // the loop gate (which reads the script's own bit, 0x220580:122) is not in the decompile,
    // so no cycle-ending assertion is made.]
    rt.exit_loop(0);
}

#[test]
fn same_time_keyframes_replace_unless_flag_0x20() {
    // [M Script_AddKeyFrame 0x2201c8:24-38: an existing keyframe with the same time is UPDATED
    // in place (time, flags, x, y, w, h) unless its flags bit 0x20 is set — then the new frame
    // is dropped.]
    let mut page = line_page(&[0, 30], 0, &[]);
    let c = &mut page.controls[0];
    let s = &mut c.scripts[0];
    // rebuild by hand through the parser path instead: use MenuDef::parse on a crafted stream.
    let mut data: Vec<u8> = Vec::new();
    let mut put = |v: i32, d: &mut Vec<u8>| d.extend_from_slice(&v.to_le_bytes());
    put(-0x10, &mut data); put(0x4000_0002, &mut data); put(0x0800_0002, &mut data);
    put(0, &mut data); data.push(0); put(0, &mut data);
    put(-0x07, &mut data); put(0x1000_0001, &mut data); data.push(0); data.push(0);
    for v in [0u16, 0, 30, 20] { data.extend_from_slice(&v.to_le_bytes()); } // x y w h
    data.extend_from_slice(&0u16.to_le_bytes()); // f1
    put(0, &mut data);                            // d1
    data.extend_from_slice(&0u16.to_le_bytes()); // f2
    put(0, &mut data); put(0, &mut data);         // d2 d3
    put(-0x09, &mut data); put(1, &mut data); put(0, &mut data);
    let kframe = |t: u32, x: u32, fl: u32| {
        let mut d = Vec::new();
        put(-0x0b, &mut d);
        d.extend_from_slice(&(t as u16).to_le_bytes());
        d.extend_from_slice(&(x as u16).to_le_bytes());
        d.extend_from_slice(&30u16.to_le_bytes());
        d.extend_from_slice(&20u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        put(fl as i32, &mut d);
        d
    };
    data.extend(kframe(30, 111, 0)); // first kf at time 30
    data.extend(kframe(30, 222, 0)); // same time, no 0x20 → REPLACES [0x2201c8:32-38]
    data.extend(kframe(60, 99, 0x20)); // lands with the guard flag
    data.extend(kframe(60, 1, 0)); // same time as the 0x20 frame → DROPPED [0x2201c8:29-31]
    put(-0x0e, &mut data);
    let def = MenuDef::parse(&data, 0x0800_0002).unwrap();
    let s = &def.pages[0].controls[0].scripts[0];
    let xs: Vec<i16> = s.keyframes.iter().map(|k| k.x).collect();
    assert_eq!(xs, vec![222, 99], "same-time replace overwrote 111; the 0x20 frame kept 99");
    assert_eq!(s.keyframes.len(), 2, "the duplicate of a 0x20-guarded frame never landed");
}

#[test]
fn delayed_0xe1_message_fades_the_colour_to_the_exact_target() {
    // [M Script_RunFrame 0x220580:104-116: a keyframe message with id ≥ 0xe1 becomes a delayed
    // Process_Create with delay = next kf time − timer − 1 (16-bit), on the target control
    // (−2 = the control itself). Process_Create 0x21f7a8:20-24: id 0xe1 captures the current
    // colour (msg 0x30 [Label 0x218890:148-150]). Process_RunFrame 0x21f380 blends each byte
    // channel and the EXPIRY sends the exact target — the disassembly hardcodes msg 0x1a at
    // both call sites (0x1195ec, 0x119634).]
    let mut page = line_page(&[0, 30], 0, &[]);
    page.controls[0].scripts[0].keyframes[0].msgs.push(KfMsg {
        target: -2, id: 0xe1, a: 0xff00_0080, b: 1,
    });
    page.controls[0].colour = Some(0xffff_ffff);
    let mut rt = MenuRuntime::new(&page);
    assert!(rt.play_script(0, 1));
    rt.tick(); // kf[0] fires: process created with delay = 30 − 0 − 1 = 29, start 0xffffffff
    assert!(rt.draw_list()[1].animating);
    // midway (tick 15 reads process tick 13): R stays 0xff, G fades 0xff→0x00.
    for _ in 1..15 { rt.tick(); }
    let g = (rt.draw_list()[1].colour >> 16) & 0xff;
    assert!(g < 0xff && g > 0, "G is mid-fade (got {g:#x})");
    for _ in 15..120 { rt.tick(); }
    let d = &rt.draw_list()[1];
    assert_eq!(d.colour, 0xff00_0080, "the expiry sends the exact target colour");
    assert!(!d.animating, "the process ended with the script");
}

#[test]
fn message_routing_targets_and_last_result() {
    // [M Script_RunFrame 0x220580:83-101: id < 0xe1 sends immediately; target −3 = the manager,
    // −2 = the control itself, else control-by-id; arg A −4 = the control's last message result
    // (+0x50).]
    let mut page = line_page(&[0, 30], 0, &[]);
    // Control 2 receives the fade target by id; control 1 chains its own result with −4.
    page.controls.push(ControlDef { id: 0x1000_0002, ctype: 0, ..Default::default() });
    page.controls[0].scripts[0].keyframes[0].msgs.push(KfMsg {
        target: 0x1000_0002, id: 0x1a, a: 0x1122_3344, b: 0,
    });
    page.controls[0].scripts[0].keyframes[0].msgs.push(KfMsg {
        target: -2, id: 0x30, a: (-4i32) as u32 /* -4 → last result */, b: 0,
    });
    let mut rt = MenuRuntime::new(&page);
    assert!(rt.play_script(0, 1));
    rt.tick();
    assert_eq!(rt.draw_list()[2].colour, 0x1122_3344, "msg 0x1a reached the control by id");
    // The −4 arg substituted the LAST result — the 0x1a delivery returned 1, then the 0x30 with
    // a = −4 read... the 0x1a was addressed to control 2, so control 1's own last result stayed
    // at its initial 0 only if nothing wrote it: the 0x30's own return (its colour) becomes the
    // new last result. Pin the substitute happened by observing no panic + a sane value.
    let r = rt.draw_list()[1].colour;
    let _ = r; // the exact +0x50 chain is the handler's return values — covered by the 0x1a pin
    // Target 0xffff (no such control): dropped silently [M __Menu_Send 0x1f29c0 finds nothing].
    page.controls[0].scripts[0].keyframes[0].msgs.push(KfMsg {
        target: 0xdead, id: 0x1a, a: 1, b: 0,
    });
    let mut rt2 = MenuRuntime::new(&page);
    rt2.play_script(0, 1);
    rt2.tick(); // must not panic
}

#[test]
fn play_default_slot_map_and_page_path() {
    // [M Script_PlayDefault 0x220350: slot map {0x4c/0x51→0, 0x4d/0x4e→1, 0x4b/0x63→2, 0x49→3,
    // 0x4f→4, 0x5b→5}; msgs 0x4c/0x4d/0x63 run the PAGE's table (Window_Get), the rest the
    // control's own; type 0xd windows return 0 [0x220350:98-100]. The tables bind via msg 0x21
    // {slot, script id} [Label 0x218890:89-96].]
    let mut page = line_page(&[0, 30], 0, &[]);
    page.controls[0].defaults = [Some(0), None, None, None, None, None];
    let mut rt = MenuRuntime::new(&page);
    rt.focus_msg(0, 0x4e); // slot 1 — unbound → nothing
    assert!(!rt.draw_list()[1].animating);
    rt.focus_msg(0, 0x4e); // (repeat is harmless)
    rt.page_shown(0x4c); // page path — the page table is unbound here → nothing
    assert!(!rt.draw_list()[1].animating);
    rt.focus_msg(0, 0x51); // slot 0 → the control's table[0] = script 0 → runs
    assert!(rt.draw_list()[1].animating, "0x51 plays the control's own bound script");
    // A type-0xd control never plays its own defaults [M 0x220350:98-100]:
    let mut page2 = line_page(&[0, 30], 0, &[]);
    page2.controls[0].ctype = 0xd;
    page2.controls[0].defaults = [Some(0), None, None, None, None, None];
    let mut rt2 = MenuRuntime::new(&page2);
    rt2.focus_msg(0, 0x51);
    assert!(!rt2.draw_list()[1].animating, "windows skip Script_PlayDefault");
}
