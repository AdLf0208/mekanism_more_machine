# mekmm — validation record

## 0.2.2 — the recycling factory's four tiers (2026-11-21, current)

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (SDK Nightly 2026-11-20,
`package_format` 6), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**. Every command
ran through `ports/tools/build-slot.sh run --job W1-mekmm --port ports/mekmm --step … --
bash -c 'cd ports/mekmm && …'` with `BUILD_LOCAL=1` and `CARGO_INCREMENTAL=0`, no
`--target-dir`, no private slot dir. The SDK was re-fetched for this pin (`.horizon/sdk`
deleted first: a repin does not refresh it). Evidence: `ports/_shared/evidence/mekmm/`.

| Step | Evidence | Result |
|---|---|---|
| tier rules (RED) | `sdk-tier-red.log` | `test result: FAILED. 11 passed; 24 failed`, e.g. `the_four_tiers_are_the_jars_process_counts` → `the tier's process count` |
| tier declarations (RED) | `sdk-tier-glue-red.log` | `test result: FAILED. 31 passed; 4 failed`, e.g. `every_tier_opens_the_smallest_vanilla_grid_that_holds_its_slots` → `not yet implemented: a tier's container` |
| all tests | `sdk-test.log` | `test result: ok. **35 passed**; 0 failed; 0 ignored` |
| clippy | `sdk-clippy.log` | `cargo clippy --all-targets --offline -- -D warnings` exit 0, **0 warnings** |
| fmt | `sdk-fmt.log` | `cargo fmt --check` clean (`--all` would format the fetched SDK) |
| package, build 1 | `sdk-build.log` | `SDK 0a875da … in ./.horizon/sdk`, `Built ./target/horizon-mod/mekmm.hmod`, then copied to the port root |
| package, build 2 | `sdk-build-second.log` | **byte-identical**, `cmp` exit 0, both **44,260 B** |
| registry check | `sdk-check.log` | `mekmm 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 44,260 B, SHA-256 `086d1d51…bf1f`, **EXIT 0** |
| fits the pinned build | `sdk-fits.log` | `0a875da  made for`, **EXIT 0** (a tautology: kept because the brief asks for it) |
| **fits a different build** | `sdk-fits-other.log` | `0b14f8a  fits`, **EXIT 0** (the real comparison) |
| archive | `sdk-entries.txt` | **43 entries**, `LICENSE` + `LICENSE-NOTICE.md` + `README.md` inside, **no `Cargo.lock`** |
| surface | `sdk-uses.json`, `sdk-uses-summary.log` | **261** entries (125 type, 59 handle, 26 method, 21 hook, 22 trait, 5 part, 2 macro, 1 capability) |
| package gate | `verify-sdk-package.py mekmm` | **1/1** |
| in-game, PASS | `mekmm-e2e/e2e-launcher.log` | **PASS** + `EXIT:0` |
| in-game, control | `mekmm-e2e/e2e-launcher-control.log` | `FAIL: expected /mekmm info at 8,64,8 … got ''` + `EXIT:1` (the whole scenario on the Nightly without the package) |
| in-game, counterfactual | `mekmm-e2e/e2e-launcher-selftest.log` | `SELFTEST=1` → `FAIL` + `EXIT:1` |
| e2e log gate | `check-e2e-logs.py mekmm-e2e --strict` | `logs=3 ok=3`, exit 0 |
| skills gate | `check-skills.py --notes ports/mekmm --strict` | PASS |

**Deliverable:** `mekmm.hmod`, **44,260 B**, SHA-256
`086d1d51d7da4bda8f20fe69a0ca92f94b319f57b35f7d9ab6eb7eefbdb0bf1f`, **43 entries**,
reproducible (two builds, `cmp` clean).

## What the 35 unit tests pin

19 from 0.1.0's runtime (the jar's constants, 10,000 FE per operation, the input consumed only
on the 200th tick, the 67/67/66 stall, each process's own recipe, `NoRecipe`/`NoInput`/
`NotEnoughInput` resetting with no energy spent, `NoEnergy`/`NoOutputSpace` holding, a missed
roll consuming the input anyway, the 17 % roll over 10,000 seeds, distinct per-process/cycle
seeds, the 11-row table, the buffer clamp, padding and round trip), 10 new:

- the four tiers' process counts, ids, block ids and display names;
- the factory buffer formula `max(storage/2, usage) × processes` and the four buffers
  (30,000 / 50,000 / 70,000 / 90,000), with the usage floor and the empty case;
- `Tier::from_block` for the four blocks and `None` for five other ids;
- the exact stall each higher tier reaches on 10,000 FE (`[40×5]`, `[29,29,29,29,28,28,28]`,
  `[23,23,22×7]`), and that each finishes every process on exactly the remaining energy
  (40,000 / 60,000 / 80,000) with one scrap per process;
- a tier's factory ticks exactly its process count and spends 50 FE per process;
- saved arrays padded/clamped to the tier (advanced: five slots, the 50,000 FE cap).

6 in `src/recycling_factory.rs`: each tier's grid and menu (9/18/18/27) with one input and one
output per process and every face reaching every slot; each tier's input-only buffer; a fresh
compound per tier; the compound round trip at advanced; the chance override's two pinned
values; and the exact `/mekmm info` line for the basic (30,000 FE) and ultimate (90,000 FE)
tiers.

## What the in-game leg pins

Quoted in `notes.md` ("Slice 2 → The in-game leg"): each tier's own buffer and process count,
the three exact stalls on 10,000 FE, each tier finishing on exactly its remaining energy with
one scrap per process, the four menu grids (9/18/18/27 slots), the basic tier's four controls
and the 67/67/66 sharing, and a stalled factory's energy, progress, cycles and slots surviving
save + relaunch. The control leg places the same block ids as air in an unmodified Nightly and
gets no `/mekmm` answer; `CONTROL=0 GAME=$CONTROL_GAME` fails the scenario at its first
assertion; `SELFTEST=1` fails on the first operation's output.

## 0.2.1 — review fixes (2026-11-19, historical)

34,306 B, sha256 `b72a04ac…1e42`; the `info` reach/operator rule and the changed-output write.
Ladder: 27 tests, clippy 0, fmt clean, two identical builds, `check`/`fits` 0, leg `PASS` +
`EXIT:0`, control `FAIL` + `EXIT:1`.

## 0.2.0 — the SDK package (2026-11-18, historical)

Pin `0b14f8a` / mixins `e59da90`: runtime carried byte-identically (`94dac43c…52b3`), 19 tests
before the glue, 5 glue tests red then 25 green, clippy/fmt clean, two identical builds,
`check`/`fits` 0, 25 entries, `32,735 B`, `55c476c9…f153c`. **No in-game run** (the launcher
was not installed then).

## 0.1.0 — the retired `.hnmod` bundle (2026-11-11, historical)

Pin `1d63e0a` / mixins `5a64af9`: 19 runtime tests, clippy/fmt, stage/check/test/build, the
`.hnmod` reproducible on both machines, `e2e-embedded.log` **PASS + EXIT:0** and
`e2e-embedded-selftest.log` **FAIL + EXIT:1**. Kept because that run is the behaviour's first
in-game evidence; the logs are in `ports/_shared/evidence/mekmm/retired-0.1.0-hnmod/`.
