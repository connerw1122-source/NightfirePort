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
pub const TEXTURES: [u32; 8] = [0x0300_0026, 0x0300_007a, 0x0300_0128, 0x0300_018d, 0x0300_018e, 0x0300_0003, 0x0300_0034, 0x0300_0002];
const CROSS_TEX: u32 = 0x0300_0026;
const FONT_TEX: u32 = 0x0300_0003;
/// SightPane textures [D 07000005.bin entries 71 + 56]: 0x03000034 = the scope overlay (256×256, the circular
/// vignette + crosshair reticle), 0x03000002 = the solid fill for the left/right letterbox strips (32×32 RGBA).
const SCOPE_TEX: u32 = 0x0300_0034;
const SCOPE_STRIP_TEX: u32 = 0x0300_0002;

/// `HUDCrossCoords` (0x2bec48, u, v, w, h) [L]: 1 = aim mode, 2 = normal, 3 = friendly.
const CROSS: [[i16; 4]; 4] = [[0, 0, 0, 0], [1, 2, 29, 29], [10, 11, 11, 11], [31, 1, 14, 13]];
/// Pane origins after `HUD_ValidateXY`'s 16-pixel margins [L]: health (16, 337), ammo (496, 394).
const HEALTH_AT: (f32, f32) = (16.0, 337.0);
const AMMO_AT: (f32, f32) = (496.0, 394.0);

pub(crate) struct Quad { pub(crate) layer: i32, pub(crate) tex: usize, pub(crate) blend: u8, pub(crate) v: [HudVertex; 6] }

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
        self.quad(anchor, s.layer as i32, s.texture, if s.flags & 0x100 != 0 { 1 } else if s.flags & 0x200 != 0 { 2 } else { 0 }, at.0 + s.x as f32, at.1 + s.y as f32, s.w as f32, s.h as f32, uv, (s.flags & 0x40 != 0, s.flags & 0x80 != 0), c);
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
        for (dx, col) in shadow.then_some((sh, gs(0, 0xff))).into_iter().chain(std::iter::once((0.0, c))) {
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
    // Crosshair: (W/2 · (1 + cx), H/2 · (1 − cy)) from the smoothed offset; cell 1 in aim mode, else 2; red, layer 49.
    let (cx, cy, aiming) = sim.map_or((0.0, 0.0, false), |s| { let (x, y) = s.crosshair(); (x, y, s.aiming) });
    let [u, v, w, hh] = CROSS[if aiming { 1 } else { 2 }].map(|k| k as f32);
    h.quad(Anchor::Centre, 49, CROSS_TEX, 0, SW / 2.0 * (1.0 + cx) - w / 2.0, SH / 2.0 * (1.0 - cy) - hh / 2.0, w, hh, [u, v, w, hh], (false, false), gs(0xff0000, 0xff));
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
    // Damage glow (sprites 22 / 23 for `BL+0x967` = 0xC, the both-sides value `Player_HandlePain` writes; the port does
    // not track the hit direction yet): full screen, additive, red at alpha `BL+0x968`.
    let flash = s.drones.health.flash;
    if flash > 0.0 {
        for k in [22usize, 23] {
            let tex = if k < 22 { 0x0300_018d } else { 0x0300_018e };
            h.quad(Anchor::Stretch, 30, tex, 1, 0.0, 0.0, SW, SH, [0.0, 0.0, 64.0, 64.0], (k == 23, k == 21), gs(0xff0000, flash as u32));
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
        h.text(Anchor::Right, &t.font, sp.layer as i32, &txt, AMMO_AT.0 + sp.x as f32, AMMO_AT.1 + sp.y as f32, true, gs(sp.colour >> 8, sp.colour & 0xff), sp.flags & 0x20 != 0);
    }
    if let Some(&[bu, bv, bw, hpr]) = t.bullets.get(ammo_type) {
        let k = if clip_size > 87.0 { 16.0 } else { 1.0 };
        let (x, bottom) = (AMMO_AT.0 - bw * k, AMMO_AT.1);
        let clip = s.gun.clip as f32;
        let (rounds, empty) = (&t.ammo[5], &t.ammo[6]);
        h.quad(Anchor::Right, rounds.layer as i32, 0x0300_0128, 0, x, bottom - clip * hpr, bw * k, clip * hpr, [bu, (clip_size - clip) * hpr + bv, bw, clip * hpr], (false, false), gs(rounds.colour >> 8, rounds.colour & 0xff));
        h.quad(Anchor::Right, empty.layer as i32, 0x0300_0128, 0, x, bottom - clip_size * hpr, bw * k, clip_size * hpr, [bu, bv, bw, clip_size * hpr], (false, false), gs(empty.colour >> 8, empty.colour & 0xff));
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
    if s.zoom > 1.01 {
        let scope_col = gs(0x7f7f7f, 0xff);
        // 4 tiles of the scope texture, mirrored per quadrant to form one full circle [M 0x1e5290].
        h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2,  32.0,   0.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (false, false), scope_col);
        h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2,  32.0, 224.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (false, true),  scope_col);
        h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2, 256.0,   0.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (true,  false), scope_col);
        h.quad(Anchor::Stretch, 39, SCOPE_TEX, 2, 256.0, 224.0, 224.0, 224.0, [0.0, 0.0, 255.0, 255.0], (true,  true),  scope_col);
        // Left strip (solid fill, subtractive)
        h.quad(Anchor::Stretch, 39, SCOPE_STRIP_TEX, 2, 0.0, 0.0, 33.0, 448.0, [0.0, 0.0, 31.0, 31.0], (false, false), scope_col);
        // Right strip (solid fill, subtractive)
        h.quad(Anchor::Stretch, 39, SCOPE_STRIP_TEX, 2, 479.0, 0.0, 33.0, 448.0, [0.0, 0.0, 31.0, 31.0], (false, false), scope_col);
    }
    h.finish()
}
