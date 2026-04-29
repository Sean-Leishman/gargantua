# gargantua

A Rust ray tracer for general relativity: shoots photons through curved
spacetime by integrating the geodesic equation in a chosen metric (currently
Schwarzschild, with hooks for Kerr and a "superposed Schwarzschild" variant).
The intent is to render images of black holes and similar GR scenes. The repo
folder is misspelled `relavistic-renderer`; the crate is named
`relavistic-renderer` and the upstream remote is `Sean-Leishman/gargantua.git`.

## Tech stack

- Rust, edition 2024, `rust-version = 1.85`
- Cargo workspace with three internal crates under `crates/`
- `nalgebra` for linear algebra (4-vectors, 4x4 metric tensor)
- `rayon` for parallelism (declared, not yet used)
- `num-complex`, `num-traits`, `thiserror`, `rand`, `serde`

## Layout

- `src/main.rs` — root binary, currently a stub `Hello, world!`
- `crates/gr-core/` — physics core: metric trait, Christoffel symbols,
  geodesic integrators (RK4 and adaptive RK45 / Dormand-Prince). The only
  crate with real code today.
- `crates/gr-tracer/` — ray-tracing layer. Has `src/main.rs` (stub) and an
  empty `src/ray.rs`.
- `crates/gr-renderer/` — image / output layer. Stub `main.rs` only.
- `Cargo.toml` — workspace manifest; pins versions in `[workspace.dependencies]`.

## Key files

- `crates/gr-core/src/metric.rs` — `Metric` trait, numerical Christoffel
  symbols via finite differences.
- `crates/gr-core/src/schwarzschild.rs` — analytical Schwarzschild metric and
  Christoffel symbols, event horizon at `r = rs = 2M`.
- `crates/gr-core/src/geodesic.rs` — `GeodesicState`, `RK4Integrator`,
  `RK45Integrator`, `StepResult { Continue, Horizon, Escaped, Singular }`.
- `crates/gr-core/src/lib.rs` — re-exports the public surface (notably
  `Kerr` and `SuperposedSchwarzschild`, which are referenced but not yet
  defined in the metric module — see notes).

## How to run / dev

```
cargo build              # build the workspace
cargo test -p gr-core    # tests for circular-orbit and flat-at-infinity
cargo run                # runs the stub root binary
```

There is no scene file, no CLI, no image output yet. Development is happening
in `gr-core` against the unit tests.

## Conventions noticed

- Spacetime coordinates are `(t, r, theta, phi)`, stored as
  `Vector4<f64>` aliased to `SpacetimePoint` / `FourVelocity`.
- `Metric` provides a numerical Christoffel default; concrete metrics override
  for analytical forms.
- Geometric units: `G = c = 1`, mass `M` sets the scale, `rs = 2M`.
- Integrator step results are typed (`StepResult`) rather than booleans.
- Crate names use hyphens (`gr-core`); the package on disk used to use
  underscores and was just renamed (see `docs/notes.md`).

## Gaps

- `Kerr` and `SuperposedSchwarzschild` are re-exported from `gr-core/src/lib.rs`
  but not actually defined in `metric.rs` — the workspace likely will not
  compile from a clean `cargo build` until those types land.
- `gr-tracer` and `gr-renderer` are skeletons.
- Workspace `Cargo.toml` lists `render = { path = "crates/render" }` but
  no such crate directory exists.
