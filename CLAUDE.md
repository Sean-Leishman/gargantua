# gargantua

A Rust ray tracer for general relativity: shoots photons through curved
spacetime by integrating the geodesic equation in a chosen metric (currently
Schwarzschild, with hooks for Kerr and a "superposed Schwarzschild" variant).
The intent is to render images of black holes and similar GR scenes, plus
serve as a general-purpose Monte Carlo path tracer. The repo folder is
misspelled `relavistic-renderer`; the upstream remote is
`Sean-Leishman/gargantua.git`.

## Tech stack

- Rust, edition 2024, `rust-version = 1.85`
- Cargo workspace, three internal crates under `crates/`
- `nalgebra` for linear algebra (4-vectors, 4x4 metric tensor, Vec3/Point3)
- `rayon` for tile-parallel rendering and post-processing
- `wide` for portable SIMD (4-wide AABB slab tests)
- `image` for PNG output, `rand` for sampling
- Optional `oidn` (Intel Open Image Denoise) behind the `denoise` feature

## Layout

- `src/main.rs` — root binary, stub `Hello, world!`.
- `crates/gr-core/` — GR physics: `Metric` trait, Christoffel symbols,
  geodesic integrators (RK4, adaptive RK45 / Dormand–Prince). Schwarzschild
  is the only metric with analytical Christoffels today.
- `crates/raytracer/` — the bulk of the code. Flat-space Monte Carlo path
  tracer plus an opt-in curved-space module that delegates to `gr-core`.
- `crates/gr-renderer/` — thin CLI binary over `raytracer::curved` for the
  black-hole renders.

The previously-mentioned `crates/gr-tracer/` has been removed.

## raytracer crate

Cargo features: `flat` (default) and `curved` (pulls in `gr-core` +
`indicatif`); `denoise` adds the optional `oidn` dep.

Module map:
- `core/` — `Color`, `Point3`/`Vec3`/`Ray`, `Aabb` (4-wide SIMD slab via
  `wide::f64x4`, both `hit` and `hit_precomputed`), `Hittable` trait,
  `HitRecord<'a>` (borrows the material from the owning primitive — no
  per-hit `Arc::clone`), `Onb`, `Interval`, `ScatterRecord`.
- `material/` — `Lambertian`, `Metal`, `Dielectric`, `Glossy`, `DiffuseLight`.
  Materials may expose a sampling PDF via `scatter_pdf` (used by NEE/MIS).
- `pdf/` — `CosinePdf`, `HittablePdf`, `MixturePdf`, `SpherePdf`,
  `UniformHemispherePdf`. `CosinePdf::generate` uses Malley's method with
  rejection-based unit-disk sampling (no `sin`/`cos`).
- `shape/` — `Sphere`, `Quad` (parallelogram), `BoxShape` (six quads),
  `ConstantMedium` (volumetric), `Isotropic`. `Quad` precomputes
  `e_alpha = v × w` and `e_beta = w × u` so `hit` is two dots, not two
  cross-and-dots.
- `accel/bvh.rs` — `BvhNode` enum (Internal | Leaf), built with a
  surface-area heuristic. Traversal is **iterative** with a fixed-size
  `[MaybeUninit<&BvhNode>; 64]` stack and a single closest-hit slot;
  Internal nodes store children's bboxes inline so AABB tests don't need
  to dereference the `Box<BvhNode>` until actual descent.
- `scene/` — `World` (flat list, builds a BVH), `LightList` (explicit
  light sampling for NEE).
- `camera/` — `PerspectiveCamera`, `ThinLensCamera`.
- `flat/renderer.rs` — `FlatRenderer` with tile-parallel rendering, optional
  Morton-order tile traversal, stratified or pure-random sampling, NEE +
  balance-heuristic MIS (`render_with_lights` / `render_hdr_with_lights`),
  Russian-roulette termination after 3 bounces, adaptive sampling
  (`render_adaptive`), per-sample firefly clamp (`with_firefly_clamp`),
  exposure + tonemap (None / Reinhard / ACES), optional OIDN denoise pass
  (`with_denoise`, requires the `denoise` feature).
