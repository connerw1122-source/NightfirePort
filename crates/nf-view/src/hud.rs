//! The overlay: the game's crosshair (`HUD_UpdateCrossHair` 0x19e6d0) and a stand-in HUD (health, rounds) as
//! screen-space quads. Spec: docs/systems/player.md "Crosshair and HUD".

use crate::gpu::HudVertex;

/// The game's screen (`Viewer+0x10C` / `+0x110` [L]): 512 × 448, shown at 4:3.
const SW: f32 = 512.0;
const SH: f32 = 448.0;

/// `HUDCrossCoords` (0x2bec48, 9 × 9 s16: u, v, w, h, …) [L]: the crosshair cells of the 64 × 64 sheet 0x03000026.
/// 1 = aim mode (29 × 29), 2 = normal (11 × 11), 3 = friendly (14 × 13), 4, 5 unused here.
const CROSS: [[i16; 4]; 6] = [[0, 0, 0, 0], [1, 2, 29, 29], [10, 11, 11, 11], [31, 1, 14, 13], [31, 15, 12, 16], [45, 14, 18, 18]];

/// Overlay geometry for one frame.
pub struct Hud { pub v: Vec<HudVertex>, aspect: f32 }

impl Hud {
    pub fn new(aspect: f32) -> Hud { Hud { v: Vec::new(), aspect } }

    /// Screen pixel (512 × 448, origin top-left) → NDC, the 4:3 picture centred in the window.
    fn ndc(&self, x: f32, y: f32) -> [f32; 2] { [(x - SW / 2.0) / (SW / 2.0) * (4.0 / 3.0) / self.aspect.max(1e-3), 1.0 - y / (SH / 2.0)] }

    fn quad(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], c: [f32; 4]) {
        let (a, b) = (self.ndc(x, y), self.ndc(x + w, y + h));
        let q = [([a[0], a[1]], [uv[0], uv[1]]), ([b[0], a[1]], [uv[2], uv[1]]), ([b[0], b[1]], [uv[2], uv[3]]),
            ([a[0], a[1]], [uv[0], uv[1]]), ([b[0], b[1]], [uv[2], uv[3]]), ([a[0], b[1]], [uv[0], uv[3]])];
        self.v.extend(q.iter().map(|&(p, uv)| HudVertex { p, uv, c }));
    }

    /// A plain rectangle in screen pixels.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) { self.quad(x, y, w, h, [-1.0; 4], c); }

    /// A sprite centred on (x, y) (`View_DrawSprites` flag 0x800), texels 1:1 from the HUD sheet; `c` on the GS scale.
    pub fn sprite(&mut self, x: f32, y: f32, cell: [i16; 4], c: [f32; 4]) {
        let [u, v, w, h] = cell.map(|k| k as f32);
        if w <= 0.0 || h <= 0.0 { return; }
        self.quad(x - w / 2.0, y - h / 2.0, w, h, [u / 64.0, v / 64.0, (u + w) / 64.0, (v + h) / 64.0], c);
    }
}

/// The frame's overlay.
pub fn overlay(sim: Option<&crate::scene::Sim>, aspect: f32) -> Vec<HudVertex> {
    let mut h = Hud::new(aspect);
    // Crosshair: at (W/2 · (1 + cx), H/2 · (1 − cy)) from the smoothed offset `BL+0x120 / +0x124`; cell 1 in aim mode,
    // else `BL+0x133` (2 normal, 3 on a friendly drone); colour 0xFF0000 with alpha 0xFF drawn as 0x7F (red).
    let (cx, cy, aiming) = sim.map_or((0.0, 0.0, false), |s| { let (x, y) = s.crosshair(); (x, y, s.aiming) });
    let cell = CROSS[if aiming { 1 } else { 2 }];
    h.sprite(SW / 2.0 * (1.0 + cx), SH / 2.0 * (1.0 - cy), cell, [2.0, 0.0, 0.0, 127.0 / 128.0]);
    if let Some(s) = sim {
        // Stand-in HUD (the game's health / ammo panes are not read): health bottom left, one pip per round in the
        // clip and the spare rounds as a thin bar bottom right; the hit flash tints the screen.
        let hp = (s.drones.health.health / 100.0).clamp(0.0, 1.0);
        h.rect(24.0, 410.0, 96.0, 8.0, [0.0, 0.0, 0.0, 0.5]);
        h.rect(25.0, 411.0, 94.0 * hp, 6.0, [0.85, 0.15, 0.1, 0.9]);
        if s.drones.health.flash > 0.0 { h.rect(0.0, 0.0, SW, SH, [0.8, 0.0, 0.0, (s.drones.health.flash / 255.0).min(0.5)]); }
        for k in 0..s.gun.clip.min(40) { h.rect(480.0 - 6.0 * k as f32, 408.0, 3.0, 10.0, [1.0, 0.85, 0.3, 0.9]); }
        h.rect(388.0, 421.0, 96.0 * (s.gun.spare as f32 / 70.0).min(1.0), 3.0, [1.0, 0.85, 0.3, 0.6]);
        if s.gun.reloading > 0 { h.rect(232.0, 260.0, 48.0, 3.0, [1.0, 1.0, 1.0, 0.7]); }
    }
    h.v
}
