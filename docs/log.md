# Log

Derived from git history, newest first. The arc, not every commit.

## Curved rendering matures

- **`curved`: sky module** — adds a sky/environment path; renderer and example
  updates.
- **`curved`: gravitational redshift** — `g⁴` redshift applied on `Hittable`
  scene hits in the curved renderer.
- **`gr-core`: `geodesic_acceleration` override** — `Metric` gains an
  overridable `geodesic_acceleration`; Schwarzschild inlines it. −28% Ir,
  ~8× faster on small renders.
- **`gr-renderer`: `--scene` flag** — a `lensed-spheres` preset, composable
  with `--disk`.
- **`curved`: compose disk + `Hittable` scene** in one render path.
- **`curved`: render arbitrary `Hittable` scenes** — the curved renderer is no
  longer sky+disk only.

## Flat path tracer — quality and performance

- **Firefly clamp + OIDN denoise feature** — per-sample firefly clamp,
  optional Intel Open Image Denoise pass behind the `denoise` feature, output
  pipeline parallelised.
- **Iterative BVH + precomputed Quad + rejection-sampled cosine PDF** —
  iterative BVH traversal with a fixed-size stack and inline child bboxes;
  `Quad` precomputes `e_alpha`/`e_beta`; `CosinePdf` uses Malley's method with
  rejection-sampled unit-disk sampling (no `sin`/`cos`). ~16% fewer
  instructions.
- **`ImageBuffer::save_png`** — the `cornell_box` example saves PNG directly.

## Workspace reorganisation

- **Remove the `gr-tracer` crate** — its responsibilities folded into the
  `raytracer` crate's `curved` module.
- **`gr-renderer` becomes a thin CLI** over `raytracer::curved`.
- **Port the curved renderer into `raytracer`** — geodesic tracer,
  `AccretionDisk`, `RayOutcome`, and visual shading moved into a feature-gated
  `curved` module; `curved` feature scaffolding added.
- **Import the flat path tracer from `unified_solver`** — the Monte Carlo path
  tracer (BVH, materials, PDFs, shapes, camera, NEE+MIS) was lifted in from
  the sibling `unified_solver` project; `cornell_box` example added.

## Earlier physics core

- `gr-core` established the `Metric` trait, Christoffel symbols (analytical
  for Schwarzschild, numerical finite-difference default otherwise), and the
  RK4 / adaptive RK45 geodesic integrators with typed `StepResult`s.
- Renderer routes `MaxSteps` to sky color; sky gradient uses the up-axis.
- `.gitignore` covers `**/target` and rendered images.

> Note: `architecture.md` / `setup.md` / `notes.md` were written against the
> pre-reorganisation three-stub-crate layout and are stale — see
> `future-work.md`.
