fn main() -> anyhow::Result<()> {
    let bytes = std::fs::read("../extracted/bf2/lvl/DATA/_LVL_PS2/SIDE/REP.LVL")?;
    let inner = mungefs::open(&bytes)?;
    let rigs = mungefs::anim::find_rigs(inner)?;
    let crcs =
        ["bone_root", "aimer_maincannon_y", "aimer_maincannon_x"].map(mungefs::anim::crc32_lower);
    for rig in rigs
        .iter()
        .filter(|r| r.joints.iter().any(|j| j.crc == crcs[2]))
    {
        println!("rig {}", rig.name);
        for j in rig.joints.iter().filter(|j| crcs.contains(&j.crc)) {
            println!(
                "{:08x} parent {:08x} rest {:?}",
                j.crc, j.parent_crc, j.rest_pos
            );
        }
    }
    for bank in mungefs::anim::find_banks(inner)?
        .iter()
        .filter(|b| b.name == "rep_walk_atte")
    {
        let k = bank
            .clip_index("idle")
            .or_else(|| bank.clip_index("rep_walk_atte_idle"))
            .unwrap();
        for t in bank
            .decode_clip(k)?
            .tracks
            .iter()
            .filter(|t| crcs.contains(&t.bone_crc))
        {
            println!(
                "clip {:08x} translated {} first {:?} last {:?}",
                t.bone_crc,
                t.translated,
                t.translations.first(),
                t.translations.last()
            );
        }
    }
    let root = game::assets::baked_root();
    let mut db = game::classdb::ClassDb::default();
    db.load(&root.join("classes/bf2_rep.ron"))?;
    for (i, seat) in game::vehicle_class::seats(&db, "rep_walk_atte")
        .iter()
        .enumerate()
    {
        println!("SEAT {i}: {seat:?}");
    }
    Ok(())
}
