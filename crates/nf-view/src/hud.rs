//! The overlay as the game draws it (docs/systems/hud.md): the crosshair (`HUD_UpdateCrossHair` 0x19e6d0), the
//! health / armour pane (`HUD_UpdateHealthPane` 0x19b9c8) with the damage-direction glow, and the ammo pane
//! (`HUD_UpdateAmmoPane` 0x19a7f0). Sprites from `HealthSprInfo` / `AmmoSprInfo` / `BulletImg` and font 2, read from
//! ACTION.ELF; textures from the level archive. Sorted by layer, higher first (`CompSpr`: lower numbers on top).

use crate::gpu::{HudFrame, HudVertex};
use nf_formats::hud::SpriteInfo;

/// The game's screen (`Viewer+0x10C` / `+0x110` [L]): 512 × 448, shown at 4:3.
const SW: f32 = 512.0;
const SH: f32 = 448.0;

/// HUD textures the overlay uses (indices + 1 in `HudFrame` runs). `fix-hud` added the crouch icons
/// (0x03000173/74), the air bar (0x0300017F) and its bubble icon (0x0300017D) [M AirSprInfo/HealthSprInfo].
/// fix-context-hints added the third-icon context textures [M 0x2FC168 = 0x03000064+ctx; only 0x66
/// (winch, ctx 2), 0x67 (hanging man, ctx 3) and 0x6A (figure, ctx 6) exist on the disc [D tex-list] —
/// the table's other four ids never resolve in `hashtable_set_sprite` and the icon stays hidden].
pub const TEXTURES: [u32; 20] = [0x0300_0026, 0x0300_007a, 0x0300_0128, 0x0300_018d, 0x0300_018e, 0x0300_0003, 0x0300_0034, 0x0300_0002, 0x0300_0173, 0x0300_0174, 0x0300_017f, 0x0300_017d, 0x0300_0055, 0x0300_0056, 0x0300_0057, 0x0300_0090, 0x0300_0141, 0x0300_0066, 0x0300_0067, 0x0300_006a];
const CROSS_TEX: u32 = 0x0300_0026;
const FONT_TEX: u32 = 0x0300_0003;
/// SightPane textures [D 07000005.bin entries 71 + 56]: 0x03000034 = the scope overlay (256×256, the circular
/// vignette + crosshair reticle), 0x03000002 = the solid fill for the left/right letterbox strips (32×32 RGBA).
const SCOPE_TEX: u32 = 0x0300_0034;
const SCOPE_STRIP_TEX: u32 = 0x0300_0002;

/// Which zoom overlay a weapon draws [M `HUD_UpdateCrossHair` 0x19e6d0:106-147]: in aim mode a scoped
/// weapon (flags +0x68 & 0x40 [L weapon_data 0x2bf150]) enables its own zoom pane — the switch runs on
/// `wid − 0x1a`: 0/1 (0x1a/0x1b, the OICW twins) → pane 17; 0x18/0x19 (0x32/0x33, the Ronin twins) →
/// pane 19; 0x3a/0x3b (0x54/0x55, the watch; 0x55 is rewritten to 0x54 by the game itself) → pane 11;
/// the default (any other scoped weapon — the sniper 0x24, zoom 10 [L]) → pane 6, the scope. The
/// micro camera 0x52 / briefcase 0x53 are not scoped (their zoom is the mission layer's, unported)
/// but map to the Camera pane — the user's "micro camera ... camera overlay, not the sniper scope".
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ZoomOverlay { Sight, Oicw, Laser, Camera }

pub fn zoom_overlay(weapon: u8) -> ZoomOverlay {
    match weapon {
        0x1a | 0x1b => ZoomOverlay::Oicw,
        0x32 | 0x33 => ZoomOverlay::Laser,
        0x52 | 0x53 | 0x54 | 0x55 => ZoomOverlay::Camera,
        _ => ZoomOverlay::Sight,
    }
}

/// `HUDCrossCoords` (0x2bec48, u, v, w, h) [L]: 1 = aim mode, 2 = normal, 3 = friendly.
const CROSS: [[i16; 4]; 4] = [[0, 0, 0, 0], [1, 2, 29, 29], [10, 11, 11, 11], [31, 1, 14, 13]];
/// Pane origins after `HUD_ValidateXY`'s 16-pixel margins [L]: health (16, 337), ammo (496, 394).
const HEALTH_AT: (f32, f32) = (16.0, 337.0);
const AMMO_AT: (f32, f32) = (496.0, 394.0);

pub(crate) struct Quad { pub(crate) layer: i32, pub(crate) tex: usize, pub(crate) blend: u8, pub(crate) v: [HudVertex; 6] }

/// The script player's overlay (fix-script-commands), drawn ABOVE the (during a cut-scene blanked)
/// HUD the way the game draws them above its own: `Camera_SetFade` 0x1bbdd8's black overlay (op 0x13
/// + `Script_Update`'s length-20 fade-out and the −22 handback fade-in), the script camera's
/// letterbox borders (`Script_CameraStart` 0x1d9140:49-61 — two solid black 512×64 bars, the bottom
/// at y 0x180 = 384 [L ACTION.ELF SpriteInfo 0x2c9468]) and the op-0x14 script sprites (their rects
/// already scaled to this 512×448 space by `Script_SpriteStart`; drawn only when their texture hash
/// is loaded, else logged once — the live check decides the visual).
pub fn script_overlay(fade: Option<nf_game::drone::ViewFade>, letterbox: bool, sprites: &[(u32, [f32; 4])], textures: &[(u32, crate::scene::TextureImage)], aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures };
    if letterbox {
        h.quad(Anchor::Stretch, 1, SCOPE_STRIP_TEX, 0, 0.0, 0.0, SW, 64.0, [0.0; 4], (false, false), [0.0, 0.0, 0.0, 1.0]);
        h.quad(Anchor::Stretch, 1, SCOPE_STRIP_TEX, 0, 0.0, 384.0, SW, 64.0, [0.0; 4], (false, false), [0.0, 0.0, 0.0, 1.0]);
    }
    for &(hash, rect) in sprites {
        if let Some((_, t)) = textures.iter().find(|(k, _)| *k == hash) {
            let _ = t;
            h.quad(Anchor::Stretch, 2, hash, 1, rect[0], rect[1], rect[2], rect[3], [0.0; 4], (false, false), [1.0, 1.0, 1.0, 1.0]);
        }
    }
    if let Some(f) = fade {
        if f.alpha > 0.0 {
            h.quad(Anchor::Stretch, 0, SCOPE_STRIP_TEX, 0, 0.0, 0.0, SW, SH, [0.0; 4], (false, false), [0.0, 0.0, 0.0, f.alpha.min(1.0)]);
        }
    }
    h.finish()
}

