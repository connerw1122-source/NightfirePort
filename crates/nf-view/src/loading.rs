//! The level loading screen (`ResetMap_GenLoadScreen` 0x1d5de0).
//!
//! Before every level but the front end (0x07000048), `ResetMap_Load` 0x1d6360 state 0 loads 07000500.bin
//! ("LevelLoadingScreens") and state 3 builds the screen [M]: one full-screen sprite (`LoadingGfx.266` at 0x2c93f8 —
//! 512×448, colour 0x7f7f7fff, the whole 512×512 texture squeezed onto the screen) whose texture hash comes from
//! `ResetMap_LevelCode2Img` 0x1d5258; the "LOADING" label 0x3c1 centred at x 248; the level's objective and one hint
//! line ("\xBA" label "\xBA " text — 0xBA switches the colour to the label gold 0x785a14ff and back to cream
//! 0x7d6d59ff [M `__Font_DrawText` 0x1eee48]). The screen draws once into both buffers, then the vblank interrupt
//! animates the six LOADING dots until `psiEndLoadingScreen` [M 0x124088, 0x11f2e8, `CreateLoadingBlob` 0x11bf70].
//! The images all live in 07000500.bin entry 15 [M texture_entries.tsv].

/// The archive that holds the pictures and the dot texture.
pub const ARCHIVE: &str = "07000500.bin";
/// The dot texture: a 16×16 grey disc [M `CreateLoadingBlob` 0x11bf70].
pub const DOT_TEX: u32 = 0x0300_0175;

/// The level's picture hash [M `ResetMap_LevelCode2Img` 0x1d5258's switch; the level code is `GameState+8 &
/// 0xff00ffff`, 0x07000018 remapped to 0x0700001b by the caller].
pub fn image_for(level: u32) -> u32 {
    match level & 0xff00_ffff {
        0x0700_0001..=0x0700_0004 => 0x0300_00d2,
        0x0700_0005..=0x0700_0008 => 0x0300_00d0,
        0x0700_0009..=0x0700_000b => 0x0300_00d5,
        0x0700_000c | 0x0700_000d => 0x0300_00d3,
        0x0700_0011..=0x0700_0013 | 0x0700_004a => 0x0300_00d6,
        0x0700_0014..=0x0700_0016 => 0x0300_00d1,
        0x0700_0018 | 0x0700_001b => 0x0300_00d4,
        0x0700_0021..=0x0700_0029 | 0x0700_004b | 0x0700_004c => 0x0300_016e,
        _ => 0x0300_00d7,
    }
}

/// The level's objective label [M 0x1d5258].
pub fn objective_for(level: u32) -> Option<u32> {
    let o = match level & 0xff00_ffff {
        0x0700_0001 => 0x06, 0x0700_0002 => 0x08, 0x0700_0003 => 0x0a, 0x0700_0004 => 0x0d,
        0x0700_0005 => 0x00, 0x0700_0006 => 0x01, 0x0700_0007 => 0x02, 0x0700_0008 => 0x03,
        0x0700_0009 => 0x16, 0x0700_000a => 0x19, 0x0700_000b => 0x1d, 0x0700_000c => 0x2c, 0x0700_000d => 0x2f,
        0x0700_0011 => 0x36, 0x0700_0012 => 0x37, 0x0700_0013 => 0x3a, 0x0700_0014 => 0x49, 0x0700_0015 => 0x4d,
        0x0700_0016 => 0x4f, 0x0700_004a => 0x39,
        // 0x1b's objective slot is overwritten by its hint pick [M 0x1d5258 — a game bug]; the general pool then
        // replaces the empty hint, so the screen shows no objective.
        _ => return None,
    };
    Some(0x0400_0000 + o)
}

/// The general hint pool: 0x1000111..=0x100013c without 0x1000129 [M 0x3054d8].
fn general_pool() -> Vec<u32> { (0x111u32..=0x13c).filter(|&i| i != 0x129).map(|i| 0x0100_0000 + i).collect() }

/// The hint line's label, picked with the caller's random numbers [M 0x1d5258: the level's own table, then — every
/// level but 05, 4a, MP and the default — a coin flip (or no hint) replaces it with a general-pool pick].
pub fn hint_for(level: u32, mut rand: impl FnMut() -> u32) -> Option<u32> {
    let l = level & 0xff00_ffff;
    let table: &[u32] = match l {
        0x0700_0001 => &[0xd3, 0xd4],
        0x0700_0002 => &[0xc7, 0xc8, 0xcb, 0xd1],
        0x0700_0003 => &[0xca, 0xcd, 0xce, 0xc9, 0xd2],
        0x0700_0004 => &[0xcf, 0xd0],
        0x0700_0005 => return Some(0x0100_0126),
        0x0700_0006 => &[0xc0, 0xc1],
        0x0700_0007 => &[0xc2],
        0x0700_0008 => &[0xc3, 0xc4, 0xc5, 0xc6],
        0x0700_0009 => &[0xd5, 0xd8, 0xdf, 0xe2],
        0x0700_000a => &[0xd7, 0xd9, 0xdb, 0xe0, 0xe3, 0xe4, 0xe6],
        0x0700_000b => &[0xdc, 0xda, 0xdd, 0xde, 0xe1, 0xe5],
        0x0700_000c => &[0xe7, 0xe9, 0xeb, 0xed, 0xef, 0xf1, 0xf4],
        0x0700_000d => &[0xe8, 0xea, 0xec, 0xee, 0xf0, 0xf2, 0xf3, 0xf5],
        0x0700_0011 => &[0xf6, 0xf8, 0xf8],
        0x0700_0012 => &[0xfe, 0xf9, 0xfa, 0xfc, 0xfd],
        0x0700_0013 | 0x0700_0018 | 0x0700_001b => &[],
        0x0700_0014 => &[0x103, 0x104, 0x105, 0x106],
        0x0700_0015 => &[0xff],
        0x0700_0016 => &[0x100, 0x101, 0x102],
        0x0700_004a => return Some(0x0100_00fb),
        0x0700_0021..=0x0700_0029 | 0x0700_004b | 0x0700_004c => return Some(0x0100_013e + rand() % 40),
        _ => return None,
    };
    let own = (!table.is_empty()).then(|| 0x0100_0000 + table[(rand() as usize) % table.len()]);
    match own {
        Some(h) if rand() & 1 != 0 => Some(h),
        _ => { let p = general_pool(); Some(p[(rand() as usize) % p.len()]) }
    }
}

