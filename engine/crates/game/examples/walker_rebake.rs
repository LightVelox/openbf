use anyhow::{Context, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = game::assets::baked_root();
    let backups = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("walker_rebake <backup_dir>")?,
    );
    for game in ["bf1", "bf2"] {
        for side in ["imp", "rep", "cis"] {
            let path = PathBuf::from(format!(
                "../extracted/{game}/lvl/DATA/_LVL_PS2/SIDE/{}.LVL",
                side.to_uppercase()
            ));
            let bytes = std::fs::read(path)?;
            let inner = mungefs::open(&bytes)?;
            let chunks = mungefs::content_chunks(inner)?;
            let banks = mungefs::anim::find_banks(inner)?;
            let skels: Vec<_> = chunks
                .iter()
                .filter(|c| &c.tag == b"skel")
                .map(mungefs::skel::parse_skel)
                .collect::<std::result::Result<_, _>>()?;
            let classes: Vec<_> = chunks
                .iter()
                .filter(|c| &c.tag == b"entc")
                .filter_map(|c| mungefs::entity::EntityClass::parse(c).ok())
                .collect();
            let dir = root.join(format!("side_{game}/vehicles/{side}"));
            let manifest: manifest::VehicleManifest =
                ron::from_str(&std::fs::read_to_string(dir.join("vehicles.ron"))?)?;
            let mut written = std::collections::HashSet::new();
            for v in manifest.vehicles {
                let Some(w) = v.walker else { continue };
                let Some(model) = w.skin else { continue };
                if !written.insert(model.clone()) {
                    continue;
                }
                let skel = skels
                    .iter()
                    .find(|s| {
                        s.name.eq_ignore_ascii_case(&model)
                            || v.class
                                .geometry
                                .as_ref()
                                .is_some_and(|g| g.eq_ignore_ascii_case(&s.name))
                    })
                    .with_context(|| format!("no skeleton {game} {model}"))?;
                let bank = banks.iter().find(|b| b.name.eq_ignore_ascii_case(&w.bank));
                let class = classes
                    .iter()
                    .find(|c| c.name == v.class.name)
                    .context("class")?;
                let glb = bake::walker::skin_glb(skel, bank, class)?;
                let target = dir.join("walkers").join(format!("{model}.glb"));
                let backup = backups.join(format!("{game}_{side}_{model}.glb"));
                std::fs::create_dir_all(&backups)?;
                if !backup.exists() {
                    std::fs::copy(&target, backup)?;
                }
                std::fs::write(&target, glb)?;
                println!("{}", target.display());
            }
        }
    }
    Ok(())
}
