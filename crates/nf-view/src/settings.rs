//! settings-persist: the PC port's own options file — `settings.toml` in the user's config
//! directory (`%APPDATA%\NightfirePort` on Windows, `$XDG_CONFIG_HOME|~/.config` +
//! `/NightfirePort` elsewhere). The front-end option pages (Advanced / Multiplayer / Audio-video)
//! reset to `default_opts()` on every launch before this; on PS2 the values live in the memory-card
//! save's global-settings blob (`LS_MakeGlobalSettings` 0x1cd9d0 — the game progress itself stays in
//! the save-game model, task 635's flow; THIS file is settings only and never touches progress).
//!
//! File shape (a TOML subset written and read here with std only — the workspace has no toml crate):
//!
//! ```toml
//! [adv]
//! 0x0196 = 1   # the game's row LABEL id (hex) = the row's value
//! [mp]
//! 0x0296 = 50
//! [av]
//! 0x0194 = 80
//! ```
//!
//! Keys are the rows' label ids **per page section**, never indexes, so reordering a page's rows
//! (or adding one) can't scramble saved settings [S the task's rule; the label ids are the game's
//! own stable text ids the option handlers read back with msg 0x35 — P_CNOPTIONS 0x213f08:24-71].
//! A page section + label pair that matches no current row is ignored; a value outside the row's
//! legal set is clamped onto the nearest legal value (sliders 0..=100, the controls' d3 0x64
//! [M P_CNAVOPTIONS 0x214818:37-60]).
//!
//! The model side stays pure: [`FrontEnd::settings_save_pending`] (menu.rs) is raised by the option
//! pages' Confirm/Back and the host drains it (lib.rs `menu_after`) into [`save_opts`]. Load happens
//! once at the front end's boot (lib.rs, where `FrontEnd::new()` runs) through [`load_default`].
//! Missing or corrupt file → the defaults with a warning on stderr. Tests drive everything through
//! `NF_CONFIG_DIR` or an explicit path — never the real user dir.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::menu::{default_opts, AV_ROWS, MP_ROWS, ADV_ROWS, OptKind, OptRow};

/// The file's name inside the config dir.
const FILE_NAME: &str = "settings.toml";
/// The three page sections, in the port's row-table order.
const SECTIONS: [&str; 3] = ["adv", "mp", "av"];

/// One page's persisted rows: (label key, value) in write order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageSettings(pub Vec<(String, i32)>);

/// The whole file's model: one [`PageSettings`] per option page (adv / mp / av).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings(pub [PageSettings; 3]);

/// The config directory: `NF_CONFIG_DIR` wins (the tests' override), then the platform's
/// user config dir + `NightfirePort`. std only — the workspace has no `dirs` crate.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("NF_CONFIG_DIR") {
        if !d.is_empty() { return Some(PathBuf::from(d)); }
    }
    if cfg!(windows) {
        // %APPDATA%\NightfirePort (the roaming config dir; the task's named path).
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("NightfirePort"))
    } else {
        let base = match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x),
            _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
        };
        Some(base.join("NightfirePort"))
    }
}

/// The settings file's full path (`<config dir>/settings.toml`), or None when no config dir
/// can be resolved (no HOME and no override).
pub fn default_path() -> Option<PathBuf> { config_dir().map(|d| d.join(FILE_NAME)) }

/// The label key the file stores: the row's game text id in lower hex (`0x0196`).
fn label_key(label: u32) -> String { format!("0x{label:04x}") }

/// The row's legal values (List: the items' values; Slider: 0..=100; Action: none).
fn legal_values(row: &OptRow) -> Option<Vec<i32>> {
    match row.kind {
        OptKind::Action => None,
        OptKind::Slider => Some(Vec::new()), // empty = the slider's clamp range below
        OptKind::List if !row.items.is_empty() => Some(row.items.iter().map(|it| it.2).collect()),
        OptKind::List => None,
    }
}

/// The value the row would take: List — the nearest legal item value (ties → the lower
/// value, so ON_OFF lands on a legal side either way); Slider — clamped 0..=100 like
/// `opt_step`'s `(v + 10*dir).clamp(0, 100)`. The distance math widens to i64: the file's
/// value is user-controlled and `i32::MIN`'s `.abs()` would overflow (a debug panic).
fn clamp_to_row(row: &OptRow, v: i32) -> Option<i32> {
    let legal = legal_values(row)?;
    if legal.is_empty() { return Some(v.clamp(0, 100)); } // slider
    let best = legal.iter().copied().min_by_key(|&l| ((l as i64 - v as i64).abs(), l));
    best
}

/// Reads the option pages' live values out of the model's map — one entry per value row
/// (Action rows carry no value and are skipped), keyed by label. A control the map lacks
/// falls back to `default_opts` (the map boots full, so this is just belt-and-braces).
pub fn collect(opts: &HashMap<u32, i32>) -> Settings {
    let defaults = default_opts();
    let page = |rows: &[OptRow]| PageSettings(
        rows.iter().filter_map(|r| {
            let v = opts.get(&r.ctrl).copied().or_else(|| defaults.get(&r.ctrl).copied())?;
            legal_values(r)?; // skips Action rows (and any odd empty List)
            Some((label_key(r.label), v))
        }).collect(),
    );
    Settings([page(ADV_ROWS), page(MP_ROWS), page(AV_ROWS)])
}