/// How a sprite keeps its place in a window wider (or narrower) than the game's 4:3 screen: pixels keep the 4:3
/// shape, and each element holds to the edge it belongs to — the health pane left, the ammo pane right, the crosshair
/// centred; full-screen sprites (the damage glow) stretch over the whole window. *Port design* (the PS2 screen was 4:3
/// or a 16:9 stretch of the same 512 × 448 space).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Anchor { Left, Centre, Right, Stretch }

pub(crate) struct Hud<'a> { pub(crate) quads: Vec<Quad>, pub(crate) aspect: f32, pub(crate) textures: &'a [(u32, crate::scene::TextureImage)] }

/// A colour 0xRRGGBB on the GS scale (0x80 = 1.0) with an alpha byte (drawn as A >> 1, 0x80 = opaque).
fn gs(rgb: u32, a: u32) -> [f32; 4] {
    [((rgb >> 16) & 0xff) as f32 / 128.0, ((rgb >> 8) & 0xff) as f32 / 128.0, (rgb & 0xff) as f32 / 128.0, ((a.min(255) >> 1) as f32 / 128.0).min(1.0)]
}

impl Hud<'_> {
    /// A texture's pixel size (for the w/h-0 "texture size" sprites). (0, 0) when not loaded.
    pub(crate) fn tex_dims(&self, tex: u32) -> (f32, f32) {
        self.textures.iter().find(|(hh, _)| *hh == tex).map_or((0.0, 0.0), |(_, t)| (t.width as f32, t.height as f32))
    }

    pub(crate) fn ndc(&self, anchor: Anchor, x: f32, y: f32) -> [f32; 2] {
        // One screen pixel in NDC across, keeping the 4:3 pixel shape at the window's height.
        let s = 2.0 / SW * (4.0 / 3.0) / self.aspect.max(1e-3);
        let nx = match anchor {
            Anchor::Left => -1.0 + x * s,
            Anchor::Right => 1.0 - (SW - x) * s,
            Anchor::Centre => (x - SW / 2.0) * s,
            Anchor::Stretch => x / SW * 2.0 - 1.0,
        };
        [nx, 1.0 - y / (SH / 2.0)]
    }

    /// A textured quad at (x, y) w × h (screen pixels) showing texels (u, v, du, dv) of `tex`; `flip` = (U, V).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn quad(&mut self, anchor: Anchor, layer: i32, tex: u32, blend: u8, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], flip: (bool, bool), c: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 || c[3] <= 0.0 { return; }
        let Some(ti) = self.textures.iter().position(|(hh, _)| *hh == tex) else { return };
        let (tw, th) = (self.textures[ti].1.width as f32, self.textures[ti].1.height as f32);
        let (mut u0, mut v0, mut u1, mut v1) = (uv[0] / tw, uv[1] / th, (uv[0] + uv[2]) / tw, (uv[1] + uv[3]) / th);
        if flip.0 { std::mem::swap(&mut u0, &mut u1); }
        if flip.1 { std::mem::swap(&mut v0, &mut v1); }
        let (a, b) = (self.ndc(anchor, x, y), self.ndc(anchor, x + w, y + h));
        let p = |q: [f32; 2], t: [f32; 2]| HudVertex { p: q, uv: t, c };
        let v = [p([a[0], a[1]], [u0, v0]), p([b[0], a[1]], [u1, v0]), p([b[0], b[1]], [u1, v1]), p([a[0], a[1]], [u0, v0]), p([b[0], b[1]], [u1, v1]), p([a[0], b[1]], [u0, v1])];
        self.quads.push(Quad { layer, tex: ti + 1, blend, v });
    }

    /// A `SpriteInfo` sprite at the pane origin with colour `c`.
    fn sprite(&mut self, anchor: Anchor, at: (f32, f32), s: &SpriteInfo, c: [f32; 4]) {
        let uv = s.uv.map(|k| k as f32);
        // w/h 0 = the texture's own size (the icon sprites: crouch 19, air bubble, damage glow)
        // [M SpriteInfo: "w / h (0 = texture size)"] — resolved against the loaded texture.
        let (mut w, mut h) = (s.w as f32, s.h as f32);
        if w == 0.0 || h == 0.0 {
            if let Some(ti) = self.textures.iter().position(|(hh, _)| *hh == s.texture) {
                if w == 0.0 { w = self.textures[ti].1.width as f32; }
                if h == 0.0 { h = self.textures[ti].1.height as f32; }
            }
        }
        self.quad(anchor, s.layer as i32, s.texture, if s.flags & 0x100 != 0 { 1 } else if s.flags & 0x200 != 0 { 2 } else { 0 }, at.0 + s.x as f32, at.1 + s.y as f32, w, h, uv, (s.flags & 0x40 != 0, s.flags & 0x80 != 0), c);
    }

    /// Font 2 text with its top-left pen at (x, y − cap) (`FE 02`: right-aligned → x − width), colour `c`; flag 0x20:
    /// a black copy at (+1, +1) first.
    pub(crate) fn text(&mut self, anchor: Anchor, font: &nf_formats::hud::Font, layer: i32, s: &str, x: f32, y: f32, right: bool, c: [f32; 4], shadow: bool) {
        self.text_scaled(anchor, font, layer, s, x, y, right, c, shadow, 1.0);
    }

    /// Scaled font 2 text (B-9: for the dev menu's large, readable text). Each glyph's w/h and the horizontal
    /// advance are multiplied by `scale` (1.0 = native ~10 px; 2.0 = ~20 px); the row's y is unchanged so callers
    /// space rows to fit the taller glyphs. The shadow offset scales too.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_scaled(&mut self, anchor: Anchor, font: &nf_formats::hud::Font, layer: i32, s: &str, x: f32, y: f32, right: bool, c: [f32; 4], shadow: bool, scale: f32) {
        if scale <= 0.0 || c[3] <= 0.0 { return; }
        let (placed, native_w) = font.layout(s, 0.0, y);
        let width = native_w * scale;
        let x0 = if right { x - width } else { x };
        let sh = 1.5 * scale;
        // The shadow fades with its text (a dim label, the pickup text's fade-out); an opaque shadow under a
        // faint label left a dark smudge [S: not yet checked against a PCSX2 frame].
        let shade = { let mut s = gs(0, 0xff); s[3] = s[3].min(c[3]); s };
        for (dx, col) in shadow.then_some((sh, shade)).into_iter().chain(std::iter::once((0.0, c))) {
            for p in &placed {
                let g = &font.glyphs[p.glyph];
                // p.x is the native absolute x (from a layout at 0); scale the offset from x0.
                let gx = x0 + p.x * scale + dx;
                let gw = g.w as f32 * scale;
                let gh = g.h as f32 * scale;
                self.quad(anchor, layer, FONT_TEX, 0, gx, p.y + dx, gw, gh, [g.u as f32, g.v as f32, g.w as f32, g.h as f32], (false, false), col);
            }
        }
    }



    pub(crate) fn finish(mut self) -> HudFrame {
        self.quads.sort_by(|a, b| b.layer.cmp(&a.layer));
        let mut f = HudFrame::default();
        for q in self.quads {
            let start = f.v.len() as u32;
            f.v.extend_from_slice(&q.v);
            match f.runs.last_mut() {
                Some(r) if r.0 == q.tex && r.1 == q.blend && r.2 + r.3 == start => r.3 += 6,
                _ => f.runs.push((q.tex, q.blend, start, 6)),
            }
        }
        f
    }

    /// Push raw NDC vertices (two triangles) as a coloured quad with the font texture (degenerate UV). For the dev
    /// overlay's world-space lines and circles (WORKLOG 0081).
    pub(crate) fn ndc_quad(&mut self, layer: i32, v: [HudVertex; 6], c: [f32; 4]) {
        let Some(ti) = self.textures.iter().position(|(hh, _)| *hh == 0x0300_0003) else { return };
        let mut qv = v;
        for q in qv.iter_mut() { if q.uv == [0.0, 0.0] { q.uv = [0.0, 0.0]; } q.c = c; }
        self.quads.push(Quad { layer, tex: ti + 1, blend: 0, v: qv });
    }
}

