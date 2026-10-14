//! The dev window: the dev menu in its OWN native window beside the game (the user, 2026-10-13: "the dev menu should
//! manifest outside of the game, but not in a browser"). A second `winit` window on the same event loop, with its own
//! `wgpu` surface on the game's device, drawn with the game's HUD font. The game keeps running while it's used (the
//! in-game F2 overlay paused the sim; this window doesn't), and every action goes through the same
//! `dev::apply_menu_action_to_scene` → `panel::apply` path as the overlay and the old HTTP panel.
//!
//! Layout and input are pure functions of (window size, scale, dev state, rows) so the headless tests can drive them;
//! the window itself can't open in a test.

use std::sync::Arc;

use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::dev::{self, Dev, MenuAction};
use crate::gpu::{HudVertex, HUD_SHADER};
use crate::scene::Scene;

pub const TITLE: &str = "Nightfire dev";
/// The game's HUD font texture (font 2) [hud.rs FONT_TEX].
const FONT_TEX: u32 = 0x0300_0003;

/// The pages shown as tabs: every `dev::PAGES` entry but the category index (the tabs ARE the index).
pub const TAB_PAGES: [usize; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
pub const TAB_LABELS: [&str; 9] = ["Weapons", "Gadgets", "Grenades", "Ammo/health", "NPCs", "Teleport", "Level", "Toggles", "Screenshot"];

/// Text height in logical pixels at 100% scaling.
const TEXT_PX: f32 = 16.0;

/// A rectangle in window pixels (x right, y down).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool { px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h }
    pub fn inside(&self, w: f32, h: f32) -> bool { self.x >= 0.0 && self.y >= 0.0 && self.x + self.w <= w + 0.5 && self.y + self.h <= h + 0.5 }
}

/// One frame's layout.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Logical → physical pixels (the window's scale factor).
    pub dpi: f32,
    pub row_h: f32,
    pub pad: f32,
    pub header: Rect,
    pub tabs: Vec<Rect>,
    pub filter: Rect,
    pub list: Rect,
    /// (row index into the page's rows, its rectangle) for the rows on screen.
    pub rows: Vec<(usize, Rect)>,
    /// How many rows fit in the list.
    pub visible: usize,
    pub scrollbar: Option<Rect>,
    pub desc: Rect,
    pub status: Rect,
    pub footer: Rect,
}

/// The layout for a `w` × `h` (physical px) window at scale `dpi`, a page with `n` rows scrolled to `scroll`.
pub fn layout(w: f32, h: f32, dpi: f32, n: usize, scroll: usize) -> Layout {
    let dpi = dpi.max(0.5);
    let pad = 8.0 * dpi;
    let row_h = (TEXT_PX + 8.0) * dpi;
    let header = Rect { x: 0.0, y: 0.0, w, h: row_h + pad };
    let tab_w = (165.0 * dpi).min(w * 0.35);
    let top = header.h + pad;
    let tabs = (0..TAB_PAGES.len()).map(|i| Rect { x: pad, y: top + i as f32 * (row_h + 4.0 * dpi), w: tab_w - pad, h: row_h }).collect();
    let x0 = tab_w + pad;
    let right_w = (w - x0 - pad).max(10.0);
    let footer = Rect { x: 0.0, y: h - row_h, w, h: row_h };
    let status = Rect { x: x0, y: footer.y - row_h - pad * 0.5, w: right_w, h: row_h };
    let desc = Rect { x: x0, y: status.y - 2.0 * row_h - pad * 0.5, w: right_w, h: 2.0 * row_h };
    let filter = Rect { x: x0, y: top, w: right_w, h: row_h };
    let list_y = filter.y + filter.h + pad * 0.5;
    let list = Rect { x: x0, y: list_y, w: right_w, h: (desc.y - pad * 0.5 - list_y).max(row_h) };
    let visible = ((list.h / row_h).floor() as usize).max(1);
    let scroll = scroll.min(n.saturating_sub(visible));
    let bar_w = 6.0 * dpi;
    let rows = (scroll..(scroll + visible).min(n)).map(|i| (i, Rect { x: list.x, y: list.y + (i - scroll) as f32 * row_h, w: list.w - bar_w - 2.0 * dpi, h: row_h })).collect();
    let scrollbar = (n > visible).then(|| {
        let th = (list.h * visible as f32 / n as f32).max(row_h * 0.5);
        let ty = list.y + (list.h - th) * scroll as f32 / (n - visible).max(1) as f32;
        Rect { x: list.x + list.w - bar_w, y: ty, w: bar_w, h: th }
    });
    Layout { dpi, row_h, pad, header, tabs, filter, list, rows, visible, scrollbar, desc, status, footer }
}