/// The pictures and the dot texture of 07000500.bin's map entries, by hash: (width, height, RGBA8).
pub fn art(archive: &[u8]) -> std::collections::HashMap<u32, (u32, u32, Vec<u8>)> {
    let mut out = std::collections::HashMap::new();
    let Ok(lb) = nf_formats::levelbin::LevelBin::parse(archive) else { return out };
    for e in lb.entries.iter().filter(|e| matches!(e.kind, 0 | 1 | 2 | 11)) {
        let map = lb.data(archive, e);
        let Ok(mf) = nf_formats::map::MapFile::parse(map) else { continue };
        let Ok((texs, _)) = nf_formats::texture::textures(map, &mf) else { continue };
        for t in &texs {
            let pal = t.palette_rgba8();
            // The sprite draws at colour 0x7f7f7f — texture × 1 on the GS scale; alpha opaque (the picture fills
            // the screen).
            let rgba: Vec<u8> = t.frame(0).iter().flat_map(|&i| { let c = pal[i as usize]; [c[0], c[1], c[2], 255] }).collect();
            out.insert(t.hashcode, (t.width as u32, t.height as u32, rgba));
        }
    }
    out
}

/// The loading screen's hint, picked once per load: `ResetMap_GenLoadScreen` 0x1d5de0 builds the screen a
/// single time per load [M], so the line must not change while the dots animate. `get` re-picks only for a
/// new level or after `reset` (the next load).
#[derive(Default, Clone, Debug)]
pub struct HintPick { picked: Option<(u32, Option<u32>)> }

impl HintPick {
    pub fn get(&mut self, level: u32, rand: impl FnMut() -> u32) -> Option<u32> {
        match self.picked {
            Some((l, h)) if l == level => h,
            _ => { let h = hint_for(level, rand); self.picked = Some((level, h)); h }
        }
    }
    pub fn reset(&mut self) { self.picked = None; }
}

/// The hint strings start with "HINT "; the game skips through the first space [M 0x1d5de0].
pub fn strip_hint(s: &str) -> &str { s.split_once(' ').map_or(s, |(_, rest)| rest) }

/// The lit-dot count for `ticks` 60 Hz vblanks into the load: BlobToDraw steps every 11 vblanks and ping-pongs
/// 0→6→0 [M `VBlankInt` 0x124088:18-29 — per vblank the u8 `BlobTime` (0x30d721) decrements and on
/// underflow reloads 10 and steps the u8 `BlobToDraw` (0x30d720) by the s8 `BlobDir` (0x30d722); the
/// direction flips at `BlobToDraw == 0` (→ 1) and `== 6` (→ −1); `ResetLoadingBlobs` 0x1249c0:6-8
/// seeds BlobToDraw 0 — so 10 countdown ticks + the underflow tick = one step per 11 vblanks, a
/// 12-step 0→6→0 ping-pong].
pub fn dots_lit(ticks: u32) -> u32 {
    let step = ticks / 11;
    let p = step % 12;
    if p <= 6 { p } else { 12 - p }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tables() {
        assert_eq!(image_for(0x0700_0005), 0x0300_00d0);
        assert_eq!(image_for(0x0705_0005), 0x0300_00d0, "the code is masked with 0xff00ffff");
        assert_ne!(image_for(0x0700_0001), image_for(0x0700_0005));
        assert_eq!(objective_for(0x0700_0005), Some(0x0400_0000));
        assert_eq!(hint_for(0x0700_0005, || 0), Some(0x0100_0126));
        assert!(!general_pool().contains(&0x0100_0129));
        assert_eq!(general_pool().len(), 43);
        assert_eq!((0..14).map(|s| dots_lit(s * 11)).collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1, 0, 1]);
        assert_eq!(strip_hint("HINT If you"), "If you");
    }

    /// Playtest 1016: "loading hints run 10000 mph" — the port re-picked every redraw. One pick per load:
    /// the same level keeps its hint across redraws whatever the random source says; a reset re-picks.
    #[test]
    fn the_loading_hint_is_picked_once_per_load() {
        let mut pick = HintPick::default();
        let mut n = 0u32;
        let first = pick.get(0x0700_000c, || { n += 7; n });
        for _ in 0..50 { assert_eq!(pick.get(0x0700_000c, || { n += 7; n }), first, "the hint changed mid-load"); }
        // Sister: the random source really varies the pick (the pin can fail) — over many fresh loads more
        // than one hint shows up.
        let mut seen = std::collections::HashSet::new();
        for k in 0..40u32 { pick.reset(); let mut m = k * 2654435761; seen.insert(pick.get(0x0700_000c, || { m = m.wrapping_mul(1103515245).wrapping_add(12345); m >> 8 })); }
        assert!(seen.len() > 1, "the pick never varied across loads");
    }
}
