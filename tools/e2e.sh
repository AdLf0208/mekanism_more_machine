#!/usr/bin/env bash
# In-game check for the mekmm slice 1: the Basic Recycling Factory
# (`mekmm:basic_recycling_factory`, 3 processes, 200 ticks, 50 FE/t each,
# 20,000 FE buffer, 17 % chance of one `mekmm:scrap` per operation).
#
# The End arena (stone floor at y=63, cleared volume to y=74):
#   A =  8,64,8   the powered factory (six item slots, menu, persistence)
#   B = 12,64,8   the control factory: never given energy
#   C = 16,64,8   the no-recipe control factory
# The mod's commands are typed with `ctl chat "/cmd"` because `ctl command`
# runs the console path and bypasses the native `perform_command` hook.
#
# Default run (must PASS):
#  - every factory places and reports `0/20000 FE`, three zero progresses and
#    six empty slots;
#  - control 1: a stone in each of B's inputs does not recycle without energy;
#  - one powered process turns one stone into one scrap in exactly 200 ticks
#    and exactly 10,000 FE (the buffer reads 10,000 afterwards);
#  - three processes share the buffer at 50 FE/t each: 10,000 FE is 67 ticks
#    for the first two and 66 for the third, then the stall;
#  - the same three operations finish on a 20,000 FE top-up, leaving three scrap
#    and an empty buffer;
#  - control 2: a forced-miss roll still consumes the input and 10,000 FE and
#    produces no scrap (the 17 % path is a real gate);
#  - control 3: an input with no recipe burns nothing and resets the progress;
#  - control 4: a full output slot holds the progress and burns nothing;
#  - the 3x3 menu opens with its nine slots and the 64 scrap in it;
#  - a machine stalled at progress 60 with 0 FE reads the same after save, quit
#    and relaunch (energy, progress, cycles and slots all survive).
#
# SELFTEST=1 flips the first operation's output assertion (it must FAIL),
# proving the scenario can fail.
#
# EMBEDDED=0 (default): the game loads `content/` as a dev mod.
# EMBEDDED=1: the game is launched with **only the native bundle** (no
# `--dev-mod`) and the content comes from the bundle's embedded copy
# (`mixins/mekmm/data/…`, the AE2 pattern). The run also checks the file the
# helper wrote, and CONTROL=1 (default with EMBEDDED) first launches the same
# binary with `.horizon-cache` unusable, where the block id must be unknown
# while the native `/mekmm` command is still present.
set -euo pipefail
export LC_ALL=C
# Every line goes to stderr, which is not buffered when redirected: a run that
# is killed still leaves its transcript in the log.
exec 1>&2

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/mekmm
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/mekmm-e2e.sock}
SELFTEST=${SELFTEST:-0}
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
KEEP=${KEEP:-0}
WORLD=MekmmE2E
CONTROL_WORLD=MekmmControl
EMBEDDED_FILE=$PORT/mixins/mekmm/data/mekmm-content.hmod
BLOCK=mekmm:basic_recycling_factory
SCRAP=mekmm:scrap
# The three factories, each as "x y z".
A="8 64 8"
B="12 64 8"
C="16 64 8"

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/content/horizon-mod.toml" ] || { echo "missing content: $PORT/content" >&2; exit 1; }
[ -f "$EMBEDDED_FILE" ] || { echo "missing embedded content: $EMBEDDED_FILE" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/mekmm-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() {
    echo "FAIL: $*" >&2
    ctl logs --lines 80 >&2 || true
    cp "$SCRATCH/game/logs/latest.log" "$CLONE/ports/_shared/evidence/mekmm/e2e-game.log" 2>/dev/null || true
    exit 1
}

# The newest `/mekmm info` line of the factory at a position, by its block id,
# so a line still in the log for another factory cannot be mistaken for it.
info_once() { # info_once X Y Z
    say "mekmm info $1 $2 $3" >/dev/null
    ctl wait 4 >/dev/null
    # The game's own log: `ctl messages` reads the window client's chat buffer,
    # whose sync under load trails the server by a few seconds.
    sleep 0.5
    local line
    line=$(grep -aE "\[CHAT\] $BLOCK: " "$SCRATCH/game/logs/latest.log" 2>/dev/null | tail -1 | sed 's/.*\[CHAT\] //' || true)
    if [ -z "$line" ]; then
        line=$(ctl messages 10 | jq -r '.chat[]' 2>/dev/null | grep -E "^$BLOCK: " | tail -1 || true)
    fi
    printf '%s\n' "$line"
}

# Poll until the factory says `TEXT` (each pass asks again, so the line read is
# the newest answer).
expect_contains() { # expect_contains X Y Z TEXT
    local line="" ok=0
    for _ in 1 2 3 4 5; do
        line=$(info_once "$1" "$2" "$3")
        case "$line" in
            *"$4"*) ok=1; break ;;
        esac
    done
    echo "  info($1,$2,$3): $line"
    [ "$ok" = 1 ] || fail "expected /mekmm info at $1,$2,$3 to say '$4', got '$line'"
}

