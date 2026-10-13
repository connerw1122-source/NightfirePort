//! GPU side of the viewer (wgpu 30): a pipeline per draw state, a bind group per texture, indexed draws per
//! material in the game's pass order (docs/systems/render.md).
//!
//! Shading follows the GS "modulate" texture function: colour = texture × vertex colour, vertex RGBA on the PS2 scale
//! (0x80 = 1.0). Per material: the GS blend (ALPHA_1), depth write (ZBUF_1), alpha test (TEST_1) of its leaf, and the
//! fog: the game's screen pass (`psiDrawFogPass`, Z bits 8–15 as destination alpha, a fog-coloured sprite behind
//! Z 0x7FFF) computed per pixel from the view depth — zb = N1 + (F1 − N1)·n·(f/d − 1)/(f − n); beyond zb < 32767,
//! colour = fog + (colour − fog)·⌊zb/256⌋/128; the alpha list fades by min(⌊zb/128⌋, 255)/255 instead. Every output
//! is scaled by the display copy's gain (`SqueezeColourIn` 0x91 / 0x80, `CreateSqueezeFrame`) — linear, so the same
//! as scaling the finished frame. No back-face culling — the GS has no winding.

use crate::scene::{Draw, Scene};
use wgpu::util::DeviceExt;

pub const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One uniform slot: vp (64 B) + fog/fogk/lp0/lp1/lc0/lc1 (6 × 16 B) = 160 B, padded to the
/// 256-byte dynamic-offset alignment (fix-lighting).
const UNIFORM_SLOT: usize = 256;

/// `SqueezeColourIn` / 0x80 [L 0x91].
pub const GAIN: f32 = 145.0 / 128.0;

const SHADER: &str = r#"
override AREF: f32 = 0.0;
override AGREATER: f32 = 0.0;
override FOG: f32 = 0.0;
override FAR: f32 = 0.0;
override NOTEX: f32 = 0.0;  // debug: 1.0 = ignore the texture sample (treat as solid white) to isolate vertex colour
override NOFOG: f32 = 0.0;   // debug: 1.0 = skip the fog pass entirely
// fog: rgb + enabled; fogk: N1, (F1 − N1)·n/(f − n), f, gain.
// lp0/lp1: the frame's two closest lights' positions (psiLight_SetLights 0x10e588's TLight, w = 1);
// lc0/lc1: their TColor rows (rgb·intensity in the game's byte scale, w = 1/r²), fix-lighting.
struct U { vp: mat4x4<f32>, fog: vec4<f32>, fogk: vec4<f32>, lp0: vec4<f32>, lp1: vec4<f32>, lc0: vec4<f32>, lc1: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
// G-043 GPU skinning: the frame's matrix palette and colour table (`Scene::skin_palette`).
@group(0) @binding(1) var<storage, read> pal: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> cols: array<vec4<f32>>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VO { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32>, @location(2) d: f32 };
@vertex fn vs(@location(0) p0: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) c0: vec4<f32>,
              @location(3) sa: vec2<u32>, @location(4) sw: f32, @location(5) sc: u32, @location(6) n0: vec4<f32>) -> VO {
    var o: VO;
    var p = p0;
    var c = c0;
    // Skinned (a != 0xFFFFFFFF): w*(P[a]*p) + (1-w)*(P[b]*p), and the object's TAmbient multiplier.
    if (sa.x != 0xffffffffu) {
        let q = vec4<f32>(p0, 1.0);
        p = sw * (pal[sa.x] * q).xyz + (1.0 - sw) * (pal[sa.y] * q).xyz;
        c = vec4<f32>(c0.rgb * cols[sc].rgb, c0.a);
    }
    // The point lights (`Light_Create` + `Lights_CalcClosestLights` 0x1cb2e0 + `psiLight_SetLights`
    // 0x10e588; fix-lighting). The VU1 microcode's formula (.vutext 0x241c80, the point-light
    // program ~832-960), in world space: L = lightpos − pos, d² = |L|², 1/d² via DIV (VF0.w = 1),
    // atten = max(0, 1/d² − 1/r²), diffuse = max(0, atten · dot(N, L)) with the RAW L, and the sum
    // adds TColor.rgb · diffuse on the tinted vertex colour (the ambient row is the object's TWEAK,
    // already folded into the vertex colour by the port's fix-render path). fix-lighting2: the
    // vertex colour arrives as Unorm8x4 = byte/255 and the game's add is in BYTES, so the add is
    // /255 (the port's /128 over-added by exactly 255/128 — the saturation the merge flagged), and
    // the program clamps the summed colour to byte 255 with MINIi (I = 255.0 loaded by the LOI at
    // I-mem 900/936, applied at 919/954) — min(·, 1.0) here. No normal (sprites, snow, the radar,
    // the flash itself): unlit.
    if (FAR < 0.5) {
        var n = n0.xyz;
        if (sa.x != 0xffffffffu) { n = sw * (pal[sa.x] * vec4<f32>(n0.xyz, 0.0)).xyz + (1.0 - sw) * (pal[sa.y] * vec4<f32>(n0.xyz, 0.0)).xyz; }
        let n2 = dot(n, n);
        if (n2 > 1e-6) {
            for (var i: u32 = 0u; i < 2u; i = i + 1u) {
                let lp = select(u.lp0, u.lp1, i == 1u);
                let lc = select(u.lc0, u.lc1, i == 1u);
                if (lc.w <= 0.0 || (lc.x + lc.y + lc.z) <= 0.0) { continue; }
                let d = lp.xyz - p;
                let d2 = dot(d, d);
                let inv_d2 = 1.0 / d2;
                let atten = max(0.0, inv_d2 - lc.w);
                let term = max(0.0, atten * dot(normalize(n), d));
                c = vec4<f32>(min(c.rgb + lc.rgb * term / 255.0, vec3<f32>(1.0)), c.a);
            }
        }
    }
    o.p = u.vp * vec4<f32>(p, 1.0);
    o.d = o.p.w;
    if (FAR > 0.5) { o.p.z = o.p.w; }
    o.uv = uv;
    o.c = c * (255.0 / 128.0);
    return o;
}
@fragment fn fs(i: VO) -> @location(0) vec4<f32> {
    let s = textureSample(tex, samp, i.uv);
    let t = select(s, vec4<f32>(1.0, 1.0, 1.0, 1.0), NOTEX > 0.5);
    let o = t * i.c;
    if (AGREATER > 0.5) { if (o.a <= AREF) { discard; } } else if (o.a < AREF) { discard; }
    var rgb = min(o.rgb, vec3<f32>(1.0));
    var a = clamp(o.a, 0.0, 1.0);
    if (FOG > 0.5 && u.fog.w > 0.5 && NOFOG < 0.5) {
        let zb = u.fogk.x + u.fogk.y * (u.fogk.z / max(i.d, 0.001) - 1.0);
        if (FOG < 1.5) {
            // The GS's fog factor is fixed-point and wraps; clamp to [0,1] so a small zb can't collapse the frame to
            // the fog colour (the "ghostly white" symptom on levels with a bright fog, e.g. 0700000c) [M render.md].
            let f = clamp(floor(zb / 256.0) / 128.0, 0.0, 1.0);
            if (zb < 32767.0) { rgb = u.fog.rgb + (rgb - u.fog.rgb) * f; }
        } else {
            a = a * min(floor(zb / 128.0), 255.0) / 255.0;
        }
    }
    return vec4<f32>(min(rgb * u.fogk.w, vec3<f32>(1.0)), a);
}
"#;

