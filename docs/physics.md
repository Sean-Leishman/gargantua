# Physics

What the code in `crates/gr-core` is actually computing, and the conventions
that go with it.

## Units and signature

- Geometric units: `G = c = 1`. Mass is a length; the Schwarzschild radius is
  `rs = 2M`.
- Metric signature: `(-, +, +, +)`. Visible in `Schwarzschild::metric_tensor`
  where `g_tt = -(1 - rs/r)`.
- Coordinates: spherical `(t, r, theta, phi)` with theta in `[0, pi]` (polar
  angle from the +z axis) and phi in `[0, 2 pi)`. The integrator normalizes
  back into these ranges after every accepted step, including a pole-crossing
  fix that reflects theta and rotates phi by pi.

## Geodesic equation

The integrators solve the geodesic equation in first-order form:

```
dx^mu / d lambda      = v^mu
dv^mu / d lambda      = -Gamma^mu_{alpha beta} v^alpha v^beta
```

where `lambda` is an affine parameter. For photons this is the null geodesic
equation; the code does not currently enforce the null condition
`g_{mu nu} v^mu v^nu = 0` -- it is the caller's job to set initial
conditions that satisfy it.

`geodesic_acceleration(gamma, vel)` is the right-hand side of the velocity
equation, written as a triple loop over `(mu, alpha, beta)`.

## Christoffel symbols

The metric trait exposes `christoffel(pos)` returning a
`[[[f64; 4]; 4]; 4]` indexed `gamma[mu][alpha][beta]`.

There are two routes:

1. **Numerical (default).** `numerical_christoffel` in `metric.rs` computes
   `partial_sigma g_{mu nu}` by central differences with `h = 1e-6`, then
   contracts with the inverse metric:
   ```
   Gamma^mu_{alpha beta}
     = (1/2) g^{mu sigma} (
         partial_alpha g_{sigma beta}
       + partial_beta  g_{sigma alpha}
       - partial_sigma g_{alpha beta}
       )
   ```
2. **Analytical override.** `Schwarzschild::christoffel` writes out the
   non-zero components directly. This is faster and avoids finite-difference
   noise near the horizon.

## Schwarzschild metric

`Schwarzschild { mass: M, rs: 2M }`. In `(t, r, theta, phi)` the metric is
diagonal:

```
g_tt   = -(1 - rs/r)
g_rr   =  1 / (1 - rs/r)
g_thth =  r^2
g_phph =  r^2 sin^2(theta)
```

The non-zero analytical Christoffel symbols, all symmetric in their lower
indices, are encoded in `schwarzschild.rs`:

```
Gamma^t_{tr}     = rs / (2 r (r - rs))
Gamma^r_{tt}     = rs (r - rs) / (2 r^3)
Gamma^r_{rr}     = -rs / (2 r (r - rs))
Gamma^r_{thth}   = -(r - rs)
Gamma^r_{phph}   = -(r - rs) sin^2(theta)
Gamma^theta_{r theta}  = 1/r
Gamma^theta_{phph}     = -sin(theta) cos(theta)
Gamma^phi_{r phi}      = 1/r
Gamma^phi_{theta phi}  = cos(theta) / sin(theta)
```

`event_horizon()` returns `Some(rs)`. The integrators stop with
`StepResult::Horizon` once `r < rs + horizon_buffer` (default `0.01`).

## Integrators

- **RK4 fixed step** -- classical 4th-order Runge-Kutta. Default step
  `0.1`, `max_radius = 100.0`, `horizon_buffer = 0.01`.
- **RK45 (Dormand-Prince) adaptive** -- 7-stage embedded RK4(5) with the
  standard tableau. Step adapts via
  `h_new = h * 0.9 * (tol/err)^0.2`, clamped to `[min_step, max_step]`.
  Rejected steps shrink by `(tol/err)^0.25`. Default tolerance `1e-6`.

Both check `is_valid_position` (no NaN, `r > 0`) at each intermediate stage
and bail out with `StepResult::Singular` (RK4) or shrink `h` (RK45) if the
trial position is invalid.

## Test physics

`tests::test_circular_orbit` in `geodesic.rs` exercises a stable circular
orbit at `r = 6M` (the ISCO for Schwarzschild) with angular velocity
`omega = sqrt(M / r^3)`, integrated for one period. The radius is asserted to
drift by less than `0.1` -- a loose-but-honest check that the integrator
preserves the orbit.

## What is not modelled yet

- No Kerr metric (`Kerr` is re-exported from `lib.rs` but not implemented).
- No `SuperposedSchwarzschild` body (binary black hole superposition in the
  weak-field sense, presumably).
- No null-geodesic sanity check on initial conditions.
- No conserved-quantity monitoring (energy, angular momentum, Carter
  constant).
- No accretion disk, sky map, redshift / Doppler shift colouring -- those
  belong in the as-yet-unwritten `gr-tracer` and `gr-renderer` crates.
