//! The dev panel's shared command layer. Since fix-ingame-dev-menu the in-window F2 menu (dev.rs)
//! carries ALL of the panel's functions — every give/spawn/clear command goes through `apply` below,
//! so the menu and the HTTP page cannot drift apart. The HTTP server itself (the small web page on
//! http://127.0.0.1:7007 that runs beside the game window) is now opt-in behind `--panel` for anyone
//! who still wants a second screen. Standard library only: one listener thread, a shared catalog
//! string and a command queue. Dev tool: never changes game logic when unused, and adds no RNG draws
//! to the game's stream.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

pub const PORT: u16 = 7007;

#[derive(Default)]
pub struct Shared {
    /// JSON describing the current level's spawnables (rebuilt by the game on level load).
    pub catalog: String,
    /// The last command result, shown in the panel.
    pub status: String,
    /// Commands (URL query strings) waiting for the game loop.
    pub queue: Vec<String>,
}

pub type Handle = Arc<Mutex<Shared>>;

/// Start the panel server. Returns None if the port is taken (another nf-view already serves it).
pub fn start() -> Option<Handle> {
    let listener = TcpListener::bind(("127.0.0.1", PORT)).ok()?;
    let shared: Handle = Arc::new(Mutex::new(Shared::default()));
    let s = shared.clone();
    std::thread::Builder::new().name("dev-panel".into()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() { continue; }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (route, query) = path.split_once('?').unwrap_or((path.as_str(), ""));
            let (ctype, body) = match route {
                "/" => ("text/html; charset=utf-8", PAGE.to_string()),
                "/api/catalog" => ("application/json", s.lock().map(|g| g.catalog.clone()).unwrap_or_default()),
                "/api/status" => ("text/plain; charset=utf-8", s.lock().map(|g| g.status.clone()).unwrap_or_default()),
                "/api/cmd" => {
                    if let Ok(mut g) = s.lock() { g.queue.push(query.to_string()); }
                    ("text/plain; charset=utf-8", "queued".to_string())
                }
                _ => ("text/plain", "not found".to_string()),
            };
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    }).ok()?;
    Some(shared)
}

/// Parse `a=1&b=x` into pairs (values are plain ASCII; `+`/`%20` → space).
pub fn query(q: &str) -> Vec<(String, String)> {
    q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (k.to_string(), v.replace('+', " ").replace("%20", " "))
    }).collect()
}