/// Writes the file's rows back onto the model's map — by label through the CURRENT row
/// tables, so a file saved before a reorder still lands on its row. Unknown labels are
/// ignored; out-of-range values are clamped ([`clamp_to_row`]).
pub fn apply(settings: &Settings, opts: &mut HashMap<u32, i32>) {
    let pages = [ADV_ROWS, MP_ROWS, AV_ROWS];
    for (i, page) in settings.0.iter().enumerate() {
        let rows = pages[i];
        for (key, v) in &page.0 {
            let Some(label) = parse_key(key) else { continue }; // unknown key text: ignored
            let Some(row) = rows.iter().find(|r| r.label == label) else { continue };
            let Some(cv) = clamp_to_row(row, *v) else { continue };
            opts.insert(row.ctrl, cv);
        }
    }
}

/// The key text → label id: `0x196`, `0x0196` and `196` all read as hex 0x196; anything
/// else is an unknown key (ignored by [`apply`]).
fn parse_key(key: &str) -> Option<u32> {
    let t = key.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(t, 16).ok()
}

/// Serializes the file. A `#`-comment header names the format so a hand editor knows the keys.
pub fn to_toml(settings: &Settings) -> String {
    let mut out = String::from(
        "# NightfirePort options (settings-persist). Keys are the game's row label ids\n\
         # (hex) under the option page's section [adv]/[mp]/[av]; unknown keys are ignored\n\
         # and out-of-range values are clamped. See docs/systems/settings-persist.md.\n",
    );
    for (i, name) in SECTIONS.iter().enumerate() {
        let page = &settings.0[i];
        if page.0.is_empty() { continue; }
        out.push_str(&format!("\n[{name}]\n"));
        for (k, v) in &page.0 { out.push_str(&format!("{k} = {v}\n")); }
    }
    out
}

/// Parses the file's text. Structural damage (a line that is neither blank, a `#` comment,
/// a `[section]` header nor a `key = integer` pair, or a key before any section) is corrupt
/// → the caller falls back to the defaults with a warning. A section the port doesn't know
/// (a NEWER port's — forward compatibility) is skipped with its keys, and unknown keys parse
/// fine and are ignored later by [`apply`] — "unknown" is not "corrupt".
pub fn parse(text: &str) -> Result<Settings, String> {
    enum Sec { None, Known(usize), Unknown }
    let mut settings = Settings::default();
    let mut section = Sec::None;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            section = match SECTIONS.iter().position(|s| s == &name) {
                Some(i) => Sec::Known(i),
                None => Sec::Unknown, // a newer port's section: skip it, keep the rest
            };
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(format!("line {}: expected `key = value`", n + 1));
        };
        // A trailing `# comment` after the integer is fine (the doc's own examples carry
        // one); values are integers, so the first `#` starts the comment.
        let v = v.split('#').next().unwrap_or("").trim();
        let Ok(v) = v.parse::<i32>() else {
            return Err(format!("line {}: value `{v}` is not an integer", n + 1));
        };
        let sec = match &section {
            Sec::Known(i) => *i,
            Sec::None => return Err(format!("line {}: key before any [section]", n + 1)),
            Sec::Unknown => continue,
        };
        settings.0[sec].0.push((k.trim().to_string(), v));
    }
    Ok(settings)
}

/// Loads the file into a full option map: `default_opts` first, then the file's values.
/// Missing or corrupt → the plain defaults and a warning line (the host prints it to stderr).
/// (The map is keyed by row CONTROL; the file by label — [`apply`] bridges them.)
pub fn load_opts(path: &std::path::Path) -> (HashMap<u32, i32>, Option<String>) {
    let mut opts = default_opts();
    let Ok(text) = std::fs::read_to_string(path) else {
        return (opts, Some(format!("no settings file at {} — the defaults stand", path.display())));
    };
    match parse(&text) {
        Ok(s) => { apply(&s, &mut opts); (opts, None) }
        Err(e) => (opts, Some(format!("corrupt settings file {} ({e}) — the defaults stand", path.display()))),
    }
}

/// [`load_opts`] against [`default_path`], for the host's boot.
pub fn load_default() -> (HashMap<u32, i32>, Option<String>) {
    match default_path() {
        Some(p) => load_opts(&p),
        None => (default_opts(), None),
    }
}

/// Collects the map into the file model and writes it (creating the config dir if needed).
/// The write is a plain truncate-and-write; a failure returns the message for the host's log.
pub fn save_opts(path: &std::path::Path, opts: &HashMap<u32, i32>) -> Result<(), String> {
    let text = to_toml(&collect(opts));
    let dir = path.parent().ok_or_else(|| format!("settings path {} has no parent", path.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("settings dir {}: {e}", dir.display()))?;
    std::fs::write(path, text).map_err(|e| format!("settings write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_key_round_trips_through_parse() {
        for label in [0x196u32, 0x032, 0x2b5, 0x5bd] {
            assert_eq!(parse_key(&label_key(label)), Some(label));
        }
        assert_eq!(parse_key("0x196"), Some(0x196));
        assert_eq!(parse_key("196"), Some(0x196));
        assert_eq!(parse_key("radar"), None);
    }
}
