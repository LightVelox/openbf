//! The render crate's integration tests (the GPU referees), linked as ONE test binary (card WEARY, SSD wear): every
//! file directly under `tests/` is its own binary that links the renderer and wgpu, so the files live here as
//! modules instead, with their shared fixtures in `common`. A test's id moves from `<file> <name>` to
//! `suite <file>::<name>`. `scripts/gpu_tests.sh` still runs them serialised (`--test-threads=1`).
//! A new test file goes in this directory with a `mod` line below.

mod common;

mod alpha_blend;
mod detail_map;
mod frame_filter;
mod gpu_smoke;
mod light_path;
mod lod_switch;
mod particle_blend;
mod skinned;
mod sky_shell;
mod specular;
mod uv_scroll;
mod water_pass;
