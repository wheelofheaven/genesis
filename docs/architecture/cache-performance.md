# Adaptive terrain receiver query checkpoint

Recorded 12 October 2026 from the existing working-tree implementation and retained raw reports.
This report completes the previously missing README link; it does not attribute that pre-existing optimization to the later publishing turn.

## Workload and result

The workload and machine match [the extraction checkpoint](extraction-performance.md):
Apple M4, macOS, aarch64, Rust 1.93.1, development profile, coast seed 7319,
generator 9, 384 × 320 samples, six dwellings, five paths, no people or economy.
Two warmups precede twenty samples per view; CPU stages execute synchronously without GPU work.
The existing `--extraction-benchmark` reports topology, samples, ground cover,
contacts, horizon and river preparation through observer hooks in `Cache::at_profiled`.
Timings do not affect geometry, world state or scheduling.

| CPU stage / view | Before median | After median | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: |
| Full cache, region | 295.691 ms | 200.009 ms | 374.137 ms | 202.474 ms |
| Full cache, near settlement | 498.093 ms | 401.142 ms | 507.871 ms | 403.719 ms |

River-stage medians changed from 167.172 to 73.416 ms regionally and
190.444 to 92.074 ms near the settlement.
Raw data: [before](measurements/cache-before.json), [after](measurements/cache-after.json).
Other stages remain in those reports; do not treat every difference as caused by this change.
These are individual diagnostic runs, not interactive FPS, release-build or GPU measurements.

## Change and preservation

`Cache::indexed_triangles_in` used to query every finest-lattice cell in a ribbon
triangle's rectangle, then deduplicate coarse patches.
It now skips to patch edges horizontally and the earliest encountered edge vertically.
The sorted receiver IDs, inclusive bounds, negative-edge clamping and geometry remain unchanged.
`terrain/query_tests.rs` compares it against the former exhaustive traversal at mixed LOD,
negative coordinates, map borders, degenerate rectangles and 160 deterministic probes.
All four surface/water presentation counts and vertex fingerprints match between reports.

Reproduce current stage measurements with:

```sh
mise run build
target/debug/genesis-app --extraction-benchmark /private/tmp/genesis-cache.json
```

The publishing checkpoint reran `mise run ci`; current verification is recorded in
[the refactor log](refactor-log.md).
No new dependency, simulation rule, schema, quality change or worker was introduced.
Terrain edits still rebuild the CPU cache/picker globally; chunk upload reuse does not eliminate that work.
Next measure actual committed edits, including picking, extraction and partitioning,
before selecting local invalidation or another preparation optimization.
