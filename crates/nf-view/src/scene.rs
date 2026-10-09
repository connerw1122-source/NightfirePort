//! CPU side of the viewer: a level archive turned into vertex/index buffers grouped by layer and material, plus the
//! per-vertex skinning data that animates characters each frame.
//!
//! Material = (map, texture index) — a batch's texture index refers to the texture list of the map its object comes
//! from (docs/formats/ps2gfx.md). Vertex colour stays on the PS2 scale (0x80 = 1.0); the shader rescales it.
//! Layers follow the game's frame: the sky pass first (`View_DrawSky`, objects centred on the viewer, then fixed sky
//! objects), then the world; characters are drawn like the world but their vertices sit at the end of the buffer and
//! are rewritten every frame (docs/formats/animation.md).

use nf_formats::anim::{self, Mat4, Sequence};
use nf_formats::character::{Skeleton, Skin};
use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    /// Skinned character meshes; vertices rewritten each frame.
    Character,
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

/// One drawn character mesh: its placement matrix, skin, skeleton and slot → bone list.
pub struct Character {
    pub matrix: Mat4,
    pub skin: Skin,
    pub skeleton: Skeleton,
    pub slot_bones: Vec<u8>,
}

/// Bind-pose data of one character vertex (same order as the Character-layer vertices).
#[derive(Clone, Copy)]
pub struct CharVertex {
    pub character: u32,
    pub pos: [f32; 3],
    pub weight: f32,
    pub slots: [u16; 2],
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
    pub characters: Vec<Character>,
    /// Index of the first Character-layer vertex; `char_verts` describe that tail of `vertices`.
    pub char_vertex_start: usize,
    pub char_verts: Vec<CharVertex>,
    /// Sequences of the level's animation archive (`07F000NN.bin`), by skeleton id.
    pub sequences: HashMap<u8, Vec<Sequence>>,
}

type Groups = BTreeMap<(Layer, Option<(usize, u32)>), (Vec<Vertex>, Vec<u32>, Vec<CharVertex>)>;

fn add_instance(level: &Level, inst: &Instance, layer: Layer, character: Option<u32>, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>) {
    let obj = &level.maps[inst.map].objects[inst.object];
    let mut m = inst.matrix;
    if layer == Layer::SkyCamera { m[3] = [0.0, 0.0, 0.0, 1.0]; }
    let mut bounds = bounds;
    for b in obj.gfx.leaves.iter().flat_map(|l| &l.batches) {
        let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
        let (vs, is, cv) = groups.entry((layer, key)).or_default();
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
            if let Some(c) = character { cv.push(CharVertex { character: c, pos: v.pos, weight: v.weight, slots: v.slots }); }
        }
        for t in b.triangles() { is.extend(t.map(|k| base + k as u32)); }
    }
}

