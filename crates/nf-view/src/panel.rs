//! The external dev panel: `nf-view --dev` serves a small web page on http://127.0.0.1:7007 that runs beside the game
//! window (any browser). It lists what the loaded level can spawn and sends commands back; the game applies them
//! between frames (`App::apply_panel`). Standard library only: one listener thread, a shared catalog string and a
//! command queue. Dev tool: never changes game logic when unused, and adds no RNG draws to the game's stream.

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
<button class="big" onclick="cmd('do=arena')">Arena mode (clear level)</button>
<button class="big" onclick="cmd('do=clear_npcs')">Remove spawned NPCs</button>
<button class="big" onclick="cmd('do=god')">God mode</button>
<button class="big" onclick="cmd('do=ammo')">Infinite ammo</button></header>
<main>
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
  const n = document.getElementById('npcs'); n.innerHTML = '';
  for (const p of cat.npcs) n.append(el(`<div class="row"><span class="name">skin ${p.skin}</span><span class="dim">${p.free}/${p.total} free</span><button onclick="cmd('do=npc&skin=${p.skin}&mode='+document.getElementById('mode').value)">Spawn</button></div>`));
  if (!cat.npcs.length) n.append(el('<div class="dim">no NPC types on this level</div>'));
  const f = document.getElementById('wf').value.toLowerCase();
  const w = document.getElementById('weapons'); w.innerHTML = '';
  const g = document.getElementById('gadgets'); g.innerHTML = '';
  for (const x of cat.weapons) {
    const label = `0x${x.id.toString(16).padStart(2,'0')} ${x.name}`;
    if (f && !label.toLowerCase().includes(f)) continue;
    (x.gadget ? g : w).append(el(`<div class="row"><span class="name">${label}${x.sel ? '' : ' <span class=dim>(variant)</span>'}</span><span class="dim">clip ${x.clip}${x.fp ? '' : ' · no 1st-person model here'}</span><button onclick="cmd('do=weapon&id=${x.id}')">Give</button><button onclick="cmd('do=ammo_for&id=${x.id}')">+Ammo</button></div>`));
  }
  const c = document.getElementById('cons'); c.innerHTML = '';
  for (const [label, q] of [['Full health', 'do=health'], ['Full armour', 'do=armour'], ['Frag grenades ×5', 'do=weapon&id=53&rounds=5'], ['Stun grenades ×5', 'do=weapon&id=54&rounds=5'], ['Ammo for every owned weapon', 'do=ammo_all']])
    c.append(el(`<div class="row"><span class="name">${label}</span><button onclick="cmd('${q}')">Give</button></div>`));
}
async function load() {
  try { const t = await (await fetch('/api/catalog')).text(); if (t) { const c = JSON.parse(t); if (!cat || c.level !== cat.level || c.stamp !== cat.stamp) { cat = c; draw(); } } status(); }
  catch (e) { document.getElementById('status').textContent = 'game not reachable — start nf-view with --dev'; }
}
load(); setInterval(load, 1000);
</script></body></html>
"#;

/// Known weapon names (docs/atlas/weapon-data.md; 0x0E from the 01-raptor PCSX2 trace). Others show their id.
fn weapon_name(id: u8) -> &'static str {
    match id {
        0x01 => "Fists", 0x02 => "P2K", 0x04 => "P2K variant", 0x06 => "Default sidearm (0x06)", 0x08 => "0x06 variant",
        0x0E => "Raptor", 0x10 => "(ammo type 11)", 0x11 => "(scoped)", 0x12 => "SMG (32 rounds)", 0x14 => "(21 rounds)",
        0x16 => "SG5 Commando", 0x19 => "(zoom 3.0)", 0x1C => "Shotgun", 0x20 => "Sniper (scoped)", 0x22 => "Sniper (10 rounds)",
        0x24 => "Sniper L96A1", 0x26 => "Sniper (scoped)", 0x28 => "Sniper (10 rounds)", 0x30 => "(4 rounds)", 0x33 => "(scoped, 100 rounds)",
        0x34 => "(1 round)", 0x35 => "Frag Grenade", 0x36 => "Stun Grenade", 0x37 => "Remote Mine", 0x41 => "(ammo type 21)",
        0x42 => "(ammo type 19)", 0x43 => "(5 rounds)", 0x44 => "(5 rounds variant)", 0x46 => "(unlimited)",
        0x50 => "Q-Laser", 0x51 => "Q-Laser variant", 0x52 => "(ammo type 27)", 0x53 => "(variant of 0x52)", 0x54 => "Laser Watch",
        _ => "",
    }
}

