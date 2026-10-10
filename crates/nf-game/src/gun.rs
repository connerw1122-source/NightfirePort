//! Bond's trigger (`Player_WeaponFiring` 0x1a5460, called from `Player_Update`): shots per pull, the refire interval
//! and the clip, plus the weapon-switch state machine (`Player_WeaponChange` 0x1a6f80 → `Player_SetWeaponAnimObj`
//! 0x1a2e38). Spec: docs/systems/player.md "Aim and bullets" + docs/systems/pickups.md "Bond's inventory".

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
    /// `anim+0xF4` reload states: 0 = idle, 5 = reload start (+0xA8), 6 = shell loop (+0xA4, one shell),
    /// 7 = reload end (+0xAC). The view-script block in `Sim::run` drives these [M Player_SetWeaponAnimObj].
    pub reload_state: u8,
    /// Ticks remaining in the current reload script (5/6/7). When it hits 0, the state transitions.
    pub reload_ticks: u32,
    /// Set by `reload_tick` when the shell loop continues (state 6 stays 6 but the +0xA4 script must replay
    /// from frame 0 for the next shell). The view-script block in `Sim::run` checks this to re-queue +0xA4.
    pub reload_loop_restarted: bool,
    /// `anim+0x63`: the switch-request weapon id (set by `Player_WeaponChange`, consumed by
    /// `Player_SetWeaponAnimObj`). When it differs from `weapon`, the holster/draw cycle runs.
    pub switch_request: u8,
    /// `anim+0x64`: the saved weapon id (set by `Player_WeaponNone` for the toggle-back from hands) [M 0x1a9858].
    pub saved_weapon: u8,
    /// `BLData+0x930`: the last weapon (for the toggle-back when cycling from a gadget to a weapon) [M 0x1a6f80].
    pub prev_weapon: u8,
    /// `BLData+0x932`: the last gadget (for the toggle-back when cycling from a weapon to a gadget) [M 0x1a6f80].
    pub prev_gadget: u8,
    /// `BLData+0x959` / `anim+0xF4`: the view-script state machine's phase.
    /// 0 = idle, 1 = holstering, 2 = holster-done (swap), 3 = drawing, 4 = draw-done, 8 = variant-swap.
    pub switch_state: u8,
}

impl Gun {
    /// `Player_WeaponChange` (0x1a6f80): the weapon cycle (`param_3` = 0, bounds 1..0x47) or the gadget cycle
    /// (`param_3` = 1, bounds 0x4A..0x5E) [M disassembly 0x1a707c: `li s6,73` for mode=1; 0x1a7084: `li v1,95`
    /// for mode=1's high bound; 0x1a7120 for mode=0's bounds]. Steps through owned, selectable, has-ammo
    /// entries starting from `weapon + dir`, wrapping within the range. The game does NOT check the first-person
    /// model here (that's `Player_WeaponSelect`'s job); weapon 1 (fists) is a valid cycle target.
    /// Returns the id to switch to, or `None`.
    pub fn cycle(&self, dir: i8, mode: CycleMode, owned: &[bool], has_ammo: impl Fn(u8) -> bool, selectable: impl Fn(u8) -> bool) -> Option<u8> {
        let dir = dir.signum() as i32;
        if dir == 0 { return None; }
        let (lo, hi) = mode.bounds();
        let mut w = self.weapon as i32;
        for _ in 0..(hi - lo + 1) {
            w += dir;
            if w > hi { w = lo; }
            if w < lo { w = hi; }
            let wu = w as u8;
            if !owned.get(w as usize).copied().unwrap_or(false) { continue; }
            if !selectable(wu) { continue; }
            if !has_ammo(wu) { continue; }
            return Some(wu);
        }
        None
    }

    /// `Player_WeaponChange` toggle-back path [M 0x1a6f80 lines 66-121]. When cycling weapons (mode=0) from a
    /// gadget, if `prev_weapon` (+0x930) is set and has ammo, switch directly to it. When cycling gadgets
    /// (mode=1) from a weapon, if `prev_gadget` (+0x932) is set and has ammo, switch to it. Returns the toggle
    /// target, or `None` if no toggle applies (fall through to the cycle).
    pub fn toggle_back(&self, mode: CycleMode, owned: &[bool], has_ammo: impl Fn(u8) -> bool, selectable: impl Fn(u8) -> bool) -> Option<u8> {
        let cur = self.weapon;
        match mode {
            CycleMode::Weapons => {
                if cur >= 0x4a && cur <= 0x5e && self.prev_weapon != 0 {
                    let w = self.prev_weapon;
                    if owned.get(w as usize).copied().unwrap_or(false) && selectable(w) && has_ammo(w) {
                        return Some(w);
                    }
                }
            }
            CycleMode::Gadgets => {
                if cur >= 1 && cur <= 0x47 && self.prev_gadget != 0 {
                    let w = self.prev_gadget;
                    if owned.get(w as usize).copied().unwrap_or(false) && selectable(w) && has_ammo(w) {
                        return Some(w);
                    }
                }
            }
        }
        None
    }

