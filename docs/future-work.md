# Future work

## Metrics beyond Schwarzschild

`gr-core` re-exports `Kerr` and `SuperposedSchwarzschild` from `metric.rs`,
but neither is fully defined. Schwarzschild is the only metric with analytical
Christoffel symbols today; other metrics would fall back to the numerical
finite-difference default until written out.

- **Kerr** — rotating black holes. Likely wants horizon-penetrating
  coordinates (e.g. Kerr-Schild) instead of the current Schwarzschild
  spherical coordinates; the existing `horizon_buffer` workaround is a sign
  the coordinate-singularity issue is already biting.
- **SuperposedSchwarzschild** — intended for binary black holes (or as a
  stress test of the numerical Christoffel path).

## Curved rendering — feature completeness

The curved path has come a long way — it now renders arbitrary `Hittable`
scenes, composes a `Hittable` scene with the accretion disk in one pass, and
applies gravitational redshift. Remaining polish:

- More scene presets beyond `lensed-spheres`.
- A sky-map / environment-texture path for the curved renderer (a `sky`
  module exists; broaden it).
- Tune the integrator (RK4 vs adaptive RK45) selection per render mode.

## Performance

Post-BVH/Quad/cosine-sampling work, the hot path is dominated by BVH `hit`
(~49% of instructions, including the SIMD AABB), `Quad::hit` (~13%), and
`HitRecord` moves (~3.5%). Next targets:

- Reduce `HitRecord` move cost on the hot path.
- Revisit BVH traversal — it's already iterative with an inline-bbox fast
  path, but it's still the single biggest cost.

## Bidirectional path tracing

`bdpt/` holds scaffolding (geometry, MIS) but isn't a working renderer yet.
Finishing BDPT would help the harder light-transport cases the unidirectional
NEE+MIS tracer struggles with.

## Housekeeping

- The root `Cargo.toml` declares `render = { path = "crates/render" }` under
  `[workspace.dependencies]`, but no `crates/render` exists — delete the
  dangling line or create the crate.
- Decide and document whether the front-end stays offline image rendering or
  eventually goes real-time GPU (no `wgpu` dependency exists today).

## Doc debt

`architecture.md`, `setup.md`, and `notes.md` predate the workspace
reorganisation — they describe the old three-stub-crate layout
(`gr-core`/`gr-tracer`/`gr-renderer`) before `gr-tracer` was removed and the
`raytracer` crate became the bulk of the code. `CLAUDE.md` carries the current
structural picture; those docs should be brought in line with it.
