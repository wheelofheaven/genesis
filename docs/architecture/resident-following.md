# Resident following

21 October 2026, following the household supply outlook.

## Player path

Water and Food overview rows offer **Inspect [name]** for each household resident.
Opening that link selects the resident, centers the camera and shows their current
activity, destination, resource commitments and highlighted route.
Dwelling inspectors also list their residents as links; a resident's **Home**
button returns to their dwelling.

**Follow resident** keeps the camera centered as the person moves.
**Stop following** leaves the camera at its current position and retains selection.
Orbit, north orientation and zoom remain available; wheel zoom centers on the
resident while following rather than the pointer.
WASD/arrows, drag panning, selecting anything else, changing tools, closing the
inspector, and F/R/G stop following.
The inspector maintains a separate scroll position for each page and world, so
opening a resident from the bottom of Supplies exposes the follow control.

## Ownership and behavior

`Session` owns an optional persistent person identity as transient presentation
state. `session/follow.rs` resolves that identity against the active world each
frame, after clock and keyboard camera updates.
Camera focus uses the same `person_position` interpolation as resident geometry
and picking, then samples the visible terrain surface.
Zoom and orbit are retained; following never changes a resident's route or intent.

Pause holds the resident and camera still; single stepping tracks the next committed
position. Opening a menu suspends updates until play resumes.
Changing selection or losing the person clears tracking.
World replacement and successful loading reset it even when the new world reuses
the same numeric resident ID. Failed loading retains the active session and target.
Saving neither persists tracking nor interrupts it.
No domain fields, actions, rules or save metadata change: schema 10 / rules 8 remain.

## Validation

Session tests compare followed and unfollowed simulations, check interpolated
motion, pause/single step, preserved orbit/zoom, navigation/tool release, replacement,
and successful/failed loads.
Real egui pointer tests cover both supply tabs, following/stopping, home/resident
links, and the final household link in a scrolled compact panel.
The tests also assert unchanged paused simulation state and no queued commands.

The native `--people-test output.png` route retains generated-resident picking and
save continuity checks, then starts following and verifies camera motion across
eight simulation steps. Save artifacts use temporary paths beside the capture,
separate from the player's slot.