    /// `Player_WeaponHasAmmo` (0x1a16c0): whether the player has ammo for weapon `w` — carried rounds of its
    /// ammo type, or rounds in the clip (`Player_AmmoInGun`), or (for `ammo_type == 0`) always true (melee /
    /// unlimited weapons), or (for flag 0x20000 weapons) a matching ammo pickup on the ground. Weapon 0x45
    /// (a grenade type) checks `BLData+0x4F4` against `weapon_data+0x48CE` [M].
    /// *Deviation*: the 0x20000 ground-pickup scan is a STANDIN (returns false; the pickup list is not in Gun).
    pub fn has_ammo(w: u8, carried: &[i16], clip: &[i16], ammo_type: impl Fn(u8) -> usize, flags: impl Fn(u8) -> u32) -> bool {
        if w == 0x45 {
            // Grenade type: `weapon_data._18638_2_` (offset 0x48CE) <= `BLData+0x4F4`. STANDIN: always true if owned.
            return true;
        }
        let t = ammo_type(w);
        if carried.get(t).copied().unwrap_or(0) != 0 { return true; }
        // `Player_AmmoInGun`: rounds in the clip.
        if clip.get(w as usize).copied().unwrap_or(0) != 0 { return true; }
        // `ammo_type == 0`: melee / unlimited weapons (fists, hands, gadgets) always have ammo [M 0x1a16c0].
        if t == 0 { return true; }
        // Flag 0x20000: check the ground for a matching ammo pickup. STANDIN: not ported (no object list in Gun).
        false
    }

    /// `Player_GetBestWeapon` (0x1a9680): scans the `BestWeapon` table (0x2bede0, 18 u16) for the first owned,
    /// has-ammo weapon. Each BestWeapon entry is a `weapon_data` index; the real weapon id is
    /// `weapon_data[entry*0x10c+2]` (the base weapon). The per-player `+0x1BC` variant-slot adjustment is a
    /// STANDIN (not ported — the Inventory has no `+0x1BC` field) [M].
    pub fn get_best_weapon(&self, best: &[u16], base_of: impl Fn(u8) -> u8, owned: &[bool], has_ammo: impl Fn(u8) -> bool) -> Option<u8> {
        for &w in best {
            let real = base_of(w as u8);
            if owned.get(real as usize).copied().unwrap_or(false) && has_ammo(real) {
                return Some(real);
            }
        }
        None
    }

    /// `Player_WeaponNone` (0x1a9858): Bond goes to "hands" (no weapon, weapon id 0x47). Saves the current weapon
    /// to `saved_weapon` (`anim+0x64`, for the toggle-back), sets `weapon = 0x47`, `switch_request = 0x47` (no
    /// switch triggered), stops the view script [M].
    pub fn go_to_hands(&mut self) {
        self.saved_weapon = self.weapon; // `anim+0x64 = anim+0x62` (save for toggle-back)
        self.weapon = 0x47;
        self.switch_request = 0x47; // `anim+0x63 = 0x47` — no switch triggered
        self.switch_state = 0;
    }

    /// Whether a weapon switch is in progress (the holster/draw cycle).
    pub fn switching(&self) -> bool { self.switch_state != 0 }

    /// Whether a reload is in progress (states 5/6/7).
    pub fn reloading(&self) -> bool { self.reload_state != 0 }

    /// Whether the view script needs to be restarted this tick (the shell loop continued — the +0xA4 script
    /// must replay from frame 0 for the next shell, matching `AnimScriptAdd(+0xA4)` in case 6 [M 0x1a2e38]).
    pub fn reload_loop_restarted(&self) -> bool { self.reload_state == 6 && self.reload_ticks == 0 }