# Put a stack into one container slot of a factory: put X Y Z SLOT ITEM COUNT.
put() { # put X Y Z SLOT ITEM COUNT
    ctl command "execute in minecraft:the_end run item replace block $1 $2 $3 container.$4 with $5 $6" >/dev/null
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD] [DIRECTORY]
    local world=${1:-$WORLD} dir=${2:-$SCRATCH}
    local attempt
    for attempt in 1 2 3; do
        rm -f "$SOCK"
        cd "$dir"
        local args=(--game "$GAME" --world "$world" --seed 20261111 --creative --headless
                    --wait --resources "$RES")
        if [ "$EMBEDDED" = 1 ]; then
            echo "  launch: no --dev-mod; the bundle carries its own content"
        else
            args+=(--dev-mod "$PORT/content")
        fi
        if "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null 2>>"$dir/game-stderr.log"; then
            ctl wait-ready 120 >/dev/null
            ctl wait 40 >/dev/null
            return 0
        fi
        echo "  (launch attempt $attempt did not come up; waiting for the world lock)"
        sleep 10
    done
    fail "the game did not launch"
}

# Quit the game and wait for its world lock to go, so the relaunch may open it.
stop_game() {
    ctl quit >/dev/null 2>&1 || true
    local lock="$SCRATCH/game/saves/$WORLD/session.lock"
    for _ in $(seq 1 40); do
        [ -e "$lock" ] || break
        sleep 1
    done
    sleep 2
}

