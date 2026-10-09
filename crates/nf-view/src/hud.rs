//! The overlay as the game draws it (docs/systems/hud.md): the crosshair (`HUD_UpdateCrossHair` 0x19e6d0), the
//! health / armour pane (`HUD_UpdateHealthPane` 0x19b9c8) with the damage-direction glow, and the ammo pane
//! (`HUD_UpdateAmmoPane` 0x19a7f0). Sprites from `HealthSprInfo` / `AmmoSprInfo` / `BulletImg` and font 2, read from
//! ACTION.ELF; textures from the level archive. Sorted by layer, higher first (`CompSpr`: lower numbers on top).

use crate::gpu::{HudFrame, HudVertex};
use nf_formats::hud::SpriteInfo;

/// The game's screen (`Viewer+0x10C` / `+0x110` [L]): 512 × 448, shown at 4:3.
const SW: f32 = 512.0;
const SH: f32 = 448.0;

/// HUD textures the overlay uses (indices + 1 in `HudFrame` runs).
pub const TEXTURES: [u32; 6] = [0x0300_0026, 0x0300_007a, 0x0300_0128, 0x0300_018d, 0x0300_018e, 0x0300_0003];
const CROSS_TEX: u32 = 0x0300_0026;
const FONT_TEX: u32 = 0x0300_0003;

/// `HUDCrossCoords` (0x2bec48, u, v, w, h) [L]: 1 = aim mode, 2 = normal, 3 = friendly.
const CROSS: [[i16; 4]; 4] = [[0, 0, 0, 0], [1, 2, 29, 29], [10, 11, 11, 11], [31, 1, 14, 13]];
/// Pane origins after `HUD_ValidateXY`'s 16-pixel margins [L]: health (16, 337), ammo (496, 394).
const HEALTH_AT: (f32, f32) = (16.0, 337.0);
const AMMO_AT: (f32, f32) = (496.0, 394.0);

struct Quad { layer: i32, tex: usize, add: bool, v: [HudVertex; 6] }

struct Hud<'a> { quads: Vec<Quad>, aspect: f32, textures: &'a [(u32, crate::scene::TextureImage)] }

/// A colour 0xRRGGBB on the GS scale (0x80 = 1.0) with an alpha byte (drawn as A >> 1, 0x80 = opaque).
fn gs(rgb: u32, a: u32) -> [f32; 4] {
    [((rgb >> 16) & 0xff) as f32 / 128.0, ((rgb >> 8) & 0xff) as f32 / 128.0, (rgb & 0xff) as f32 / 128.0, ((a.min(255) >> 1) as f32 / 128.0).min(1.0)]
}

impl Hud<'_> {
    fn ndc(&self, x: f32, y: f32) -> [f32; 2] { [(x - SW / 2.0) / (SW / 2.0) * (4.0 / 3.0) / self.aspect.max(1e-3), 1.0 - y / (SH / 2.0)] }

    /// A textured quad at (x, y) w × h (screen pixels) showing texels (u, v, du, dv) of `tex`; `flip` = (U, V).
    #[allow(clippy::too_many_arguments)]
    fn quad(&mut self, layer: i32, tex: u32, add: bool, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], flip: (bool, bool), c: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 || c[3] <= 0.0 { return; }
        let Some(ti) = self.textures.iter().position(|(hh, _)| *hh == tex) else { return };
        let (tw, th) = (self.textures[ti].1.width as f32, self.textures[ti].1.height as f32);
        let (mut u0, mut v0, mut u1, mut v1) = (uv[0] / tw, uv[1] / th, (uv[0] + uv[2]) / tw, (uv[1] + uv[3]) / th);
        if flip.0 { std::mem::swap(&mut u0, &mut u1); }
        if flip.1 { std::mem::swap(&mut v0, &mut v1); }
        let (a, b) = (self.ndc(x, y), self.ndc(x + w, y + h));
        let p = |q: [f32; 2], t: [f32; 2]| HudVertex { p: q, uv: t, c };
        let v = [p([a[0], a[1]], [u0, v0]), p([b[0], a[1]], [u1, v0]), p([b[0], b[1]], [u1, v1]), p([a[0], a[1]], [u0, v0]), p([b[0], b[1]], [u1, v1]), p([a[0], b[1]], [u0, v1])];
        self.quads.push(Quad { layer, tex: ti + 1, add, v });
    }

    /// A `SpriteInfo` sprite at the pane origin with colour `c`.
    fn sprite(&mut self, at: (f32, f32), s: &SpriteInfo, c: [f32; 4]) {
        let uv = s.uv.map(|k| k as f32);
        self.quad(s.layer as i32, s.texture, s.flags & 0x100 != 0, at.0 + s.x as f32, at.1 + s.y as f32, s.w as f32, s.h as f32, uv, (s.flags & 0x40 != 0, s.flags & 0x80 != 0), c);
    }

    /// Font 2 text with its top-left pen at (x, y − cap) (`FE 02`: right-aligned → x − width), colour `c`; flag 0x20:
    /// a black copy at (+1, +1) first.
    fn text(&mut self, font: &nf_formats::hud::Font, layer: i32, s: &str, x: f32, y: f32, right: bool, c: [f32; 4], shadow: bool) {
        let (_, width) = font.layout(s, 0.0, y);
        let x0 = if right { x - width } else { x };
        let (placed, _) = font.layout(s, x0, y);
        for (dx, col) in shadow.then_some((1.0, gs(0, 0xff))).into_iter().chain(std::iter::once((0.0, c))) {
            for p in &placed {
                let g = &font.glyphs[p.glyph];
                self.quad(layer, FONT_TEX, false, p.x + dx, p.y + dx, g.w as f32, g.h as f32, [g.u as f32, g.v as f32, g.w as f32, g.h as f32], (false, false), col);
            }
        }
    }

    fn finish(mut self) -> HudFrame {
        self.quads.sort_by(|a, b| b.layer.cmp(&a.layer));
        let mut f = HudFrame::default();
        for q in self.quads {
            let start = f.v.len() as u32;
            f.v.extend_from_slice(&q.v);
            match f.runs.last_mut() {
                Some(r) if r.0 == q.tex && r.1 == q.add && r.2 + r.3 == start => r.3 += 6,
                _ => f.runs.push((q.tex, q.add, start, 6)),
            }
        }
        f
    }
}

