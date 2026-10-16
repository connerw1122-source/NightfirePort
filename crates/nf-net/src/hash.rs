//! FNV-1a 64 over the shared deterministic state (the match-host layer's
//! `Hasher`, moved here in the consolidation — one hash for one session
//! layer). The app folds in a fixed order (slot order, then the shared
//! world), so equal states give equal hashes on every machine; the hashed
//! field set is the port's own choice ([S]) and lives in the app's
//! `net_state_hash`.

/// FNV-1a 64 state.
pub struct Hasher(pub u64);

impl Hasher {
    pub fn new() -> Self {
        Hasher(0xcbf2_9ce4_8422_2325)
    }
    pub fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x100_0000_01b3);
        }
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }
    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    pub fn bool(&mut self, v: bool) {
        self.bytes(&[v as u8]);
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Hasher::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_folds_same_hash() {
        let mut a = Hasher::new();
        let mut b = Hasher::new();
        a.u32(7);
        a.f32(1.5);
        a.bool(true);
        b.u32(7);
        b.f32(1.5);
        b.bool(true);
        assert_eq!(a.finish(), b.finish());
        // Sister: a different fold order must NOT land on the same hash.
        let mut c = Hasher::new();
        c.f32(1.5);
        c.u32(7);
        c.bool(true);
        assert_ne!(c.finish(), a.finish());
    }
}