- `bdpt/` — bidirectional path tracing scaffolding (geometry, MIS).
- `output/` — `HdrBuffer` + `ImageBuffer`. Finalize / sRGB encode / bloom
  blur passes are all rayon-parallel.
- `curved/` — feature-gated. `GeodesicRay`, `RayOutcome`, `trace_ray`,
  `render_with_disk`. Wraps `gr-core` integrators behind a renderer-style
  API. **Currently only renders sky + accretion disk** — does not accept
  arbitrary `Hittable` scene objects.

Examples: `cornell_box.rs` and `profile_nee.rs` (a small scene used for
profiling; tunable via `PROFILE_W` / `PROFILE_H` / `PROFILE_SPP` /
`PROFILE_MAX_DEPTH` env vars).

## gr-core crate

- `metric.rs` — `Metric` trait, numerical Christoffel symbols via finite
  differences as a default. Aliases `SpacetimePoint = FourVelocity =
  Vector4<f64>` over `(t, r, θ, φ)`.
- `schwarzschild.rs` — analytical Schwarzschild metric + Christoffels,
  horizon at `r = rs = 2M`.
- `geodesic.rs` — `GeodesicState`, `RK4Integrator`, `RK45Integrator`,
  `StepResult { Continue, Horizon, Escaped, Singular }`.
- Geometric units: `G = c = 1`; `M` sets the scale, `rs = 2M`.

## Profiling

Release profile carries `debug = "line-tables-only"` so callgrind / samply
can resolve symbols without paying full debuginfo cost. `samply` and
`valgrind` are available; `perf` is not (WSL2). Typical loop:

```
cargo build --release -p raytracer --example profile_nee
RAYON_NUM_THREADS=1 PROFILE_W=128 PROFILE_H=128 PROFILE_SPP=8 \
  valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
  --cache-sim=no --branch-sim=no \
  ./target/release/examples/profile_nee
callgrind_annotate --auto=no --threshold=80 --inclusive=no /tmp/cg.out
```

After the recent BVH / Quad / cosine-sampling work, BVH `hit` (incl. the
SIMD AABB) sits at ~49% of instructions; `Quad::hit` ~13%; HitRecord
moves (`manually_drop.rs`) ~3.5%.

## How to run / dev

```
cargo build                                   # default features (flat)
cargo build --features curved -p raytracer    # enable curved-space path
cargo build --features denoise -p raytracer   # OIDN — needs the runtime
cargo test -p raytracer                       # unit tests
cargo test -p gr-core
cargo run -p gr-renderer -- ...               # CLI for black-hole renders
cargo run --release --example cornell_box -p raytracer
```

OIDN requires the `OpenImageDenoise` library on `pkg-config`'s path; the
build error if missing is loud and explains the fix.

## Conventions

- Materials are stored as `Arc<dyn Material>` on the owning primitive;
  `HitRecord` borrows a `&dyn Material` so the hot path doesn't clone Arcs.
- Spacetime coordinates `(t, r, θ, φ)`, `Vector4<f64>`.
- `Metric` provides a numerical Christoffel default; concrete metrics
  override for analytical forms.
- Geometric units: `G = c = 1`, mass `M` sets the scale, `rs = 2M`.
- Integrator step results are typed (`StepResult`) rather than booleans.
- Tonemap / exposure / sRGB encode happens in `HdrBuffer::finalize`; the
  renderer never writes to an `ImageBuffer` mid-pipeline.

## Known gaps

- `Kerr` and `SuperposedSchwarzschild` are referenced from `gr-core` but
  not fully defined.
- `curved::CurvedRenderer` does not yet accept arbitrary `Hittable`
  scenes — only sky + disk. Wiring this up is the next planned step
  ("general rendering through GR").