/// What the ammo strip draws for a clip count: the drawn height is `strip_clip(clip, clip_size) · hpr`
/// (hud.rs, `HUD_UpdateAmmoPane` 0x19a7f0's pane layout). A clip can exceed its size only through the dev
/// tools (infinite ammo / no reload), and the strip must clamp to the clip size — the user's stacked/garbled
/// icons on the right of the screen were the strip drawing `clip · hpr` tall, past the pane's empty-column
/// sprite (`clip_size · hpr`). The game itself can't reach this state: pickups and reloads both cap the clip
/// at its size [M pickup.rs `Inventory::give` / the reload paths] — the clamp is the HUD standing safe.
pub fn strip_clip(clip: f32, clip_size: f32) -> f32 { clip.min(clip_size.max(1.0)) }

/// Build 6 NDC vertices (two triangles) for a rectangle from (x0,y0) to (x1,y1).
fn ndc_rect(x0: f32, y0: f32, x1: f32, y1: f32) -> [crate::gpu::HudVertex; 6] {
    use crate::gpu::HudVertex;
    [
        HudVertex { p: [x0, y0], uv: [0.0, 0.0], c: [0.0; 4] },
        HudVertex { p: [x1, y0], uv: [0.0, 0.0], c: [0.0; 4] },
        HudVertex { p: [x1, y1], uv: [0.0, 0.0], c: [0.0; 4] },
        HudVertex { p: [x0, y0], uv: [0.0, 0.0], c: [0.0; 4] },
        HudVertex { p: [x1, y1], uv: [0.0, 0.0], c: [0.0; 4] },
        HudVertex { p: [x0, y1], uv: [0.0, 0.0], c: [0.0; 4] },
    ]
}

