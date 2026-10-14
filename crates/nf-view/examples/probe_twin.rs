//! weapon-matrix4 probe: the record fields behind the matrix's by-design arms (selectable +4,
//! the twin byte +5, the base +2, the ammo type +0x90, the reload scripts +0xA8/+0xA4/+0xAC).
//! Run: cargo run --release -p nf-view --example probe_twin  (NF_ISO set)
fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let ids: Vec<u8> = vec![0x1a, 0x1e, 0x38, 0x59, 0x5a, 0x6c];
    let sc = nf_view::load_weapons(&iso, "07000002.bin", false, &ids).expect("load");
    let sim = sc.sim.as_ref().unwrap();
    for id in ids {
        let Some(w) = sim.weapons.records.get(id as usize) else { println!("{id:#04x}: no record"); continue };
        let r16 = |o: usize| u16::from_le_bytes([w[o], w[o + 1]]);
        let r32 = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        println!("{id:#04x} selectable(+4)={} twin(w5)={} base(+2)={} clip_size={} ammo_type={} flags68={:08x} fp={:08x} relA8={:08x} relA4={:08x} relAC={:08x}",
            w[4], w[5], r16(2), r16(0x92), sim.weapons.ammo_type(id as usize), r32(0x68), r32(0xDC), r32(0xA8), r32(0xA4), r32(0xAC));
    }
}