/// Keep the selection on screen: the scroll offset after moving to `sel`.
pub fn scroll_to(scroll: usize, sel: usize, visible: usize) -> usize {
    if sel < scroll { sel } else if sel >= scroll + visible { sel + 1 - visible } else { scroll }
}

/// What a dev-window event asks the App to do.
#[derive(Debug, Clone, Copy)]
pub enum Request { None, Redraw, Apply(MenuAction), Hide }

/// Interaction state (the dev menu's page/selection/filter live in `Dev`, shared with the F2 overlay).
#[derive(Default)]
pub struct Ui {
    pub scroll: usize,
    pub mouse: (f32, f32),
    pub hover: Option<usize>,
    last_click: Option<(std::time::Instant, usize)>,
}

/// Move to `page` (a `dev::PAGES` index): selection 0, filter cleared.
fn set_page(d: &mut Dev, ui: &mut Ui, page: usize) {
    if d.page != page { d.page = page; d.menu_sel = 0; d.filter.clear(); ui.scroll = 0; }
}

/// The tab index (into `TAB_PAGES`) of the dev state's page, or None on the index page.
pub fn current_tab(d: &Dev) -> Option<usize> { TAB_PAGES.iter().position(|&p| p == d.page) }

/// One input, already decoded from the window event (the tests drive these directly: a `WindowEvent` can't be built
/// outside winit).
#[derive(Clone, Debug, PartialEq)]
pub enum Input { Move(f32, f32), Leave, Wheel(f32), Click, Key(KeyCode, Option<String>), Close }

/// Decode a window event into an `Input`.
pub fn decode(event: &WindowEvent, row_h: f32) -> Option<Input> {
    Some(match event {
        WindowEvent::CloseRequested => Input::Close,
        WindowEvent::CursorMoved { position, .. } => Input::Move(position.x as f32, position.y as f32),
        WindowEvent::CursorLeft { .. } => Input::Leave,
        WindowEvent::MouseWheel { delta, .. } => Input::Wheel(match delta { MouseScrollDelta::LineDelta(_, y) => -*y * 3.0, MouseScrollDelta::PixelDelta(p) => -(p.y as f32) / row_h.max(1.0) }),
        WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => Input::Click,
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
            let PhysicalKey::Code(k) = event.physical_key else { return None };
            Input::Key(k, event.text.as_ref().map(|t| t.to_string()))
        }
        _ => return None,
    })
}

/// Apply one window event to the UI and the dev state.
pub fn handle(event: &WindowEvent, d: &mut Dev, ui: &mut Ui, rows: &[dev::MenuRow], size: (f32, f32), dpi: f32) -> Request {
    let row_h = layout(size.0, size.1, dpi, rows.len(), ui.scroll).row_h;
    match decode(event, row_h) { Some(i) => input(&i, d, ui, rows, size, dpi), None => Request::None }
}

