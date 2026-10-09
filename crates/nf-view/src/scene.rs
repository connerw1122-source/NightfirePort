//! CPU side of the viewer: a level archive turned into vertex/index buffers grouped by layer and material.
//!
//! Material = (map, texture index) — a batch's texture index refers to the texture list of the map its object comes
//! from (docs/formats/ps2gfx.md). Vertex colour stays on the PS2 scale (0x80 = 1.0); the shader rescales it.
//! Layers follow the game's frame: the sky pass first (`View_DrawSky`, objects centred on the viewer, then fixed sky
//! objects), then the world (docs/formats/placement.md §Sky).

use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;
use std::collections::{BTreeMap, BTreeSet};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Sky objects with glist flag bit 2: drawn around the viewer (vertices in object space, rotation only).
    SkyCamera,
    /// Sky objects at their placement translation, drawn in the sky pass (no depth).
    SkyFixed,
    World,
}

pub struct Material {
    pub layer: Layer,
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

type Groups = BTreeMap<(Layer, Option<(usize, u32)>), (Vec<Vertex>, Vec<u32>)>;

fn add_instance(level: &Level, inst: &Instance, layer: Layer, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>) {
    let obj = &level.maps[inst.map].objects[inst.object];
    let mut m = inst.matrix;
    if layer == Layer::SkyCamera { m[3] = [0.0, 0.0, 0.0, 1.0]; }
    let mut bounds = bounds;
    for b in obj.gfx.leaves.iter().flat_map(|l| &l.batches) {
        let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
        let (vs, is) = groups.entry((layer, key)).or_default();
        let base = vs.len() as u32;
        for v in &b.verts {
            let p = transform(&m, v.pos);
            if let Some((lo, hi)) = bounds.as_deref_mut() {
                if p.iter().all(|c| c.is_finite()) {
                    for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
                }
            }
            let uv = if v.uv.iter().all(|c| c.is_finite()) { v.uv } else { [0.0, 0.0] };
            vs.push(Vertex { pos: p, uv, rgba: v.rgba });
        }
        for t in b.triangles() { is.extend(t.map(|k| base + k as u32)); }
    }
}

impl Scene {
    pub fn build(level: &Level, cels_only: bool) -> Scene {
        let (mut instances, missing) = level.instances(cels_only);
        // Sky objects are drawn by the sky pass only (with --all they would also appear as dynamic entities).
        instances.retain(|i| i.placement.flags & 0xff != 0x2a);
        // Characters (oDrone placements → skin → skinned meshes, bind pose) are drawn in both modes.
        let (characters, chars_missing) = level.character_instances();
        let n_characters = characters.len();
        instances.extend(characters);
        let sky = level.sky_instances();
        let mut groups: Groups = BTreeMap::new();
        let mut bounds = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let (mut skinned, mut hidden) = (0, 0);
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            if inst.placement.creates_cel() && obj.skipped_by_cel_pass() { hidden += 1; continue; }
            if obj.gfx.is_skinned() { skinned += 1; }
            add_instance(level, inst, Layer::World, &mut groups, Some(&mut bounds));
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, &mut groups, None);
        }
        let (mut vertices, mut indices, mut materials) = (Vec::new(), Vec::new(), Vec::new());
        for ((layer, key), (vs, is)) in groups {
            if is.is_empty() { continue; }
            let base = vertices.len() as u32;
            materials.push(Material { layer, key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
        }
        let keys: BTreeSet<(usize, u32)> = materials.iter().filter_map(|m| m.key).collect();
        let textures = keys.into_iter().map(|key| {
            let t = &level.maps[key.0].textures[key.1 as usize];
            let pal = t.palette_rgba8();
            TextureImage { key, width: t.width as u32, height: t.height as u32, rgba: t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect() }
        }).collect();
        let stats = format!("{} instances ({hidden} skipped by the cel pass: flag 0x10; {n_characters} character meshes, {chars_missing} drones with a skin from another archive; {skinned} skinned, bind pose), {} sky objects, {missing} unresolved, {} triangles, {} materials",
            instances.len(), sky.len(), indices.len() / 3, materials.len());
        Scene { vertices, indices, materials, textures, bounds, start, stats }
    }
}
