# Genesis engine comparison

The [accepted engine decision](../../docs/engine-decision.md) selects the custom
Rust path. This lab preserves the comparison fixture and provides the starting
renderer for migration; Bevy parity is no longer a prerequisite for adoption.

**Partial validation complete:** the custom Rust renderer builds and renders on
Apple M4 / Metal, its native window opens, and both shared-fixture tests pass.
The captured preview is `results/custom.png`. Bevy compilation and visual parity
remain unverified. See the [evaluation](../../docs/engine-evaluation.md) for
observations and limits; no comparative performance claim is established.

This isolated workspace compares a small Bevy application with a direct Rust
`winit`/`wgpu` application. It does not replace Genesis or change the main
workspace. The same static settlement mesh, linear vertex colors, orthographic
camera, back-face culling, and disabled multisampling are used in both views.

The scene contains 12 cuboid buildings and 48 placeholder inhabitants, combined
into one mesh (4,392 vertices). Face lighting is baked into vertex colors.
There is no social simulation, animation, dynamic lighting, outline effect,
selection UI, or asset importer. This is an infrastructure experiment, not the
Sable art study or a population-performance benchmark.

## Run

From the repository root, use the pinned Rust toolchain:

```sh
mise exec -- cargo run --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-custom
mise exec -- cargo run --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-bevy
```

Close the window to exit. The custom view redraws only when requested; Bevy uses
its normal application loop. Their idle CPU use and frame rates must not be
compared as equivalent workloads.

## Capture

Create the destination directory first. The custom renderer captures offscreen;
Bevy opens a window, captures after 60 startup frames, and exits after saving.
It reports failure if the screenshot has not arrived within 30 simulation seconds.

```sh
mkdir -p experiments/engine-lab/results
mise exec -- cargo run --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-custom -- --capture experiments/engine-lab/results/custom.png
mise exec -- cargo run --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-bevy -- --capture experiments/engine-lab/results/bevy.png
```

GPU initialization requires access to Metal; Bevy capture also needs a graphical
session. The shared camera and colors enable visual comparison, not a promise of
pixel identity across different pipelines and render targets.

## Validate

```sh
mise exec -- cargo fmt --manifest-path experiments/engine-lab/Cargo.toml --all -- --check
mise exec -- cargo check --manifest-path experiments/engine-lab/Cargo.toml --workspace --locked
mise exec -- cargo clippy --manifest-path experiments/engine-lab/Cargo.toml --workspace --locked -- -D warnings
mise exec -- cargo test --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-scene --locked
```

## Measure builds

Fetch dependencies before timing, then run the script within `mise exec` so it
inherits the pinned compiler. Run on an otherwise idle machine. Do not edit the
prototype sources while this runs: the script temporarily changes each app's
window-title string and restores the exact original bytes in a `finally` block.

```sh
mise exec -- cargo fetch --manifest-path experiments/engine-lab/Cargo.toml --locked
mise exec -- python3 experiments/engine-lab/measure.py
```

Each backend gets a fresh target directory, using the same development profile.
The script runs serially, records an artifact-cold build, three no-change builds,
and three semantic app edits. Dependency downloads are excluded; filesystem and
OS caches are not flushed. Results and logs go under `results/`; artifacts under
`target/`. It does not delete older results. The script records failures and
uses a per-build timeout to avoid waiting indefinitely on a stalled toolchain.

This measures these small programs, not a mature custom engine versus all of
Bevy. Bevy uses a reduced feature set, static linking, and its standard material
pipeline; dynamic linking is a separate future experiment. The direct renderer
has far fewer capabilities. Both use the wgpu 27 generation. Bevy is pinned to
Genesis's existing 0.18.1 for this baseline, not the latest release.

## Ownership exposed by this experiment

The direct renderer owns GPU and surface initialization, mesh buffers, shader
bindings, pipeline state, camera uniform uploads, depth targets, resize and
surface recovery, PNG readback, and application lifecycle. Bevy supplies those
facilities through plugins and components; the view supplies mesh and camera
configuration plus capture handling.

The next comparison must add an animated imported character, picking, a person
inspector, live material editing, and the same simulation adapter to both paths.
Only then can content workflow and maintenance costs be evaluated fairly.
