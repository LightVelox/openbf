//! Narrow corrections to shipped gameplay data; raw parsing stays lossless.
use super::*;

pub(super) fn vader_terminal_target(combo: &mut ComboDef) {
    // PC and Xbox IMP.LVL both explicitly return Vader's second attack to
    // IDLE. PS2 names missing RECOVER2; its loader resolves that to IDLE too.
    // Correct the label, without inventing a recovery state or importing the
    // donors' four unrelated MustShowOneFrame flags (analysis/re-combo.md).
    if combo.name_hash != 0xe4c0_31da || combo.state("RECOVER2").is_some() {
        return;
    }
    let Some(state) = combo.fields.iter_mut().find(|field| {
        field.keyword == K::State
            && field
                .text(0)
                .is_some_and(|name| name.eq_ignore_ascii_case("ATTACK2"))
    }) else {
        return;
    };
    let Some(terminal) = state
        .children
        .iter_mut()
        .rfind(|field| field.keyword == K::Transition)
    else {
        return;
    };
    // Match this shipped label exactly, not a generalized typo correction.
    if terminal.args.len() == 1
        && terminal.text(0) == Some("RECOVER2")
        && !terminal.scoped
        && terminal.children.is_empty()
    {
        terminal.args[0] = ComboValue::Text("IDLE".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(keyword: K, name: &str, children: Vec<ComboField>) -> ComboField {
        ComboField {
            keyword,
            args: vec![ComboValue::Text(name.into())],
            scoped: keyword == K::State,
            children,
        }
    }

    fn program(hash: u32, target: &str, recovery: bool) -> ComboDef {
        let mut fields = vec![
            field(K::State, "IDLE", vec![]),
            field(
                K::State,
                "ATTACK2",
                vec![field(K::Transition, target, vec![])],
            ),
        ];
        if recovery {
            fields.push(field(K::State, "RECOVER2", vec![]));
        }
        ComboDef {
            name_hash: hash,
            fields,
        }
    }

    #[test]
    fn the_shipped_correction_is_idempotent() {
        let mut raw = program(0xe4c0_31da, "RECOVER2", false);
        let corrected = program(0xe4c0_31da, "IDLE", false);
        vader_terminal_target(&mut raw);
        assert_eq!(raw, corrected);
        vader_terminal_target(&mut raw);
        assert_eq!(raw, corrected);
    }

    #[test]
    fn valid_recovery_other_programs_and_other_labels_are_untouched() {
        for mut source in [
            program(0xe4c0_31da, "RECOVER2", true),
            program(0xa11a_0085, "RECOVER2", false),
            program(0xe4c0_31da, "IDLE", false),
            program(0xe4c0_31da, "recover2", false),
        ] {
            let before = source.clone();
            vader_terminal_target(&mut source);
            assert_eq!(source, before);
        }
    }
}
