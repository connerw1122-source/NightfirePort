//! The monitor consoles: placement 0x30 (`Monitor_Create` 0x1b08b0, `Monitor_Activate` 0x1b0058,
//! `Monitor_Update` 0x1afcc8).
//!
//! Semantics [M]:
//! - `Monitor_Create` 0x1b08b0: instance 0x14, class 0x38; `obj+0xF6` = param 0 (the monitor
//!   group id, matched against the sensors' param-6 byte); param 1 == 0 → health 1.0
//!   (shootable), else health 0 (immune).
//! - `Monitor_Activate` 0x1b0058 (the E path, `Player_Activate` case 0x38): requires state 0 and
//!   a class-3 activator; picks a sensor from `CameraList` (param 5 == 0, not destroyed) whose
//!   `+0x5E` (param 6) equals the monitor's group; state 2; `Camera_SetUpdator` (the view renders
//!   the sensor's pose — draw side), `Input_SetAction(0xE)`; `Player_Disable`; HUD panes.
//! - `Monitor_Update` 0x1afcc8 while viewing: writes
//!   `switch_channels[watched_sensor+0x5C] = 1` (the sensor's param-8 monitor-view channel) every
//!   tick, mirrors the sensor pose into the viewer (draw side) and sets the sensor's +0x60
//!   unhide; action 0xE cycles to the next same-group camera, and with none left (or the watched
//!   camera destroyed) restores and cools down (state 9 → 4 → 0).
//! - Shot (`Monitor_ApplyDamage`): health − damage < 0 → `Control_NextLOD`, state 3 and
//!   `Explode_Create(r·6, r, 0.5, …, script 0x6000027, 200, 200)`.
//!
//! The camera-view rendering (the sensor's picture on screen) is the draw side's work; the sim
//! delivers the state, the watched-sensor selection and the channel writes.

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::level::Level;

#[derive(Clone, Debug)]
pub struct Monitor {
    pub placement: usize,
    /// The object position (obj+0x80 stand-in: the placement translation).
    pub pos: V3,
    /// `obj+0xF6` = param 0 — the group id matched against the sensors' param-6 byte.
    pub group: u8,
    /// param 1 == 0 → 1.0 (shootable), else 0.0 (immune) [M 0x1b08b0:16-21].
    pub health: f32,
    /// obj+0xF4: 0 idle, 2 viewing, 3 destroyed, 9/4 the exit cooldown [M 0x1b0058/0x1afcc8].
    pub state: u8,
    /// The watched sensor's index into `Sensors::list` while state 2.
    pub watched: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Monitors {
    pub list: Vec<Monitor>,
}

impl Monitors {
    /// `Monitor_Create` over the main map's 0x30 placements (placement order, case 0x30
    /// [M 0x1d04f0]). The position is the placed object's glist-bounds centre — 07000001's
    /// monitor (#105 `room_communication_tv`) sits at the identity transform with world-space
    /// object verts [D placements], like the rooms; the game keys the placed object's own
    /// obj+0x80 (rebuilt from the matrix + glist centre by the 0x1336f8 pass 6).
    pub fn create(level: &Level) -> Monitors {
        let Some(m) = level.main_map() else { return Monitors::default() };
        let mut list = Vec::new();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0x30 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = crate::props::bounds(level, m, p);
            let pos = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
            list.push(Monitor {
                placement: i,
                pos,
                group: (param(0) & 0xff) as u8,
                health: if param(1) == 0 { 1.0 } else { 0.0 },
                state: 0,
                watched: None,
            });
        }
        Monitors { list }
    }

    /// `Monitor_Activate` 0x1b0058: state 0 → pick the group's first `CameraList` sensor
    /// (`Sensor.listed`, not destroyed) → state 2. Called with the use key; while state 2 the
    /// same edge is action 0xE (cycle or exit) [M 0x1afcc8's update arm].
    pub fn activate(&mut self, i: usize, sensors: &crate::sensor::Sensors) -> bool {
        let Some(m) = self.list.get_mut(i) else { return false };
        if m.state == 3 { return false; }
        if m.state == 2 {
            // The 0xE arm: the next same-group camera after the watched one, else the exit.
            let group = m.group;
            let cur = m.watched;
            let next = sensors.list.iter().enumerate()
                .filter(|(si, s)| s.listed && s.monitor_id == group && Some(*si) != cur && s.state != 3)
                .map(|(si, _)| si)
                .find(|_| true);
            match next {
                Some(si) => { m.watched = Some(si); return true; }
                None => { m.state = 9; m.watched = None; return true; }
            }
        }
        if m.state != 0 { return false; }
        let group = m.group;
        let first = sensors.list.iter().enumerate()
            .filter(|(_, s)| s.listed && s.monitor_id == group && s.state != 3)
            .map(|(si, _)| si)
            .find(|_| true);
        match first {
            Some(si) => { m.state = 2; m.watched = Some(si); true }
            None => false,
        }
    }