/// The overlay (crosshair, HUD): screen-space triangles in normalised device coordinates, alpha-blended, no depth.
const HUD_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
struct HO { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32> };
@vertex fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) c: vec4<f32>) -> HO {
    var o: HO;
    o.p = vec4<f32>(p, 0.0, 1.0);
    o.uv = uv;
    o.c = c;
    return o;
}
// uv < 0: a plain colour; else the HUD texture modulated by the colour on the GS scale (1.0 = 0x80, so up to 2×).
@fragment fn fs(i: HO) -> @location(0) vec4<f32> {
    if (i.uv.x < 0.0) { return vec4<f32>(min(i.c.rgb * 1.1328125, vec3<f32>(1.0)), i.c.a); }
    let t = textureSample(tex, samp, i.uv);
    return vec4<f32>(min(t.rgb * i.c.rgb * 1.1328125, vec3<f32>(1.0)), min(t.a * i.c.a, 1.0));
}
"#;

/// One overlay vertex: NDC position (x right, y up) and RGBA (0–1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HudVertex { pub p: [f32; 2], pub uv: [f32; 2], pub c: [f32; 4] }

/// One frame's overlay: vertices and the runs drawn in order — (texture index into `Scene::hud_textures` + 1, 0 =
/// none; blend mode; first vertex; count). Blend: 0 = normal, 1 = additive, 2 = subtractive.
#[derive(Default)]
pub struct HudFrame { pub v: Vec<HudVertex>, pub runs: Vec<(usize, u8, u32, u32)> }

impl HudFrame {
    /// Append another frame's vertices and runs (the dev overlay, WORKLOG 0081).
    pub fn append(&mut self, other: &HudFrame) {
        let offset = self.v.len() as u32;
        self.v.extend_from_slice(&other.v);
        for &(tex, add, first, count) in &other.runs {
            self.runs.push((tex, add, first + offset, count));
        }
    }
}

/// Most snow vertices per frame (4096 flakes × 6).
const SNOW_MAX: usize = 4096 * 6;

/// Most overlay vertices per frame.
const HUD_MAX: usize = 16384;

/// Extract the 6 frustum planes from a column-major `view_proj` (Gribb-Hartmann). Each plane is `(a, b, c, d)` with
/// `a·x + b·y + c·z + d ≥ 0` inside the frustum, normalised so a sphere of radius `r` centred at `p` is fully outside
/// when `a·p.x + b·p.y + c·p.z + d < −r`.
///
/// `view_proj` is the column-major `Mat4` produced by `glam` and stored as `[[col0], [col1], [col2], [col3]]` —
/// `vp[i]` is the i-th column. The i-th row (what Gribb-Hartmann combines with the w-row 3) is
/// `(vp[0][i], vp[1][i], vp[2][i], vp[3][i])`. With the right-handed Vulkan/wgpu projection used here
/// (`Mat4::perspective_rh`, depth 0..1), the near plane is `row3 + row2` (the OpenGL convention, slightly conservative
/// — culls a 0.05-unit strip behind the camera but never a visible object) and the far plane is `row3 − row2`.
pub fn extract_frustum(vp: [[f32; 4]; 4]) -> [[f32; 4]; 6] {
    let row = |i: usize| [vp[0][i], vp[1][i], vp[2][i], vp[3][i]];
    let r3 = row(3);
    // 6 planes — (row index, sign): plane = r3 + sign·row(i). left/right, bottom/top, near/far.
    let combos: [(usize, f32); 6] = [(0, 1.0), (0, -1.0), (1, 1.0), (1, -1.0), (2, 1.0), (2, -1.0)];
    let mut planes = [[0.0f32; 4]; 6];
    for (k, (i, s)) in combos.iter().enumerate() {
        let ri = row(*i);
        let p = [r3[0] + s * ri[0], r3[1] + s * ri[1], r3[2] + s * ri[2], r3[3] + s * ri[3]];
        let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
        planes[k] = [p[0] / n, p[1] / n, p[2] / n, p[3] / n];
    }
    planes
}

/// Test if a sphere is fully outside any of the 6 frustum planes (Gribb-Hartmann point-plane test extended by radius).
/// `Vision_InView`'s shape as the port reads it: a box (a cel's `cel+0x40..0x58` min/max bbox) is
/// outside the view when all eight corners lie outside the SAME frustum plane (the standard
/// p-vertex test). `View_AddCels` (0x1e6ba0) frustum/bbox-tests each cel per viewer [M].
pub fn box_outside(planes: &[[f32; 4]; 6], min: [f32; 3], max: [f32; 3]) -> bool {
    for pl in planes {
        // The p-vertex: the corner most along the plane normal; if even that is outside, the whole box is.
        let px = if pl[0] >= 0.0 { max[0] } else { min[0] };
        let py = if pl[1] >= 0.0 { max[1] } else { min[1] };
        let pz = if pl[2] >= 0.0 { max[2] } else { min[2] };
        if pl[0] * px + pl[1] * py + pl[2] * pz + pl[3] < 0.0 { return true; }
    }
    false
}

pub fn sphere_outside(planes: &[[f32; 4]; 6], centre: [f32; 3], radius: f32) -> bool {
    planes.iter().any(|p| p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2] + p[3] < -radius)
}

/// An alpha-shaped texture: more than a quarter of its texels are below half its peak alpha (the shadow decals'
/// textures keep their shape in alpha; wall textures are uniformly opaque). Drawn after the opaque list-0 draws.
pub fn alpha_shaped(rgba: &[u8]) -> bool {
    let peak = rgba.chunks_exact(4).map(|p| p[3]).max().unwrap_or(0) as u32;
    let n = rgba.len() / 4;
    peak > 0 && rgba.chunks_exact(4).filter(|p| (p[3] as u32) * 2 < peak).count() * 4 > n
}