/// The frame's overlay.
pub fn overlay(scene: &crate::scene::Scene, aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures: &scene.hud_textures };
    let sim = scene.sim.as_ref();
    // Crosshair: (W/2 · (1 + cx), H/2 · (1 − cy)) from the smoothed offset; the cell from
    // `Check_Target`'s state (BLData+0x133): 2 normal, 3 = the red "no" mark on a non-enemy target
    // [M 0x1a7810:181-182 + HUD_UpdateCrossHair 0x19e6d0:69-79]; in aim mode non-scope weapons force
    // cell 1 [M 0x19e6d0:106-112]. Red, layer 49.
    let (cx, cy, aiming) = sim.map_or((0.0, 0.0, false), |s| { let (x, y) = s.crosshair(); (x, y, s.aiming) });
    let state = sim.map_or(2, |s| s.crosshair_state());
    let scoped = sim.map_or(false, |s| s.weapons.records.get(s.gun.weapon as usize).is_some_and(|w| u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap()) & 0x40 != 0));
    let cell = if aiming && !scoped { 1 } else { state };
    let [u, v, w, hh] = CROSS[cell.min(3) as usize].map(|k| k as f32);
    // A zoomed weapon hides the crosshair: the aim-mode path of `HUD_UpdateCrossHair` 0x19e6d0 ends at
    // LAB_0019e9c8 (layer 0xff = not drawn) whenever the weapon is scoped — the zoom overlay replaces it.
    let zoomed = sim.map_or(false, |s| s.zoom_target > 1.0);
    if !zoomed {
        h.quad(Anchor::Centre, 49, CROSS_TEX, 0, SW / 2.0 * (1.0 + cx) - w / 2.0, SH / 2.0 * (1.0 - cy) - hh / 2.0, w, hh, [u, v, w, hh], (false, false), gs(0xff0000, 0xff));
    }
    let (Some(s), Some(t)) = (sim, scene.hud.as_ref()) else { return h.finish() };
    let frame = s.drones.frame;
    // Health / armour pane (fade factor F = 1: `PlayerSetting+0xB` = 0 [L] holds `BL+0x8BC` at 1).
    let f = 1.0f32;
    let armour = s.drones.health.armour * 0.16;
    if armour > 0.0 {
        for k in 0..8 {
            let a = if k == 0 { 1.0 } else if (k as f32) < armour { 0.75 } else { 0.35 };
            let sp = &t.health[k];
            h.sprite(Anchor::Left, HEALTH_AT, sp, gs(sp.colour >> 8, (a * f * 255.0) as u32));
        }
    }
    let hp = s.drones.health.health.clamp(0.0, 100.0) * 0.07;
    let colour = if hp >= 4.0 { 0x52a88b } else if hp >= 2.0 { 0xc6984e } else { 0x9d1912 };
    for i in 0..7 {
        let mut a = if (i as f32) < hp { 0.55 } else { 0.15 };
        if i == 0 && hp < 0.5 { a = if frame % 30 < 15 { 0.55 } else { 0.15 }; }
        h.sprite(Anchor::Left, HEALTH_AT, &t.health[8 + i], gs(colour, (a * f * 255.0) as u32));
    }
    h.sprite(Anchor::Left, HEALTH_AT, &t.health[15], gs(colour, (f * 140.0) as u32));
    // Damage glow (sprites 20–23, `HealthSprInfo[20..24]`): full screen, additive, red at alpha `BL+0x968`,
    // gated per bit of `BL+0x967` — bit 1 → sprite 20 (tex 0x0300018D), bit 2 → 21 (flipped V), bit 4 → 22
    // (tex 0x0300018E), bit 8 → 23 (flipped U) [M HUD_UpdateHealthPane 0x19b9c8:92-113]. `pain_from`
    // (Player_DealWithObjHit 0x194458) writes the bits per the attacker's view-space direction; the
    // generic pain paths write 0xC (both sides).
    let flash = s.drones.health.flash;
    let dir_bits = s.drones.health.dir_bits;
    if flash > 0.0 {
        let mut lit = [false; 4];
        if dir_bits & 0xC == 0xC { lit[2] = true; lit[3] = true; }
        else {
            if dir_bits & 0x1 != 0 { lit[0] = true; }
            if dir_bits & 0x2 != 0 { lit[1] = true; }
            if dir_bits & 0x4 != 0 { lit[2] = true; }
            if dir_bits & 0x8 != 0 { lit[3] = true; }
        }
        for (k, on) in lit.iter().enumerate() {
            if !*on { continue; }
            let tex = if k < 2 { 0x0300_018d } else { 0x0300_018e };
            h.quad(Anchor::Stretch, 30, tex, 1, 0.0, 0.0, SW, SH, [0.0, 0.0, 64.0, 64.0], (k == 3, k == 1), gs(0xff0000, flash as u32));
        }
    }
    // Ammo pane: "clip-spare" in font 2 right-aligned at (496, 412) with a black shadow, the rounds strip above it.
    let wi = s.gun.weapon as usize;
    let rec = &s.weapons.records[wi];
    let ammo_type = rec[0x90] as usize;
    let clip_size = u16::from_le_bytes([rec[0x92], rec[0x93]]) as f32;
    let flags68 = u32::from_le_bytes(rec[0x68..0x6c].try_into().unwrap());
    // `BL+0x95E` running (a weapon switch): the game clears the ammo text (sprite +0x20 = 0) and takes the rounds
    // strip off-screen (layer +0x38 = 0xFF); only the weapon name shows [M HUD_UpdateAmmoPane 0x19a7f0:176-189,
    // :248-273, :276-300]. The user's PCSX2 play agrees: name alone on a switch, then ammo + fire mode.
    let switching = s.hud.weapon_label_timer > 0;
    if flags68 & 0x20 == 0 && !switching {
        let sp = &t.ammo[0];
        let txt = if matches!(ammo_type, 0x14 | 0x1a | 0x1f) { format!("{}", (s.gun.clip as f32 * 100.0 / clip_size.max(1.0)) as i32) } else { format!("{}-{}", s.gun.clip, s.gun.spare) };
        h.text(Anchor::Right, &t.font, sp.layer as i32, &txt, AMMO_AT.0 + sp.x as f32, AMMO_AT.1 + sp.y as f32, true, gs(sp.colour >> 8, sp.colour & 0xff), sp.flags & 0x20 != 0);
    }
    if let (false, Some(&[bu, bv, bw, hpr])) = (switching, t.bullets.get(ammo_type)) {
        let k = if clip_size > 87.0 { 16.0 } else { 1.0 };
        let (x, bottom) = (AMMO_AT.0 - bw * k, AMMO_AT.1);
        // fix-dev T3: the dev tools can push the clip past its size (infinite ammo / no reload) — the strip
        // must draw at most the clip size, or it overflows the pane (the user's stacked/garbled icons).
        let clip = strip_clip(s.gun.clip as f32, clip_size);
        let (rounds, empty) = (&t.ammo[5], &t.ammo[6]);
        h.quad(Anchor::Right, rounds.layer as i32, 0x0300_0128, 0, x, bottom - clip * hpr, bw * k, clip * hpr, [bu, (clip_size - clip) * hpr + bv, bw, clip * hpr], (false, false), gs(rounds.colour >> 8, rounds.colour & 0xff));
        h.quad(Anchor::Right, empty.layer as i32, 0x0300_0128, 0, x, bottom - clip_size * hpr, bw * k, clip_size * hpr, [bu, bv, bw, clip_size * hpr], (false, false), gs(empty.colour >> 8, empty.colour & 0xff));
    }
    // The fire-mode / weapon-name labels (sprites 1 / 2, both at pane (0,38) → (496, 432), colour 0x7D6D59 with
    // the black shadow). Sprite +0x38 is the draw LAYER, not alpha: 0x1C = layer 28 (drawn), 0xFF = not drawn.
    // While `BL+0x95E` runs the name is on layer 28 and the fire mode is hidden; at rest the reverse
    // [M HUD_UpdateAmmoPane 0x19a7f0:218-226]. The fire-mode label = `Txt_BindLabel(weapon+0x30 + mode*4)` (the
    // port always fires mode 0: no switching path yet), the name = `Txt_BindLabel(weapon+0x38)`. Both right-
    // aligned at x = 496 (PCSX2: "Semi" under "7-6" at the right edge).
    if let Some(lang) = s.lang.as_ref() {
        // The fire-mode label reads the mode byte the fire path writes (`HUD_UpdateAmmoPane` 0x19a7f0
        // reads `BL + w*0xC + 0x1BB` and picks `weapon_data[w]+0x30 + mode*4`) (fix-weapon-modes).
        let mode = s.drones.inventory.fire_mode.get(wi).copied().unwrap_or(0) as usize;
        let (idx, id) = if switching { (2, s.weapons.name_label(wi)) } else { (1, s.weapons.fire_mode_label(wi, mode)) };
        if let (Some(sp), true) = (t.ammo.get(idx), id != u32::MAX && id != 0) {
            if let Some(txt) = lang.bind(id).filter(|t| !t.is_empty()) {
                h.text(Anchor::Right, &t.font, sp.layer as i32, txt, AMMO_AT.0 + sp.x as f32, AMMO_AT.1 + sp.y as f32, true, gs(sp.colour >> 8, 0xff), true);
            }
        }
    }
    // The pickup message bar (pane 21, `Text_AddMsg` 0x1b0c30 + `HUD_UpdateStatusPane` 0x19c310): three sprites
    // copied into `PickupStatusSprInfo` from 0x2FC460 (stride 0x30) [L ee_start_05.bin; M static init 0x1a0058]:
    // the middle is a 1-texel column of the HUD sheet 0x0300007A (u 190, v 0..56) stretched to 256×26 at
    // (128, 364), the left / right caps are 5-texel columns (u 185 / 191) placed against its ends
    // [M 0x19c310:340-346]; all colour 0x7F7F7F, alpha 0x8D, layer 30. The text is centred at (256, 380) in
    // font 2, colour 0x7D6D59, fading over the last 15 ticks of its 180 (alpha = time·17); then the bar closes
    // 16 px per tick around its centre.
    {
        let (bw, bh, bx, by) = (256.0f32, 26.0f32, 128.0f32, 364.0f32);
        // Centre-anchored like its text: on a widescreen window a Left anchor left the bar off to the left of the
        // centred text (user playtest).
        let bar = |h: &mut Hud, w: f32| {
            let x = bx + (bw - w) / 2.0;
            let c = gs(0x7f7f7f, 0x8d);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x, by, w, bh, [190.0, 0.0, 1.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x - 5.0, by, 5.0, bh, [185.0, 0.0, 5.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x + w, by, 5.0, bh, [191.0, 0.0, 5.0, 56.0], (false, false), c);
        };
        if s.drones.hud_frozen {
            // `HUD_UpdateStatusPane` case 0 polls `Text_UpdateMsg`, which returns 0 while the viewer's
            // +0x2a HUD gate is off [M 0x1b11d8:16-18] — the pane never opens during a cut-scene, so
            // nothing draws (fix-party-mission item 5: banners wait for the NIS to end).
        } else if let Some(msg) = s.hud.msgs.front() {
            let a = (msg.time * 17.0).min(255.0) as u32;
            bar(&mut h, bw);
            // Centred on (256, 380): the pen starts half the text's width left of the centre (it started AT the
            // centre, so the text ran off to the right of the bar).
            let (_, tw) = t.font.layout(&msg.text, 0.0, 0.0);
            h.text(Anchor::Centre, &t.font, 29, &msg.text, SW / 2.0 - tw / 2.0, 380.0, false, gs(0x7d6d59, a), true);
        } else if s.hud.bar_close > 0.0 {
            bar(&mut h, s.hud.bar_close);
        }
    }
    // The info / hint bar (pane 4, `MsgInfoStatusPane` 0x319810 + `InfoStatusSprInfo` 0x319730
    // [L ee_start_05.bin] — fix-context-hints): the TOP bar the Hint markers (placement 0x33) and
    // `ThirdIcon_Update`'s type-6 "Press Action" fill through TXTMSG type 1. The same
    // `HUD_UpdateStatusPane` 0x19c310 machine as the pickup bar: the middle is a 1-texel column
    // of 0x0300007A (u 190, v 0..56) stretched to 480×26 at (16, 0) — the screen's top edge —
    // the caps 5-texel columns (u 185 / 191) riding its ends [M 0x19c310:342-347]; all colour
    // 0x7F7F7F alpha 0x8D, layer 30. The text is the centre-justify style 0x30cdb0 sprite
    // (template (128, 16) + the pane x 128 = 256 [L], so centred) in font 2, colour 0x7D6D59,
    // black shadow, layer 29, wrapping at TextMsgFormats[1]'s maxWidth 480 [L 0x2BF110+4],
    // fading over the last 15 ticks (alpha = time·17); the bar closes 16 px per tick around its
    // centre (the CREATE's slide 0x10 [L 0x319810+0x14]).
    {
        let (bw, bh, bx, by) = (480.0f32, 26.0f32, 16.0f32, 0.0f32);
        let bar = |h: &mut Hud, w: f32| {
            let x = bx + (bw - w) / 2.0;
            let c = gs(0x7f7f7f, 0x8d);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x, by, w, bh, [190.0, 0.0, 1.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x - 5.0, by, 5.0, bh, [185.0, 0.0, 5.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x + w, by, 5.0, bh, [191.0, 0.0, 5.0, 56.0], (false, false), c);
        };
        if s.drones.hud_frozen {
            // The pane cannot open while the viewer's +0x2a HUD gate is off [M 0x1b11d8:16-18].
        } else if let Some(msg) = s.hud.info_msgs.front() {
            let a = (msg.time * 17.0).min(255.0) as u32;
            bar(&mut h, bw);
            let wrapped = t.font.word_wrap(&msg.text, 480.0);
            let lines: Vec<&str> = wrapped.split('\n').collect();
            let mut maxw = 0.0f32;
            for l in &lines { maxw = maxw.max(t.font.layout(l, 0.0, 0.0).1); }
            let x0 = SW / 2.0 - maxw / 2.0;
            let lh = t.font.line_height();
            for (k, l) in lines.iter().enumerate() {
                h.text(Anchor::Centre, &t.font, 29, l, x0, 16.0 + k as f32 * lh, false, gs(0x7d6d59, a), true);
            }
        } else if s.hud.info_close > 0.0 {
            bar(&mut h, s.hud.info_close);
        }
    }
    // The objective banner (pane 3, `MsgObjectiveStatusPane` 0x319710 + `ObjectiveStatusSprInfo`
    // 0x319608 [L ee_start_05.bin], `HUD_UpdateStatusPane` 0x19c310): the middle is a 1-texel column
    // of the HUD sheet 0x0300007A (u 190, v 0..56) stretched to 480×56 — template (16, 0) + the pane
    // x 128, re-pinned to the template x on every open (`HUD_UpdateStatusPane` case 1 resets the
    // middle's x/w from the SprInfo entry [M 0x19c310:101-111]) — so the bar spans 16..496, the caps
    // 5-texel columns (u 185 / 191) riding its ends; all colour 0x7F7F7F, alpha 0x8D, layer 30. Both
    // text rows are text sprites whose screen x = template x + pane x = 128 + 128 = **256** with the
    // fmt 0x30cdb0 justify byte 3 = CENTRE (`Font_ParseFormat` 0x1ee3e0 case 0xfe/3 → flag 2;
    // `Font_DrawText` 0x1eee48 flag 2 → x −= width/2; the status label sits at template (128, 20),
    // the body at (128, 40) [L 0x319608 entries 5/0 + CREATE 0x319710 x=128]) — i.e. centred on the
    // bar's own centre. The port used to draw both rows left-aligned from x=128, which left the
    // "Fail: …" notice hugging the bar's left edge (the user's off-centre fail message). Colour
    // 0x7D6D59, layer 29; the status label rebinds per display: 0x2000051 "NEW OBJECTIVE" while the
    // message is not "OBJECTIVE COMPLETE", else 0x2000053 [M 0x19c310:127-144]. Alpha = time·17 over
    // the last 15 ticks; the banner closes 16 px per tick around its centre (the pane's slide 0x10
    // [M 0x1a1070]).
    {
        let (bw, bh, bx, by) = (480.0f32, 56.0f32, 16.0f32, 0.0f32);
        let bar = |h: &mut Hud, w: f32| {
            let x = bx + (bw - w) / 2.0;
            let c = gs(0x7f7f7f, 0x8d);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x, by, w, bh, [190.0, 0.0, 1.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x - 5.0, by, 5.0, bh, [185.0, 0.0, 5.0, 56.0], (false, false), c);
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, x + w, by, 5.0, bh, [191.0, 0.0, 5.0, 56.0], (false, false), c);
        };
        if s.drones.hud_frozen {
            // Same closed pane [M 0x19c310 case 0]: an objective banner queued mid-NIS waits.
        } else if let Some(msg) = s.hud.objective_msgs.front() {
            let a = (msg.time * 17.0).min(255.0) as u32;
            bar(&mut h, bw);
            // The status label rebinds per display [M 0x19c310 case 2: the message text strcmp'd
            // against Txt_BindLabel(0x2000053) — the port's `status` field is 0x0200_0053 exactly
            // then]: "OBJECTIVE COMPLETE" for a completion, "NEW OBJECTIVE" otherwise. Its steady
            // colour is the gold 0x785a14ff (case 3's else branch stores it unconditionally), the
            // body's 0x7d6d59ff.
            let complete = msg.status == 0x0200_0053;
            if let Some(lang) = s.lang.as_ref() {
                if let Some(st) = lang.bind(msg.status) {
                    let (_, sw) = t.font.layout(st, 0.0, 0.0);
                    h.text(Anchor::Centre, &t.font, 29, st, SW / 2.0 - sw / 2.0, 20.0, false, gs(0x785a14, a), true);
                }
            }
            // The body row: the wrapped message text. For a COMPLETE message the game shows NO body
            // row — case 2 forces the body sprite's colour to 0 after binding the status label and
            // case 3 keeps it 0 in both its phases [M 0x19c310 case 2 + case 3's `*(iVar8+0x10)=0`
            // stores] — the status label alone carries "OBJECTIVE COMPLETE" (the port used to draw
            // the body row too: the user's two stacked OBJECTIVE COMPLETE banners).
            if !complete {
                // `Text_AddMsg` 0x1b0c30 wraps the message text with `Font_WordWrapString` at
                // TextMsgFormats[2]'s maxWidth = 480 [L 0x2bf108+0x14 = 0x1e0] — the box's own width
                // (ObjectiveStatusSprInfo[2] x 16 w 480 [L 0x319608+0x58]); the port wraps at draw
                // time (the font lives in the scene tables, the queue stores the raw bind).
                // __Font_DrawText 0x1ee598:65-71: each '\n' line draws from the same block start x
                // and the next line sits `glyph0.h + gap + 1.0` lower; Font_DrawText 0x1eee48's
                // justify-2 centre shifts the block start x -= width/2 by the WIDEST line, so the
                // block centres on 256 with its lines left-aligned from there.
                let wrapped = t.font.word_wrap(&msg.text, 480.0);
                let lines: Vec<&str> = wrapped.split('\n').collect();
                let mut maxw = 0.0f32;
                for l in &lines { maxw = maxw.max(t.font.layout(l, 0.0, 0.0).1); }
                let x0 = SW / 2.0 - maxw / 2.0;
                let lh = t.font.line_height();
                for (k, l) in lines.iter().enumerate() {
                    h.text(Anchor::Centre, &t.font, 29, l, x0, 40.0 + k as f32 * lh, false, gs(0x7d6d59, a), true);
                }
            }
        } else if s.hud.objective_close > 0.0 {
            bar(&mut h, s.hud.objective_close);
        }
    }
    // The mission banner (pane 2, `MsgMissionStatusPane` 0x2bd538 + `MissionStatusSprInfo` 0x319580
    // [L], `HUD_CreateMissionStatusPane` 0x1a1028 sets its TXTMSG type 3 and the 0x20 slide): the
    // middle is uv(198,0) 1×103 stretched to 512×103 at (0, 156), no caps [L SprInfo — only 3
    // sprites]; the main text ("Mission Complete" / "Mission Fail", type 3 from `Mission_Update`)
    // centred at (256, 42) (fmt 0x30cdb8, align 1); the fail reason (`FailLabel`, rebound per
    // display [M 0x19c310:171-173]) at (256, 64) in fmt 0x30cce0 (align 2, as the objective's).
    // It flashes 0x785A14FF while a type-2 message is up [M 0x19c310:209]. Closes 32 px per tick.
    {
        let (bw, bh, bx, by) = (512.0f32, 103.0f32, 0.0f32, 156.0f32);
        let bar = |h: &mut Hud, hgt: f32| {
            let y = by + (bh - hgt) / 2.0;
            h.quad(Anchor::Centre, 30, 0x0300_007a, 0, bx, y, bw, hgt, [198.0, 0.0, 1.0, 103.0], (false, false), gs(0x7f7f7f, 0x8d));
        };
        if s.drones.hud_frozen {
            // Same closed pane [M 0x19c310 case 0]: the mission banner waits too.
        } else if let Some(msg) = s.hud.mission_msgs.front() {
            let a = (msg.time * 17.0).min(255.0) as u32;
            bar(&mut h, bh);
            // Both rows centre on the box: the styles 0x30cdb8 (main) and 0x30cce0 (fail label) are
            // `fe 03` = justify flag 2 = CENTRE [L 0x30cdb8/0x30cce0; M Font_ParseFormat 0x1ee3e0
            // case 4 byte 3 → flag 2, Font_DrawText 0x1eee48 `flags & 2 → x -= width·0.5`] — the
            // port passed its right-justify flag instead, which shifted both rows left by half
            // their width (the user's "Mission Fail / Cover Lost" text sitting left of the box's
            // centre).
            let (_, tw) = t.font.layout(&msg.text, 0.0, 0.0);
            h.text(Anchor::Centre, &t.font, 29, &msg.text, SW / 2.0 - tw / 2.0, 42.0 + by, false, gs(0x7d6d59, a), true);
            if let (Some(lang), true) = (s.lang.as_ref(), s.hud.fail_label != 0) {
                if let Some(fl) = lang.bind(s.hud.fail_label) {
                    // The flash while an objective notice is up [M 0x19c310:209 0x785a14ff].
                    let flash = !s.hud.objective_msgs.is_empty() && frame % 30 < 15;
                    let (c, al) = if flash { (0x785a14, a) } else { (0x7d6d59, a) };
                    let (_, fw) = t.font.layout(fl, 0.0, 0.0);
                    h.text(Anchor::Centre, &t.font, 29, fl, SW / 2.0 - fw / 2.0, 64.0 + by, false, gs(c, al), true);
                }
            }
        } else if s.hud.mission_close > 0.0 {
            bar(&mut h, s.hud.mission_close);
        }
    }
    // The air/oxygen pane (pane 5, `HUD_UpdateAirPane` 0x1a11c8, `AirSprInfo` 0x2bd490): shown while
    // Bond swims (the game enables it in player modes 3/6 — `Player_MonitorAir` 0x191a70:50); the fill
    // bar's width is `air·2 + 28` of the 256-wide track, its colour 0x68D6B1 fully submerged (mode 6) /
    // 0x68B1D6 swimming (mode 3); the bubble icon is additive, bright under water (0xFF) and 0x1F at the
    // surface, with the bar at 0x1E/0x1F [M 0x1a11c8].
    {
        let under = s.bond.underwater;
        if under || s.bond.air < 100.0 {
            let air = s.bond.air.clamp(0.0, 100.0);
            let (a_fill, a_bar, a_icon) = if under { (0xFF, 0xFF, 0xFF) } else { (0x1E, 0x1F, 0x1F) };
            let fill_col = if under { 0x68d6b1 } else { 0x68b1d6 };
            if t.air.len() >= 2 {
                let (bar, fill) = (&t.air[0], &t.air[1]);
                let w = (air * 2.0 + 28.0).clamp(1.0, 256.0);
                h.sprite(Anchor::Left, (0.0, 0.0), bar, gs(bar.colour >> 8, a_bar));
                h.quad(Anchor::Left, fill.layer as i32, fill.texture, 0, fill.x as f32, fill.y as f32, w, fill.h as f32, [fill.uv[0] as f32, fill.uv[1] as f32, w, fill.uv[3] as f32], (false, false), gs(fill_col, a_fill));
            }
            if let Some(icon) = t.air.get(2) {
                h.sprite(Anchor::Left, (0.0, 0.0), icon, gs(icon.colour >> 8, a_icon));
            }
        }
    }
    // The crouch icon (health pane sprite 19): while crouched (player mode 4) tex 0x03000173; after standing
    // 0x03000174 for 60 ticks, alpha 0xFF then fading over the last 30 (alpha = timer·255/30). The game writes
    // the sprite's screen position (+0x24 / +0x26) as 0x30, 0x30 on every crouch, overriding the template's
    // (320, −300), and +0x38 = 0x1D is the LAYER (29), not alpha [M HUD_UpdateHealthPane 0x19b9c8:305-328].
    // Drawn at the texture's size centred on (48, 48) [S: the centring; the user's PCSX2 play puts it top left].
    {
        let (crouched, timer) = (s.bond.crouched, s.hud.crouch_icon_timer);
        if crouched || timer > 0 {
            let (tex, a) = if crouched || timer >= 30 { (if crouched { 0x0300_0173u32 } else { 0x0300_0174u32 }, 0xFFu32) }
                else { (0x0300_0174u32, (timer as u32 * 255 / 30).max(1)) };
            let (tw, th) = h.tex_dims(tex);
            h.quad(Anchor::Left, 29, tex, 0, 48.0 - tw / 2.0, 48.0 - th / 2.0, tw, th, [0.0, 0.0, tw, th], (false, false), gs(0x7f7f7f, a));
        }
    }
    // The context icon (health pane sprite 18, `HUD_UpdateHealthPane` 0x19b9c8:273-303 —
    // fix-context-hints). While `BLData+0x95F != 0xFF` the sprite's texture is the 0x2FC168
    // table's entry for the context id (0x03000064+ctx — `hashtable_set_sprite` fills w/h and
    // uv with the texture's own size [M 0x1ca3d8:36-46]), the runtime position is (400, 48)
    // [M disasm 0x19c124-0x19c14c — the same runtime x/y override the crouch icon uses], layer
    // 29, colour 0x7f7f7fff, and `ThirdIconTimer` = FRAME_RATE·0.5 = 30; on 0xFF the icon fades
    // out over that timer (alpha = timer·255/30, colour 0x7f7f7f00|alpha [M 0x19c160-0x19c1d4]).
    // Only ctx 2/3/6 resolve to a disc texture (0x66 winch / 0x67 hanging man / 0x6A figure
    // [D tex-list]); the others leave the sprite hidden exactly like the game's failed lookup.
    {
        let timer = s.hud.context_icon_timer;
        // The shown texture: the current context id, or — through the fade — the last one the
        // icon showed (the game's runtime sprite keeps its texture from the last successful
        // `hashtable_set_sprite`).
        let ctx = if s.bond.icon_ctx != 0xff { s.bond.icon_ctx } else { s.hud.last_icon_ctx };
        let (tex, a) = if s.bond.icon_ctx != 0xff {
            (0x0300_0064u32 + ctx as u32, 0xFFu32)
        } else if timer > 0 {
            // alpha = timer·255/30 [M 0x19c190-0x19c1cc: (timer·0x100 − timer)/(FRAME_RATE·0.5)].
            (0x0300_0064u32 + ctx as u32, (timer as u32 * 255 / 30) as u32)
        } else { (0u32, 0) };
        if tex != 0 && a > 0 {
            let (tw, th) = h.tex_dims(tex);
            if tw > 0.0 {
                h.quad(Anchor::Left, 29, tex, 0, 400.0, 48.0, tw, th, [0.0, 0.0, tw, th], (false, false), gs(0x7f7f7f, a));
            }
        }
    }
    // Scope overlay (the SightPane, `HUD_UpdateCrossHair` 0x19e6d0 pane[6]): 3 sprites at layer 39 [D].
    // Sprite 0: the scope texture (0x03000034, 256×256) centered at x=32, y=0, w=448, h=448, UV 0..511.
    //   Flag 0x0600 = subtractive blend (0x200) + tile+mirror (0x400). The texture is one quadrant of the
    //   scope; the game tiles it 2×2 with per-tile UV flip to make a full circle [M View_DrawSprites 0x1e5290].
    //   Each tile is 224×224 (448/2), with UV 0..255 (one full texture), flipped per quadrant.
    // Sprite 1: left strip (0x03000002, 32×32) at x=0, w=33, h=448. Flag 0x0200 = subtractive.
    // Sprite 2: right strip (0x03000002) at x=479, w=33, h=448. Flag 0x0200 = subtractive.
    // All colour 0x7f7f7fff (white on the GS 0x80=1.0 scale), subtractive blend (Cd - Cs·As/128).
    // Subtractive: white texture → framebuffer darkened to black (scope frame); black texture → unchanged (clear centre).
    // Drive visibility off zoom_target (the button state), not the interpolated zoom value — the game's
    // HUD_UpdateCrossHair (0x19e6d0) toggles the SightPane off the aim flag `*(player+0x60) & 1`, which
    // drops the instant the button is released. Using the interpolated zoom would delay the overlay's
    // disappearance by ~40 ticks (the zoom-out interpolation time).
    if s.zoom_target > 1.0 {
        match zoom_overlay(s.gun.weapon) {
            // SightPane 6 [L SightSprInfo 0x2bd558]: the scope — 4 mirrored tiles of 0x03000034 + the two
            // subtractive side strips [M View_DrawSprites 0x1e5290 tiles the quadrant texture 2×2].
            ZoomOverlay::Sight => {
                let scope_col = gs(0x7f7f7f, 0xff);
                h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2,  32.0,   0.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (false, false), scope_col);
                h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2,  32.0, 224.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (false, true),  scope_col);
                h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2, 256.0,   0.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (true,  false), scope_col);
                h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2, 256.0, 224.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (true,  true),  scope_col);
                h.quad(Anchor::Stretch, 39, SCOPE_STRIP_TEX, 2, 0.0, 0.0, 33.0, 448.0, [0.0, 0.0, 31.0, 31.0], (false, false), scope_col);
                h.quad(Anchor::Stretch, 39, SCOPE_STRIP_TEX, 2, 479.0, 0.0, 33.0, 448.0, [0.0, 0.0, 31.0, 31.0], (false, false), scope_col);
            }
            // CameraPane 11 [L CameraSprInfo 0x2bd600] — the watch's (0x54/0x55) and the micro camera's
            // (0x52/0x53) viewfinder: the frame texture over the whole screen + four corner brackets
            // (sprite 3 h-flipped by flag 0x40). Sprite 0 is the photo flash, hidden at rest and shown
            // by `HUD_UpdateCameraPane` 0x1a12d8 only during its post-shot countdown; sprite 8 (flags
            // 0x900) is 3D-positioned — neither is drawn here.
            ZoomOverlay::Camera => {
                let c = gs(0x7f7f7f, 0xff);
                let (tw56, th56) = h.tex_dims(0x0300_0056);
                let (tw57, th57) = h.tex_dims(0x0300_0057);
                h.quad(Anchor::Stretch, 39, 0x0300_0055, 2, 0.0, 0.0, SW, SH, [0.0, 0.0, 255.0, 127.0], (false, false), c);
                h.quad(Anchor::Stretch, 39, 0x0300_0056, 1, 48.0, 192.0, tw56, th56, [0.0, 0.0, tw56, th56], (false, false), c);
                h.quad(Anchor::Stretch, 39, 0x0300_0056, 1, 336.0, 192.0, tw56, th56, [0.0, 0.0, tw56, th56], (true, false), c);
                for (x, y) in [(48.0, 120.0), (432.0, 120.0), (48.0, 256.0), (432.0, 256.0)] {
                    h.quad(Anchor::Stretch, 39, 0x0300_0057, 1, x, y, tw57, th57, [0.0, 0.0, tw57, th57], (false, false), c);
                }
            }
            // LaserPane 19 [L LaserSprInfo 0x2be288] — the Ronin's (0x32/0x33) frame: six solid-texture
            // strips (top/bottom bars + four centre ticks), blend 1, colour 0x7f7f7f alpha 0x78.
            ZoomOverlay::Laser => {
                let c = gs(0x7f7f7f, 0x78);
                for (x, y, w, hh2, layer) in [(140.0, 64.0, 232.0, 4.0, 40), (140.0, 384.0, 232.0, 4.0, 34), (192.0, 223.0, 40.0, 2.0, 40), (280.0, 223.0, 40.0, 2.0, 40), (255.0, 160.0, 2.0, 40.0, 40), (255.0, 248.0, 2.0, 40.0, 40)] {
                    h.quad(Anchor::Stretch, layer, SCOPE_STRIP_TEX, 1, x, y, w, hh2, [0.0, 0.0, 31.0, 31.0], (false, false), c);
                }
            }
            // OICWPane 17 [L OICWSprInfo 0x2be008] — the OICW twins' (0x1a/0x1b) digital zoom: a
            // subtractive cyan full-screen tint (sprite 0, col 0x00ffffff) + the dim scope circle
            // 0x03000141 (sprite 1, alpha 0x40). Sprites 2–6 are text sprites (the zoom readout) — the
            // port has no reader for their text yet (the live check).
            ZoomOverlay::Oicw => {
                h.quad(Anchor::Stretch, 40, SCOPE_STRIP_TEX, 2, 0.0, 0.0, SW, SH, [0.0, 0.0, 31.0, 31.0], (false, false), gs(0x00ffff, 0xff));
                h.quad(Anchor::Stretch, 39, 0x0300_0141, 1, 192.0, 160.0, 128.0, 128.0, [1.0, 1.0, 127.0, 127.0], (false, false), gs(0x7f7f7f, 0x40));
            }
        }
    }
    h.finish()
}