    /// Advance the reload state machine one tick. Called by `Sim::run` after advancing the view script.
    /// `trigger_held` = the fire button is currently held (firing interrupt). `p` = the weapon params.
    /// Returns the new reload state (for the view-script block to pick the right script).
    pub fn reload_tick(&mut self, trigger_held: bool, p: GunParams) -> u8 {
        self.reload_loop_restarted = false;
        if self.reload_state == 0 { return 0; }
        if self.reload_ticks > 0 { self.reload_ticks -= 1; }
        if self.reload_ticks > 0 { return self.reload_state; }
        // The current script has finished. Transition.
        match self.reload_state {
            5 => {
                // Reload start done → queue +0xA4, go to state 6 [M case 5].
                self.reload_state = 6;
                self.reload_ticks = p.reload_loop_ticks.max(1);
            }
            6 => {
                // Shell loop: commit one shell [M case 6: Player_ReloadAmmoType(_, 0)].
                let n = if p.one_at_a_time { 1 } else { (p.clip - self.clip).min(self.spare) };
                let n = n.min(self.spare);
                self.clip += n;
                self.spare -= n;
                // Probe: more ammo to load? [M case 6: Player_ReloadAmmoType(_, 1)]
                let more = self.spare > 0 && self.clip < p.clip;
                if more && !trigger_held {
                    // Continue the loop: queue +0xA4 again [M case 6]. Set the flag so Sim::run restarts the
                    // view script from frame 0 (matching `AnimScriptAdd(+0xA4)` in the game's case 6).
                    self.reload_loop_restarted = true;
                    self.reload_ticks = p.reload_loop_ticks.max(1);
                } else {
                    // Stop: clip full, no spare, or trigger held (fire interrupt). Queue +0xAC, go to state 7.
                    self.reload_state = 7;
                    self.reload_ticks = p.reload_end_ticks.max(1);
                }
            }
            7 => {
                // Reload end done → go to idle [M case 7]. Clear the trigger action.
                self.reload_state = 0;
                self.reload_ticks = 0;
                self.held = false; // Input_ClearAction(9)
                self.pending = 0;
            }
            _ => {}
        }
        self.reload_state
    }
}

/// The cycle mode: weapons (1..0x47) or gadgets (0x4A..0x5E) [M `Player_WeaponChange` 0x1a6f80].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CycleMode {
    /// `param_3` = 0: cycle weapons (ids 1..0x47). Triggered by L2/R2 or Left/Right on the d-pad.
    Weapons,
    /// `param_3` = 1: cycle gadgets (ids 0x4A..0x5E). Triggered by Up/Down on the d-pad.
    Gadgets,
}

impl CycleMode {
    /// The cycle bounds [M disassembly: `slti v0,a0,71` (0x47) for weapons; `li s6,73`/`li v1,95` (0x49/0x5F)
    /// for gadgets, wrapping 0x4A..0x5E].
    pub fn bounds(&self) -> (i32, i32) {
        match self {
            CycleMode::Weapons => (1, 0x47),
            CycleMode::Gadgets => (0x4a, 0x5e),
        }
    }
}

/// The fields of a `weapon_data` record the trigger uses.
#[derive(Clone, Copy, Debug)]
pub struct GunParams {
    pub shots_per_pull: i32,
    pub interval: f32,
    pub clip: u32,
    pub ammo_per_shot: u32,
    pub reload_ticks: u32,       // legacy: total reload time for non-shotgun weapons
    pub one_at_a_time: bool,
    pub reload_start_ticks: u32, // state 5: +0xA8 script length (0 if no start script)
    pub reload_loop_ticks: u32,  // state 6: +0xA4 script length per shell
    pub reload_end_ticks: u32,   // state 7: +0xAC script length
    pub ammo_type: u8,           // +0x90: 0 = melee (unlimited, no clip check), 1+ = ranged
}