/// One indexed draw: its layer, pass, pipeline, texture bind group, index range and the object's world centre + radius.
#[derive(Clone, Copy)]
struct DrawCall { layer: crate::scene::Layer, pass: u8, pipeline: usize, tex: usize, first: u32, count: u32, centre: [f32; 3], radius: f32, cel: Option<u16>, chain: u32, node: u32, lflags: u16 }

pub struct Renderer {
    hud_pipeline: wgpu::RenderPipeline,
    hud_add_pipeline: wgpu::RenderPipeline,
    hud_sub_pipeline: wgpu::RenderPipeline,
    hud_vbuf: wgpu::Buffer,
    hud_bgs: Vec<wgpu::BindGroup>,
    pipelines: Vec<(Draw, wgpu::RenderPipeline)>,
    /// Snow: the particle pipeline (additive, depth test, no write), a per-frame vertex buffer and the flake texture.
    snow_pipeline: wgpu::RenderPipeline,
    snow_vbuf: wgpu::Buffer,
    snow_bg: wgpu::BindGroup,
    /// The level's fog (`psiPreGameRun`): colour 0-255, `Ps2Far1`.
    fog: Option<([u8; 3], f32)>,
    /// Debug overrides (Bug 1): `notex` ignores texture samples (solid white), `nofog` skips the fog pass.
    notex: bool,
    nofog: bool,
    uniform: wgpu::Buffer,
    uniform_bg: wgpu::BindGroup,
    /// View-projection without the camera translation (sky objects that follow the viewer).
    uniform_rot: wgpu::Buffer,
    uniform_rot_bg: wgpu::BindGroup,
    vbuf: wgpu::Buffer,
    skin_buf: wgpu::Buffer,
    snow_skin: wgpu::Buffer,
    pal_buf: wgpu::Buffer,
    col_buf: wgpu::Buffer,
    pub skin_layout: crate::scene::SkinLayout,
    /// The frame's lights, resolved (kind-2 positions against their parts). Per draw the renderer
    /// picks the ≤2 closest within the draw's bounding sphere (`Lights_CalcClosestLights`
    /// 0x1cb2e0 / `psiLight_SetLights` 0x10e588) into the draw's uniform slot.
    pub lights: nf_game::light::ResolvedLights,
    ibuf: wgpu::Buffer,
    tex_bgs: Vec<wgpu::BindGroup>,
    /// Per texture slot: its first bind group, frame count and rate (animated textures).
    tex_frames: Vec<(usize, u32, u32)>,
    /// Per material: layer, first index, index count (sorted by layer: sky pass first).
    draws: Vec<DrawCall>,
    /// Per-frame sort buffer reused across frames (A-11e): the per-frame draw order — `(pass, sort key, draw index)`
    /// — is `clear`ed + refilled every render instead of allocating ~64 KB each frame.
    order: Vec<(u8, f32, usize)>,
    /// The room cels' frustum-test spheres (`Scene::cel_sphere`), built once: `View_AddCels`
    /// (0x1e6ba0) frustum-tests each cel's bbox per viewer; fix-render3 skips a static World draw
    /// whose cel fails the same test. `NF_NOCELCULL` disables.
    cel_boxes: Vec<([f32; 3], [f32; 3])>,
    /// The runtime cel graph + portals (fix-perf-level): the walk's per-full-cel set gates static
    /// World draws (the `View_AddCels` GfxList set — only portal-visited cels are added). Graphs
    /// without portals fall back to the fix-render3 box test.
    cel_graph: crate::scene::CelGraph,
    /// The viewer's persistent cel (`viewer+4`, `Camera_CheckLocation` 0x1bae90) — carried across
    /// frames, updated by the move raycast / bbox keep / `build_FindCel` chain. fix-portal.
    pub viewer_cel: crate::scene::ViewerCel,
    /// This frame's room-level visibility mask, computed once per frame by `cel_visibility` (the
    /// skin palette's object cull and `render`'s World cull share it — one walk per frame).
    pub last_vis: Option<Vec<bool>>,
    /// This frame's CELL-level game mask (`View_AddCels` 0x1e6ba0: the VisCelList plus the `+0x18`
    /// chain pass) — `None` when the walk cannot run (the room box fallback gates everything).
    /// fix-portal2.
    pub last_cell_vis: Option<Vec<bool>>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, scene: &Scene, notex: bool, nofog: bool) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nf-view"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let ubgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                // fix-lighting: one 256-byte slot per draw (dynamic offset) — the game uploads a
                // per-object light pair (`Lights_CalcClosestLights` + `psiLight_SetLights` per object/cel).
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: None },
                count: None,
            },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let tbgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&ubgl), Some(&tbgl)], immediate_size: 0 });
        // World: depth-tested. Sky pass: painted first in slot order without depth (`View_DrawSky` before the world).
        // One pipeline per draw state: blend (ALPHA_1), depth write (ZBUF_1), alpha test (TEST_1) and fog as shader
        // overrides; the pre-world sky pass ignores depth, the post-fog sky is pinned to the far plane (drawn only where
        // nothing else is).
        let make_pipeline = |d: Draw| {
            let (compare, write) = match d.pass { 0 => (wgpu::CompareFunction::Always, false), 4 => (wgpu::CompareFunction::LessEqual, false), _ => (wgpu::CompareFunction::LessEqual, d.zwrite) };
            let alpha_keep = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add };
            let blend = match d.blend {
                1 => wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add }, alpha: alpha_keep },
                2 => wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::ReverseSubtract }, alpha: alpha_keep },
                // (Cs−Cd)·FIX0 + Cd (07000041's mod batches): the framebuffer comes back unchanged, but the
                // depth test/write and the alpha test still run as configured (the GS evaluates the whole
                // equation to D = Cd) — scene.rs `Draw::from_gs` blend 3.
                3 => wgpu::BlendState { color: alpha_keep, alpha: alpha_keep },
                // (Cs−0)·Ad + Cd (07000041's env-map overlays): add the source only where the base batch
                // wrote destination alpha; the alpha channel itself comes back unchanged (D = Cd).
                4 => wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::DstAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add }, alpha: alpha_keep },
                _ => wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add }, alpha: alpha_keep },
            };
            // As >= 1 (TEST 0x5001B) -> a >= 1/128; As > 0x7F (0x507FD) -> a > 127/128.
            let (aref, greater) = match d.atest { 1 => (1.0 / 128.0, 0.0), 2 => (127.0 / 128.0, 1.0), _ => (0.0, 0.0) };
            let constants = [("AREF", aref), ("AGREATER", greater), ("FOG", d.fog as f64), ("FAR", if d.pass == 4 { 1.0 } else { 0.0 }),
                ("NOTEX", if notex { 1.0 } else { 0.0 }), ("NOFOG", if nofog { 1.0 } else { 0.0 })];
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("nf-view"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader, entry_point: Some("vs"), compilation_options: wgpu::PipelineCompilationOptions { constants: &constants, ..Default::default() },
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<crate::scene::Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4, 6 => Snorm8x4],
                    }), Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<crate::scene::SkinAttr>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![3 => Uint32x2, 4 => Float32, 5 => Uint32],
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH, depth_write_enabled: Some(write), depth_compare: Some(compare),
                    stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader, entry_point: Some("fs"), compilation_options: wgpu::PipelineCompilationOptions { constants: &constants, ..Default::default() },
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nf-view hud"), source: wgpu::ShaderSource::Wgsl(HUD_SHADER.into()) });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&tbgl)], immediate_size: 0 });
        let make_hud = |blend: wgpu::BlendState| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nf-view hud"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader, entry_point: Some("vs"), compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<HudVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(), bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader, entry_point: Some("fs"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // GS ALPHA 0x44 (normal), 0x48 (Cs·As + Cd: sprite flag 0x100), and 0x42 (Cd - Cs·As: sprite flag 0x200, subtractive).
        let hud_pipeline = make_hud(wgpu::BlendState::ALPHA_BLENDING);
        let hud_add_pipeline = make_hud(wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        });
        let hud_sub_pipeline = make_hud(wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::ReverseSubtract },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        });
        let hud_vbuf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (HUD_MAX * std::mem::size_of::<HudVertex>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut pipelines: Vec<(Draw, wgpu::RenderPipeline)> = Vec::new();
        for m in &scene.materials { if !pipelines.iter().any(|(d, _)| *d == m.draw) { pipelines.push((m.draw, make_pipeline(m.draw))); } }
        // G-043: the skinning layout, palette and colour buffers (sized once per scene).
        let (skin_layout, char_attrs) = scene.skin_layout();
        let pal_buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("palette"), size: (skin_layout.n_mats.max(1) as u64) * 64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let col_buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("colours"), size: (skin_layout.col_one as u64 + 1) * 16, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let slots = (scene.materials.len() + 2) as u64;
        let make_uniform = || {
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: UNIFORM_SLOT as u64 * slots, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &ubgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &buf, offset: 0, size: Some(std::num::NonZero::new(UNIFORM_SLOT as u64).unwrap()) }) }, wgpu::BindGroupEntry { binding: 1, resource: pal_buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: col_buf.as_entire_binding() }] });
            (buf, bg)
        };
        let (uniform, uniform_bg) = make_uniform();
        let (uniform_rot, uniform_rot_bg) = make_uniform();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default()
        });
        let make_tex =|w: u32, h: u32, rgba: &[u8]| {
            let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: None, size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) }, size);
            let view = t.create_view(&Default::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &tbgl,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
            })
        };
        let mut tex_bgs = vec![make_tex(1, 1, &[255, 255, 255, 255])];
        let mut tex_frames = vec![(0, 1, 1)];
        let hud_bgs = std::iter::once(make_tex(1, 1, &[255, 255, 255, 255])).chain(scene.hud_textures.iter().map(|(_, t)| make_tex(t.width, t.height, &t.rgba))).collect();
        for t in &scene.textures {
            tex_frames.push((tex_bgs.len(), 1 + t.more.len() as u32, t.rate));
            tex_bgs.push(make_tex(t.width, t.height, &t.rgba));
            for f in &t.more { tex_bgs.push(make_tex(t.width, t.height, f)); }
        }
        let material_tex: Vec<usize> = scene.materials.iter()
            .map(|m| m.key.and_then(|k| scene.textures.iter().position(|t| t.key == k)).map_or(0, |i| i + 1)).collect();
        // Bug A-2a diagnostic: log every material that falls back to the 1×1 white texture (index 0), so the user
        // can tell whether the "white-translucent character" symptom on a level is the texture fallback (key=None or
        // missing TextureImage) or a blend-mode mismatch (alpha & 0xff == 0x48 = additive — see scene.rs).
        if std::env::var_os("NF_TEX_TRACE").is_some() {
            for (m, &t) in scene.materials.iter().zip(&material_tex) {
                if t == 0 {
                    eprintln!("gpu: WHITE FALLBACK material: L={:?} pass={} blend={} atest={} key={:?} (first_index={} count={})",
                        m.layer, m.draw.pass, m.draw.blend, m.draw.atest, m.key, m.first_index, m.index_count);
                }
            }
        }
        // The Character-layer tail holds the bind-pose positions; the shader poses them with the palette.
        let mut verts = scene.vertices.clone();
        for (v, cv) in verts[scene.char_vertex_start..].iter_mut().zip(&scene.char_verts) { v.pos = cv.pos; }
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&verts), usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST });
        let stat = crate::scene::SkinAttr { a: u32::MAX, b: u32::MAX, w: 1.0, col: 0 };
        let mut attrs = vec![stat; scene.char_vertex_start];
        attrs.extend_from_slice(&char_attrs);
        attrs.resize(verts.len(), stat);
        let skin_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("skin attrs"), contents: bytemuck::cast_slice(&attrs), usage: wgpu::BufferUsages::VERTEX });
        let snow_skin = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("snow attrs"), contents: bytemuck::cast_slice(&vec![stat; SNOW_MAX]), usage: wgpu::BufferUsages::VERTEX });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&scene.indices), usage: wgpu::BufferUsages::INDEX });
        // Draw order (docs/systems/render.md): sky before the world, list 0, characters, list 1 (alpha), sky after the fog
        // pass, effects; the vertex buffer keeps its layout (the character tail stays contiguous).
        // List 0 order per `View_DrawObjList` (0x1e69a0) + `View_AddCels` (0x1e6ba0): the viewer's per-list array holds
        // both cel entries and object entries, QuickSorted by key. `View_AddCels` pushes the ROOM cels' primary
        // glists — one per 0xc023/0xc047 placement — into list 0 with the SYNTHETIC key −90000 + 10k over the
        // +0x20 chain `Vision_Portal_Recurse` (0x1e86e8) threads per frame from the depth-sorted `VisCelList`
        // (QuickSort 0x1d30a8 comparator[3]: ascending portal depth); the non-room 0x8000 placements (the baked
        // decals, walkways, blockers — the +0x18 inner nodes) and the dynamic World objects draw at ±dist²
        // (`View_AddObjects` 0x1e62d8). So the rooms draw first (per-frame chain order), then the distance-keyed
        // entries front-to-back. `Material::chain` carries the room's build-time parse-rank fallback, `node` its
        // per-frame chain cel (u32::MAX = a distance-keyed entry).
        let draws: Vec<DrawCall> = scene.materials.iter().zip(&material_tex)
            .map(|(m, &tex)| DrawCall { layer: m.layer, pass: m.draw.pass, pipeline: pipelines.iter().position(|(d, _)| *d == m.draw).unwrap(), tex, first: m.first_index, count: m.index_count, centre: m.centre, radius: m.radius, cel: m.cel, chain: m.chain, node: m.node, lflags: m.lflags }).collect();
        let fog = scene.fog;
        let snow_pipeline = make_pipeline(Draw { pass: 5, blend: 1, zwrite: false, atest: 0, fog: 0 });
        let snow_vbuf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (SNOW_MAX * std::mem::size_of::<crate::scene::Vertex>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let snow_bg = match &scene.snow_texture { Some(t) => make_tex(t.width, t.height, &t.rgba), None => make_tex(1, 1, &[255, 255, 255, 255]) };
        let order_cap = draws.len();
        let cel_boxes = scene.cels.iter().map(|c| (c.min, c.max)).collect();
        let cel_graph = scene.cel_graph.clone();
        Renderer { snow_pipeline, snow_vbuf, snow_bg, hud_pipeline, hud_add_pipeline, hud_sub_pipeline, hud_vbuf, hud_bgs, pipelines, fog, notex, nofog, uniform, uniform_bg, uniform_rot, uniform_rot_bg, vbuf, skin_buf, snow_skin, pal_buf, col_buf, skin_layout, lights: Default::default(), ibuf, tex_bgs, tex_frames, draws, order: Vec::with_capacity(order_cap), cel_boxes, cel_graph, viewer_cel: Default::default(), last_vis: None, last_cell_vis: None }
    }

    /// The game's depth-0 pyramid for the walk [M `Vision_Calculate_View_Fustrum` 0x1e7df8,
    /// disassembled]: the four fov side planes through the eye (the Gribb-Hartmann extraction's
    /// first four are the same half-spaces for this fov/aspect) plus the near plane through the
    /// 200-unit quad corners (c1,c3,c2) whose normal points BACK at the eye — it keeps
    /// `fw·p <= fw·eye + 200` (beyond 200 units is fog; the game clips those portal polygons away).
    /// `fw` = the GH near plane's normalized normal (it points forward, away from the eye).
    fn depth0_game(planes: &[[f32; 4]; 6], eye: [f32; 3]) -> [[f32; 4]; 5] {
        let fw = [planes[4][0], planes[4][1], planes[4][2]];
        let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
        [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
    }

    /// This frame's room-level visibility mask — one walk per frame, shared by the skin palette's
    /// object cull (`View_AddObjects` 0x1e62d8 draws only the visible cels' objects) and `render`'s
    /// World cull. The eye's cel comes from the persistent `Camera_CheckLocation` (0x1bae90) state
    /// via `scene::eye_cell`; the walk from that cell is `Vision_Portal_Recurse` 0x1e86e8 with the
    /// game's depth-0 planes. `None` (the walk cannot run: no portals / no cel) = the fix-render3
    /// box test everywhere, and `NF_NOPORTAL` forces it (the walk is the default again, fix-portal).
    pub fn cel_visibility(&mut self, eye: [f32; 3], view_proj: [[f32; 4]; 4]) -> Option<Vec<bool>> {
        let planes = extract_frustum(view_proj);
        let portal_enabled = std::env::var_os("NF_NOPORTAL").is_none();
        // The cell-level game mask (`View_AddCels` 0x1e6ba0: the VisCelList plus the +0x18 chain
        // pass under `Vision_InView`) gates the World draws; the strict VisCelList mask gates the
        // rooms/objects (`View_DrawObjList` 0x1e69a0 walks the VisCelList cels' object lists —
        // chain nodes never enter the VisCelList). fix-portal2.
        let (cell_vis, strict_vis) = if portal_enabled && !self.cel_graph.portals.is_empty() {
            let depth0 = Self::depth0_game(&planes, eye);
            let sides = [planes[0], planes[1], planes[2], planes[3]];
            match crate::scene::eye_cell(&self.cel_graph, &mut self.viewer_cel, eye) {
                Some(cell) => (
                    crate::scene::portal_visible_cels_game_from_cell(&self.cel_graph, cell, eye, &depth0, &sides),
                    crate::scene::portal_visible_cels_from_cell(&self.cel_graph, cell, eye, &depth0),
                ),
                None => (None, None),
            }
        } else { (None, None) };
        self.last_cell_vis = cell_vis;
        let vis = strict_vis.map(|cell_vis| self.cel_graph.room_to_cell.iter().map(|m| m.map(|c| cell_vis[c]).unwrap_or(true)).collect::<Vec<bool>>());
        let vis = vis.unwrap_or_else(|| self.cel_boxes.iter().map(|&(min, max)| !box_outside(&planes, min, max)).collect());
        self.last_vis = Some(vis.clone());
        Some(vis)
    }

    /// G-043: upload one frame's skinning palette and colour table (replaces re-uploading every posed vertex).
    /// `vis`: the frame's room visibility mask (see `Scene::skin_palette`).
    pub fn update_skin(&self, queue: &wgpu::Queue, scene: &Scene, choice: Option<usize>, time: f32, opts: crate::scene::ViewOpts, vis: Option<&[bool]>) {
        let (pal, cols) = scene.skin_palette(&self.skin_layout, choice, time, opts, vis);
        if !pal.is_empty() { queue.write_buffer(&self.pal_buf, 0, bytemuck::cast_slice(&pal)); }
        queue.write_buffer(&self.col_buf, 0, bytemuck::cast_slice(&cols));
    }

    /// Upload an already-built (possibly interpolated) palette and colour table.
    pub fn upload_skin(&self, queue: &wgpu::Queue, pal: &[nf_formats::anim::Mat4], cols: &[[f32; 4]]) {
        if !pal.is_empty() { queue.write_buffer(&self.pal_buf, 0, bytemuck::cast_slice(pal)); }
        if !cols.is_empty() { queue.write_buffer(&self.col_buf, 0, bytemuck::cast_slice(cols)); }
    }

    /// `view_proj`: full camera; `view_proj_rot`: the same without the camera translation.
    /// Rewrites the Character-layer tail of the vertex buffer (starting at vertex `start`).
    pub fn update_vertices(&self, queue: &wgpu::Queue, start: usize, vertices: &[crate::scene::Vertex]) {
        if vertices.is_empty() { return; }
        queue.write_buffer(&self.vbuf, (start * std::mem::size_of::<crate::scene::Vertex>()) as u64, bytemuck::cast_slice(vertices));
    }

    /// `hud`: overlay triangles drawn last.
    /// `eye`: the camera position (the per-frame draw sort: list 0 front to back, list 1 back to front by the objects'
    /// squared distance, as `View_AddCels` orders cels).
    ///
    /// A-11e: CPU-side frustum cull. Before the draw loop the 6 frustum planes are extracted from `view_proj`
    /// (Gribb-Hartmann); each draw's bounding sphere is tested, and draws fully outside the frustum are skipped
    /// (`continue`) — no pipeline, bind group or draw_indexed call is issued for them. Sky draws (pass 4, drawn after
    /// the fog pass with the rotation-only uniform) follow the viewer and are never culled. The per-frame `order` Vec
    /// is `clear`ed + refilled (not reallocated); `set_pipeline`/`set_bind_group(0)` are skipped when the value is
    /// unchanged from the previous draw.
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView, depth: &wgpu::TextureView, view_proj: [[f32; 4]; 4], view_proj_rot: [[f32; 4]; 4], eye: [f32; 3], ticks: u32, snow: &[crate::scene::Vertex], hud: &HudFrame) -> wgpu::CommandEncoder {
        let ns = snow.len().min(SNOW_MAX);
        if ns > 0 { queue.write_buffer(&self.snow_vbuf, 0, bytemuck::cast_slice(&snow[..ns])); }
        let n = hud.v.len().min(HUD_MAX) / 3 * 3;
        if n > 0 { queue.write_buffer(&self.hud_vbuf, 0, bytemuck::cast_slice(&hud.v[..n])); }
        // Fog constants: N1 = `Ps2Near1` 8, n = `Ps2Near2` 0.1, f = `Ps2Far2` 2^24 [L]; F1 per level.
        let (fog_rgb, fog_on, k) = match self.fog {
            Some((c, f1)) => ([c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0], 1.0, (f1 - 8.0) * 0.1 / (16_777_216.0 - 0.1)),
            None => ([0.0; 3], 0.0, 0.0),
        };
        let extra = [fog_rgb[0], fog_rgb[1], fog_rgb[2], fog_on, 8.0, k, 16_777_216.0, GAIN];
        // Per-draw uniform slots (fix-lighting): the game uploads the ≤2 closest lights per object
        // (`View_DrawObjList` 0x1e69a0 / `View_DrawCelGlist` 0x1e6538 → `Lights_CalcClosestLights`
        // 0x1cb2e0 + `psiLight_SetLights` 0x10e588). Slot 0 = the eye's own sphere (the snow pass — the
        // game's particle path inherits the previous upload and the flakes have no normals, so the
        // pick is inert); slot k+1 = draw k. fix-lighting2: the per-draw flags are the source object's
        // glist flags word (`Control_SetGList` 0x134960 → obj+0xF0 / cel+0x94), not the hardcoded 0x80
        // that let the kind-0 ambient radiators crowd the muzzle light off the world draws; the sky
        // draws upload NO lights (`View_DrawSky` 0x1e5ad0 / `View_DrawSprites` 0x1e5290 call
        // `psiLight_SetLights(0,0,0,0,0)`).
        let slot_f = UNIFORM_SLOT / 4;
        let n_draws = self.draws.len();
        let mut staged = vec![0f32; slot_f * (n_draws + 1)];
        {
            let s = &mut staged[..slot_f];
            s[16..24].copy_from_slice(&extra);
            let (lp, lc) = self.lights.pick2([eye[0], eye[1], eye[2], 2.0], 0x80);
            for (i, v) in lp.iter().chain(lc.iter()).enumerate() { s[24 + i * 4..28 + i * 4].copy_from_slice(v); }
        }
        for (k, dc) in self.draws.iter().enumerate() {
            let s = &mut staged[(k + 1) * slot_f..][..slot_f];
            s[16..24].copy_from_slice(&extra);
            if matches!(dc.layer, crate::scene::Layer::SkyCamera | crate::scene::Layer::SkyFixed) {
                // The game zeroes the light slots for the sky and sprite passes [M 0x1e5ad0:89].
                continue;
            }
            let (lp, lc) = self.lights.pick2([dc.centre[0], dc.centre[1], dc.centre[2], dc.radius], dc.lflags as u32);
            for (i, v) in lp.iter().chain(lc.iter()).enumerate() { s[24 + i * 4..28 + i * 4].copy_from_slice(v); }
        }
        for (m, vp) in [(&self.uniform, view_proj), (&self.uniform_rot, view_proj_rot)] {
            let mut u = staged.clone();
            for slot in 0..=n_draws {
                let s = &mut u[slot * slot_f..][..slot_f];
                for r in 0..4 { for c in 0..4 { s[r * 4 + c] = vp[r][c]; } }
            }
            queue.write_buffer(m, 0, bytemuck::cast_slice(&u));
        }
        // The frame starts as fog (what the fog pass covers where nothing was drawn: zb 0).
        let clear = if fog_on > 0.0 { wgpu::Color { r: (fog_rgb[0] * GAIN) as f64, g: (fog_rgb[1] * GAIN) as f64, b: (fog_rgb[2] * GAIN) as f64, a: 1.0 } } else { wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 } };
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target, depth_slice: None, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }), stencil_ops: None,
                }),
                timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
            });
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_vertex_buffer(1, self.skin_buf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            use crate::scene::Layer;
            let d2 = |c: [f32; 3]| (c[0] - eye[0]).powi(2) + (c[1] - eye[1]).powi(2) + (c[2] - eye[2]).powi(2);
            // The 6 frustum planes for the CPU-side cull (Gribb-Hartmann from `view_proj`).
            let planes = extract_frustum(view_proj);
            // fix-cel-order: the room cels' −90000 + 10·k list-0 keys are handed out over the chain
            // `Vision_Portal_Recurse` (0x1e86e8) threads per frame from the depth-sorted `VisCelList`
            // (`QuickSort` 0x1d30a8 comparator[3] @0x1d3430: ascending portal depth, ties in the
            // game's own unstable order). Compute that chain for THIS eye and map each room cel
            // (`Material::node`) to its k. When the walk cannot run (no graph / the eye's room
            // unmapped) the build-time parse-rank fallback (`Material::chain`) stands in. Ordering
            // only — the cull below is untouched (NF_PORTAL still gates the cull walk).
            let chain_ord: Option<Vec<Option<u32>>> = if !self.cel_graph.portals.is_empty() {
                // The same resolved viewer cel + the game's depth-0 planes the walk used (the
                // chain order is the VisCelList the game threaded from ITS walk [M 0x1e86e8]).
                let depth0 = Self::depth0_game(&planes, eye);
                self.viewer_cel.cell.and_then(|cell| crate::scene::portal_cel_chain_from_cell(&self.cel_graph, cell, eye, &depth0))
                    .map(|chain| {
                        let mut ord: Vec<Option<u32>> = vec![None; self.cel_graph.cells.len()];
                        for (k, c) in chain.into_iter().enumerate() { if c < ord.len() { ord[c] = Some(k as u32); } }
                        ord
                    })
            } else { None };
            // Reuse the cached `order` Vec (A-11e): clear + refill, no per-frame alloc.
            self.order.clear();
            self.order.extend(self.draws.iter().enumerate().map(|(k, dc)| {
                // List 0 (pass 1): the room cels' −90000 + 10·k (`View_AddCels` 0x1e6ba0 over the
                // per-frame depth-sorted chain, `Material::node` → k; `chain` = the build-time
                // parse-rank fallback when the walk cannot run), every distance-keyed entry (the
                // +0x18 inner-node statics and `View_AddObjects`' objects) by squared distance.
                // List 1 (pass 3): back-to-front (−dist²) for everything. Ties fall through to the
                // draw index (the material order = the build's group order, i.e. leaf/batch order
                // within one object).
                let key = match dc.pass {
                    1 => if dc.chain != u32::MAX {
                        chain_ord.as_ref().and_then(|o| o.get(dc.node as usize).copied().flatten())
                            .map(|k2| -90_000.0 + 10.0 * k2 as f32)
                            .unwrap_or_else(|| -90_000.0 + 10.0 * dc.chain as f32)
                    } else { d2(dc.centre) },
                    3 => -d2(dc.centre),
                    _ => 0.0,
                };
                (dc.pass * 2, key, k)
            }));
            self.order.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
            // fix-render2: the World draws' bounding spheres are exact — static geometry through the
            // placement matrix (scene.rs add_instance) — so they cull always. Everything posed per
            // frame keeps its build-time centre (doors swinging, pickups, the truck, searchlight
            // parts, muzzle flashes, tracers — the Character/Effect layers) and the sky follows the
            // viewer, so those only cull under the NF_CULL test override. The HUD draws from its own
            // buffer below and is never in this list. This was A-11's vanish bug: the full cull made
            // the truck and the HUD disappear by view direction (user playtest); scoping the cull to
            // the static World layer keeps every moving thing drawn while still skipping the bulk —
            // 07000006's start view submits ~4k draws unculled (render2_live counts them).
            let test_cull = std::env::var_os("NF_CULL").is_some();
            let cull = |dc: &DrawCall| test_cull || dc.layer == Layer::World;
            // Counters for the worklog (env-gated; no per-frame cost otherwise).
            let trace = std::env::var_os("NF_DRAWS").is_some();
            let (mut submitted, mut culled) = (0usize, 0usize);
            // fix-render3: the room-cel visibility test (`View_AddCels` 0x1e6ba0's `Vision_InView`
            // per cel bbox). Computed once per frame — ~200 sphere tests; a World draw whose cel is
            // out is skipped before the pipeline/bind work. The eye's own cel always passes (its
            // bbox contains the eye, so its sphere can never be outside the frustum).
            let cel_cull = std::env::var_os("NF_NOCELCULL").is_none();
            // fix-perf-level: the visibility set is the game's own portal walk
            // (`Vision_Portal_Recurse` 0x1e86e8 → `Vision_Recurse` 0x1e82e0, the GfxList set) from the
            // eye's room cel, with the depth-0 pyramid = the frustum's four side planes + near (the
            // game's five `Vision_Calculate_View_Fustrum` 0x1e7df8 planes). Cels the walk cannot run
            // for (no graph / no portals / eye outside every cel) keep the fix-render3 box test.
            // fix-portal: the walk is the default again (the collapse is fixed: the viewer cel is
            // `Camera_CheckLocation` 0x1bae90's persistent state and the depth-0 pyramid is the
            // game's [M 0x1e7df8]); `NF_NOPORTAL` disables it. The mask is computed once per frame
            // by `cel_visibility` (the skin palette shares it) — reuse it when fresh.
            let cel_visible: Vec<bool> = if cel_cull {
                self.last_vis.clone().unwrap_or_else(|| self.cel_visibility(eye, view_proj).unwrap())
            } else { Vec::new() };
            // fix-portal2: the cell-level game mask. A cel placement's glist draws when ITS cel is
            // visible (`View_AddCels` 0x1e6ba0 — the VisCelList cel itself or a `+0x18` chain node
            // under `Vision_InView`); `Material::node` = the placement's own cell for every
            // creates_cel placement (u32::MAX = the distance-keyed objects, gated by their
            // containing room as before).
            let cell_visible: Option<&Vec<bool>> = self.last_cell_vis.as_ref();
            let mut cel_culled = 0usize;
            // Skip redundant `set_pipeline` calls (A-11e). The group-0 bind now carries a per-draw
            // dynamic offset (fix-lighting: each draw's own light pair), so it is issued every draw.
            let mut last_pipeline: Option<usize> = None;
            // Debug: `NF_SKIP_MAT=i,j,..` skips those material indices (bisecting a wrong-looking surface).
            static SKIP: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
            let skip = SKIP.get_or_init(|| std::env::var("NF_SKIP_MAT").map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect()).unwrap_or_default());
            for &(_, _, k) in &self.order {
                if skip.contains(&k) { continue; }
                let dc = self.draws[k];
                // Sky after the fog pass (pass 4) follows the viewer and is never culled.
                if cull(&dc) && dc.pass != 4 && sphere_outside(&planes, dc.centre, dc.radius) {
                    if trace { culled += 1; }
                    continue;
                }
                if cel_cull && dc.layer == Layer::World {
                    let hidden = match (cell_visible, dc.node) {
                        (Some(cv), n) if n != u32::MAX => !cv[n as usize],
                        _ => dc.cel.is_some_and(|c| !cel_visible[c as usize]),
                    };
                    if hidden {
                        if trace { cel_culled += 1; }
                        continue;
                    }
                }
                if trace { submitted += 1; }
                let (layer, first, count) = (dc.layer, dc.first, dc.count);
                let (uniform, off) = if layer == Layer::SkyCamera {
                    (&self.uniform_rot_bg, (k + 1) as u32 * UNIFORM_SLOT as u32)
                } else {
                    (&self.uniform_bg, (k + 1) as u32 * UNIFORM_SLOT as u32)
                };
                if last_pipeline != Some(dc.pipeline) {
                    pass.set_pipeline(&self.pipelines[dc.pipeline].1);
                    last_pipeline = Some(dc.pipeline);
                }
                pass.set_bind_group(0, uniform, &[off]);
                let (first_bg, n_frames, rate) = self.tex_frames[dc.tex];
                pass.set_bind_group(1, &self.tex_bgs[first_bg + ((ticks / rate) % n_frames) as usize], &[]);
                pass.draw_indexed(first..first + count, 0, 0..1);
            }
            if trace {
                eprintln!("render: {submitted} draws submitted, {culled} frustum-culled, {cel_culled} cel-culled ({} total, {:.1}% skipped)",
                    submitted + culled + cel_culled, 100.0 * (culled + cel_culled) as f32 / (submitted + culled + cel_culled).max(1) as f32);
            }
            // `DrawDrops` runs after the fog pass, the alpha list and the shards.
            if ns > 0 {
                pass.set_pipeline(&self.snow_pipeline);
                pass.set_bind_group(0, &self.uniform_bg, &[0]);
                pass.set_bind_group(1, &self.snow_bg, &[]);
                pass.set_vertex_buffer(0, self.snow_vbuf.slice(..));
                pass.set_vertex_buffer(1, self.snow_skin.slice(..));
                pass.draw(0..ns as u32, 0..1);
            }
            if n > 0 {
                pass.set_vertex_buffer(0, self.hud_vbuf.slice(..));
                for &(tex, blend, first, count) in &hud.runs {
                    let end = (first + count).min(n as u32);
                    if first >= end { continue; }
                    pass.set_pipeline(match blend { 1 => &self.hud_add_pipeline, 2 => &self.hud_sub_pipeline, _ => &self.hud_pipeline });
                    pass.set_bind_group(0, &self.hud_bgs[tex.min(self.hud_bgs.len() - 1)], &[]);
                    pass.draw(first..end, 0..1);
                }
            }
        }
        enc
    }
}