    /// `Monitor_Update` 0x1afcc8's per-tick body for the viewing monitors: the watched sensor's
    /// monitor-view channel latches 1 every tick, the sensor's +0x60 unhide sets; the cooldown
    /// states drain 9 → 4 → 0.
    pub fn tick(&mut self, sensors: &mut crate::sensor::Sensors, channels: &mut Channels, frame: u32) {
        for m in &mut self.list {
            match m.state {
                2 => {
                    if let Some(si) = m.watched {
                        if let Some(s) = sensors.list.get_mut(si) {
                            channels.set(s.monitor_ch as u32, 1, frame);
                            s.unhidden = true;
                        }
                    }
                }
                9 => m.state = 4,
                4 => m.state = 0,
                _ => {}
            }
        }
    }

    /// The bullet arm (`Monitor_ApplyDamage`): health − damage < 0 → state 3 + the explode event.
    /// Returns true when this shot destroyed it. Immune monitors (param 1 != 0, health 0) never
    /// die [M 0x1b08b0:16-21].
    pub fn bullet_hit(&mut self, a: V3, b: V3, damage: f32) -> bool {
        for m in &mut self.list {
            if m.state == 3 || m.health <= 0.0 { continue; }
            // The use-sphere stand-in (crate::props::seg_box's rule): the segment's AABB against
            // the monitor's small box.
            let (lo, hi) = ([m.pos[0] - 0.6, m.pos[1] - 0.6, m.pos[2] - 0.6], [m.pos[0] + 0.6, m.pos[1] + 0.6, m.pos[2] + 0.6]);
            let hit = (0..3).all(|k| a[k].max(b[k]) >= lo[k] && a[k].min(b[k]) <= hi[k]);
            if hit {
                m.health -= damage;
                if m.health < 0.0 {
                    m.state = 3;
                    m.watched = None;
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor() -> Monitor {
        Monitor { placement: 0, pos: [0.0, 6.0, 0.0], group: 1, health: 1.0, state: 0, watched: None }
    }

    #[test]
    fn activating_picks_the_group_sensor_and_the_view_writes_its_monitor_channel() {
        // 07000001: the monitor's p0 = 1 and the camera sensor's p6 = 1 [D placements dump];
        // the sensor's p8 (monitor_ch) is 0 here, so the game writes switch_channels[0] = 1
        // every viewed tick [M 0x1afcc8 — `switch_channels[sensor_inst+0x5C] = 1`].
        let mut mons = Monitors { list: vec![monitor()] };
        let mut sensors = crate::sensor::Sensors { list: vec![crate::sensor::Sensor {
            placement: 1, pos: [1.0; 3], matrix: Default::default(), quat: Default::default(),
            keys: Vec::new(), limit: 0, step: 0.0, t: 0.0, cos_half: 0.0, sin_half: 0.0,
            range: 6.0, out: 0, disable: 0, monitor_id: 1, listed: true, lens: 0, mount: 0,
            monitor_ch: 0, state: 0, detect: 0.0, beep: 0.0, seen: 0, last_hit: false, unhidden: false,
        }] };
        assert!(mons.activate(0, &sensors), "state 0 + a group-1 listed sensor → state 2 [M 0x1b0058]");
        assert_eq!(mons.list[0].state, 2);
        let mut ch = Channels::default();
        mons.tick(&mut sensors, &mut ch, 10);
        assert!(ch.on(0), "the viewed tick writes the sensor's monitor-view channel [M 0x1afcc8]");
        assert!(sensors.list[0].unhidden, "the sensor's +0x60 unhide sets while viewed [M 0x1afcc8]");
    }

    #[test]
    fn the_sister_idle_monitor_never_writes_and_a_shot_destroys_the_console() {
        // Sister: no activation → no channel, ever.
        let mut mons = Monitors { list: vec![monitor()] };
        let mut sensors = crate::sensor::Sensors { list: Vec::new() };
        let mut ch = Channels::default();
        mons.tick(&mut sensors, &mut ch, 10);
        assert!(!ch.on(0), "an idle monitor writes nothing");
        assert!(mons.activate(0, &sensors) == false, "no same-group sensor → the activate refuses");
        // A 1.0-health console dies to one 25-damage bullet [M Monitor_ApplyDamage].
        let mut mons = Monitors { list: vec![monitor()] };
        assert!(mons.bullet_hit([-1.0, 6.0, 0.0], [1.0, 6.0, 0.0], 25.0));
        assert_eq!(mons.list[0].state, 3, "state 3 [M Monitor_ApplyDamage]");
        // The sister miss.
        let mut mons = Monitors { list: vec![monitor()] };
        assert!(!mons.bullet_hit([-1.0, 60.0, 0.0], [1.0, 60.0, 0.0], 25.0));
        assert_eq!(mons.list[0].state, 0);
    }
}