/// The frame's overlay.
pub fn overlay(scene: &crate::scene::Scene, aspect: f32) -> HudFrame {
    let mut h = Hud { quads: Vec::new(), aspect, textures: &scene.hud_textures };
    let sim = scene.sim.as_ref();
    // Crosshair: (W/2 · (1 + cx), H/2 · (1 − cy)) from the smoothed offset; cell 1 in aim mode, else 2; red, layer 49.
    let (cx, cy, aiming) = sim.map_or((0.0, 0.0, false), |s| { let (x, y) = s.crosshair(); (x, y, s.aiming) });
    let [u, v, w, hh] = CROSS[if aiming { 1 } else { 2 }].map(|k| k as f32);
    h.quad(49, CROSS_TEX, false, SW / 2.0 * (1.0 + cx) - w / 2.0, SH / 2.0 * (1.0 - cy) - hh / 2.0, w, hh, [u, v, w, hh], (false, false), gs(0xff0000, 0xff));
    let (Some(s), Some(t)) = (sim, scene.hud.as_ref()) else { return h.finish() };
    let frame = s.drones.frame;
    // Health / armour pane (fade factor F = 1: `PlayerSetting+0xB` = 0 [L] holds `BL+0x8BC` at 1).
    let f = 1.0f32;
    let armour = s.drones.health.armour * 0.16;
    if armour > 0.0 {
        for k in 0..8 {
            let a = if k == 0 { 1.0 } else if (k as f32) < armour { 0.75 } else { 0.35 };
            let sp = &t.health[k];
            h.sprite(HEALTH_AT, sp, gs(sp.colour >> 8, (a * f * 255.0) as u32));
        }
    }
    let hp = s.drones.health.health.clamp(0.0, 100.0) * 0.07;
    let colour = if hp >= 4.0 { 0x52a88b } else if hp >= 2.0 { 0xc6984e } else { 0x9d1912 };
    for i in 0..7 {
        let mut a = if (i as f32) < hp { 0.55 } else { 0.15 };
        if i == 0 && hp < 0.5 { a = if frame % 30 < 15 { 0.55 } else { 0.15 }; }
        h.sprite(HEALTH_AT, &t.health[8 + i], gs(colour, (a * f * 255.0) as u32));
    }
    h.sprite(HEALTH_AT, &t.health[15], gs(colour, (f * 140.0) as u32));
    // Damage glow (sprites 22 / 23 for `BL+0x967` = 0xC, the both-sides value `Player_HandlePain` writes; the port does
    // not track the hit direction yet): full screen, additive, red at alpha `BL+0x968`.
    let flash = s.drones.health.flash;
    if flash > 0.0 {
        for k in [22usize, 23] {
            let tex = if k < 22 { 0x0300_018d } else { 0x0300_018e };
            h.quad(30, tex, true, 0.0, 0.0, SW, SH, [0.0, 0.0, 64.0, 64.0], (k == 23, k == 21), gs(0xff0000, flash as u32));
        }
    }
    // Ammo pane: "clip-spare" in font 2 right-aligned at (496, 412) with a black shadow, the rounds strip above it.
    let wi = s.gun.weapon as usize;
    let rec = &s.weapons.records[wi];
    let ammo_type = rec[0x90] as usize;
    let clip_size = u16::from_le_bytes([rec[0x92], rec[0x93]]) as f32;
    let flags68 = u32::from_le_bytes(rec[0x68..0x6c].try_into().unwrap());
    if flags68 & 0x20 == 0 {
        let sp = &t.ammo[0];
        let txt = if matches!(ammo_type, 0x14 | 0x1a | 0x1f) { format!("{}", (s.gun.clip as f32 * 100.0 / clip_size.max(1.0)) as i32) } else { format!("{}-{}", s.gun.clip, s.gun.spare) };
        h.text(&t.font, sp.layer as i32, &txt, AMMO_AT.0 + sp.x as f32, AMMO_AT.1 + sp.y as f32, true, gs(sp.colour >> 8, sp.colour & 0xff), sp.flags & 0x20 != 0);
    }
    if let Some(&[bu, bv, bw, hpr]) = t.bullets.get(ammo_type) {
        let k = if clip_size > 87.0 { 16.0 } else { 1.0 };
        let (x, bottom) = (AMMO_AT.0 - bw * k, AMMO_AT.1);
        let clip = s.gun.clip as f32;
        let (rounds, empty) = (&t.ammo[5], &t.ammo[6]);
        h.quad(rounds.layer as i32, 0x0300_0128, false, x, bottom - clip * hpr, bw * k, clip * hpr, [bu, (clip_size - clip) * hpr + bv, bw, clip * hpr], (false, false), gs(rounds.colour >> 8, rounds.colour & 0xff));
        h.quad(empty.layer as i32, 0x0300_0128, false, x, bottom - clip_size * hpr, bw * k, clip_size * hpr, [bu, bv, bw, clip_size * hpr], (false, false), gs(empty.colour >> 8, empty.colour & 0xff));
    }
    h.finish()
}
