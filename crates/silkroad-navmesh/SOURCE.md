# Source

A frozen copy of `crates/caravan-navmesh` from `~/Workspace/silkroad/caravan`, commit `0ad7987`
(2026-10-21). Re-copied only when a caravan fix is needed; record each re-copy here.

Copied: `Cargo.toml`, `src/{lib,reader,region,object,navmesh,heights}.rs`, unchanged except:
- crate name `caravan-navmesh` -> `silkroad-navmesh` (also in the `cargo test -p` doc lines);
- the test data default (`lib.rs`, `tests::data_path`): `~/Workspace/silkroad/Data` instead of
  `../Data` next to the caravan repo. `CARAVAN_GAME_DATA` still overrides it.

Left out: nothing else in caravan (protocol, import, daemon). Tests that need the game data skip
when it is missing, so CI stays green. Our own additions (step D wall queries) go in new modules.