# The first launch only: a stone floor and a clear volume in The End.
arena() {
    ctl command "execute in minecraft:the_end run tp @s 0.5 64 3.5"
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5
        if [ "$(ctl block 8 63 8 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill -8 63 -8 24 63 24 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 64 -8 24 74 24 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/mekmm-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/mekmm.hmod" \
        || fail "the embedded content differs from content/mekmm.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/mekmm.hmod"
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. The block id must then be unknown while the native `/mekmm`
# command still answers; if it resolves, the run proves nothing about the
# embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    local dir=$SCRATCH/control
    mkdir -p "$dir"
    : > "$dir/.horizon-cache"
    (
        export HORIZON_HOME=$dir/game HORIZON_LAUNCHER_HOME=$dir/launcher \
               XDG_DATA_HOME=$dir/data XDG_CONFIG_HOME=$dir/config
        mkdir -p "$XDG_CONFIG_HOME"
        launch "$CONTROL_WORLD" "$dir"
        arena
        ctl command "execute in minecraft:the_end run setblock 8 64 8 $BLOCK" >/dev/null || true
        ctl wait 6
        local placed
        placed=$(ctl block 8 64 8 | jq -r '.block // ""')
        echo "  placed block: $placed"
        [ "$placed" = "minecraft:air" ] || fail "the control game holds $placed, so the block id was known"
        # The native half is in the binary: its command still answers, so the run
        # distinguishes "no content" from "no mod at all".
        local reply
        say "mekmm info 8 64 8"
        ctl wait 4
        sleep 0.5
        reply=$(grep -a "\[CHAT\] No block entity there." "$dir/game/logs/latest.log" 2>/dev/null | tail -1 | sed 's/.*\[CHAT\] //' || true)
        if [ -z "$reply" ]; then
            reply=$(ctl messages 8 | jq -r '.chat[]' | grep -m1 "No block entity there." || true)
        fi
        echo "  native command: $reply"
        [ -n "$reply" ] || fail "the control's native command did not answer at all"
        [ -f "$dir/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
        echo "  without the embedded file: no factory block; the native command still answers (it is in the binary)"
        ctl quit >/dev/null 2>&1 || true
    )
    sleep 2
    rm -rf "$dir"
}

if [ "$CONTROL" = 1 ]; then control_leg; fi

echo "== launch (The End arena, content + native bundle) =="
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

echo "== the factories place and their block entities answer =="
ctl command "execute in minecraft:the_end run setblock 8 64 8 $BLOCK"
ctl command "execute in minecraft:the_end run setblock 12 64 8 $BLOCK"
ctl command "execute in minecraft:the_end run setblock 16 64 8 $BLOCK"
ctl wait 20
expect_contains $A "0/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, empty], out [empty, empty, empty]"
expect_contains $B "0/20000 FE, progress [0, 0, 0]/200"
expect_contains $C "0/20000 FE, progress [0, 0, 0]/200"

echo "== control 1: without energy three stones do not recycle =="
put $B 0 minecraft:stone 1
put $B 1 minecraft:stone 1
put $B 2 minecraft:stone 1
ctl wait 60
expect_contains $B "0/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [empty, empty, empty]"

echo "== one powered process: one stone -> one scrap in 200 ticks for 10,000 FE =="
say "mekmm chance 1 $A"
ctl wait 2
put $A 0 minecraft:stone 1
say "mekmm energy 20000 $A"
ctl wait 4
ctl wait 260
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting the output is still empty)"
    expect_contains $A "out [empty, empty, empty]"
else
    expect_contains $A "10000/20000 FE, progress [0, 0, 0]/200, cycles [1, 0, 0], in [empty, empty, empty], out [$SCRAP x1, empty, empty]"
fi

echo "== three processes share the buffer: 10,000 FE is 67/67/66 ticks =="
# Empty the buffer before the stones go in, so all three processes start on the
# same tick (a head start would change the 67/67/66 stall).
say "mekmm energy 0 $A"
ctl wait 4
put $A 0 minecraft:stone 1
put $A 1 minecraft:stone 1
put $A 2 minecraft:stone 1
ctl wait 4
say "mekmm energy 10000 $A"
ctl wait 4
ctl wait 90
expect_contains $A "0/20000 FE, progress [67, 67, 66]/200, cycles [1, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [$SCRAP x1, empty, empty]"

echo "== the three operations finish on a 20,000 FE top-up =="
say "mekmm energy 20000 $A"
ctl wait 4
ctl wait 220
expect_contains $A "0/20000 FE, progress [0, 0, 0]/200, cycles [2, 1, 1], in [empty, empty, empty], out [$SCRAP x2, $SCRAP x1, $SCRAP x1]"

echo "== control 2: a missed chance roll consumes the input and the energy, and makes no scrap =="
say "mekmm chance 0 $A"
ctl wait 2
put $A 0 minecraft:stone 1
say "mekmm energy 20000 $A"
ctl wait 4
ctl wait 240
expect_contains $A "10000/20000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [empty, empty, empty], out [$SCRAP x2, $SCRAP x1, $SCRAP x1]"

echo "== control 3: an input with no recipe burns nothing and resets the progress =="
put $C 2 minecraft:diamond 1
say "mekmm energy 20000 $C"
ctl wait 4
ctl wait 60
expect_contains $C "20000/20000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, minecraft:diamond x1], out [empty, empty, empty]"

echo "== control 4: a full output slot holds the progress and burns nothing =="
put $A 3 $SCRAP 64
say "mekmm chance 1 $A"
ctl wait 2
put $A 0 minecraft:stone 1
say "mekmm energy 20000 $A"
ctl wait 4
ctl wait 60
expect_contains $A "20000/20000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, empty, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"

echo "== the factory opens its 3x3 menu with the nine slots =="
ctl command "execute in minecraft:the_end run tp @s 8.5 64 10.5"
ctl wait 5
ctl interact 8 64 8
ctl wait 6
state=$(ctl screen)
jq -e '.screen == "container"' <<<"$state" >/dev/null \
    || fail "the factory did not open a container: $(jq -c '{screen}' <<<"$state")"
jq -e '[.slots[] | select(.slot >= 300)] | length == 9' <<<"$state" >/dev/null \
    || fail "the factory's menu does not have 9 container slots: $(jq -c '[.slots[] | select(.slot >= 300)] | length' <<<"$state")"
jq -e --arg scrap "$SCRAP" '[.slots[] | select(.slot >= 300 and .item == $scrap and .count == 64)] | length == 1' <<<"$state" >/dev/null \
    || fail "the factory's menu does not show the 64 scrap: $(jq -c '[.slots[] | select(.slot >= 300)]' <<<"$state")"
ctl close
ctl wait 4

echo "== persistence: a factory process stalled at progress 60 with 0 FE reads back after a relaunch =="
# Process 0 is held by its full output slot; process 1 gets one stone and
# exactly 3,000 FE = 60 ticks at 50 FE/t, so it stalls at progress 60.
say "mekmm energy 0 $A"
ctl wait 4
put $A 1 minecraft:stone 1
ctl wait 6
say "mekmm energy 3000 $A"
ctl wait 4
ctl wait 140
BEFORE=$(info_once $A)
echo "  before save: $BEFORE"
case "$BEFORE" in
    *"0/20000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"*) ;;
    *) fail "the factory did not stall at progress 60 with 0 FE: $BEFORE" ;;
esac
ctl save | sed 's/^/  /'
stop_game
launch
ctl command "execute in minecraft:the_end run tp @s 8.5 64 10.5" >/dev/null
ctl wait 40 >/dev/null
AFTER=$(info_once $A)
echo "  after relaunch: $AFTER"
case "$AFTER" in
    *"0/20000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"*) ;;
    *) fail "the factory's energy, progress, cycles and slots did not survive the save: $AFTER" ;;
esac
echo "  (the saved compound survives: FE, progress, cycles and slots read back)"

if [ "$SELFTEST" = 1 ]; then
    echo "SELFTEST finished without failing (this is a bug in the test)"
    exit 1
fi
echo "PASS: the Basic Recycling Factory places and refuses to run without energy, one process turns a stone into a scrap in 200 ticks for 10,000 FE, three processes share the 20,000 FE buffer at 50 FE/t each, a missed 17 % roll still consumes input and energy, a full output slot and an input with no recipe burn nothing, the nine-slot menu opens, and energy, progress, cycles and slots survive a save and relaunch"