impl Scene {
    /// `skeletons`: every skeleton on the disc by id; `sequences`: the level's animation archive, grouped by skeleton.
    pub fn build(level: &Level, cels_only: bool, skeletons: &HashMap<u8, Skeleton>, sequences: HashMap<u8, Vec<Sequence>>) -> Scene {
        let (mut instances, missing) = level.instances(cels_only);
        // Sky objects are drawn by the sky pass only (with --all they would also appear as dynamic entities).
        instances.retain(|i| i.placement.flags & 0xff != 0x2a);
        let (char_instances, chars_missing) = level.character_instances();
        let sky = level.sky_instances();
        let mut groups: Groups = BTreeMap::new();
        let mut bounds = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let mut hidden = 0;
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            if inst.placement.creates_cel() && obj.skipped_by_cel_pass() { hidden += 1; continue; }
            add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds));
        }
        let mut characters = Vec::new();
        for inst in &char_instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            let skin_hash = inst.placement.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
            let Some(skin) = level.skins.get(&skin_hash) else { continue };
            let Some(skeleton) = skeletons.get(&skin.skeleton) else { continue };
            if !obj.gfx.is_skinned() || skin.inverse_bind.is_empty() {
                add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds));
                continue;
            }
            add_instance(level, inst, Layer::Character, Some(characters.len() as u32), &mut groups, Some(&mut bounds));
            characters.push(Character { matrix: inst.matrix, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: obj.gfx.slot_bones.clone() });
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, None, &mut groups, None);
        }
        let (mut vertices, mut indices, mut materials, mut char_verts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut char_vertex_start = None;
        for ((layer, key), (vs, is, cv)) in groups {
            if is.is_empty() { continue; }
            if layer == Layer::Character && char_vertex_start.is_none() { char_vertex_start = Some(vertices.len()); }
            let base = vertices.len() as u32;
            materials.push(Material { layer, key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
            char_verts.extend(cv);
        }
        let keys: BTreeSet<(usize, u32)> = materials.iter().filter_map(|m| m.key).collect();
        let textures = keys.into_iter().map(|key| {
            let t = &level.maps[key.0].textures[key.1 as usize];
            let pal = t.palette_rgba8();
            TextureImage { key, width: t.width as u32, height: t.height as u32, rgba: t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect() }
        }).collect();
        let n_seq: usize = sequences.values().map(Vec::len).sum();
        let stats = format!("{} world instances ({hidden} skipped by the cel pass: flag 0x10), {} animated character meshes, {chars_missing} drones with a skin from another archive, {} sky objects, {missing} unresolved, {} triangles, {} materials, {n_seq} sequences",
            instances.len(), characters.len(), sky.len(), indices.len() / 3, materials.len());
        let char_vertex_start = char_vertex_start.unwrap_or(vertices.len());
        Scene { vertices, indices, materials, textures, bounds, start, stats, characters, char_vertex_start, char_verts, sequences }
    }

    /// Skins every character at `frame` of the sequence chosen by `choice` (index into its skeleton's list, modulo
    /// the list length) and returns the new Character-layer vertex positions (world space).
    pub fn animate(&self, choice: usize, frame_time: f32) -> Vec<[f32; 3]> {
        let mut cache: HashMap<(u32, usize), Vec<Mat4>> = HashMap::new();
        let mats: Vec<Option<Vec<Mat4>>> = self.characters.iter().map(|c| {
            let seqs = self.sequences.get(&c.skin.skeleton)?;
            if seqs.is_empty() { return None; }
            let idx = choice % seqs.len();
            let seq = &seqs[idx];
            let span = (seq.frames as f32 - 1.0).max(1.0);
            let frame = 1 + (frame_time % span) as u32;
            Some(cache.entry((c.skin.hashcode, idx * 100_000 + frame as usize)).or_insert_with(|| {
                let world = anim::world_pose(&anim::local_pose(seq, &c.skeleton, &c.skin, frame), &c.skin);
                anim::skin_matrices(&world, &c.skin)
            }).clone())
        }).collect();
        self.char_verts.iter().map(|v| {
            let c = &self.characters[v.character as usize];
            let p = match &mats[v.character as usize] {
                Some(m) => anim::skin_vertex(v.pos, v.weight, v.slots, &c.slot_bones, m),
                None => v.pos,
            };
            transform(&c.matrix, p)
        }).collect()
    }

    /// Name of the chosen sequence for each skeleton in use (for the window title).
    pub fn sequence_label(&self, choice: usize) -> String {
        let mut used: Vec<u8> = self.characters.iter().map(|c| c.skin.skeleton).collect();
        used.sort();
        used.dedup();
        used.iter().filter_map(|s| self.sequences.get(s).filter(|v| !v.is_empty()).map(|v| {
            let q = &v[choice % v.len()];
            format!("skel {s}: {:08x} ({} frames, {}/{})", q.hashcode, q.frames, choice % v.len() + 1, v.len())
        })).collect::<Vec<_>>().join("; ")
    }
}