impl GunParams {
    /// `script_len(hash)`: the length of a first-person script (frames), for the reload time.
    pub fn from_record(w: &[u8], mode: usize, script_len: impl Fn(u32) -> Option<u32>) -> GunParams {
        let u32_at = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let flags68 = u32_at(0x68);
        let len_of = |h: u32| -> u32 {
            if h == 0 { return 0; }
            script_len(h).unwrap_or(60)
        };
        let start_h = u32_at(0xa8);
        let reload_h = u32_at(0xa4);
        let end_h = u32_at(0xac);
        GunParams {
            shots_per_pull: u16::from_le_bytes([w[0x28 + 2 * mode.min(3)], w[0x29 + 2 * mode.min(3)]]) as i32,
            interval: u32_at(0x40) as f32,
            clip: u16::from_le_bytes([w[0x92], w[0x93]]) as u32,
            ammo_per_shot: (w[0x91] as u32).max(1),
            reload_ticks: reload_ticks(start_h, reload_h, flags68, &script_len),
            one_at_a_time: flags68 & 0x100 != 0,
            reload_start_ticks: len_of(start_h),
            reload_loop_ticks: len_of(reload_h),
            reload_end_ticks: len_of(end_h),
            ammo_type: w[0x90],
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
    pub fn new(weapon: u8, clip: u32, spare: u32) -> Gun { Gun { weapon, clip, spare, timer: 0.0, pending: 0, held: false, reload_state: 0, reload_ticks: 0, reload_loop_restarted: false, switch_request: weapon, saved_weapon: weapon, prev_weapon: 0, prev_gadget: 0, switch_state: 0 } }

    /// One tick with the trigger (action 9) `pressed` (no reload key, gun idle).
    pub fn update(&mut self, pressed: bool, p: GunParams) -> bool { self.update_with(pressed, false, false, p) }

    /// One tick (`Player_WeaponFiring` 0x1a5460): `reload` = action 14 pressed (R2: activate, else reload),
    /// `busy` = the gun's animation state is not idle (a fire script still playing). A fresh press with the timer run
    /// out arms `shots_per_pull` shots and sets the timer to 1; the timer falls by `FRAME_RATE_MUL` (1) per tick; a shot
    /// needs the timer at 0 or below, a shot left in the pull and a round in the clip (`Player_RoundToFire`), and sets
    /// the timer to max(interval, 1). A reload starts with the reload key, or with the trigger released and no round to
    /// fire — only while the gun is idle and there are spare rounds.
    ///
    /// The reload state machine (cases 5/6/7 of `Player_SetWeaponAnimObj`): state 5 (reload start, +0xA8) → state 6
    /// (shell loop, +0xA4, one shell per iteration) → state 7 (reload end, +0xAC) → state 0 (idle). For one-at-a-time
    /// weapons (flag 0x100), each state-6 iteration loads 1 shell. Firing interrupts: if the trigger is held when a
    /// shell's +0xA4 finishes, the reload aborts to state 7 (+0xAC). The `Sim::run` method in nf-view drives the
    /// view scripts and calls `reload_tick` to advance the state machine.
    pub fn update_with(&mut self, pressed: bool, reload: bool, busy: bool, p: GunParams) -> bool {
        // If a reload is in progress, the Sim::run method drives it via reload_tick. Here we just block firing.
        if self.reload_state != 0 {
            self.held = pressed;
            return false;
        }
        let can_reload = !busy && self.spare > 0 && self.clip < p.clip;
        if can_reload && (reload || (!pressed && self.clip < p.ammo_per_shot)) {
            // Start the reload. `Player_WeaponFiring` sets anim+0xF4 = 5 if +0xA8 (reload_start) exists, else 6.
            self.reload_state = if p.reload_start_ticks > 0 { 5 } else { 6 };
            self.reload_ticks = if p.reload_start_ticks > 0 { p.reload_start_ticks } else { p.reload_loop_ticks };
            self.pending = 0;
            self.held = pressed;
            return false;
        }
        if !pressed { self.pending = 0; }
        if pressed && !self.held && self.timer <= 0.0 { self.pending = p.shots_per_pull; self.timer = 1.0; }
        self.held = pressed;
        self.timer -= 1.0;
        let mut fired = false;
        if self.timer <= 0.0 {
            self.pending -= 1;
            if self.pending >= 0 {
                // `Player_RoundToFire` (0x1a18e0): for ammo_type == 0 (melee), always round to fire and
                // don't touch the clip [M]. For ranged weapons, check and decrement the clip.
                if p.ammo_type == 0 || self.clip >= p.ammo_per_shot {
                    if p.ammo_type != 0 { self.clip -= p.ammo_per_shot; }
                    self.timer = p.interval.max(1.0);
                    fired = true;
                } else { self.pending = 0; }
            } else { self.pending = 0; }
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shot_per_press_at_most_every_interval() {
        let p = GunParams { shots_per_pull: 1, interval: 8.0, clip: 7, ammo_per_shot: 1, reload_ticks: reload_ticks(0, 0x0600_0827, 0x1004, &|_| Some(155)), one_at_a_time: false, reload_start_ticks: 0, reload_loop_ticks: 111, reload_end_ticks: 0, ammo_type: 1 };
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
        // Released: the reload starts (state 6, no start script for the P2K).
        assert!(!g.update(false, p));  // trigger released + empty clip → reload starts
        assert_eq!(g.reload_state, 6); // state 6 (shell loop)
        // Run the reload loop: each tick calls reload_tick. The P2K (not one-at-a-time) loads all rounds in one shell.
        for _ in 0..p.reload_loop_ticks { g.reload_tick(false, p); }
        // After the loop: clip is full (7), spare is 34.
        // The P2K has no +0xAC (reload_end), so reload_end_ticks=0 but reload_tick uses .max(1) → 1 tick.
        g.reload_tick(false, p); // enters state 7 (reload_ticks = max(0,1) = 1)
        g.reload_tick(false, p); // state 7 done (ticks=0) → state 0
        assert_eq!(g.reload_state, 0); // → state 0 (idle)
        assert_eq!((g.clip, g.spare), (7, 34));
    }

    #[test]
    fn cycle_skips_unowned_unselectable_and_empty() {
        // Three weapons: 2 (P2K, owned, has ammo), 0x16 (SG5, owned, has ammo), 0x24 (sniper, owned, no ammo).
        let owned = vec![false; 0x73];
        let mut owned = owned;
        owned[2] = true; owned[0x16] = true; owned[0x24] = true;
        let selectable = |w: u8| w == 2 || w == 0x16 || w == 0x24;
        let has_ammo = |w: u8| w == 2 || w == 0x16; // 0x24 has no ammo
        let g = Gun::new(2, 7, 41);
        // Next from P2K → SG5 (0x16).
        assert_eq!(g.cycle(1, CycleMode::Weapons, &owned, has_ammo, selectable), Some(0x16));
        // Next from SG5 → wraps to P2K (0x24 skipped: no ammo).
        let g2 = Gun::new(0x16, 30, 60);
        assert_eq!(g2.cycle(1, CycleMode::Weapons, &owned, has_ammo, selectable), Some(2));
        // Prev from P2K → wraps to SG5 (0x24 skipped).
        assert_eq!(g.cycle(-1, CycleMode::Weapons, &owned, has_ammo, selectable), Some(0x16));
        // Sister: with no ammo at all, cycle returns None.
        let no_ammo = |_w: u8| false;
        assert_eq!(g.cycle(1, CycleMode::Weapons, &owned, no_ammo, selectable), None);
    }

    #[test]
    fn cycle_weapons_and_gadgets_are_separate() {
        // Weapons: 2 (owned, ammo). Gadgets: 0x4A (owned, ammo), 0x50 (owned, ammo).
        let owned = vec![false; 0x73];
        let mut owned = owned;
        owned[2] = true; owned[0x4a] = true; owned[0x50] = true;
        let selectable = |w: u8| w == 2 || w == 0x4a || w == 0x50;
        let has_ammo = |_w: u8| true;
        let g = Gun::new(2, 7, 41);
        // Weapon cycle from P2K (2) → wraps to 2 (only weapon).
        assert_eq!(g.cycle(1, CycleMode::Weapons, &owned, has_ammo, selectable), Some(2));
        // Gadget cycle from P2K (2, a weapon) → starts at 2, steps to 0x4A (first gadget).
        assert_eq!(g.cycle(1, CycleMode::Gadgets, &owned, has_ammo, selectable), Some(0x4a));
        // Gadget cycle from 0x4A → 0x50.
        let g2 = Gun::new(0x4a, 1, 1);
        assert_eq!(g2.cycle(1, CycleMode::Gadgets, &owned, has_ammo, selectable), Some(0x50));
        // Sister: weapon cycle from a gadget (0x4A) → wraps to 2 (first weapon).
        assert_eq!(g2.cycle(1, CycleMode::Weapons, &owned, has_ammo, selectable), Some(2));
    }

    /// The cycle bounds are pinned to the game's disassembly values [M disassembly 0x1a707c/0x1a7084:
    /// `li s6,73` (0x49) and `li v1,95` (0x5F) for gadgets; `slti v0,a0,71` (0x47) for weapons].
    #[test]
    fn cycle_bounds_match_the_disassembly() {
        assert_eq!(CycleMode::Weapons.bounds(), (1, 0x47));
        assert_eq!(CycleMode::Gadgets.bounds(), (0x4a, 0x5e));
        // Sister: a weapon at 0x48 is NOT in the weapon range (0x48 > 0x47) — it would be skipped.
        assert!(0x48 > CycleMode::Weapons.bounds().1);
        // Sister: a gadget at 0x5F is NOT in the gadget range (0x5F > 0x5E).
        assert!(0x5f > CycleMode::Gadgets.bounds().1);
    }

    /// `Player_WeaponNone` (0x1a9858) + `Player_GetBestWeapon` (0x1a9680): when Bond is at hands (id 0x47),
    /// `GetBestWeapon` scans the `BestWeapon` table for the first owned+has-ammo weapon and returns it [M].
    #[test]
    fn hands_recovers_to_best_weapon() {
        const BEST_WEAPON: [u16; 18] = [0x42, 0x32, 0x1a, 0x18, 0x1c, 0x16, 0x15, 0x08, 0x10, 0x0e, 0x0d, 0x0c, 0x12, 0x06, 0x04, 0x02, 0x0a, 0x01];
        let owned = vec![false; 0x73];
        let mut owned = owned;
        // Bond owns only the P2K (2) with ammo.
        owned[2] = true;
        let has_ammo = |w: u8| w == 2;
        let base_of = |w: u8| w; // identity: BestWeapon entries are weapon_data indices; base(id) = id for the P2K
        let mut g = Gun::new(0x47, 0, 0); // hands
        // GetBestWeapon finds the P2K.
        assert_eq!(g.get_best_weapon(&BEST_WEAPON, base_of, &owned, has_ammo), Some(2));
        // Go to hands saves the current weapon to saved_weapon (anim+0x64), sets weapon=0x47, switch_request=0x47.
        g.weapon = 2;
        g.go_to_hands();
        assert_eq!(g.weapon, 0x47); // hands
        assert_eq!(g.switch_request, 0x47); // no switch triggered [M 0x1a9858]
        assert_eq!(g.saved_weapon, 2); // saved for toggle-back (anim+0x64)
        // Sister: if nothing is owned, GetBestWeapon returns None.
        let empty = vec![false; 0x73];
        assert_eq!(g.get_best_weapon(&BEST_WEAPON, base_of, &empty, has_ammo), None);
    }

    /// `Player_GetBestWeapon` looks up the real weapon id via `weapon_data[entry*0x10c+2]` (the base weapon).
    /// BestWeapon entry 0x15 has base 0x14; entry 0x32 has base 0x33; entry 0x18 has base 0x19 [M 0x1a9680].
    #[test]
    fn get_best_weapon_uses_base_weapon_lookup() {
        const BEST_WEAPON: [u16; 18] = [0x42, 0x32, 0x1a, 0x18, 0x1c, 0x16, 0x15, 0x08, 0x10, 0x0e, 0x0d, 0x0c, 0x12, 0x06, 0x04, 0x02, 0x0a, 0x01];
        // Only weapon 0x14 (the base of BestWeapon entry 0x15) is owned with ammo.
        let owned = vec![false; 0x73];
        let mut owned = owned;
        owned[0x14] = true;
        let has_ammo = |w: u8| w == 0x14;
        // base_of: entry 0x15 → base 0x14 (the variant points at its base).
        let base_of = |w: u8| if w == 0x15 { 0x14 } else { w };
        let g = Gun::new(0x47, 0, 0);
        // GetBestWeapon should find 0x14 (the base of entry 0x15), not 0x15 itself.
        assert_eq!(g.get_best_weapon(&BEST_WEAPON, base_of, &owned, has_ammo), Some(0x14));
        // Sister: if base_of is identity (wrong), 0x15 is checked instead and not owned → None.
        let identity = |w: u8| w;
        assert_eq!(g.get_best_weapon(&BEST_WEAPON, identity, &owned, has_ammo), None);
    }

    /// The toggle-back: cycling weapons from a gadget jumps to the last weapon [M 0x1a6f80 lines 93-121].
    #[test]
    fn toggle_back_from_gadget_to_weapon() {
        let owned = vec![true; 0x73];
        let has_ammo = |_w: u8| true;
        let selectable = |_w: u8| true;
        let mut g = Gun::new(0x4a, 1, 1); // a gadget
        g.prev_weapon = 2; // last weapon was the P2K
        // Cycling weapons from a gadget → toggle to the P2K (not the cycle).
        assert_eq!(g.toggle_back(CycleMode::Weapons, &owned, has_ammo, selectable), Some(2));
        // Sister: cycling gadgets from a gadget → no toggle (None).
        assert_eq!(g.toggle_back(CycleMode::Gadgets, &owned, has_ammo, selectable), None);
        // Sister: prev_weapon = 0 → no toggle.
        g.prev_weapon = 0;
        assert_eq!(g.toggle_back(CycleMode::Weapons, &owned, has_ammo, selectable), None);
    }
}