pub fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format: DEPTH, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    }).create_view(&Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A view_proj for the camera `Camera::view_proj` produces: at the origin (0, 0, 0) looking down +z, 60° vertical
    /// FOV, 16:9 aspect, near 0.05 far 4000 (the live values from `lib.rs` and `Camera::view_proj`). Column-major,
    /// stored as `[[col0], [col1], [col2], [col3]]` — `vp[i]` is the i-th column.
    fn vp_origin_looking_z() -> [[f32; 4]; 4] {
        // proj columns: (f/aspect, 0, 0, 0), (0, f, 0, 0), (0, 0, far/(near-far), -1), (0, 0, near*far/(near-far), 0)
        // — f = 1/tan(30°) ≈ 1.7320508, aspect = 16/9, so f/aspect ≈ 0.9742786.
        // view columns (yaw=0, pitch=0, eye=origin): (-1, 0, 0, 0), (0, 1, 0, 0), (0, 0, -1, 0), (0, 0, 0, 1).
        // view_proj = proj * view → columns as below (worked out by hand).
        [
            [-0.9742786, 0.0, 0.0, 0.0],          // col 0
            [0.0, 1.7320508, 0.0, 0.0],            // col 1
            [0.0, 0.0, 1.0000125, 1.0],           // col 2
            [0.0, 0.0, -0.05, 0.0],               // col 3
        ]
    }

    #[test]
    fn frustum_cull_keeps_a_sphere_in_front_of_the_camera() {
        let vp = vp_origin_looking_z();
        let planes = extract_frustum(vp);
        // 10 units in front of the camera along +z, radius 1: should be inside the frustum.
        assert!(!sphere_outside(&planes, [0.0, 0.0, 10.0], 1.0), "in-frustum sphere was culled");
    }

    #[test]
    fn frustum_cull_drops_a_sphere_behind_the_camera() {
        let vp = vp_origin_looking_z();
        let planes = extract_frustum(vp);
        // 10 units BEHIND the camera (−z), small radius: should be culled.
        assert!(sphere_outside(&planes, [0.0, 0.0, -10.0], 0.5), "behind-camera sphere was kept");
    }

    #[test]
    fn frustum_cull_drops_a_sphere_far_off_to_the_side() {
        let vp = vp_origin_looking_z();
        let planes = extract_frustum(vp);
        // 1000 units to +x (outside the 60° FOV cone), small radius: should be culled.
        assert!(sphere_outside(&planes, [1000.0, 0.0, 10.0], 0.5), "off-axis sphere was kept");
    }

    #[test]
    fn frustum_cull_keeps_a_large_sphere_even_just_past_the_far_plane() {
        let vp = vp_origin_looking_z();
        let planes = extract_frustum(vp);
        // Centre 5000 units away (past far = 4000), but radius 2000 → intersects the far plane, must NOT be culled.
        assert!(!sphere_outside(&planes, [0.0, 0.0, 5000.0], 2000.0), "large sphere intersecting the far plane was culled");
        // Same centre, tiny radius → fully outside the far plane, must be culled.
        assert!(sphere_outside(&planes, [0.0, 0.0, 5000.0], 1.0), "small sphere past the far plane was kept");
    }

    #[test]
    fn frustum_cull_planes_are_normalised() {
        let vp = vp_origin_looking_z();
        let planes = extract_frustum(vp);
        for p in &planes {
            let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((n - 1.0).abs() < 1e-5, "plane normal not unit length: {n}");
        }
    }
}

