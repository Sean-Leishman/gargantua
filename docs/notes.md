# Notes

Loose observations. Honest about what is unfinished.

## Naming

- The folder is `relavistic-renderer` (sic -- "relavistic" instead of
  "relativistic"). The Cargo package name matches the folder.
- The git remote is `Sean-Leishman/gargantua.git`. "Gargantua" is the black
  hole in *Interstellar*; the project's actual name is **gargantua**.
- The internal crates (`gr-core`, `gr-renderer`, `gr-tracer`) use `gr` as a
  prefix for "general relativity".

## In-flight changes captured in the same commit as these docs

`git status` at the time of writing showed a half-finished rename plus some
new scaffolding. All of the following were uncommitted:

- The crate directories were renamed from underscore to hyphen names:
  `crates/gr_core/` -> `crates/gr-core/`,
  `crates/gr_renderer/` -> `crates/gr-renderer/`,
  `crates/gr_tracer/` -> `crates/gr-tracer/`.
  Git sees this as deletes of the old underscore paths and untracked new
  hyphen directories.
- `Cargo.toml` was promoted from a single-package manifest to a workspace,
  with `[workspace]`, `[workspace.package]`, and `[workspace.dependencies]`
  added. The members list now points at the hyphen paths.
- New top-level `src/main.rs` (stub `Hello, world!`) and a corresponding
  `Cargo.lock` update.

These were all included in the same commit as the docs because they are the
current state of development, not work-in-progress on a feature branch.

## Known inconsistencies

- `crates/gr-core/src/lib.rs` re-exports `Kerr` and `SuperposedSchwarzschild`
  from `metric`, but those types are not defined in `metric.rs`. A clean
  build will fail until they are added or the re-exports are removed.
- The root `Cargo.toml` declares
  `render = { path = "crates/render" }`
  under `[workspace.dependencies]`, but no `crates/render` directory
  exists. The workspace `members` list does not include `crates/render`,
  so the build does not currently trip over it -- but any crate that tries
  to depend on `render` will fail.
- `crates/gr-tracer/src/ray.rs` exists but is empty.
- `gr-tracer` and `gr-renderer` are pure stubs with `Hello, world!` mains.
- Stale `target/` directories live inside individual crate folders from
  earlier per-crate builds; harmless thanks to `**/target` in `.gitignore`.

## Style observations

- The author leans on type aliases (`SpacetimePoint`, `FourVelocity`,
  `MetricTensor`) to make the physics readable instead of bare
  `Vector4<f64>`.
- The Christoffel array is a triply-nested fixed-size array
  (`[[[f64; 4]; 4]; 4]`) rather than a flat `Vec` -- nice for cache
  behaviour and stack allocation, slightly verbose to index.
- `StepResult` uses an enum (`Continue`, `Horizon`, `Escaped`, `Singular`)
  rather than booleans or sentinel floats. Easy to extend (e.g. a
  `HitDisk` variant when the tracer lands).
- Inline math docstrings use Unicode (Greek letters, partials) which reads
  well in editors but does not render in HTML doc output without help.

## Open questions / things I could not infer

- Whether the eventual front-end is offline image rendering, real-time
  GPU/wgpu, or something else. There is no GPU dependency yet
  (no `wgpu`, no shader directory).
- Whether `SuperposedSchwarzschild` is intended for binary black holes or
  for testing the numerical Christoffel path.
- Whether the chosen coordinates (Schwarzschild spherical) will be replaced
  with horizon-penetrating ones (e.g. Kerr-Schild) before Kerr lands. The
  current code's `horizon_buffer` workaround suggests the author is aware
  of the coordinate-singularity issue.
- Why `serde` is in `[workspace.dependencies]` -- nothing in `gr-core`
  derives it yet; presumably for future scene-file support.

## Ideas that look natural next

- Write `Kerr` and `SuperposedSchwarzschild` so `lib.rs` re-exports compile.
- Either delete the dangling `render = { path = "crates/render" }` line in
  `Cargo.toml` or create the crate.
- Move the geodesic test out of `geodesic.rs` and into an
  `tests/integration.rs`-style file once the integrator's API stabilises.
- Define the `gr-tracer` -> `gr-core` boundary by giving `gr-tracer` a
  `Camera`, a `Photon`, and a `Scene` trait.
