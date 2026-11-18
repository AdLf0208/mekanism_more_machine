# mekmm slice 1 — validation record

## 0.2.0 — the SDK package (2026-11-18, current)

Pin **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (SDK nightly, `package_format` 6),
mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**. Every command ran through
`ports/tools/build-slot.sh run --job mekmm-sdk --port ports/mekmm --step … -- bash -c 'cd
ports/mekmm && …'` with `BUILD_LOCAL=1` and `CARGO_INCREMENTAL=0`, no `--target-dir`.
Evidence logs: `ports/_shared/evidence/mekmm/sdk-*.log`.

| Step | Evidence | Result |
|---|---|---|
| runtime carried over | `runtime-carryover.txt` | `cmp` exit 0, SHA-256 `94dac43c…52b3`, the packaged entry the same |
| runtime tests before the glue (`lib.rs` = `pub mod runtime;`) | `sdk-runtime-first.log` | `test result: ok. **19 passed**; 0 failed` |
| glue tests (RED, `todo!()`) | `sdk-glue-red.log` | **5 failed**, 20 passed — the glue tests can fail |
| all tests | `sdk-test.log` | `test result: ok. **25 passed**; 0 failed` |
| clippy | `sdk-clippy.log` | `cargo clippy --all-targets --offline -- -D warnings` exit 0, 0 warnings |
| fmt | `sdk-fmt.log` | `cargo fmt --check` clean; `src/runtime.rs` still `cmp`-identical |
| package | `sdk-build.log` | `Built ./target/horizon-mod/mekmm.hmod` |
| second build | `sdk-build-second.log` | **byte-identical**, `55c476c9…f153c` |
| registry check | `sdk-check.log` | `mekmm 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **32,735 B**, **EXIT 0** |
| fits the pinned build | `sdk-fits.log` | `0b14f8a  made for`, **EXIT 0** |
| archive | `sdk-entries.txt` | 25 entries, `LICENSE` + `LICENSE-NOTICE.md` inside, **no `Cargo.lock`** |
| surface | `sdk-uses.json`, `sdk-uses-summary.log` | **265** entries (137 type, 56 handle, 24 method, 20 hook, 20 trait, 5 part, 2 macro, 1 capability) |
| carried content | this record | `diff -r` against the retired `content/`: **identical**, 16 files |
| skills gate | `check-skills.py --notes ports/mekmm --strict` | PASS |

**Deliverable:** `mekmm.hmod`, **32,735 B**, SHA-256
`55c476c91af941d70d4d4c9c7469e5f08b1df1bd36cff2dae2ba09ea7bbf153c`, 25 entries, reproducible
(two builds, `cmp` clean).

## 0.1.0 — the retired `.hnmod` bundle (historical)

*The table below is the retired format's; the logs are kept because the in-game run is the
behaviour's evidence. No 0.2.0 in-game run exists: `horizon-launcher` is not installed here.*

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every step ran
through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0` and no
`--target-dir`; the compile steps forwarded to the second machine. Evidence
logs: `ports/_shared/evidence/mekmm/`.

| Step | Evidence | Result |
|---|---|---|
| runtime tests, before the implementation | `unit-red.log` | exit 101, **110 errors** (the tests' API did not exist) |
| runtime tests | `test-green.log` | `test result: ok. **19 passed**; 0 failed` |
| clippy | `clippy.log` | exit 0, 0 warnings (`--all-targets --locked --offline -D warnings`) |
| fmt | `fmt.log`, `fmt-helper.log` | clean |
| stage | `stage.log` | exit 0, `stage_changed: true` |
| check (staged) | `check.log` | `cargo_exit: 0`, 0 warnings |
| embedded test (staged) | `test.log`, `test-count.log` | `test result: ok. **19 passed**; 0 failed; 1240 filtered out`, `cargo_exit: 0` |
| fast build | `build.log` | `cargo_exit: 0` |
| content + native package | `package.log` | `.hmod` 6,830 B, 17 entries, `2035b553…12e50d0`; `.hnmod` 58,547 B, 13 entries, `31b43c0b…99418abf` (byte-identical on both machines); `cargo horizon-mod check` passes |
| e2e, one `.hnmod`, no `--dev-mod`, control leg first | `e2e-embedded.log` | **PASS**, `EXIT:0` |
| fail-capability | `e2e-embedded-selftest.log` | **FAIL** as required, `EXIT:1` |
| e2e log gate | `check-e2e-logs.py mekmm --strict` | exit 0 (2 logs, both with an EXIT line) |
| skills gate | `check-skills.py --notes ports/mekmm` | PASS |
| upstream readiness | `upstream_ready.py mekmm` | exit 1 — ledger row `pending` (orchestrator) and parent `mekanism` not yet in the upstream registry; see `notes.md` |

## What the unit tests pin

19 tests in `src/runtime.rs` (unchanged): the jar's constants (200 ticks, 50 FE/t,
20,000 FE, 3 processes), 10,000 FE per operation, one tick = 50 FE and one
progress tick, the input consumed only on the 200th tick, three processes
sharing the buffer (10,000 FE stalls at `[67, 67, 66]`), each process on its own
recipe, `NoRecipe`/`NoInput`/`NotEnoughInput` resetting the progress with no
energy spent, `NoEnergy`/`NoOutputSpace` holding it, a missed roll consuming the
input anyway, the 17 % roll over 10,000 seeds and the distinct per-process/per-
cycle seeds, the 11-row recycling table, one item per operation from a stack,
the buffer clamp and the `from_saved` padding/round trip. 6 more in
`src/basic_recycling_factory.rs`: the nine-slot container (three inputs, three
outputs, every face, the 3×3 menu), the input-only buffer, the empty compound, the
`energy`/`progress`/`cycles` round trip, the chance override's two pinned values,
and the exact `/mekmm info` line.

## What the in-game check pins

The exact lines are quoted in `notes.md` ("The in-game check", historical 0.1.0
run): the 3-process stall `[67, 67, 66]` on 10,000 FE, the 20,000 FE top-up
finishing three operations, one operation costing exactly 10,000 FE, the
no-energy, no-recipe, full-output and missed-roll controls, the nine-slot menu,
and a stalled factory's energy, progress, cycles and slots surviving save +
relaunch. `tools/e2e.sh` keeps every assertion and adds the one-package control
(`CONTROL_GAME=`: an unmodified build has no block and no `/mekmm`).
