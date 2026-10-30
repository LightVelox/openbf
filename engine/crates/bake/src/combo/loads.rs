//! Native ReadDataFile requests identify hashed children of lvl_, not classes.
use super::*;
use std::collections::BTreeMap;

pub(super) fn index(buf: &[u8]) -> Result<BTreeMap<u32, Vec<u32>>> {
    let mut loads = BTreeMap::new();
    for root in mungefs::chunks(mungefs::open(buf)?) {
        let root = root?;
        if &root.tag != b"lvl_" {
            continue;
        }
        for resource in root.children() {
            let resource = resource?;
            if mungefs::CONTENT_TAGS.contains(&&resource.tag) {
                continue;
            }
            let request = u32::from_le_bytes(resource.tag);
            let mut names = Vec::new();
            for child in resource.children() {
                let child = child?;
                if &child.tag == b"comb" {
                    let name = child.child(b"NAME").context("checked combo missing NAME")?;
                    names.push(u32::from_le_bytes(name.payload.try_into()?));
                }
            }
            if loads.insert(request, names).is_some() {
                bail!("duplicate native LVL resource 0x{request:08x}");
            }
        }
    }
    Ok(loads)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        out
    }

    fn combo(hash: u32, state: &str) -> Vec<u8> {
        let mut field = mungefs::fnv1a_lowercase("State").to_le_bytes().to_vec();
        field.push(1);
        field.extend(4u32.to_le_bytes());
        field.extend((state.len() as u32 + 1).to_le_bytes());
        field.extend(state.as_bytes());
        field.push(0);
        chunk(
            b"comb",
            &[chunk(b"NAME", &hash.to_le_bytes()), chunk(b"DATA", &field)].concat(),
        )
    }

    #[test]
    fn native_load_requests_keep_empty_resources_and_combo_record_order() {
        let records = [combo(9, "IDLE"), combo(7, "IDLE"), combo(9, "IDLE")].concat();
        let resources = [
            chunk(&3u32.to_le_bytes(), &records),
            chunk(&5u32.to_le_bytes(), &[]),
        ]
        .concat();
        let bytes = chunk(b"ucfb", &chunk(b"lvl_", &resources));
        let parsed = combos_from_lvl(&bytes).unwrap();
        assert!(parsed.scope_provenance && parsed.load_provenance);
        assert_eq!(parsed.combos.len(), 2);
        assert_eq!(parsed.loads.get(&3), Some(&vec![9, 7, 9]));
        assert_eq!(parsed.loads.get(&5), Some(&Vec::new()));
        assert!(!parsed.loads.contains_key(&7));
        let text = ron::to_string(&parsed).unwrap();
        assert_eq!(ron::from_str::<ComboSet>(&text).unwrap(), parsed);
    }

    #[test]
    fn conflicting_same_side_combo_hash_cannot_be_used_as_a_load_reference() {
        let mut different = combo(7, "IDLE");
        // Different authored state case still names IDLE but changes the tree.
        let at = different.windows(5).position(|s| s == b"IDLE\0").unwrap();
        different[at..at + 4].copy_from_slice(b"idle");
        let resources = [
            chunk(&3u32.to_le_bytes(), &combo(7, "IDLE")),
            chunk(&5u32.to_le_bytes(), &different),
        ]
        .concat();
        let bytes = chunk(b"ucfb", &chunk(b"lvl_", &resources));
        let error = combos_from_lvl(&bytes).unwrap_err();
        assert!(
            error.to_string().contains("different definitions"),
            "{error:#}"
        );
    }
}
