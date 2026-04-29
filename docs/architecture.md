# Architecture

The project is laid out as a Cargo workspace with a thin root binary and three
internal crates that mirror the conceptual layers of a GR ray tracer:
**physics core -> ray tracer -> renderer**.

```
relavistic-renderer/                 (root crate, "gargantua")
|-- Cargo.toml                       (workspace manifest)
|-- src/main.rs                      (stub entry point)
`-- crates/
    |-- gr-core/        (1) metric, Christoffel, geodesics
    |-- gr-tracer/      (2) ray generation and integration
    `-- gr-renderer/    (3) camera, image buffer, output
```

Only `gr-core` is implemented today. The other two crates are stubs.

## gr-core (physics layer)

Owns everything that is purely about general relativity. No ray, pixel, or
image concept lives here.

### `metric.rs`

- Type aliases:
  - `SpacetimePoint = Vector4<f64>` -- coordinates `(t, r, theta, phi)`
  - `FourVelocity = Vector4<f64>` -- `dx^mu / d lambda`
  - `MetricTensor = Matrix4<f64>` -- `g_{mu nu}`
  - `ChristoffelSymbols = [[[f64; 4]; 4]; 4]` -- indexed
    `gamma[mu][alpha][beta]`
- `trait Metric: Send + Sync` exposes:
  - `metric_tensor(pos)` -- required
  - `inverse_metric(pos)` -- default uses `try_inverse`
  - `christoffel(pos)` -- default `numerical_christoffel` via central
    finite differences (`h = 1e-6`); concrete metrics override
  - `event_horizon()` -> `Option<f64>` -- defaults to `None`
  - `is_inside_horizon(pos)` -- compares `pos[1]` (r) to `event_horizon()`

### `schwarzschild.rs`

Concrete `Schwarzschild { mass, rs }` (with `rs = 2M`, geometric units).
Provides analytical `metric_tensor`, analytical `christoffel`, and the event
horizon at `r = rs`. The metric tensor is diagonal in `(t, r, theta, phi)`:

```
g_tt   = -(1 - rs/r)
g_rr   =  1/(1 - rs/r)
g_thth =  r^2
g_phph =  r^2 sin^2(theta)
```

A small near-singularity guard clamps `r` to `1e-10`.

### `geodesic.rs`

- `GeodesicState { position, velocity, lambda }`.
- `StepResult { Continue, Horizon, Escaped, Singular }` -- the integrators
  return one of these per step.
- `RK4Integrator` -- fixed step. Aborts with `Escaped` past `max_radius`,
  with `Horizon` once `r < rs + horizon_buffer`.
- `RK45Integrator` -- adaptive Dormand-Prince. Step size adjusts by the
  classic `0.9 * (tol/err)^(1/5)` rule, clamped to `[min_step, max_step]`.
- Both use the same `geodesic_acceleration(gamma, vel)` helper computing
  `d^2 x^mu / d lambda^2 = -Gamma^mu_{alpha beta} v^alpha v^beta`.
- After each accepted step, `normalize_spherical_coords` keeps theta in
  `[0, pi]` and phi in `[0, 2 pi)` (handles the pole crossing by reflecting
  theta and shifting phi by pi).

### Public surface

`crates/gr-core/src/lib.rs` re-exports `GeodesicState`, the two integrators,
`StepResult`, and the metric types. It also re-exports `Kerr` and
`SuperposedSchwarzschild`, which are not yet implemented in `metric.rs`.

## gr-tracer (ray layer, not yet built)

Intended to live between physics and image output: build a camera, generate
photon initial conditions (`GeodesicState` with a null 4-velocity in the
chosen metric), drive the integrator, and report what each ray hit (horizon,
escape to a sky map, accretion disk, etc.). Today: `main.rs` stub and an
empty `ray.rs` placeholder.

## gr-renderer (image layer, not yet built)

Intended to own the pixel grid, sample distribution, parallel scheduling
(rayon is in the dep list), and image output. Today: `main.rs` stub.

## Data flow (target)

```
Scene (metric + emitters)
        |
        v
Camera --> Photon initial conditions  (gr-tracer)
        |
        v
Integrator (RK4 / RK45 in gr-core)
        |
        v
Ray outcome --> Color sample           (gr-tracer)
        |
        v
Image buffer --> file                  (gr-renderer)
```

The contract between layers is the `Metric` trait plus `GeodesicState`.