/// Apply one input. Pure apart from the clock (double-click timing): `rows` are the current page's rows, `size`
/// the window's physical size, `dpi` its scale factor.
pub fn input(inp: &Input, d: &mut Dev, ui: &mut Ui, rows: &[dev::MenuRow], size: (f32, f32), dpi: f32) -> Request {
    if d.page == 0 { set_page(d, ui, TAB_PAGES[0]); }
    let n = rows.len();
    let lay = layout(size.0, size.1, dpi, n, ui.scroll);
    match inp {
        Input::Close => Request::Hide,
        Input::Move(x, y) => {
            ui.mouse = (*x, *y);
            let hv = lay.rows.iter().find(|(_, r)| r.contains(*x, *y)).map(|(i, _)| *i);
            if hv != ui.hover { ui.hover = hv; Request::Redraw } else { Request::None }
        }
        Input::Leave => { ui.hover = None; Request::Redraw }
        Input::Wheel(lines) => {
            let max = n.saturating_sub(lay.visible) as f32;
            ui.scroll = (ui.scroll as f32 + lines.round()).clamp(0.0, max) as usize;
            Request::Redraw
        }
        Input::Click => {
            let (mx, my) = ui.mouse;
            if let Some(t) = lay.tabs.iter().position(|r| r.contains(mx, my)) { set_page(d, ui, TAB_PAGES[t]); return Request::Redraw; }
            if let Some(&(i, _)) = lay.rows.iter().find(|(_, r)| r.contains(mx, my)) {
                let now = std::time::Instant::now();
                let double = ui.last_click.is_some_and(|(t, j)| j == i && now.duration_since(t).as_millis() < 450);
                d.menu_sel = i;
                ui.last_click = Some((now, i));
                if double { ui.last_click = None; return activate(d, ui, rows); }
                return Request::Redraw;
            }
            Request::None
        }
        Input::Key(k, text) => {
            let tab = current_tab(d).unwrap_or(0);
            let step = |d: &mut Dev, ui: &mut Ui, delta: isize| {
                if n == 0 { return; }
                d.menu_sel = (d.menu_sel as isize + delta).clamp(0, n as isize - 1) as usize;
                ui.scroll = scroll_to(ui.scroll, d.menu_sel, lay.visible);
            };
            match k {
                KeyCode::ArrowUp => step(d, ui, -1),
                KeyCode::ArrowDown => step(d, ui, 1),
                KeyCode::PageUp => step(d, ui, -(lay.visible as isize)),
                KeyCode::PageDown => step(d, ui, lay.visible as isize),
                KeyCode::Home => step(d, ui, -(n as isize)),
                KeyCode::End => step(d, ui, n as isize),
                KeyCode::ArrowLeft => set_page(d, ui, TAB_PAGES[(tab + TAB_PAGES.len() - 1) % TAB_PAGES.len()]),
                KeyCode::ArrowRight | KeyCode::Tab => set_page(d, ui, TAB_PAGES[(tab + 1) % TAB_PAGES.len()]),
                KeyCode::Enter | KeyCode::NumpadEnter => return activate(d, ui, rows),
                KeyCode::Backspace => { d.filter.pop(); d.menu_sel = 0; ui.scroll = 0; }
                KeyCode::Escape => { d.filter.clear(); d.menu_sel = 0; ui.scroll = 0; }
                _ => {
                    // Type to filter: printable ASCII (the game font's range).
                    let Some(t) = text.as_ref() else { return Request::None };
                    let mut typed = false;
                    for c in t.chars().filter(|c| c.is_ascii_graphic() || *c == ' ') { if d.filter.len() < 32 { d.filter.push(c.to_ascii_lowercase()); typed = true; } }
                    if !typed { return Request::None; }
                    d.menu_sel = 0; ui.scroll = 0;
                }
            }
            Request::Redraw
        }
    }
}

/// Enter / double-click on the selected row. Page switches stay inside the window (the tabs are the index).
fn activate(d: &mut Dev, ui: &mut Ui, rows: &[dev::MenuRow]) -> Request {
    let Some((_, _, a)) = rows.get(d.menu_sel) else { return Request::None };
    match *a {
        MenuAction::Page(p) => { set_page(d, ui, p.max(1)); Request::Redraw }
        MenuAction::Back | MenuAction::Close => Request::Redraw,
        a => Request::Apply(a),
    }
}

/// Word-wrap `s` into lines no wider than `max_w` (measured with `width`).
pub fn wrap(s: &str, max_w: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if width(&cand) <= max_w || cur.is_empty() { cur = cand; } else { lines.push(std::mem::replace(&mut cur, word.to_string())); }
    }
    if !cur.is_empty() { lines.push(cur); }
    lines
}

/// The frame builder: coloured rectangles and game-font text in window pixels → NDC triangles.
pub struct Painter<'a> {
    pub w: f32,
    pub h: f32,
    font: Option<(&'a nf_formats::hud::Font, f32, f32, f32)>, // (font, texture w, texture h, text scale)
    top: f32,
    pub v: Vec<HudVertex>,
    /// (bind group 0 = white / 1 = font, first vertex, count)
    pub runs: Vec<(usize, u32, u32)>,
}