/// The current level's spawnables as JSON.
pub fn catalog(scene: &crate::scene::Scene) -> String {
    let Some(sim) = scene.sim.as_ref() else { return "{\"level\":\"none\",\"stamp\":0,\"weapons\":[],\"npcs\":[]}".into() };
    let wd = &sim.weapons;
    let mut weapons = Vec::new();
    for id in 1..0x72u8 {
        // Every record with a model or rounds (variants included); 0x47 is "no weapon".
        if id == 0x47 || (wd.model(id as usize) == 0 && wd.clip_size(id as usize) <= 0) { continue; }
        let clip = wd.clip_size(id as usize);
        let name = weapon_name(id);
        let name = if name.is_empty() { format!("weapon {id:#04x}") } else { name.to_string() };
        // Gadgets are ids 0x48 and up (`Player_WeaponChange`: weapons cycle 1..0x47, gadgets 0x4A..0x5E).
        let gadget = id >= 0x48;
        weapons.push(format!("{{\"id\":{id},\"name\":\"{}\",\"clip\":{clip},\"fp\":{},\"gadget\":{gadget},\"sel\":{}}}", js(&name), scene.view_models.contains_key(&id), wd.selectable(id)));
    }
    let mut skins: Vec<u32> = Vec::new();
    for s in &scene.npc_pool { if !skins.contains(&s.skin) { skins.push(s.skin); } }
    let mut free_total = 0;
    let npcs: Vec<String> = skins.iter().map(|&skin| {
        let slots: Vec<_> = scene.npc_pool.iter().filter(|s| s.skin == skin).collect();
        let free = slots.iter().filter(|s| s.chars.iter().all(|&c| sim.char_drone.get(c).copied().flatten().is_none())).count();
        free_total += free;
        format!("{{\"skin\":\"{skin:08x}\",\"free\":{free},\"total\":{}}}", slots.len())
    }).collect();
    format!("{{\"level\":\"{:08x}\",\"stamp\":{free_total},\"weapons\":[{}],\"npcs\":[{}]}}", sim.level, weapons.join(","), npcs.join(","))
}

/// Apply one panel command to the scene. Returns the status text and, for a given weapon, the weapon to switch to.
pub fn apply(scene: &mut crate::scene::Scene, q: &[(String, String)]) -> (String, Option<u8>) {
    let get = |k: &str| q.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str());
    let num = |k: &str| get(k).and_then(|v| v.trim_start_matches("0x").parse::<u32>().ok());
    let Some(what) = get("do") else { return ("no command".into(), None) };
    match what {
        "weapon" => {
            let Some(sim) = scene.sim.as_mut() else { return ("no game".into(), None) };
            let Some(id) = num("id").filter(|&i| i < 0x72) else { return ("bad weapon id".into(), None) };
            let rounds = num("rounds").map(|r| r as i16).unwrap_or_else(|| sim.weapons.clip_size(id as usize).max(1));
            let wd = sim.weapons.clone();
            sim.drones.inventory.equip_weapon(&wd, id as usize, rounds, true);
            let carry = (wd.clip_size(id as usize).max(1) * 3).max(rounds);
            sim.drones.inventory.equip_ammo(&wd, id as usize, carry);
            (format!("gave weapon {id:#04x} ({})", weapon_name(id as u8)), Some(id as u8))
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
