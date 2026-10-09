//! CPU side of the viewer: a level archive turned into world-space vertex/index buffers grouped by material.
//!
//! Material = (map, texture index) — a batch's texture index refers to the texture list of the map its object comes
//! from (docs/formats/ps2gfx.md). Vertex colour stays on the PS2 scale (0x80 = 1.0); the shader rescales it.

use nf_formats::level::Level;
use nf_formats::placement::transform;
use std::collections::BTreeMap;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
}

pub struct Material {
    /// `(map, texture)`; `None` = untextured.
    pub key: Option<(usize, u32)>,
    pub first_index: u32,
    pub index_count: u32,
}

pub struct TextureImage {
    pub key: (usize, u32),
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub materials: Vec<Material>,
    pub textures: Vec<TextureImage>,
    pub bounds: ([f32; 3], [f32; 3]),
    /// Camera start: the first placement whose object is named "Player1" (translation, yaw), if any.
    pub start: Option<([f32; 3], f32)>,
    pub stats: String,
}

impl Scene {
    pub fn build(level: &Level, cels_only: bool) -> Scene {
        let (instances, missing) = level.instances(cels_only);
        let mut groups: BTreeMap<Option<(usize, u32)>, (Vec<Vertex>, Vec<u32>)> = BTreeMap::new();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let (mut skinned, mut hidden) = (0, 0);
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            if inst.placement.creates_cel() && obj.skipped_by_cel_pass() { hidden += 1; continue; }
            if obj.gfx.is_skinned() { skinned += 1; }
            for b in obj.gfx.leaves.iter().flat_map(|l| &l.batches) {
                let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
                let (vs, is) = groups.entry(key).or_default();
                let base = vs.len() as u32;
                for v in &b.verts {
                    let p = transform(&inst.matrix, v.pos);
                    if p.iter().all(|c| c.is_finite()) {
                        for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
                    }
                    let uv = if v.uv.iter().all(|c| c.is_finite()) { v.uv } else { [0.0, 0.0] };
                    vs.push(Vertex { pos: p, uv, rgba: v.rgba });
                }
                for t in b.triangles() { is.extend(t.map(|k| base + k as u32)); }
            }
        }
        let (mut vertices, mut indices, mut materials) = (Vec::new(), Vec::new(), Vec::new());
        for (key, (vs, is)) in groups {
            if is.is_empty() { continue; }
            let base = vertices.len() as u32;
            materials.push(Material { key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
        }
        let mut textures = Vec::new();
        for key in materials.iter().filter_map(|m| m.key) {
            let t = &level.maps[key.0].textures[key.1 as usize];
            let pal = t.palette_rgba8();
            let rgba = t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect();
            textures.push(TextureImage { key, width: t.width as u32, height: t.height as u32, rgba });
        }
        let stats = format!("{} instances ({hidden} skipped by the cel pass: flag 0x10; {skinned} skinned, not drawn yet), {missing} unresolved, {} triangles, {} materials",
            instances.len(), indices.len() / 3, materials.len());
        Scene { vertices, indices, materials, textures, bounds: (lo, hi), start, stats }
    }
}