impl<'a> Painter<'a> {
    pub fn new(scene: &'a Scene, w: f32, h: f32, dpi: f32) -> Painter<'a> {
        let font = scene.hud.as_ref().map(|t| &t.font);
        let tex = scene.hud_textures.iter().find(|(k, _)| *k == FONT_TEX).map(|(_, t)| (t.width as f32, t.height as f32));
        let top = font.and_then(|f| f.glyphs.first()).map_or(16.0, |g| (g.h + g.yoff) as f32).max(1.0);
        let font = match (font, tex) { (Some(f), Some((tw, th))) => Some((f, tw, th, TEXT_PX * dpi / top)), _ => None };
        Painter { w, h, font, top, v: Vec::new(), runs: Vec::new() }
    }

    fn push(&mut self, bg: usize, quad: [HudVertex; 6]) {
        let start = self.v.len() as u32;
        self.v.extend_from_slice(&quad);
        match self.runs.last_mut() { Some(r) if r.0 == bg && r.1 + r.2 == start => r.2 += 6, _ => self.runs.push((bg, start, 6)) }
    }

    fn ndc(&self, x: f32, y: f32) -> [f32; 2] { [x / self.w * 2.0 - 1.0, 1.0 - y / self.h * 2.0] }

    pub fn rect(&mut self, r: Rect, c: [f32; 4]) {
        let (a, b) = (self.ndc(r.x, r.y), self.ndc(r.x + r.w, r.y + r.h));
        let p = |q: [f32; 2]| HudVertex { p: q, uv: [-1.0, -1.0], c };
        self.push(0, [p(a), p([b[0], a[1]]), p(b), p(a), p(b), p([a[0], b[1]])]);
    }

    /// The text's width in pixels.
    pub fn text_w(&self, s: &str) -> f32 {
        self.font.map_or(s.len() as f32 * 9.0, |(f, _, _, k)| f.layout(&dev::fit(s, 9999), 0.0, 0.0).1 * k)
    }

    /// Text with its top-left at (x, y), clipped to `max_w` pixels (whole glyphs).
    pub fn text(&mut self, s: &str, x: f32, y: f32, max_w: f32, c: [f32; 4]) {
        let Some((font, tw, th, k)) = self.font else { return };
        let (placed, _) = font.layout(&dev::fit(s, 9999), 0.0, self.top);
        for p in placed {
            let g = &font.glyphs[p.glyph];
            let (gx, gy, gw, gh) = (x + p.x * k, y + p.y * k, g.w as f32 * k, g.h as f32 * k);
            if gx + gw > x + max_w { break; }
            let (u0, v0, u1, v1) = (g.u as f32 / tw, g.v as f32 / th, (g.u + g.w) as f32 / tw, (g.v + g.h) as f32 / th);
            let (a, b) = (self.ndc(gx, gy), self.ndc(gx + gw, gy + gh));
            let q = |pp: [f32; 2], uv: [f32; 2]| HudVertex { p: pp, uv, c };
            self.push(1, [q(a, [u0, v0]), q([b[0], a[1]], [u1, v0]), q(b, [u1, v1]), q(a, [u0, v0]), q(b, [u1, v1]), q([a[0], b[1]], [u0, v1])]);
        }
    }
}

const BG: [f32; 4] = [0.07, 0.07, 0.09, 1.0];
const PANEL: [f32; 4] = [0.12, 0.12, 0.15, 1.0];
const SEL: [f32; 4] = [0.16, 0.30, 0.16, 1.0];
const HOVER: [f32; 4] = [0.18, 0.18, 0.22, 1.0];
const ACCENT: [f32; 4] = [0.85, 0.75, 0.25, 1.0];
const TEXT: [f32; 4] = [0.85, 0.85, 0.85, 1.0];
const DIM: [f32; 4] = [0.55, 0.55, 0.58, 1.0];
const GOOD: [f32; 4] = [0.6, 0.85, 0.6, 1.0];

/// Draw the whole window for one frame.
pub fn paint(scene: &Scene, d: &Dev, ui: &Ui, rows: &[dev::MenuRow], w: f32, h: f32, dpi: f32) -> (Vec<HudVertex>, Vec<(usize, u32, u32)>) {
    let lay = layout(w, h, dpi, rows.len(), ui.scroll);
    let mut p = Painter::new(scene, w, h, dpi);
    let pad = lay.pad;
    let ty = |r: &Rect| r.y + 4.0 * dpi;
    // Header: the level and Bond's spot.
    p.rect(lay.header, PANEL);
    let level = scene.sim.as_ref().map_or(String::from("no level"), |s| format!("level {:08x}", s.level));
    let spot = scene.sim.as_ref().map_or(String::new(), |s| format!("   Bond {:.0}, {:.0}, {:.0}   health {:.0}", s.bond.pos[0], s.bond.pos[1], s.bond.pos[2], s.drones.health.health));
    p.text(&format!("{level}{spot}"), pad, ty(&lay.header), w - 2.0 * pad, ACCENT);
    // Tabs.
    let cur = current_tab(d);
    for (i, r) in lay.tabs.iter().enumerate() {
        let on = cur == Some(i);
        p.rect(*r, if on { SEL } else { PANEL });
        if on { p.rect(Rect { w: 3.0 * dpi, ..*r }, ACCENT); }
        p.text(TAB_LABELS[i], r.x + 8.0 * dpi, ty(r), r.w - 10.0 * dpi, if on { ACCENT } else { TEXT });
    }
    // Filter.
    p.rect(lay.filter, PANEL);
    let ftxt = if d.filter.is_empty() { "type to filter".to_string() } else { format!("filter: {}_", d.filter) };
    p.text(&ftxt, lay.filter.x + 6.0 * dpi, ty(&lay.filter), lay.filter.w - 12.0 * dpi, if d.filter.is_empty() { DIM } else { GOOD });
    // List.
    p.rect(lay.list, PANEL);
    if rows.is_empty() { p.text("(nothing matches)", lay.list.x + 6.0 * dpi, lay.list.y + 4.0 * dpi, lay.list.w, DIM); }
    for &(i, r) in &lay.rows {
        let sel = i == d.menu_sel;
        if sel { p.rect(r, SEL); p.rect(Rect { w: 3.0 * dpi, ..r }, ACCENT); } else if ui.hover == Some(i) { p.rect(r, HOVER); }
        p.text(&rows[i].0, r.x + 8.0 * dpi, ty(&r), r.w - 12.0 * dpi, if sel { ACCENT } else { TEXT });
    }
    if let Some(sb) = lay.scrollbar { p.rect(sb, DIM); }
    // The selected row's description, wrapped.
    if let Some((_, desc, _)) = rows.get(d.menu_sel) {
        let lines = wrap(desc, lay.desc.w - 12.0 * dpi, |s| p.text_w(s));
        for (k, l) in lines.iter().take(2).enumerate() { p.text(l, lay.desc.x + 6.0 * dpi, lay.desc.y + k as f32 * lay.row_h + 4.0 * dpi, lay.desc.w - 12.0 * dpi, DIM); }
    }
    // Status: the last action's result.
    if let Some(m) = d.result_msg.as_ref() {
        p.rect(lay.status, PANEL);
        p.text(m, lay.status.x + 6.0 * dpi, ty(&lay.status), lay.status.w - 12.0 * dpi, GOOD);
    }
    // Footer: the keys.
    p.rect(lay.footer, PANEL);
    p.text("Enter/double-click: apply   Left/Right: page   type: filter", pad, ty(&lay.footer), w - 2.0 * pad, DIM);
    (p.v, p.runs)
}

/// The window and its GPU resources.
pub struct DevWindow {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bgs: Vec<wgpu::BindGroup>,
    vbuf: wgpu::Buffer,
    vcap: usize,
    pub ui: Ui,
    last_tick: std::time::Instant,
}

impl DevWindow {
    /// Open the window beside `game` (to its right when the monitor has room), on the game's device.
    pub fn new(el: &ActiveEventLoop, game: &Window, instance: &wgpu::Instance, adapter: &wgpu::Adapter, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) -> Option<DevWindow> {
        let mut attrs = Window::default_attributes().with_title(TITLE).with_inner_size(LogicalSize::new(640.0, 760.0)).with_min_inner_size(LogicalSize::new(420.0, 360.0));
        if let (Ok(pos), Some(mon)) = (game.outer_position(), game.current_monitor()) {
            let gs = game.outer_size();
            let (mx, mw) = (mon.position().x, mon.size().width as i32);
            let want = (640.0 * mon.scale_factor()) as i32;
            let right = pos.x + gs.width as i32 + 8;
            let x = if right + want <= mx + mw { right } else { (mx + mw - want).max(mx) };
            attrs = attrs.with_position(PhysicalPosition::new(x, pos.y));
        }
        let window = Arc::new(el.create_window(attrs).map_err(|e| eprintln!("[dev window] {e}")).ok()?);
        let surface = instance.create_surface(window.clone()).map_err(|e| eprintln!("[dev window] surface: {e}")).ok()?;
        let size = window.inner_size();
        let mut config = surface.get_default_config(adapter, size.width.max(1), size.height.max(1))?;
        config.format = config.format.remove_srgb_suffix();
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(device, &config);
        let tbgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dev window"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dev window"), source: wgpu::ShaderSource::Wgsl(HUD_SHADER.into()) });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&tbgl)], immediate_size: 0 });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dev window"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs"), compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<HudVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4] })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: config.format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            multiview_mask: None,
            cache: None,
        });
        // Linear filtering: the font is scaled up to the window's text size.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let make_tex = |w: u32, h: u32, rgba: &[u8]| {
            let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let t = device.create_texture(&wgpu::TextureDescriptor { label: None, size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
            queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) }, size);
            let view = t.create_view(&Default::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &tbgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }] })
        };
        let white = make_tex(1, 1, &[255, 255, 255, 255]);
        let font = scene.hud_textures.iter().find(|(k, _)| *k == FONT_TEX).map(|(_, t)| make_tex(t.width, t.height, &t.rgba)).unwrap_or_else(|| make_tex(1, 1, &[255, 255, 255, 255]));
        let vcap = 16384;
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("dev window"), size: (vcap * std::mem::size_of::<HudVertex>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        window.request_redraw();
        Some(DevWindow { window, surface, config, pipeline, bgs: vec![white, font], vbuf, vcap, ui: Ui::default(), last_tick: std::time::Instant::now() })
    }

    pub fn id(&self) -> winit::window::WindowId { self.window.id() }

    /// Called every game frame: refresh the window ~6 times a second (Bond's spot, health, the status line).
    pub fn tick(&mut self) {
        if self.last_tick.elapsed().as_millis() >= 160 { self.last_tick = std::time::Instant::now(); self.window.request_redraw(); }
    }

    /// Show and raise the window (F2 in the game window).
    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.set_minimized(false);
        self.window.focus_window();
        self.window.request_redraw();
    }

    pub fn resize(&mut self, device: &wgpu::Device, w: u32, h: u32) {
        if w == 0 || h == 0 { return; }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(device, &self.config);
        self.window.request_redraw();
    }

    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene, d: &Dev) {
        if self.config.width == 0 || self.config.height == 0 || !self.window.is_visible().unwrap_or(true) { return; }
        let rows = dev::menu_items(d, Some(scene));
        let dpi = self.window.scale_factor() as f32;
        let (v, runs) = paint(scene, d, &self.ui, &rows, self.config.width as f32, self.config.height as f32, dpi);
        if v.len() > self.vcap {
            self.vcap = v.len().next_power_of_two();
            self.vbuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("dev window"), size: (self.vcap * std::mem::size_of::<HudVertex>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        }
        if !v.is_empty() { queue.write_buffer(&self.vbuf, 0, bytemuck::cast_slice(&v)); }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => { self.surface.configure(device, &self.config); self.window.request_redraw(); return; }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dev window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: BG[0] as f64, g: BG[1] as f64, b: BG[2] as f64, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            for (bg, first, count) in runs {
                pass.set_bind_group(0, &self.bgs[bg.min(self.bgs.len() - 1)], &[]);
                pass.draw(first..first + count, 0..1);
            }
        }
        queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        queue.present(frame);
    }
}
