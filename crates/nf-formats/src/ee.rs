//! Minimal R5900 (EE) interpreter for running the game's own static initialisers over the user's ELF, so tables the
//! game builds at boot (`weapon_data`, filled by `_GLOBAL_$I$weapon_data` → `gcc2_compiled.` 0x1b3488) can be read
//! without committing their bytes. Only the instructions those initialisers use are implemented; anything else is an
//! error, not a guess. `memset` (the only call they make) is run natively. Spec: docs/formats/weapon-data.md.

use crate::buf::{invalid, Result};
use crate::elf::Elf;

const STACK_TOP: u32 = 0x01f0_0000;
const STACK_SIZE: u32 = 0x1_0000;
/// Return address the run starts with; `jr` to it ends the run.
const SENTINEL: u32 = 0xffff_fff0;

pub struct Machine {
    /// Loaded image: `(vaddr, bytes)` (memsz, bss zeroed), plus a small stack.
    regions: Vec<(u32, Vec<u8>)>,
    r: [u128; 32],
    f: [u32; 32],
    hi: u64,
    lo: u64,
    memset: Option<u32>,
    pub steps: u64,
}

fn sx32(v: u32) -> u128 { v as i32 as i64 as u64 as u128 }

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(code: &[u32]) -> Machine {
        let mut img = vec![0u8; 0x100];
        for (i, w) in code.iter().enumerate() { img[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes()); }
        Machine { regions: vec![(0x1000, img), (STACK_TOP - STACK_SIZE, vec![0; STACK_SIZE as usize])], r: [0; 32], f: [0; 32], hi: 0, lo: 0, memset: None, steps: 0 }
    }

    #[test]
    fn stores_constants_branches_with_delay_slots_and_chops_floats() {
        let code = [
            0x3c08_1234, // lui  t0, 0x1234
            0x3508_5678, // ori  t0, t0, 0x5678
            0x3c09_0000, // lui  t1, 0
            0xad28_1080, // sw   t0, 0x1080(t1)
            0x1000_0002, // beq  zero, zero, +2   (taken)
            0x240a_0007, // addiu t2, zero, 7      (delay slot: runs)
            0x240a_0063, // addiu t2, zero, 99     (skipped)
            0xa12a_1084, // sb   t2, 0x1084(t1)
            0x3c0b_3f80, // lui  t3, 0x3f80        (1.0f)
            0x3c0c_4040, // lui  t4, 0x4040        (3.0f)
            0x448b_0000, // mtc1 t3, f0
            0x448c_0800, // mtc1 t4, f1
            0x4601_0083, // div.s f2, f0, f1       (1/3, chopped)
            0xe522_1088, // swc1 f2, 0x1088(t1)
            0x03e0_0008, // jr   ra
            0x0000_0000, // nop
        ];
        let mut m = machine(&code);
        m.call(0x1000, 0, 0, 100).unwrap();
        assert_eq!(m.read(0x1080, 4).unwrap(), 0x1234_5678u32.to_le_bytes());
        assert_eq!(m.read(0x1084, 1).unwrap(), [7]);
        let third = f32::from_le_bytes(m.read(0x1088, 4).unwrap().try_into().unwrap());
        assert_eq!(f32::from_bits(third.to_bits() + 1), 1.0f32 / 3.0, "chopped one ulp below nearest");
        // An unimplemented instruction is an error, not a guess.
        let mut bad = machine(&[0x7000_0000, 0x03e0_0008, 0]);
        assert!(bad.call(0x1000, 0, 0, 10).is_err());
    }
}

/// f64 → f32 rounding toward zero.
fn round_to_zero(v: f64) -> f32 {
    let n = v as f32; // nearest
    if (n as f64).abs() > v.abs() { f32::from_bits(n.to_bits() - 1) } else { n }
}

impl Machine {
    /// The ELF's PT_LOAD segments (and `memset`'s address from the symbol table, if present).
    pub fn from_elf(data: &[u8]) -> Result<Machine> {
        let elf = Elf::parse(data)?;
        let mut regions = Vec::new();
        for (va, bytes, memsz) in elf.load_segments(data)? {
            let mut v = bytes.to_vec();
            v.resize(memsz.max(bytes.len() as u32) as usize, 0);
            regions.push((va, v));
        }
        regions.push((STACK_TOP - STACK_SIZE, vec![0; STACK_SIZE as usize]));
        let memset = elf.symbols(data)?.iter().find(|s| s.name == "memset").map(|s| s.value);
        Ok(Machine { regions, r: [0; 32], f: [0; 32], hi: 0, lo: 0, memset, steps: 0 })
    }

