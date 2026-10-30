use super::{Listener, LoopRequest, LoopSource, SoundLib};
impl SoundLib {
    pub fn walker_fire_loops(
        &mut self,
        audio: &mut audio::Audio,
        walk: &crate::walk::WalkSim,
        listener: &Listener,
    ) {
        let mut live = std::collections::HashSet::new();
        for (unit, class, name, pos) in crate::walker_shots::loops(walk) {
            let request = LoopRequest {
                source: LoopSource::WalkerFire(unit, class),
                sound: crate::classdb::fnv1a_lowercase(name),
                pos,
                gain: 1.0,
                pitch: 1.0,
            };
            if self.play_loop_voice(audio, &request, listener) {
                live.insert(request.source);
            }
        }
        self.loops
            .retain(|k, _| !matches!(k, LoopSource::WalkerFire(..)) || live.contains(k));
    }
}
