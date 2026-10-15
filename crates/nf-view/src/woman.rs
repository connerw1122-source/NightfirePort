//! The burning woman in the pause and fail menus (`psiDecompressWoman` 0x1202a8).
//!
//! The pause page (0x4000004b) and the fail menu (0x40000042) draw control 0x10000184 with texture 0x030000db. On
//! the disc that texture holds a placeholder (palette blobs and a basketball player). The game overwrites its texels
//! every other frame while paused (`Game_Run` 0x1c9790:36-41 calls the decoder in the paused branch) with one frame
//! of a run-length mask, the level archive's kind-16 entry "woman" (`LoaderProcess` 0x1d4880 case 0x10 →
//! `PS2SetUpWoman` 0x120c20), filled from the fire texture 0x030000da (128×256, 8bpp) at a scrolling offset.
//! `Conv8to32` 0x11eeb0 only re-lays the bytes for the GS upload, so the result stays an 8bpp image drawn through
//! 0x030000db's own CLUT.
//!
//! The decoder, verbatim [M 0x1202a8]: per frame 0x4000 output bytes; the source pointer starts at
//! `fire + (frame & 0x3f)·0x100 + (0x80 − (frame & 0x7f))` and advances with the output. The stream alternates two
//! record kinds, starting with a SKIP: a skip is a big-endian u16 count of zero bytes; a run list is a big-endian
//! u16 `n` followed by `n` byte counts that alternate copy (from the source) / zero, starting with copy. A frame
//! ends when 0x4000 bytes are written (the kind alternation carries over frames). After frame 0x15c (348) the
//! stream and the frame counter restart. The decoder runs every second call (`skipit = 2`, minus
//! `PS2FramesToSkip` = 1 per call).

/// The mask's frame count before the stream restarts [M 0x1202a8: `WomanFrame == 0x15c`].
pub const FRAMES: u32 = 0x15c;
/// Output size: 128×128 8bpp.
pub const W: usize = 128;
pub const H: usize = 128;

pub struct Woman {
    data: Vec<u8>,
    pos: usize,
    /// The frame counter (`WomanFrame`); every decoded frame starts with a skip record (`iVar11 = 1` [M :26]).
    frame: u32,
    skip: i32,
    fire: Vec<u8>,
    palette: Vec<[u8; 4]>,
    /// The current frame's indices.
    pub indices: Vec<u8>,
}

impl Woman {
    /// `data`: the "woman" entry; `fire`: 0x030000da's indices (128×256); `palette`: 0x030000db's CLUT (RGBA8).
    pub fn new(data: Vec<u8>, fire: Vec<u8>, palette: Vec<[u8; 4]>) -> Woman {
        Woman { data, pos: 0, frame: 0, skip: 0, fire, palette, indices: vec![0; W * H] }
    }

    /// One `psiDecompressWoman` call. Returns true when a new frame was decoded.
    pub fn tick(&mut self) -> bool {
        self.skip -= 1; // PS2FramesToSkip = 1 at 60 Hz
        if self.skip >= 1 { return false; }
        self.skip = 2;
        let src0 = (self.frame & 0x3f) as usize * 0x100 + (0x80 - (self.frame & 0x7f) as usize);
        let mut out = 0usize;
        let mut src = src0;
        let mut kind_skip = true;
        let rd = |d: &[u8], p: &mut usize| -> u8 { let v = d.get(*p).copied().unwrap_or(0); *p += 1; v };
        let mut guard = 0;
        while out < W * H && guard < 100_000 {
            guard += 1;
            if kind_skip {
                let n = ((rd(&self.data, &mut self.pos) as usize) << 8) | rd(&self.data, &mut self.pos) as usize;
                let n = n.min(W * H - out);
                self.indices[out..out + n].fill(0);
                out += n; src += n;
            } else {
                let runs = ((rd(&self.data, &mut self.pos) as usize) << 8) | rd(&self.data, &mut self.pos) as usize;
                let mut copy = true;
                for _ in 0..runs {
                    let n = (rd(&self.data, &mut self.pos) as usize).min(W * H - out);
                    for k in 0..n {
                        self.indices[out + k] = if copy { self.fire.get(src + k).copied().unwrap_or(0) } else { 0 };
                    }
                    out += n; src += n;
                    copy = !copy;
                }
            }
            kind_skip = !kind_skip;
        }
        self.frame += 1;
        if self.frame == FRAMES { self.frame = 0; self.pos = 0; }
        true
    }

    /// The current frame through 0x030000db's CLUT.
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(W * H * 4);
        for &i in &self.indices { out.extend_from_slice(&self.palette.get(i as usize).copied().unwrap_or([0, 0, 0, 0])); }
        out
    }
}
