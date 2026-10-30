//! Validation at the boundary where baked figure data becomes gameplay input.
use super::*;

pub(super) fn edge(edge: BladeSegment) -> Result<()> {
    if !edge.start.is_finite() || !edge.end.is_finite() {
        bail!("nonfinite physical blade segment");
    }
    Ok(())
}

pub(super) fn matrix(matrix: Mat4) -> Result<()> {
    if !matrix.is_finite() {
        bail!("nonfinite physical attachment matrix");
    }
    Ok(())
}

pub(super) fn skeleton(skeleton: &Skeleton) -> Result<()> {
    for local in &skeleton.local_bind {
        matrix(*local)?;
    }
    for global in &skeleton.global_bind {
        matrix(*global)?;
        // The palette uses this inverse. Tiny invertible scales remain valid.
        let determinant = global.determinant();
        if !determinant.is_finite() || determinant == 0.0 || !global.inverse().is_finite() {
            bail!("physical skeleton bind has no finite inverse");
        }
    }
    for inverse in &skeleton.inverse_bind {
        matrix(*inverse)?;
    }
    Ok(())
}

pub(super) fn pose(pose: &Pose) -> Result<()> {
    if pose
        .0
        .iter()
        .any(|b| !b.translation.is_finite() || !b.rotation.is_finite())
    {
        bail!("nonfinite physical animation key");
    }
    Ok(())
}

fn fields(fields: &[ComboField]) -> Result<()> {
    for field in fields {
        if field
            .args
            .iter()
            .any(|a| matches!(a, manifest::combo::ComboValue::Number(n) if !n.is_finite()))
        {
            bail!("nonfinite physical combo argument");
        }
        self::fields(&field.children)?;
    }
    Ok(())
}

pub(super) fn class(class: &SaberClass) -> Result<()> {
    fields(&class.combo.fields)?;
    if !class.damage.is_finite() {
        bail!("nonfinite base saber damage");
    }
    matrix(class.weapon_bind)?;
    for segment in class.throw_edges.iter().chain(&class.deflect_edges) {
        edge(*segment)?;
    }
    for binding in &class.throw_bindings {
        if !binding.seconds.is_finite() || binding.seconds < 0.0 {
            bail!("invalid throw sample time");
        }
        matrix(binding.attachment)?;
    }
    for clip in &class.clips {
        if clip.root_speed.iter().any(|v| !v.is_finite())
            || clip.root_frames.iter().any(|v| !v.is_finite())
        {
            bail!("{}: nonfinite physical root profile", clip.name);
        }
        for frame in &clip.frames {
            for segment in frame {
                edge(*segment)?;
            }
        }
        for frame in &clip.weapon_frames {
            matrix(*frame)?;
        }
    }
    if class
        .state_seconds
        .iter()
        .any(|t| !t.is_finite() || *t < 0.0)
    {
        bail!("invalid combo clip duration");
    }
    Ok(())
}