/// Escape a string for JSON.
pub fn js(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

const PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>nf-view dev panel</title>
<style>
:root { --bg:#14161a; --panel:#1d2026; --text:#e6e8eb; --muted:#9aa1ab; --accent:#d8b45a; --line:#2c3038; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--text); font:14px/1.4 system-ui, sans-serif; }
header { position:sticky; top:0; background:var(--panel); border-bottom:1px solid var(--line); padding:10px 14px; display:flex; gap:12px; align-items:center; flex-wrap:wrap; z-index:1; }
header h1 { font-size:15px; margin:0; color:var(--accent); }
#status { color:var(--muted); font-size:13px; flex:1; min-width:200px; }
main { padding:12px 14px; display:grid; gap:14px; grid-template-columns:repeat(auto-fit, minmax(300px, 1fr)); }
section { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:10px 12px; }
h2 { font-size:13px; text-transform:uppercase; letter-spacing:.06em; color:var(--muted); margin:0 0 8px; }
.row { display:flex; align-items:center; gap:8px; padding:4px 0; border-top:1px solid var(--line); }
.row:first-of-type { border-top:none; }
.name { flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.dim { color:var(--muted); font-size:12px; }
button { background:#2a2f38; color:var(--text); border:1px solid #3a404b; border-radius:6px; padding:4px 9px; cursor:pointer; font:inherit; font-size:13px; }
button:hover { border-color:var(--accent); }
button.big { padding:6px 12px; }
select, input { background:#2a2f38; color:var(--text); border:1px solid #3a404b; border-radius:6px; padding:3px 6px; font:inherit; font-size:13px; }
input[type=search] { width:100%; margin-bottom:6px; }
</style></head><body>
<header><h1>nf-view dev panel</h1><span id="level" class="dim"></span><span id="status">connecting…</span>
<button class="big" onclick="cmd('do=levelkit')">Level kit (the game's loadout)</button>
<button class="big" onclick="cmd('do=arena')">Arena mode (clear level)</button>
<button class="big" onclick="cmd('do=clear_npcs')">Remove spawned NPCs</button>
<button class="big" onclick="cmd('do=god')">God mode</button>
<button class="big" onclick="cmd('do=ammo')">Infinite ammo</button>
<button class="big" onclick="cmd('do=shot')">Screenshot + log spot</button></header>
<main>
<section><h2>Current weapon</h2><div id="cur" class="dim">-</div></section>
<section><h2>NPCs (spawn in front of Bond)</h2>
<div class="dim" style="margin-bottom:6px">Mode: <select id="mode"><option value="">level default</option><option value="0">normal</option><option value="2">sniper</option><option value="16">ambush</option></select></div>
<div id="npcs"></div></section>
<section><h2>Weapons (give to Bond)</h2><input type="search" id="wf" placeholder="filter…" oninput="draw()"><div id="weapons"></div></section>
<section><h2>Gadgets</h2><div class="dim" style="margin-bottom:6px">Gadgets aren't ported yet: giving one only puts it in the inventory.</div><div id="gadgets"></div></section>
<section><h2>Consumables</h2><div id="cons"></div></section>
</main>
<script>
let cat = null;
async function cmd(q) {
  try { await fetch('/api/cmd?' + q); setTimeout(status, 150); } catch (e) { document.getElementById('status').textContent = 'game not reachable'; }
}
async function status() { try { document.getElementById('status').textContent = await (await fetch('/api/status')).text(); } catch (e) {} }
function el(h) { const d = document.createElement('div'); d.innerHTML = h; return d.firstElementChild; }
function draw() {
  if (!cat) return;
  document.getElementById('level').textContent = 'level ' + cat.level;
  const c = cat.current || {};
  document.getElementById('cur').innerHTML = c.name ? `<b>${c.name}</b> (0x${(+c.weapon).toString(16).padStart(2,'0')})` +
    (c.fire_mode ? ` · fire mode: <b>${c.fire_mode}</b>` : '') +
    ` · ammo: <b>${c.ammo_type || '?'}</b> · clip ${c.clip} · spare ${c.spare}` : '-';
  const n = document.getElementById('npcs'); n.innerHTML = '';
  for (const p of cat.npcs) n.append(el(`<div class="row"><span class="name">skin ${p.skin}</span><span class="dim">${p.free}/${p.total} free</span><button onclick="cmd('do=npc&skin=${p.skin}&mode='+document.getElementById('mode').value)">Spawn</button></div>`));
  if (!cat.npcs.length) n.append(el('<div class="dim">no NPC types on this level</div>'));
  const f = document.getElementById('wf').value.toLowerCase();
  const w = document.getElementById('weapons'); w.innerHTML = '';
  const g = document.getElementById('gadgets'); g.innerHTML = '';
  for (const x of cat.weapons) {
    const label = `0x${x.id.toString(16).padStart(2,'0')} ${x.name}`;
    if (f && !label.toLowerCase().includes(f)) continue;
    const mesh = x.proj ? ` · proj 0x${(+x.proj).toString(16)}` : '';
    (x.gadget ? g : w).append(el(`<div class="row"><span class="name">${label}${x.sel ? '' : ' <span class=dim>(variant)</span>'}</span><span class="dim">clip ${x.clip}${x.ammo ? ' · ' + x.ammo : ''}${mesh}${x.fp ? '' : ' · no 1st-person model here'}</span><button onclick="cmd('do=weapon&id=${x.id}')">Give</button><button onclick="cmd('do=ammo_for&id=${x.id}')">+Ammo</button></div>`));
  }
  const c = document.getElementById('cons'); c.innerHTML = '';
  // The grenade ids are the game's own: 0x34 (52) Fragmentation, 0x35 (53) Stun, 0x36 (54) Smoke
  // [L USATxt.dat via Txt_BindLabel(+0x38); the old buttons gave 0x35 for "Frag" — the STUN
  // grenade — the user's "a frag grenade spawns what looks to be a stun grenade"].
  for (const [label, q] of [['Full health', 'do=health'], ['Full armour', 'do=armour'], ['Fragmentation grenades ×5', 'do=weapon&id=52&rounds=5'], ['Stun grenades ×5', 'do=weapon&id=53&rounds=5'], ['Smoke grenades ×5', 'do=weapon&id=54&rounds=5'], ['Ammo for every owned weapon', 'do=ammo_all']])
    c.append(el(`<div class="row"><span class="name">${label}</span><button onclick="cmd('${q}')">Give</button></div>`));
}
async function load() {
  try { const t = await (await fetch('/api/catalog')).text(); if (t) { const c = JSON.parse(t); if (!cat || c.level !== cat.level || c.stamp !== cat.stamp || JSON.stringify(c.current) !== JSON.stringify(cat.current)) { cat = c; draw(); } } status(); }
  catch (e) { document.getElementById('status').textContent = 'game not reachable — start nf-view with --dev'; }
}
load(); setInterval(load, 1000);
</script></body></html>
"#;

/// The game's own name for a record: `Txt_BindLabel(weapon_data[w]+0x38)` over USATxt.dat — the same
/// lane the game itself binds for the pickup message and the HUD's ammo pane (`Pickup_Handler`
/// 0x18e650:246, `HUD_UpdateAmmoPane` 0x19a7f0) [M; docs/fields/weapon_data.md +0x38 = the SP name].
/// The old hardcoded table asserted from memory and was wrong all over (it put the AT-600 Scorpion on
/// 0x3A — the bank's Laser Tripbomb; "Korsakov K5" on 0x33 — the bank's Phoenix Samurai; "Militek
/// MGL" on 0x45 — the bank's Oddjob's Hat; "Q-Laser"/"Laser Watch" on 0x50/0x54 — the bank's
/// Grapple/Micro-Camera), which is the user's "I can't tell what's what / no clue what the rocket
/// launcher is": the real rocket launchers are 0x2C AT-420 Sentinel and 0x2E AT-600 Scorpion, and the
/// grenades are 0x34 Fragmentation (radius 8, dmg 100 [L +0x08/+0x0C]) / 0x35 Stun (radius 10, dmg 0)
/// / 0x36 Smoke / 0x37 Remote Mine. 0x01 (fists) carries no label — the game's own code equips it as
/// the fists tail (`Player_InitWeapon` 0x1a7ef8:319) [M]. Anything else unnamed shows its id.
fn record_name(lang: Option<&nf_formats::hud::Lang>, wd: &nf_formats::weapon::WeaponData, id: u8) -> String {
    let label = wd.name_label(id as usize);
    if label != 0 && label != 0xFFFFFFFF {
        if let Some(t) = lang.and_then(|l| l.bind(label)) {
            let t = t.trim();
            if !t.is_empty() { return t.to_string(); }
        }
    }
    if id == 0x01 { return "Fists".into(); }
    if id >= 0x4a { format!("gadget {id:#04x}") } else { format!("weapon {id:#04x}") }
}

/// One panel row: the record's own name, label id and meshes, so the panel shows (and the tests
/// pin) exactly what a click spawns.
pub struct Entry {
    pub id: u8,
    /// The game's own name (`Txt_BindLabel(+0x38)`; `record_name`'s fallback otherwise).
    pub name: String,
    /// The raw +0x38 label id (0xFFFFFFFF = none).
    pub label: u32,
    pub clip: i16,
    /// The ammo type's own bank name (`ammo_data[type]+8`, the pickup's `%s` [M 0x18e650 case 1]); "" when none.
    pub ammo: String,
    /// The level carries the first-person model (a view model was built for it).
    pub fp: bool,
    /// Gadgets are ids 0x4A and up (`Player_WeaponChange`'s gadget cycle bounds) [M 0x1a6f80].
    pub gadget: bool,
    /// `weapon_data+0x04 == 1`: appears in the switch cycle.
    pub sel: bool,
    /// The held view-entity model (+0x7C; `AnimDatumSetEntity` per draw, `Player_WeaponFiring`
    /// 0x1a5460:106 [M]), the third-person/pickup model (+0x80) and the flying projectile's mesh
    /// (+0x5C; `Bullet_update` 0x12e7b0 disasm 0x12e8f0 sets it on the bullet object [M]). 0 = none.
    pub held: u32,
    pub tp: u32,
    pub proj: u32,
}

/// The spawnable records for the panel, each resolved through its `weapon_data` record.
pub fn catalog_entries(scene: &crate::scene::Scene) -> Vec<Entry> {
    let Some(sim) = scene.sim.as_ref() else { return Vec::new() };
    let wd = &sim.weapons;
    let lang = sim.lang.as_ref();
    let mut out = Vec::new();
    for id in 1..0x72u8 {
        // 0x47 is "no weapon" (`AnimDatumGetWeaponInfo`'s bounds exclude it from the cycle;
        // `Player_WeaponChange` cycles 1..0x47) [M]. Everything else that the game names, shows or
        // cycles stays listed — INCLUDING the clip-0 gadgets (0x50 Grapple, 0x56/0x57 Decryptor,
        // 0x58 Q-Worm: tp model 0 and clip 0, but named and selectable, and the game's loadouts
        // grant them — the old filter's `model==0 && clip<=0` silently dropped them).
        if id == 0x47 { continue; }
        let label = wd.name_label(id as usize);
        let has_mesh = wd.model(id as usize) != 0 || wd.projectile_mesh(id as usize) != 0;
        // Nameless, meshless, non-selectable padding (0x48/0x49 — the gadget cycle starts at 0x4A
        // [M 0x1a6f80] — and the AI-clip records 0x61/0x71) would only add "weapon 0x48" noise rows.
        if (label == 0 || label == 0xFFFFFFFF) && !wd.selectable(id) && !has_mesh { continue; }
        let at = wd.ammo_type(id as usize);
        let al = wd.ammo_name_label(at);
        let ammo = if al != 0 { lang.and_then(|l| l.bind(al)).unwrap_or("").trim().to_string() } else { String::new() };
        let norm = |h: u32| if h == u32::MAX { 0 } else { h };
        let rec = &wd.records[id as usize];
        let u32_at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        out.push(Entry {
            id,
            name: record_name(lang, wd, id),
            label,
            clip: wd.clip_size(id as usize),
            ammo,
            fp: scene.view_models.contains_key(&id),
            gadget: id >= 0x48,
            sel: wd.selectable(id),
            held: norm(u32_at(0x7c)),
            tp: wd.model(id as usize),
            proj: norm(wd.projectile_mesh(id as usize)),
        });
    }
    out
}

/// The current weapon's state for the panel: its game name, the fire-mode label text
/// (`weapon_data+0x30 + mode*4` — the HUD's own read, `HUD_UpdateAmmoPane` 0x19a7f0:229 [M]; the
/// port's trigger runs mode 0, `GunParams::from_record(w, 0, ..)`), the ammo type's bank name
/// (`ammo_data[type]+8` [M]) and the HUD's clip/spare counts.
fn current_json(sim: &crate::scene::Sim) -> String {
    let wi = sim.gun.weapon;
    let wd = &sim.weapons;
    let lang = sim.lang.as_ref();
    let name = record_name(lang, wd, wi);
    let fl = wd.fire_mode_label(wi as usize, 0);
    let fire = if fl != 0 { lang.and_then(|l| l.bind(fl)).unwrap_or("").trim().to_string() } else { String::new() };
    let at = wd.ammo_type(wi as usize);
    let al = wd.ammo_name_label(at);
    let ammo = if al != 0 { lang.and_then(|l| l.bind(al)).unwrap_or("").trim().to_string() } else { format!("ammo type {at}") };
    format!("{{\"weapon\":{},\"name\":\"{}\",\"fire_mode\":\"{}\",\"ammo_type\":\"{}\",\"clip\":{},\"spare\":{}}}",
        wi, js(&name), js(&fire), js(&ammo), sim.gun.clip, sim.gun.spare)
}

/// The current level's spawnables as JSON (plus the current weapon's fire mode and ammo type).
pub fn catalog(scene: &crate::scene::Scene) -> String {
    let Some(sim) = scene.sim.as_ref() else { return "{\"level\":\"none\",\"stamp\":0,\"current\":{},\"weapons\":[],\"npcs\":[]}".into() };
    let mut weapons = Vec::new();
    for e in catalog_entries(scene) {
        weapons.push(format!("{{\"id\":{},\"name\":\"{}\",\"label\":{},\"clip\":{},\"ammo\":\"{}\",\"fp\":{},\"gadget\":{},\"sel\":{},\"held\":{},\"tp\":{},\"proj\":{}}}",
            e.id, js(&e.name), e.label, e.clip, js(&e.ammo), e.fp, e.gadget, e.sel, e.held, e.tp, e.proj));
    }
    let current = current_json(sim);
    let mut skins: Vec<u32> = Vec::new();
    for s in &scene.npc_pool { if !skins.contains(&s.skin) { skins.push(s.skin); } }
    let mut free_total = 0;
    let npcs: Vec<String> = skins.iter().map(|&skin| {
        let slots: Vec<_> = scene.npc_pool.iter().filter(|s| s.skin == skin).collect();
        let free = slots.iter().filter(|s| s.chars.iter().all(|&c| sim.char_drone.get(c).copied().flatten().is_none())).count();
        free_total += free;
        format!("{{\"skin\":\"{skin:08x}\",\"free\":{free},\"total\":{}}}", slots.len())
    }).collect();
    format!("{{\"level\":\"{:08x}\",\"stamp\":{free_total},\"current\":{current},\"weapons\":[{}],\"npcs\":[{}]}}", sim.level, weapons.join(","), npcs.join(","))
}

/// Apply one panel command to the scene. Returns the status text and, for a given weapon, the weapon to switch to.
pub fn apply(scene: &mut crate::scene::Scene, q: &[(String, String)]) -> (String, Option<u8>) {
    let get = |k: &str| q.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str());
    // `0x`-prefixed values parse as hex (the old code stripped the prefix and parsed DECIMAL, so
    // every hex id silently failed); plain values stay decimal (the page's ids are decimal).
    let num = |k: &str| get(k).and_then(|v| {
        let v = v.trim();
        match v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
            Some(d) => u32::from_str_radix(d, 16).ok(),
            None => v.parse::<u32>().ok(),
        }
    });
    // The bank name for a status line comes from the sim's own Lang (resolved inside each arm —
    // a shared closure here would hold `scene` immutably across the mutable borrows).
    let Some(what) = get("do") else { return ("no command".into(), None) };
    match what {
        "weapon" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let Some(id) = num("id").filter(|&i| i < 0x72) else { return ("bad weapon id".into(), None) };
            let rounds = num("rounds").map(|r| r as i16).unwrap_or_else(|| sim.weapons.clip_size(id as usize).max(1));
            let wd = sim.weapons.clone();
            let name = record_name(sim.lang.as_ref(), &wd, id as u8);
            // `Player_EquipWeapon` 0x1a8a58:20 refuses a weapon whose first-person model (+0xDC) is
            // 0 — the game would refuse too, so the panel says so instead of switching to a record
            // Bond cannot hold (the AI-only records list for completeness; the row's dim text says
            // "no 1st-person model here").
            if !sim.drones.inventory.equip_weapon(&wd, id as usize, rounds, true) {
                return (format!("weapon {id:#04x} ({name}): refused — the record has no first-person model (Player_EquipWeapon 0x1a8a58:20)"), None);
            }
            let carry = (wd.clip_size(id as usize).max(1) * 3).max(rounds);
            sim.drones.inventory.equip_ammo(&wd, id as usize, carry);
            // fix-aims20-modes-lights: a twin-linked record (+0x05, e.g. the AIMS-20 0x1a ↔ its
            // grenade twin 0x1b) leaves the give with the twin's own clip slot dry, and the mode
            // press is dead forever — `Player_Weapon` 0x1a24c8:246-260 gates the twin switch on
            // `Player_WeaponHasAmmo` 0x1a16c0, and the ground truth measured the gate ("with no
            // grenade rounds the press did nothing; after picking up the 0x1b rounds (4) the press
            // swapped"). The give is the port's acquire affordance (the game's pickups give the
            // twin's rounds separately — 07000014 carries the 0x1b pickups), so fill the twin's
            // slot once (its own slot: the pair's ammo types differ, +0x90 5 vs 12 [L ee_snapshot
            // weapon_data], the `Player_AmmoInGun` own-slot pick) the way a taken 0x1b pickup does.
            let twin = wd.records.get(id as usize).map(|r| r[5] as i8).unwrap_or(0);
            if twin != 0 {
                let t = (id as i32 + twin as i32) as usize;
                if let Some(c) = sim.drones.inventory.clip.get_mut(t) {
                    if *c == 0 { *c = wd.clip_size(t).max(1); }
                }
            }
            (format!("gave weapon {id:#04x} ({name})"), Some(id as u8))
        }
        "ammo_for" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let Some(id) = num("id") else { return ("bad weapon id".into(), None) };
            let wd = sim.weapons.clone();
            let n = sim.drones.inventory.equip_ammo(&wd, id as usize, wd.clip_size(id as usize).max(1) * 3);
            (format!("ammo for weapon {id:#04x}: +{n}"), None)
        }
        "ammo_all" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let wd = sim.weapons.clone();
            let owned: Vec<usize> = (1..0x72).filter(|&w| sim.drones.inventory.owned.get(w).copied().unwrap_or(false)).collect();
            for &w in &owned { sim.drones.inventory.equip_ammo(&wd, w, wd.clip_size(w).max(1) * 3); }
            (format!("ammo for {} owned weapons", owned.len()), None)
        }
        // The game's own loadout for the current level: `Player_InitWeapon` (0x1a7ef8) on a cold
        // load (`save = None` clears through `Player_InitAmmoWeapons` first), the same call the
        // level-load path runs — so a playtest starts like the real game (weapons AND gadgets).
        "levelkit" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let wd = sim.weapons.clone();
            let level = sim.level;
            let req = {
                let skins = &sim.skins;
                let loaded: Vec<bool> = (0..0x72u32).map(|w| skins.contains_key(&wd.first_person(w as usize))).collect();
                let drones = &mut sim.drones;
                // Borrowed (not moved) capture: the closure must be Copy (`init_level_loadout` runs
                // it per equip, like the level-load call in nf_view::load).
                nf_game::player::init_level_loadout(level, None, &mut drones.health, &mut drones.inventory,
                    &mut drones.channels, &wd, |w| loaded.get(w as usize).copied().unwrap_or(false))
            };
            sim.gun.switch_request = req;
            let owned = sim.drones.inventory.owned.iter().filter(|&&o| o).count();
            (format!("level kit: {owned} owned weapons, switch request {req:#04x}"), None)
        }
        "health" => { if let Some(sim) = scene.sim.as_mut() { sim.drones.health.health = 100.0; } ("health 100".into(), None) }
        "armour" => { if let Some(sim) = scene.sim.as_mut() { sim.drones.health.armour = 50.0; } ("armour 50".into(), None) }
        "npc" => {
            let Some(skin) = get("skin").and_then(|s| u32::from_str_radix(s, 16).ok()) else { return ("bad skin".into(), None) };
            let mode = num("mode");
            match spawn_npc(scene, skin, mode) {
                Ok(i) => (format!("spawned NPC #{i} (skin {skin:08x})"), None),
                Err(e) => (e, None),
            }
        }
        "clear_npcs" => {
            let n = clear_npcs(scene);
            (format!("removed {n} spawned NPCs"), None)
        }
        "arena" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let mut n = 0;
            for dr in sim.drones.list.iter_mut() { if dr.present { n += 1; } dr.present = false; dr.enabled = false; }
            for p in sim.drones.pickups.iter_mut() { p.gone = true; }
            // Triggers: deleted, except level exits — a gone LOAD_LEVEL trigger means "fired" and would end the level.
            // Those are disarmed instead (gate 0 never fires, `Trigger_Update` case 10).
            for t in sim.drones.triggers.iter_mut() {
                if t.kind == nf_game::triggers::kind::LOAD_LEVEL { t.gate = 0; } else { t.gone = true; }
            }
            // Spawners would bring guards straight back: give them nothing to spawn.
            for sp in sim.drones.spawners.iter_mut() { sp.templates.clear(); sp.used.clear(); sp.max_alive = 0; }
            for c in sim.char_drone.iter_mut() { *c = None; }
            (format!("arena: removed {n} NPCs, all pickups and triggers"), None)
        }
        other => (format!("unknown command {other}"), None),
    }
}

