//! The gamepad backend (fix-input, behind the `gamepad` feature): `gilrs` reads the OS pad and the
//! backend maps it onto the PS2 pad bits and raw analog bytes ([M docs/systems/input.md §2, the
//! WORKLOG 0006 layout]), then the game's own path runs on them — `nf_game::input::actions` (the
//! dead zone, `psiInput_MapInputs`' style table, `StickCompensation2/3`) and the `PadDriver`'s ramps
//! — so pad play is the game's. The keyboard/mouse stay the default: with no pad connected this
//! backend touches nothing.

use nf_game::input::{PadConfig, PadDriver, PadState};
use nf_game::player::Input;

pub struct Backend {
    gilrs: gilrs::Gilrs,
    driver: PadDriver,
    tuning: nf_formats::tuning::Tuning,
    buttons: u16,
    rx: u8,
    ry: u8,
    lx: u8,
    ly: u8,
    connected: bool,
}

impl Backend {
    /// Opens the pad device and reads the game's curve table + tuning from the user's ACTION.ELF.
    pub fn new(iso_path: &str) -> Option<Backend> {
        let gilrs = gilrs::Gilrs::new().ok()?;
        let iso = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
        let elf = iso.read_file(iso.find("/ACTION.ELF")?).ok()?;
        let cfg = PadConfig::from_action_elf(&elf)?;
        let tuning = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap_or_default();
        Some(Backend { gilrs, driver: PadDriver::new(cfg), tuning, buttons: 0, rx: 0x7f, ry: 0x7f, lx: 0x7f, ly: 0x7f, connected: false })
    }

    fn byte(v: f32) -> u8 { (127.0 + v.clamp(-1.0, 1.0) * 127.0).round().clamp(0.0, 255.0) as u8 }

    /// Drains gilrs' events and maps the first connected pad onto the PS2 layout. Buttons [M the
    /// libpad layout, docs/systems/input.md §2]: Cross 0x40, Circle 0x20, Square 0x80, Triangle
    /// 0x10, L1 0x4, R1 0x8, L2 0x1, R2 0x2, Select 0x100, Start 0x800, L3 0x200, R3 0x400,
    /// Up 0x1000, Right 0x2000, Down 0x4000, Left 0x8000. Analog bytes: 0 = left/up, 255 =
    /// right/down — gilrs' +1 = right/down for X, +1 = UP for Y, so Y negates.
    fn poll(&mut self) -> PadState {
        use gilrs::{Button, Event, EventType};
        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::Connected => self.connected = true,
                EventType::Disconnected => { self.connected = false; self.buttons = 0; self.rx = 0x7f; self.ry = 0x7f; self.lx = 0x7f; self.ly = 0x7f; }
                EventType::ButtonPressed(b, _) | EventType::ButtonRepeated(b, _) => self.buttons |= Self::bit(b),
                EventType::ButtonReleased(b, _) => self.buttons &= !Self::bit(b),
                EventType::AxisChanged(axis, v, _) => match axis {
                    gilrs::Axis::LeftStickX => self.lx = Self::byte(v),
                    gilrs::Axis::LeftStickY => self.ly = Self::byte(-v),
                    gilrs::Axis::RightStickX => self.rx = Self::byte(v),
                    gilrs::Axis::RightStickY => self.ry = Self::byte(-v),
                    gilrs::Axis::LeftZ => { if v > 0.5 { self.buttons |= 0x1 } else { self.buttons &= !0x1 } }
                    gilrs::Axis::RightZ => { if v > 0.5 { self.buttons |= 0x2 } else { self.buttons &= !0x2 } }
                    gilrs::Axis::DPadX => { if v < -0.5 { self.buttons |= 0x8000 } else { self.buttons &= !0x8000 } if v > 0.5 { self.buttons |= 0x2000 } else { self.buttons &= !0x2000 } }
                    gilrs::Axis::DPadY => { if v < -0.5 { self.buttons |= 0x1000 } else { self.buttons &= !0x1000 } if v > 0.5 { self.buttons |= 0x4000 } else { self.buttons &= !0x4000 } }
                    _ => {}
                },
                _ => {}
            }
        }
        PadState { buttons: self.buttons, rx: self.rx, ry: self.ry, lx: self.lx, ly: self.ly, connected: self.connected }
    }

    fn bit(b: gilrs::Button) -> u16 {
        match b {
            gilrs::Button::South => 0x40,    // Cross
            gilrs::Button::East => 0x20,     // Circle
            gilrs::Button::West => 0x80,     // Square
            gilrs::Button::North => 0x10,    // Triangle
            gilrs::Button::LeftTrigger => 0x4,  // L1
            gilrs::Button::RightTrigger => 0x8, // R1
            gilrs::Button::Select => 0x100,
            gilrs::Button::Start => 0x800,
            gilrs::Button::LeftThumb => 0x200,  // L3
            gilrs::Button::RightThumb => 0x400, // R3
            gilrs::Button::DPadUp => 0x1000,
            gilrs::Button::DPadDown => 0x4000,
            gilrs::Button::DPadLeft => 0x8000,
            gilrs::Button::DPadRight => 0x2000,
            _ => 0,
        }
    }

    /// One tick: poll the pad, run the game's mapping/curves/ramps, merge into the keyboard's
    /// Input (axes add and clamp, buttons OR). With no pad connected this is a no-op — the
    /// keyboard/mouse Input passes through untouched.
    pub fn merge(&mut self, input: &mut Input, zoom: f32, underwater: bool, scoped: bool) {
        let pad = self.poll();
        if !pad.connected { return; }
        let (p, _actions) = self.driver.tick(&pad, &self.tuning, zoom, underwater, scoped);
        input.forward = (input.forward + p.forward).clamp(-1.0, 1.0);
        input.strafe = (input.strafe + p.strafe).clamp(-1.0, 1.0);
        input.turn += p.turn;
        input.look += p.look;
        input.jump |= p.jump;
        input.crouch |= p.crouch;
        input.aiming |= p.aiming;
        input.fire |= p.fire;
        input.zoom |= p.zoom;
        if input.weapon_cycle == 0 { input.weapon_cycle = p.weapon_cycle; }
        if input.gadget_cycle == 0 { input.gadget_cycle = p.gadget_cycle; }
        if input.switch_to.is_none() { input.switch_to = p.switch_to; }
    }
}