#[cfg(test)]
mod shader_tests {
    /// The WGSL must parse and validate offline (the sandbox has no GPU adapter; the main agent's
    /// picture checks run elsewhere). fix-lighting touched the main shader's uniform struct and
    /// light block — a syntax or validation error would break every draw at Renderer::new.
    #[test]
    fn the_shaders_parse_and_validate() {
        for (name, src) in [("main", super::SHADER), ("hud", super::HUD_SHADER)] {
            let module = naga::front::wgsl::parse_str(src).unwrap_or_else(|e| panic!("{name} WGSL parse: {e}"));
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
                .validate(&module).unwrap_or_else(|e| panic!("{name} WGSL validate: {e}"));
        }
    }

    /// fix-lighting2: the light add is the game's byte-scale `TColor.rgb · term` over the Unorm8x4
    /// (byte/255) vertex colour — /255, not /128 — and the summed colour clamps at byte 255 like the
    /// microcode's MINIi (I = 255.0, .vutext I-mem 919/954). The old /128 over-added by exactly
    /// 255/128 ≈ 2× (the view-model saturation the merge flagged) and never clamped.
    #[test]
    fn the_light_add_is_in_byte_255_units_with_the_minii_clamp() {
        assert!(super::SHADER.contains("min(c.rgb + lc.rgb * term / 255.0, vec3<f32>(1.0))"),
            "the shader must add the byte-scale light over the byte/255 vertex colour and clamp to byte 255");
        assert!(!super::SHADER.contains("term / 128.0"), "the old 2× over-add must stay gone");
    }
}
