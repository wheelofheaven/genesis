# Decision: staged terrain commands with committed outcomes

Accepted 12 October 2026 after the durable-identity checkpoint.

## Authority and commit

`rules::try_apply_terraform` validates and stages a brush in simulation code.
It rejects nonfinite strength, empty grids and bounded-world brushes that do not
intersect any sample. Partially intersecting bounded brushes retain cropping;
wrapped worlds retain modulo coordinates. Coordinates and offsets use widened
integer arithmetic so extreme action coordinates cannot overflow.

The operation stages a sparse patch keyed by canonical sample index. Repeated
visits in a wrapped brush accumulate in the original stencil order with the
original per-visit elevation clamp. Radius still caps at the largest grid axis,
falloff remains unchanged, and zero radius still edits the center. Floating-point
representation is preserved, including signed zero. Staging has at most one entry
per sample and does not clone the world. No plan escapes the synchronous call;
there is no stale plan or new background queue.

Only after staging succeeds does the patch commit. The reported changed count
counts distinct final changed samples, not stencil visits. Later same-tick
commands observe the complete new elevation field. As before, water depth,
climate, life and subsurface systems update after the ordered action batch;
extraction and saves occur only after the completed logical tick. Rejection has
no effects attributable to the rejected command; it does not suppress normal
elapsed-time effects or undo earlier accepted commands in the batch.

`SimState::step` emits `TerrainChanged`, `TerrainUnchanged`, or
`TerraformRejected`, with requested coordinates and a changed count or reason as
applicable. The legacy `apply_terraform` setup helper remains a wrapper for
callers that do not consume events. Live app/headless paths use the same fallible
operation through actions. The bounded eight-event UI history remains disposable.

The app advances its terrain geometry revision and requests urgent rebuilding
only for `TerrainChanged`. Invalid and unchanged terrain intent still updates UI
feedback, but does not invalidate geometry/picking or discard a worker solely
because that terrain action was queued. Normal periodic climate/color extraction
continues. Existing whole-view revisions invalidate the entire affected cache,
including interpolation neighborhoods; no new partial-chunk invalidation scheme
is introduced. Multiple committed edits in one tick may increment revision more
than once; revisions are transient invalidation generations, not domain time.

## Preserved model and limits

Environmental samples are interpolated with a 4×4 cubic B-spline and a finer
physical surface lattice. Buildings and paths retain identity, horizontal
position and geometry, with ground heights derived from the updated surface.
Exposed building foundations already have presentation support. This slice does
not introduce demolition, structural damage, blocked routes or protected zones.

Generated river routes, widths, rainfall and drainage are scenario data.
Channel surface levels and incision query current ground and update after edits;
route/connectivity indexes remain valid because their source polylines are
unchanged. They are not a live discharge solver. Upstream edits do not recompute
catchments or redirect rivers. Those capabilities require a separate model and
product decision, not cache rebuilding alone.

Sea depth is derived from a prescribed sea level, representing an open reservoir;
it is not a conserved finite water volume. No displaced surface water is modeled.
Aquifer datum/capacity/stored quantities remain separate, and existing wells stop
withdrawing when submerged. Future foundations, navigation, water redistribution
or resource costs must validate coupled changes before this commit point.

## Compatibility and validation

No saved domain fields, schema, content markers, or normal numerical rules change.
V1 migration and V2 saves remain supported; queued terrain intent still executes
once at its saved next tick. New event variants affect event consumers, not the
world save format. Empty/outside/nonfinite requests now have an explicit outcome
where they previously had none; extreme bounded coordinates no longer risk
integer overflow. No cross-build event-log replay promise is made.

Tests cover wrapping repeated visits, clipped/negative boundaries, extreme
coordinates, finite limits, no partial effects, same-tick placement ordering,
existing foundations/channel queries, pending save continuation and host revision
behavior. Native smoke and fixture comparison evidence belongs in the checkpoint
log. The patch adds temporary allocation and tree lookup work; this is a command
correctness change, not a performance improvement. No throughput claim is made.
