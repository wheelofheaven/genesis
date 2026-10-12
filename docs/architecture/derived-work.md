# Decision: world replacement and terrain preparation ownership

Accepted 12 October 2026 after staged terrain commands.

## Preserve the existing guards

The inspected implementation already prevented stale terrain publication with
terrain revision, camera LOD key, survey settings and a view-local edit epoch.
Workers already consumed an owned simulation snapshot and returned disposable
CPU data. No stale-world overwrite was reproduced. This slice makes that
contract enforceable in one place and tests transitions with delayed results;
it does not introduce an asynchronous simulation or a replacement job system.

`Session::replace` now centralizes activation of a replacement session. Generated
worlds enter through `replace_sim`; regeneration and frontend arrival use that
method. A successful staged save load uses the same activation boundary after
restoring time, identity and pending intent. Every activation advances the
existing terrain revision, including reloading the same persistent world ID.
Failed loads never reach activation and keep the valid world and its revision.
Initial executable setup still constructs sessions before a view exists.

Terrain revision serves as the combined world/terrain invalidation generation
for this single-view host. The persistent save identity remains separate.
The view's edit epoch additionally invalidates asynchronous work after immediate
publication, even when other keys match. Adding another world epoch would
currently duplicate these guards. Neither transient generation is serialized.

## Bounded preparation and delivery

`view::preparation::Preparation` owns one optional pending request, including a
completed result not yet consumed. `request` refuses additional work before
cloning simulation input. The worker's result channel has capacity one. There
is no backlog of waiting input snapshots, coalesced deltas or retained history.
This preserves the old single-request behavior and makes its capacity explicit.

The owner exposes `poll(current_tag, urgent)` rather than a raw result receiver.
It returns ready data only if all source/view keys still match and no urgent
publication supersedes the work. A stale or disconnected completion is discarded,
releases the slot and asks the view to refresh. An unfinished stale worker keeps
the slot until it completes; repeated world loads cannot detach it and spawn an
unbounded sequence of workers. The latest view can request work after that slot
is released. A worker that never finishes still occupies the slot; no forceful
thread cancellation or timeout policy is introduced.

The renderer consumes accepted prepared geometry, cache and optional picker.
Fresh geometry carries a matching picker; color-only work reuses the existing
cache and leaves picking alone. Terrain revision changes still rebuild current
terrain/picking synchronously and invalidate contacts, scenery and dwelling
placements. Urgent edits/replacement publication remains synchronous, so waiting
for obsolete disposable work cannot block logical simulation advancement or make
world replacement depend on its completion time.

CPU preparation carries no GPU resource handles. Device-generation recovery is
not implemented by this slice. The worker bound applies to one live view;
renderer/device recreation remains a separate lifecycle concern.

## Evidence and compatibility

Tests exercise the production admission/completion owner. A real old-world
prepared result is held at a controlled delivery boundary while a different save
is loaded repeatedly. Every attempted extra request is refused, including when
the old completion is buffered but unconsumed. The old result is discarded; the
next worker produces the new height and a matching oblique picking result.
World state remains untouched by preparation.

Other cases cover failed loads retaining valid work, generated replacement,
changed camera/survey/edit epoch, urgent publication, disconnected workers and
reused color caches. Existing frontend arrival and session tests remain in place.
Native load/regeneration results and command outcomes are recorded in the
checkpoint log. The controlled delivery test models late completion deterministically;
native smoke is not claimed to force a particular worker race.

No domain rule, timing, save schema, content identity, or dependency changed.
Whole-world worker snapshots and synchronous urgent extraction remain; this is
not a measured memory or frame-rate optimization. No performance improvement is
claimed. A later optimization must use an actual extraction workload and compare
before/after measurements without weakening this delivery contract.
