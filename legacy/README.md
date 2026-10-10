# Legacy Bevy prototype

This separate workspace preserves the earlier Bevy application, renderer, and
planet crate as a reference. New engine development belongs in the root workspace.

From the repository root:

```sh
mise run legacy-run
```

It continues to use the shared `../crates/sim_core` and root `../assets` directory.
Its manifest and asset path were adjusted for this move; the Bevy implementation
is otherwise preserved. The migration checked Cargo metadata, not a fresh native
Bevy build. The custom application does not depend on these crates.
