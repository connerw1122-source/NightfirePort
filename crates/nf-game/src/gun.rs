//! Bond's trigger (`Player_WeaponFiring` 0x1a5460, called from `Player_Update`): shots per pull, the refire interval
//! and the clip. Spec: docs/systems/drone-damage.md "Bond's gunfire".

/// The trigger state of the selected weapon: clip (`BLData+0x1B8 + w·0xC`), spare rounds of its ammo type
/// (`BLData+0x170`), the refire timer (`+0x92C`), the shots left in this pull (`+0x936`).
#[derive(Clone, Debug)]
pub struct Gun {
    pub weapon: u8,
    pub clip: u32,
    pub spare: u32,
    pub timer: f32,
    pub pending: i32,
    held: bool,
    /// Ticks until a reload finishes.
    pub reloading: u32,
}

/// The fields of a `weapon_data` record the trigger uses.
#[derive(Clone, Copy, Debug)]
pub struct GunParams { pub shots_per_pull: i32, pub interval: f32, pub clip: u32, pub ammo_per_shot: u32, pub reload_ticks: u32, pub one_at_a_time: bool }

impl GunParams {
    /// `script_len(hash)`: the length of a first-person script (frames), for the reload time.
    pub fn from_record(w: &[u8], mode: usize, script_len: impl Fn(u32) -> Option<u32>) -> GunParams {
        let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let flags68 = u32_at(0x68);
        GunParams {
            shots_per_pull: u32_at(0x28 + 4 * mode.min(3)) as i32,
            interval: f32::from_le_bytes(w[0x40..0x44].try_into().unwrap()),
            clip: w[0x92] as u32,
            ammo_per_shot: (w[0x91] as u32).max(1),
            reload_ticks: reload_ticks(u32_at(0xa8), u32_at(0xa4), flags68, &script_len),
            one_at_a_time: flags68 & 0x100 != 0,
        }
    }
}

/// The reload time (`Player_WeaponFiring` → anim state 6 → `Player_SetWeaponAnimObj` case 6: the rounds move when the
/// reload script stops): the start script `+0xA8` if any, then the reload script `+0xA4`; without a start script and
/// flag 0x200 clear the reload plays at 1.35 (`AnimScriptAddSpeed(0x3FACCCCD)`), from frame 6 when flag 0x1000
/// (`AnimListBuild` … `+0x90` = 6.0). P2K: (155 − 6) / 1.35 → 111 ticks (*suggested* to ±1: the stop rule of the
/// script player is not read). Unknown lengths fall back to 60.
pub fn reload_ticks(start: u32, reload: u32, flags68: u32, script_len: &impl Fn(u32) -> Option<u32>) -> u32 {
    let len = |h: u32| if h == 0 { Some(0) } else { script_len(h) };
    let (Some(a), Some(b)) = (len(start), len(reload)) else { return 60 };
    if start != 0 || flags68 & 0x200 != 0 { return a + b; }
    let from = if flags68 & 0x1000 != 0 { 6.0 } else { 0.0 };
    ((b as f32 - from).max(0.0) / 1.35).ceil() as u32
}

impl Gun {
    /// Bond's start on 07000005 (`Player_InitWeapon`): weapon 2 with 7 in the clip and 41 spare [L].
    pub fn new(weapon: u8, clip: u32, spare: u32) -> Gun { Gun { weapon, clip, spare, timer: 0.0, pending: 0, held: false, reloading: 0 } }

    /// One tick with the trigger (action 9) `pressed`; true when a shot leaves the gun this tick. A fresh press with the
    /// timer run out arms `shots_per_pull` shots and sets the timer to 1; the timer falls by `FRAME_RATE_MUL` (1) per
    /// tick; a shot needs the timer at 0 or below, a shot left in the pull and a round in the clip (`Player_RoundToFire`),
    /// and sets the timer to max(interval, 1). An empty clip reloads from the spare rounds.
    pub fn update(&mut self, pressed: bool, p: GunParams) -> bool {
        if self.reloading > 0 {
            self.reloading -= 1;
            // `Player_ReloadAmmoType(…, 0)`: the missing rounds, or one with flag 0x100.
            if self.reloading == 0 { let n = (p.clip - self.clip).min(self.spare).min(if p.one_at_a_time { 1 } else { u32::MAX }); self.clip += n; self.spare -= n; }
        }
        if !pressed { self.pending = 0; }
        if pressed && !self.held && self.timer <= 0.0 { self.pending = p.shots_per_pull; self.timer = 1.0; }
        self.held = pressed;
        self.timer -= 1.0;
        let mut fired = false;
        if self.timer <= 0.0 && self.reloading == 0 {
            self.pending -= 1;
            if self.pending >= 0 {
                if self.clip >= p.ammo_per_shot {
                    self.clip -= p.ammo_per_shot;
                    self.timer = p.interval.max(1.0);
                    fired = true;
                } else { self.pending = 0; }
            } else { self.pending = 0; }
        }
        if self.clip < p.ammo_per_shot && self.spare > 0 && self.reloading == 0 { self.reloading = p.reload_ticks.max(1); }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shot_per_press_at_most_every_interval() {
        let p = GunParams { shots_per_pull: 1, interval: 8.0, clip: 7, ammo_per_shot: 1, reload_ticks: reload_ticks(0, 0x0600_0827, 0x1004, &|_| Some(155)), one_at_a_time: false };
        assert_eq!(p.reload_ticks, 111);
        let mut g = Gun::new(2, 7, 41);
        // Holding the trigger fires once.
        let held: Vec<bool> = (0..20).map(|_| g.update(true, p)).collect();
        assert_eq!(held.iter().filter(|&&f| f).count(), 1);
        assert!(held[0]);
        // Tapping every 4 ticks: a press only arms once the timer has run out before it, so presses 8 ticks after a
        // shot (timer still 1) are lost and the next one 12 ticks after fires.
        let mut g = Gun::new(2, 7, 41);
        let taps: Vec<usize> = (0..40).filter(|&t| g.update(t % 4 == 0, p)).collect();
        assert_eq!(taps, [0, 12, 24, 36]);
        assert_eq!(g.clip, 3);
        // Sister: an empty clip does not fire, then reloads.
        let mut g = Gun::new(2, 0, 41);
        assert!(!g.update(true, p));
        for _ in 0..p.reload_ticks { g.update(false, p); }
        assert_eq!((g.clip, g.spare), (7, 34));
    }
}
