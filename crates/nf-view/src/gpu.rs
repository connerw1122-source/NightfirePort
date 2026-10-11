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
struct U { vp: mat4x4<f32>, fog: vec4<f32>, fogk: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VO { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32>, @location(2) d: f32 };
@vertex fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) c: vec4<f32>) -> VO {
    var o: VO;
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
pub fn sphere_outside(planes: &[[f32; 4]; 6], centre: [f32; 3], radius: f32) -> bool {
    planes.iter().any(|p| p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2] + p[3] < -radius)
}

/// One indexed draw: its layer, pass, pipeline, texture bind group, index range and the object's world centre + radius.
#[derive(Clone, Copy)]
struct DrawCall { layer: crate::scene::Layer, pass: u8, pipeline: usize, tex: usize, first: u32, count: u32, centre: [f32; 3], radius: f32 }

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
    ibuf: wgpu::Buffer,
    tex_bgs: Vec<wgpu::BindGroup>,
    /// Per texture slot: its first bind group, frame count and rate (animated textures).
    tex_frames: Vec<(usize, u32, u32)>,
    /// Per material: layer, first index, index count (sorted by layer: sky pass first).
    draws: Vec<DrawCall>,
    /// Per-frame sort buffer reused across frames (A-11e): the per-frame draw order — `(pass, sort key, draw index)`
    /// — is `clear`ed + refilled every render instead of allocating ~64 KB each frame.
    order: Vec<(u8, f32, usize)>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, scene: &Scene, notex: bool, nofog: bool) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("nf-view"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let ubgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
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
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
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
        let make_uniform = || {
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 96, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &ubgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] });
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
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&scene.vertices), usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&scene.indices), usage: wgpu::BufferUsages::INDEX });
        // Draw order (docs/systems/render.md): sky before the world, list 0, characters, list 1 (alpha), sky after the fog
        // pass, effects; the vertex buffer keeps its layout (the character tail stays contiguous).
        let draws: Vec<DrawCall> = scene.materials.iter().zip(&material_tex)
            .map(|(m, &tex)| DrawCall { layer: m.layer, pass: m.draw.pass, pipeline: pipelines.iter().position(|(d, _)| *d == m.draw).unwrap(), tex, first: m.first_index, count: m.index_count, centre: m.centre, radius: m.radius }).collect();
        let fog = scene.fog;
        let snow_pipeline = make_pipeline(Draw { pass: 5, blend: 1, zwrite: false, atest: 0, fog: 0 });
        let snow_vbuf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (SNOW_MAX * std::mem::size_of::<crate::scene::Vertex>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let snow_bg = match &scene.snow_texture { Some(t) => make_tex(t.width, t.height, &t.rgba), None => make_tex(1, 1, &[255, 255, 255, 255]) };
        let order_cap = draws.len();
        Renderer { snow_pipeline, snow_vbuf, snow_bg, hud_pipeline, hud_add_pipeline, hud_sub_pipeline, hud_vbuf, hud_bgs, pipelines, fog, notex, nofog, uniform, uniform_bg, uniform_rot, uniform_rot_bg, vbuf, ibuf, tex_bgs, tex_frames, draws, order: Vec::with_capacity(order_cap) }
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
        let mut u = [0f32; 24];
        for (m, vp) in [(&self.uniform, view_proj), (&self.uniform_rot, view_proj_rot)] {
            for r in 0..4 { for c in 0..4 { u[r * 4 + c] = vp[r][c]; } }
            u[16..].copy_from_slice(&extra);
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
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            use crate::scene::Layer;
            let d2 = |c: [f32; 3]| (c[0] - eye[0]).powi(2) + (c[1] - eye[1]).powi(2) + (c[2] - eye[2]).powi(2);
            // Reuse the cached `order` Vec (A-11e): clear + refill, no per-frame alloc.
            self.order.clear();
            self.order.extend(self.draws.iter().enumerate().map(|(k, dc)| {
                let key = match dc.pass { 1 => d2(dc.centre), 3 => -d2(dc.centre), _ => 0.0 };
                (dc.pass, key, k)
            }));
            self.order.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
            // The 6 frustum planes for the CPU-side cull (Gribb-Hartmann from `view_proj`).
            let planes = extract_frustum(view_proj);
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
            // Skip redundant `set_pipeline` / `set_bind_group(0)` calls (A-11e): `last_pipeline` and `last_bg0_rot`
            // (whether the previous bind-group-0 was the rotation-only sky one) let us issue each only on a change.
            let mut last_pipeline: Option<usize> = None;
            let mut last_bg0_rot: Option<bool> = None;
            for &(_, _, k) in &self.order {
                let dc = self.draws[k];
                // Sky after the fog pass (pass 4) follows the viewer and is never culled.
                if cull(&dc) && dc.pass != 4 && sphere_outside(&planes, dc.centre, dc.radius) {
                    if trace { culled += 1; }
                    continue;
                }
                if trace { submitted += 1; }
                let (layer, first, count) = (dc.layer, dc.first, dc.count);
                let uniform = if layer == Layer::SkyCamera { &self.uniform_rot_bg } else { &self.uniform_bg };
                if last_pipeline != Some(dc.pipeline) {
                    pass.set_pipeline(&self.pipelines[dc.pipeline].1);
                    last_pipeline = Some(dc.pipeline);
                }
                let bg0_rot = layer == Layer::SkyCamera;
                if last_bg0_rot != Some(bg0_rot) {
                    pass.set_bind_group(0, uniform, &[]);
                    last_bg0_rot = Some(bg0_rot);
                }
                let (first_bg, n_frames, rate) = self.tex_frames[dc.tex];
                pass.set_bind_group(1, &self.tex_bgs[first_bg + ((ticks / rate) % n_frames) as usize], &[]);
                pass.draw_indexed(first..first + count, 0, 0..1);
            }
            if trace {
                eprintln!("render: {submitted} draws submitted, {culled} culled ({} total, {:.1}% skipped)",
                    submitted + culled, 100.0 * culled as f32 / (submitted + culled).max(1) as f32);
            }
            // `DrawDrops` runs after the fog pass, the alpha list and the shards.
            if ns > 0 {
                pass.set_pipeline(&self.snow_pipeline);
                pass.set_bind_group(0, &self.uniform_bg, &[]);
                pass.set_bind_group(1, &self.snow_bg, &[]);
                pass.set_vertex_buffer(0, self.snow_vbuf.slice(..));
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
