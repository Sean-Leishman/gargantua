# Usage

> **Naming:** the on-disk folder is `relavistic-renderer` (sic); the project's
> real name is **gargantua** and the git remote is `Sean-Leishman/gargantua`.

The workspace builds three crates. Day-to-day you use two binaries: the
`cornell_box` example (flat path tracer) and the `gr-renderer` CLI
(curved-space black-hole renders).

## Build

```
cargo build                                   # default features — flat path tracer only
cargo build --features curved -p raytracer    # enable the curved-space (GR) path
cargo build --features denoise -p raytracer   # add the optional OIDN denoise pass
```

`denoise` needs the Intel Open Image Denoise (`OpenImageDenoise`) library on
`pkg-config`'s path — the build error if it's missing is loud and explains the
fix.

## Flat path tracer

The `cornell_box` example is the worked scene for the flat (Euclidean) Monte
Carlo path tracer:

```
cargo run --release --example cornell_box -p raytracer
```

It writes a PNG directly. The renderer supports tile-parallel rendering,
optional Morton-order tiles, stratified or pure-random sampling, NEE +
balance-heuristic MIS, Russian-roulette termination after 3 bounces, adaptive
sampling, a per-sample firefly clamp, exposure + tonemap (None / Reinhard /
ACES), and an optional OIDN denoise pass.

## Curved-space renderer

`gr-renderer` is a thin CLI over `raytracer::curved`:

```
cargo run --features curved -p gr-renderer -- --disk
cargo run --features curved -p gr-renderer -- --scene lensed-spheres
cargo run --features curved -p gr-renderer -- --scene lensed-spheres --disk
```

`--disk` renders the accretion disk; `--scene` selects a `Hittable` scene
preset; the two compose in a single render path. Curved renders include
gravitational redshift (`g⁴`) on scene hits. The metric is Schwarzschild
(`G = c = 1`, `rs = 2M`).

## Tests

```
cargo test -p raytracer
cargo test -p gr-core           # Schwarzschild → Minkowski at large r; ISCO orbit at r = 6M
```

## Profiling

The release profile carries `debug = "line-tables-only"` so callgrind / samply
resolve symbols cheaply. Typical loop with the `profile_nee` example:

```
cargo build --release -p raytracer --example profile_nee
RAYON_NUM_THREADS=1 PROFILE_W=128 PROFILE_H=128 PROFILE_SPP=8 \
  valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
  --cache-sim=no --branch-sim=no ./target/release/examples/profile_nee
callgrind_annotate --auto=no --threshold=80 --inclusive=no /tmp/cg.out
```

`profile_nee` is tunable via the `PROFILE_W` / `PROFILE_H` / `PROFILE_SPP` /
`PROFILE_MAX_DEPTH` env vars. `perf` is unavailable under WSL2; `samply` and
`valgrind` work.

For the physics behind the curved path — the metric, Christoffel symbols, and
geodesic integrators — see `physics.md`.