/// Bring a free pool NPC of `skin` to life a few metres in front of Bond, facing him.
pub fn spawn_npc(scene: &mut crate::scene::Scene, skin: u32, mode: Option<u32>) -> Result<usize, String> {
    let slot = {
        let sim = scene.sim.as_ref().ok_or("no game")?;
        scene.npc_pool.iter().find(|s| s.skin == skin && s.chars.iter().all(|&c| sim.char_drone.get(c).copied().flatten().is_none()))
            .cloned().ok_or_else(|| format!("no free {skin:08x} NPC (pool of {} per type)", crate::scene::NPC_POOL))?
    };
    let sim = scene.sim.as_mut().ok_or("no game")?;
    let eye = sim.bond.head();
    let look = sim.bond.look();
    let flat = (look[0] * look[0] + look[2] * look[2]).sqrt().max(1e-6);
    let mut pos = [eye[0] + look[0] / flat * 4.0, eye[1], eye[2] + look[2] / flat * 4.0];
    pos[1] = sim.world.floor_below([pos[0], eye[1] + 1.0, pos[2]], 20.0).unwrap_or(sim.bond.pos[1]);
    let yaw = (eye[0] - pos[0]).atan2(eye[2] - pos[2]);
    let params = slot.params.clone();
    let param = |k: u32| match (k, mode) {
        (16 | 21, Some(m)) => m,
        // A dev spawn is active at once: no switch channel to wait on (3), no spawner group (4).
        (3 | 4, _) => 0,
        _ => params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v),
    };
    // A separate RNG for the dev spawn: the game's own stream is untouched.
    let mut rng = nf_game::drone::Rng::new(0xdead_beef ^ sim.drones.list.len() as u32);
    let dr = {
        let d = sim.data();
        nf_game::drone::Drone::new(&d, pos, yaw, param, &mut rng)
    };
    let mut dr = dr;
    dr.present = true;
    dr.enabled = true;
    dr.started = false;
    let idx = sim.drones.list.len();
    sim.drones.list.push(dr);
    sim.drones.llist.push(idx);
    for &c in &slot.chars {
        if let Some(cd) = sim.char_drone.get_mut(c) { *cd = Some(idx); }
        if let Some(a) = sim.char_ambient.get_mut(c) { *a = [0xff; 3]; }
    }
    Ok(idx)
}

/// Remove every NPC spawned from the pool.
pub fn clear_npcs(scene: &mut crate::scene::Scene) -> usize {
    let Some(sim) = scene.sim.as_mut() else { return 0 };
    let mut n = 0;
    for slot in &scene.npc_pool {
        let mut hit = false;
        for &c in &slot.chars {
            if let Some(di) = sim.char_drone.get(c).copied().flatten() {
                if let Some(dr) = sim.drones.list.get_mut(di) { dr.present = false; dr.enabled = false; }
                sim.drones.llist.retain(|&k| k != di);
                sim.char_drone[c] = None;
                hit = true;
            }
        }
        if hit { n += 1; }
    }
    n
}