    fn slot(&self, a: u32, n: usize) -> Result<(usize, usize)> {
        for (i, (va, b)) in self.regions.iter().enumerate() {
            if a >= *va && (a - va) as usize + n <= b.len() { return Ok((i, (a - va) as usize)); }
        }
        invalid(format!("EE access {a:#x}+{n} outside the image"))
    }
    pub fn read(&self, a: u32, n: usize) -> Result<&[u8]> {
        let (i, o) = self.slot(a, n)?;
        Ok(&self.regions[i].1[o..o + n])
    }
    fn write(&mut self, a: u32, bytes: &[u8]) -> Result<()> {
        let (i, o) = self.slot(a, bytes.len())?;
        self.regions[i].1[o..o + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
    fn ld(&self, a: u32, n: usize) -> Result<u128> {
        let mut b = [0u8; 16];
        b[..n].copy_from_slice(self.read(a, n)?);
        Ok(u128::from_le_bytes(b))
    }
    fn st(&mut self, a: u32, v: u128, n: usize) -> Result<()> { self.write(a, &v.to_le_bytes()[..n]) }
    fn set(&mut self, rd: usize, v: u128) { if rd != 0 { self.r[rd] = v; } }

    /// Runs the function at `pc` with arguments `a0`, `a1` until it returns; `max_steps` bounds the run.
    pub fn call(&mut self, pc: u32, a0: u32, a1: u32, max_steps: u64) -> Result<()> {
        self.r[4] = a0 as u128;
        self.r[5] = a1 as u128;
        self.r[29] = (STACK_TOP - 0x100) as u128;
        self.r[31] = SENTINEL as u128;
        let mut pc = pc;
        loop {
            if pc == SENTINEL { return Ok(()); }
            if Some(pc) == self.memset {
                // memset(a0, a1, a2) → a0; return through ra.
                let (dst, val, n) = (self.r[4] as u32, self.r[5] as u8, self.r[6] as u32);
                self.write(dst, &vec![val; n as usize])?;
                self.r[2] = self.r[4];
                pc = self.r[31] as u32;
                continue;
            }
            self.steps += 1;
            if self.steps > max_steps { return invalid(format!("EE run exceeded {max_steps} steps at {pc:#x}")); }
            let w = u32::from_le_bytes(self.read(pc, 4)?.try_into().unwrap());
            let branch = self.step(pc, w)?;
            pc = pc.wrapping_add(4);
            if let Some(t) = branch {
                // Execute the delay slot, then jump.
                let ds = u32::from_le_bytes(self.read(pc, 4)?.try_into().unwrap());
                self.steps += 1;
                if self.step(pc, ds)?.is_some() { return invalid(format!("branch in a delay slot at {pc:#x}")); }
                pc = t;
            }
        }
    }

    /// Executes one instruction; returns the branch target when it branches.
    fn step(&mut self, pc: u32, w: u32) -> Result<Option<u32>> {
        let (op, rs, rt, rd, sa) = (w >> 26, (w >> 21 & 31) as usize, (w >> 16 & 31) as usize, (w >> 11 & 31) as usize, w >> 6 & 31);
        let imm = w as u16;
        let simm = imm as i16 as i32 as u32;
        let base = |m: &Machine| (m.r[rs] as u32).wrapping_add(simm);
        let unknown = || invalid(format!("EE instruction {w:08x} at {pc:#x} not implemented"));
        match op {
            0 => match w & 0x3f {
                0x00 => self.set(rd, sx32((self.r[rt] as u32) << sa)),
                0x02 => self.set(rd, sx32((self.r[rt] as u32) >> sa)),
                0x03 => self.set(rd, sx32(((self.r[rt] as u32 as i32) >> sa) as u32)),
                0x08 => return Ok(Some(self.r[rs] as u32)),
                0x18 => {
                    let p = (self.r[rs] as u32 as i32 as i64) * (self.r[rt] as u32 as i32 as i64);
                    self.lo = p as i32 as i64 as u64;
                    self.hi = (p >> 32) as i32 as i64 as u64;
                    self.set(rd, self.lo as u128);
                }
                0x21 => self.set(rd, sx32((self.r[rs] as u32).wrapping_add(self.r[rt] as u32))),
                0x23 => self.set(rd, sx32((self.r[rs] as u32).wrapping_sub(self.r[rt] as u32))),
                0x24 => self.set(rd, self.r[rs] & self.r[rt]),
                0x25 => self.set(rd, self.r[rs] | self.r[rt]),
                0x2b => self.set(rd, ((self.r[rs] as u64) < (self.r[rt] as u64)) as u128),
                0x2d => self.set(rd, (self.r[rs] as u64).wrapping_add(self.r[rt] as u64) as u128),
                _ => return unknown(),
            },
            0x02 | 0x03 => {
                if op == 3 { self.r[31] = pc.wrapping_add(8) as u128; }
                return Ok(Some((pc.wrapping_add(4) & 0xf000_0000) | (w & 0x03ff_ffff) << 2));
            }
            0x04 | 0x05 => {
                let eq = self.r[rs] as u64 == self.r[rt] as u64;
                // Not taken: fall through; the delay slot runs as the next instruction (beq/bne are not "likely").
                if eq == (op == 4) { return Ok(Some(pc.wrapping_add(4).wrapping_add(simm << 2))); }
            }
            0x09 => self.set(rt, sx32((self.r[rs] as u32).wrapping_add(simm))),
            0x0a => self.set(rt, ((self.r[rs] as u64 as i64) < simm as i32 as i64) as u128),
            0x0b => self.set(rt, ((self.r[rs] as u64) < simm as i32 as i64 as u64) as u128),
            0x0c => self.set(rt, self.r[rs] & imm as u128),
            0x0d => self.set(rt, self.r[rs] | imm as u128),
            0x0f => self.set(rt, sx32((imm as u32) << 16)),
            0x11 => match rs {
                0x00 => self.set(rt, sx32(self.f[rd])),
                0x04 => self.f[rd] = self.r[rt] as u32,
                // COP1 single: fd = sa field, fs = rd field, ft = rt. The EE FPU rounds toward zero (*sourced*:
                // PCSX2 FPU notes, GPL — facts only); f32 products and quotients are computed in f64 and truncated.
                0x10 => {
                    let (fs, ft) = (f32::from_bits(self.f[rd]) as f64, f32::from_bits(self.f[rt]) as f64);
                    let fd = sa as usize;
                    let v = match w & 0x3f {
                        0x00 => fs + ft,
                        0x01 => fs - ft,
                        0x02 => fs * ft,
                        0x03 => fs / ft,
                        0x05 => fs.abs(),
                        0x06 => fs,
                        0x07 => -fs,
                        0x24 => { self.f[fd] = fs.trunc() as i32 as u32; return Ok(None); }
                        _ => return unknown(),
                    };
                    self.f[fd] = round_to_zero(v).to_bits();
                }
                0x14 if w & 0x3f == 0x20 => self.f[sa as usize] = round_to_zero(self.f[rd] as i32 as f64).to_bits(),
                _ => return unknown(),
            },
            0x19 => self.set(rt, (self.r[rs] as u64).wrapping_add(simm as i32 as i64 as u64) as u128),
            0x1a | 0x1b => {
                let a = base(self);
                let b = (a & 7) as u32;
                let mem = self.ld(a & !7, 8)? as u64;
                let old = self.r[rt] as u64;
                let low = |n: u32| if n >= 8 { u64::MAX } else { (1u64 << (8 * n)) - 1 };
                let v = if op == 0x1a { (old & low(7 - b)) | (mem << (8 * (7 - b))) } else { (old & !low(8 - b)) | (mem >> (8 * b)) };
                self.set(rt, (self.r[rt] & !(u64::MAX as u128)) | v as u128);
            }
            0x1e => { let v = self.ld(base(self) & !15, 16)?; self.set(rt, v) }
            0x1f => { let a = base(self) & !15; self.st(a, self.r[rt], 16)? }
            0x20 => { let v = self.ld(base(self), 1)? as u8 as i8 as i32 as u32; self.set(rt, sx32(v)) }
            0x21 => { let v = self.ld(base(self), 2)? as u16 as i16 as i32 as u32; self.set(rt, sx32(v)) }
            0x23 => { let v = self.ld(base(self), 4)? as u32; self.set(rt, sx32(v)) }
            0x24 => { let v = self.ld(base(self), 1)?; self.set(rt, v) }
            0x25 => { let v = self.ld(base(self), 2)?; self.set(rt, v) }
            0x28 => { let a = base(self); self.st(a, self.r[rt], 1)? }
            0x29 => { let a = base(self); self.st(a, self.r[rt], 2)? }
            0x2b => { let a = base(self); self.st(a, self.r[rt], 4)? }
            0x2c | 0x2d => {
                let a = base(self);
                let b = (a & 7) as u32;
                let mem = self.ld(a & !7, 8)? as u64;
                let v = self.r[rt] as u64;
                let low = |n: u32| if n >= 8 { u64::MAX } else { (1u64 << (8 * n)) - 1 };
                let out = if op == 0x2c { (mem & !low(b + 1)) | (v >> (8 * (7 - b))) } else { (mem & low(b)) | (v << (8 * b)) };
                self.st(a & !7, out as u128, 8)?;
            }
            0x31 => { let v = self.ld(base(self), 4)? as u32; self.f[rt] = v }
            0x37 => { let v = self.ld(base(self), 8)?; self.set(rt, (self.r[rt] & !(u64::MAX as u128)) | v) }
            0x39 => { let a = base(self); self.st(a, self.f[rt] as u128, 4)? }
            0x3f => { let a = base(self); self.st(a, self.r[rt], 8)? }
            _ => return unknown(),
        }
        Ok(None)
    }
}
