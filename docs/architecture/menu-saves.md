# Save and resume through the expedition menus

21 October 2026, following the irrigation checkpoint.

The pause menu offers **Save expedition** and **Load saved expedition**.
The title screen offers **Load saved expedition** across application launches.
**Continue expedition** still resumes the live, in-memory world.
Loading over an open world requires confirmation; cancellation preserves that world and its queued actions.
Successful loading opens the saved world paused and resets the camera and selection.
When buildings exist, it focuses on the settlement; unbuilt worlds use the regional overview.

The quit dialog offers **Save and quit**, **Quit without saving**, and **Stay here**.
Replacing a live expedition likewise offers **Save and deploy** or deployment without saving.
A failed save leaves the dialog and live world open; it cannot trigger quitting or generation.
The old claims that saving was unavailable have been removed.

## Storage ownership

Menu actions call the same immutable session snapshot, staged validation and atomic writer as F5/F9.
`frontend/saves.rs` owns navigation and feedback; `persistence.rs` resolves the native slot and shared save/load helpers.
Simulation advancement and filesystem access remain separate.
Rendering a menu performs no save or load.
Saving does not consume pending actions or advance time.
Failed loading preserves world identity, mode, pending actions, camera and presentation revision.
Successful loading advances that revision so stale view work cannot replace the restored world.

The single slot remains `~/Library/Application Support/Genesis/world.json` on macOS.
Each successful replacement retains the previous valid primary as `world.json.bak`.
The existing writer refuses to overwrite a corrupt primary; the error remains visible and the live world stays available.
Backup recovery and alternate filenames remain filesystem/headless operations, not menu controls in this slice.
There is no autosave, slot browser or asynchronous save worker.
Encoding and loading can pause the interface for large worlds, as with the existing shortcuts.

## Expedition role and compatibility

Schema 8 adds optional `expedition_mode` metadata with `Settlement` and `WorldBuilder` values.
The typed role lives in `sim_core::catalog`, separately from population and world geometry.
Application saves record the current session's role, including when a different role is being chosen for the next expedition.
Loading restores that role so later region regeneration retains the correct starting mode.
The headless runner preserves a loaded role when resaving; its own scenarios have no player role.

V1–V7 migration leaves the role unspecified instead of inferring it from residents or buildings.
For those files, the app retains its current session role; a fresh application defaults to Settlement.
Old World Builder saves retain their world contents, but cannot recover role information that was never stored.
Rules remain `genesis:rules/6`; simulation behavior, content identity and generator version are unchanged by this metadata addition.
Unknown roles and role fields injected into old schema versions are rejected.
The pre-change V7 fixture contains active irrigation at tick 500 and verifies exact domain-state preservation.

## Verification

Real egui input tests cover saving, confirmation/cancellation, fresh-launch loading,
pending irrigation execution exactly once, mode restoration, failed saves, corrupt/missing loads,
backup preservation, and save-and-quit.
Compact layout checks verify the save/load controls remain visible at 563×400,
760×540 and 1280×800 logical pixels, without writing files or advancing the world.
Storage tests cover V7 migration and all role values.

The native `--menu-test` uses a capture-sibling `.menu.world.json` slot.
It saves the deployed world with pending intent, cancels a replacement load,
models a fresh application's placeholder session, then loads and compares the restored world and identity.
It captures the load confirmation and paused restored HUD as part of the existing expedition/menu checks.
User save slots are not used by captures.
