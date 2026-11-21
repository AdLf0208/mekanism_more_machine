#!/usr/bin/env bash
# In-game check for the mekmm recycling factory's four real tiers
# (`mekmm:{basic,advanced,elite,ultimate}_recycling_factory`): 3/5/7/9 processes,
# 200 ticks per operation, 50 FE/t per process, one shared buffer of 30,000 /
# 50,000 / 70,000 / 90,000 FE, and a 17 % chance of one `mekmm:scrap` per
# operation, for the flat SDK package `mekmm.hmod` (mod API 0.2).
#
# The package carries the blocks, their items, their data AND the Rust that runs
# them, so the game must be built against it by the launcher:
#
#   horizon-launcher native-build . --instance <instance> --trust-native-code
#
# and `GAME` must name that build's client. There is no `--dev-mod` leg: the
# package is compiled in, and a content half loaded beside it would be a second
# copy of the same blocks.
#
# The End arena (stone floor at y=63, cleared volume to y=74):
#   A =  8,64,8   the basic factory (one/two/three processes, controls, menu)
#   B = 12,64,8   the control factory: never given energy
#   C = 16,64,8   the no-recipe control factory
#   D = 20,64,8   the advanced factory (5 processes, 50,000 FE)
#   E = 24,64,8   the elite factory (7 processes, 70,000 FE)
#   F = 28,64,8   the ultimate factory (9 processes, 90,000 FE)
# The mod's commands are typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `run_mod_command` hook.
#
# Run:
#  - control "unmodified game" (CONTROL=1, the default): the same scenario on a
#    launcher build WITHOUT this package — the block id must be unknown and
#    `/mekmm` must not answer, so the main run's factories can only come from
#    this package. Set CONTROL_GAME to that build's client; with CONTROL_GAME
#    unset the leg is skipped with a note (CONTROL=0 does the same).
#  - every factory places and reports its own buffer, all processes at 0 and its
#    slots empty;
#  - control 1: a stone in each of B's inputs does not recycle without energy;
#  - one powered process turns one stone into one scrap in exactly 200 ticks
#    and exactly 10,000 FE (the buffer reads 20,000 of 30,000 afterwards);
#  - three processes share the buffer at 50 FE/t each: 10,000 FE is 67 ticks
#    for the first two and 66 for the third, then the stall;
#  - the same three operations finish on a 20,000 FE top-up, leaving three scrap
#    and an empty buffer;
#  - control 2: a forced-miss roll still consumes the input and 10,000 FE and
#    produces no scrap (the 17 % path is a real gate);
#  - control 3: an input with no recipe burns nothing and resets the progress;
#  - control 4: a full output slot holds the progress and burns nothing;
#  - each higher tier stalls at its own exact tick pattern on 10,000 FE
#    (advanced 40/40/40/40/40, elite 29/29/29/29/28/28/28, ultimate
#    23/23/22 x7) and finishes every process on exactly the energy it has left
#    (40,000 / 60,000 / 80,000 FE), producing one scrap per process;
#  - each tier opens its own vanilla grid — 9, 18, 18 and 27 slots — with its
#    construction slots where the container declares them;
#  - a machine stalled at progress 60 with 0 FE reads the same after save, quit
#    and relaunch (energy, progress, cycles and slots all survive).
#
# SELFTEST=1 flips the first operation's output assertion (it must FAIL),
# proving the scenario can fail. KEEP=1 keeps the scratch folder.
# The game runs `--offscreen` (a renderer, software) because a menu assertion
# needs the client to tick its screen; `--headless` hangs `ctl screen`/`menu`.
set -euo pipefail
export LC_ALL=C
# Every line goes to stderr, which is not buffered when redirected: a run that
# is killed still leaves its transcript in the log.
exec 1>&2

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/mekmm
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}   # pin-matched shared ctl
GAME=${GAME:-}                       # a launcher build carrying this package
CONTROL_GAME=${CONTROL_GAME:-}       # a launcher build without it, for the control leg
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/mekmm-e2e.sock}
SELFTEST=${SELFTEST:-0}
CONTROL=${CONTROL:-1}
KEEP=${KEEP:-0}
WORLD=MekmmE2E
CONTROL_WORLD=MekmmControl
BASIC=mekmm:basic_recycling_factory
ADVANCED=mekmm:advanced_recycling_factory
ELITE=mekmm:elite_recycling_factory
ULTIMATE=mekmm:ultimate_recycling_factory
SCRAP=mekmm:scrap
# The six factories, each as "x y z".
A="8 64 8"
B="12 64 8"
C="16 64 8"
D="20 64 8"
E="24 64 8"
F="28 64 8"

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "set GAME to a launcher build carrying ports/mekmm:" >&2
    echo "  horizon-launcher native-build $PORT --instance <i> --trust-native-code" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/mekmm-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
# A ctl call that must not hang the leg: the whole script still fails if it
# times out (30 s), which is a reported finding, not a retry.
ctl30() { timeout 30 "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() {
    echo "FAIL: $*" >&2
    ctl logs --lines 80 >&2 || true
    cp "$SCRATCH/game/logs/latest.log" "$CLONE/ports/_shared/evidence/mekmm/e2e-game.log" 2>/dev/null || true
    exit 1
}

# The newest `/mekmm info` line of the factory at a position, by its position, so
# a line still in the log for another factory cannot be mistaken for it.
info_once() { # info_once X Y Z
    say "mekmm info $1 $2 $3" >/dev/null
    ctl wait 4 >/dev/null
    # The game's own log: `ctl messages` reads the window client's chat buffer,
    # whose sync under load trails the server by a few seconds.
    sleep 0.5
    local line
    line=$(grep -aE "\[CHAT\] mekmm:[a-z_]+_recycling_factory at $1,$2,$3: " \
        "$SCRATCH/game/logs/latest.log" 2>/dev/null | tail -1 | sed 's/.*\[CHAT\] //' || true)
    if [ -z "$line" ]; then
        line=$(ctl messages 10 | jq -r '.chat[]' 2>/dev/null \
            | grep -E "^mekmm:[a-z_]+_recycling_factory at $1,$2,$3: " | tail -1 || true)
    fi
    printf '%s\n' "$line"
}

# Poll until the factory says `TEXT` (each pass asks again, so the line read is
# the newest answer).
expect_contains() { # expect_contains X Y Z TEXT
    stand_at "$1"
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
    local game=${3:-$GAME}
    local attempt
    for attempt in 1 2 3; do
        rm -f "$SOCK"
        cd "$dir"
        local reply
        if reply=$("$CTL" --socket "$SOCK" launch --game "$game" --world "$world" --seed 20261111 \
                --creative --offscreen --wait --resources "$RES" \
                2>>"$dir/game-stderr.log"); then
            GAME_PID=$(jq -r '.pid // empty' <<<"$reply" 2>/dev/null || true)
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
    ctl command "execute in minecraft:the_end run fill -8 63 -8 32 63 32 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 64 -8 32 74 32 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

# The control for the one-package format: the same scenario on a launcher build
# WITHOUT this package. The block id must be unknown (a `/setblock` of it does
# nothing) and `/mekmm` must not answer, so nothing the main run proves could
# have come from the plain build.
control_leg() {
    echo "== control: a launcher build without this package =="
    local dir=$SCRATCH/control
    mkdir -p "$dir"
    (
        export HORIZON_HOME=$dir/game HORIZON_LAUNCHER_HOME=$dir/launcher \
               XDG_DATA_HOME=$dir/data XDG_CONFIG_HOME=$dir/config
        mkdir -p "$XDG_CONFIG_HOME"
        launch "$CONTROL_WORLD" "$dir" "$CONTROL_GAME"
        arena
        ctl command "execute in minecraft:the_end run setblock 8 64 8 $BASIC" >/dev/null || true
        ctl command "execute in minecraft:the_end run setblock 20 64 8 $ADVANCED" >/dev/null || true
        ctl wait 6
        local placed advanced
        placed=$(ctl block 8 64 8 | jq -r '.block // ""')
        advanced=$(ctl block 20 64 8 | jq -r '.block // ""')
        echo "  placed blocks: $placed, $advanced"
        [ "$placed" = "minecraft:air" ] || fail "the control game holds $placed, so the block id was known"
        [ "$advanced" = "minecraft:air" ] || fail "the control game holds $advanced, so the tier id was known"
        say "mekmm info 8 64 8"
        ctl wait 4
        sleep 0.5
        local reply
        reply=$(grep -aE "\[CHAT\] (mekmm:[a-z_]+_recycling_factory at |No block entity there\.)" "$dir/game/logs/latest.log" 2>/dev/null | tail -1 | sed 's/.*\[CHAT\] //' || true)
        if [ -z "$reply" ]; then
            reply=$(ctl messages 8 | jq -r '.chat[]' 2>/dev/null | grep -E "^(mekmm:[a-z_]+_recycling_factory at |No block entity there\.)" | tail -1 || true)
        fi
        echo "  native command: ${reply:-<no answer>}"
        [ -z "$reply" ] || fail "the plain game answered /mekmm, so the mod was in it: $reply"
        echo "  without the package: no factory blocks and no /mekmm answer"
        ctl quit >/dev/null 2>&1 || true
    )
    sleep 2
    rm -rf "$dir"
}

# Stand where the factory at X Y Z is in reach (the player is teleported first:
# `/mekmm` and `interact` both reach only so far).
stand_at() { # stand_at X
    ctl command "execute in minecraft:the_end run tp @s $(awk -v x="$1" 'BEGIN { print x + 0.5 }') 64 4.5" >/dev/null
    ctl wait 4 >/dev/null
}

# The menu a factory opens, as its container-slot count: `ctl screen` lists the
# open menu's slots, and a container's start at id 300 (the player's inventory
# is 0-35, a station's slots at 600).
menu_slots() { # menu_slots X Y Z
    stand_at "$1"
    ctl30 interact "$1" "$2" "$3" >/dev/null || return 1
    ctl30 wait-for screen container --timeout 15 >/dev/null || return 1
    local slots
    slots=$(ctl30 screen | jq '[.slots[] | select(.slot >= 300)] | length')
    ctl30 close >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    printf '%s\n' "$slots"
}

if [ "$CONTROL" = 1 ]; then
    if [ -n "$CONTROL_GAME" ]; then
        control_leg
    else
        echo "== control: skipped (set CONTROL_GAME to a launcher build without this package) =="
    fi
fi

echo "== launch (The End arena, the package compiled into the game) =="
launch
arena

echo "== the factories place and their block entities answer =="
# Stand among the factories: the readout is a player command, so it needs the
# operator's reach rule.
ctl command "execute in minecraft:the_end run tp @s 16.5 64 4.5"
ctl command "execute in minecraft:the_end run setblock 8 64 8 $BASIC"
ctl command "execute in minecraft:the_end run setblock 12 64 8 $BASIC"
ctl command "execute in minecraft:the_end run setblock 16 64 8 $BASIC"
ctl command "execute in minecraft:the_end run setblock 20 64 8 $ADVANCED"
ctl command "execute in minecraft:the_end run setblock 24 64 8 $ELITE"
ctl command "execute in minecraft:the_end run setblock 28 64 8 $ULTIMATE"
# Every factory rolls the real 17 %; pin the higher tiers' roll on so their
# completion assertion is about the processes, not about a lucky roll (the
# 17 % path itself is the basic tier's control 2).
say "mekmm chance 1 $D"
say "mekmm chance 1 $E"
say "mekmm chance 1 $F"
ctl wait 20
expect_contains $A "0/30000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, empty], out [empty, empty, empty]"
expect_contains $B "0/30000 FE, progress [0, 0, 0]/200"
expect_contains $C "0/30000 FE, progress [0, 0, 0]/200"
expect_contains $D "0/50000 FE, progress [0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0], in [empty, empty, empty, empty, empty], out [empty, empty, empty, empty, empty]"
expect_contains $E "0/70000 FE, progress [0, 0, 0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0, 0, 0]"
expect_contains $F "0/90000 FE, progress [0, 0, 0, 0, 0, 0, 0, 0, 0]/200, cycles [0, 0, 0, 0, 0, 0, 0, 0, 0]"

echo "== control 1: without energy three stones do not recycle =="
put $B 0 minecraft:stone 1
put $B 1 minecraft:stone 1
put $B 2 minecraft:stone 1
ctl wait 60
expect_contains $B "0/30000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [empty, empty, empty]"

echo "== one powered process: one stone -> one scrap in 200 ticks for 10,000 FE =="
say "mekmm chance 1 $A"
ctl wait 2
# Empty the buffer before the stone goes in: a stone in a powered machine ticks
# between two commands, and that stray tick would make the energy assertion
# depend on command latency instead of on the machine.
say "mekmm energy 0 $A"
ctl wait 4
put $A 0 minecraft:stone 1
say "mekmm energy 30000 $A"
ctl wait 4
ctl wait 260
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting the output is still empty)"
    expect_contains $A "out [empty, empty, empty]"
else
    expect_contains $A "20000/30000 FE, progress [0, 0, 0]/200, cycles [1, 0, 0], in [empty, empty, empty], out [$SCRAP x1, empty, empty]"
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
expect_contains $A "0/30000 FE, progress [67, 67, 66]/200, cycles [1, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [$SCRAP x1, empty, empty]"

echo "== the three operations finish on a 20,000 FE top-up =="
say "mekmm energy 30000 $A"
ctl wait 4
ctl wait 220
expect_contains $A "10000/30000 FE, progress [0, 0, 0]/200, cycles [2, 1, 1], in [empty, empty, empty], out [$SCRAP x2, $SCRAP x1, $SCRAP x1]"

echo "== control 2: a missed chance roll consumes the input and the energy, and makes no scrap =="
say "mekmm chance 0 $A"
ctl wait 2
say "mekmm energy 0 $A"
ctl wait 4
put $A 0 minecraft:stone 1
say "mekmm energy 30000 $A"
ctl wait 4
ctl wait 240
expect_contains $A "20000/30000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [empty, empty, empty], out [$SCRAP x2, $SCRAP x1, $SCRAP x1]"

echo "== control 3: an input with no recipe burns nothing and resets the progress =="
say "mekmm energy 0 $C"
ctl wait 4
put $C 2 minecraft:diamond 1
say "mekmm energy 30000 $C"
ctl wait 4
ctl wait 60
expect_contains $C "30000/30000 FE, progress [0, 0, 0]/200, cycles [0, 0, 0], in [empty, empty, minecraft:diamond x1], out [empty, empty, empty]"

echo "== control 4: a full output slot holds the progress and burns nothing =="
put $A 3 $SCRAP 64
say "mekmm chance 1 $A"
ctl wait 2
# Empty the buffer before the stone goes in: a stone in a powered machine ticks
# between two commands, and that stray tick would make the energy assertion
# depend on command latency instead of on the machine.
say "mekmm energy 0 $A"
ctl wait 4
put $A 0 minecraft:stone 1
say "mekmm energy 30000 $A"
ctl wait 4
ctl wait 60
expect_contains $A "30000/30000 FE, progress [0, 0, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, empty, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"

echo "== the higher tiers stall at their own tick pattern on 10,000 FE =="
# Each tier's readout needs the player in reach, so stand by it first.
# Each tier gets all of its inputs filled, then exactly 10,000 FE. 50 FE/t per
# process means the buffer runs out mid-tick and the leftover FE advances only
# the first processes: 5 -> 40 ticks each (10,000 / 250); 7 -> 28 ticks each and
# 200 FE left, so four reach 29; 9 -> 22 ticks each and 100 FE left, so two
# reach 23. A machine with the wrong process count or usage cannot read these.
say "mekmm energy 0 $D"
say "mekmm energy 0 $E"
say "mekmm energy 0 $F"
ctl wait 4
for slot in 0 1 2 3 4; do put $D $slot minecraft:stone 1; done
for slot in 0 1 2 3 4 5 6; do put $E $slot minecraft:stone 1; done
for slot in 0 1 2 3 4 5 6 7 8; do put $F $slot minecraft:stone 1; done
ctl wait 8
say "mekmm energy 10000 $D"
say "mekmm energy 10000 $E"
say "mekmm energy 10000 $F"
ctl wait 4
ctl wait 60
expect_contains $D "0/50000 FE, progress [40, 40, 40, 40, 40]/200, cycles [0, 0, 0, 0, 0], in [minecraft:stone x1, minecraft:stone x1, minecraft:stone x1, minecraft:stone x1, minecraft:stone x1], out [empty, empty, empty, empty, empty]"
expect_contains $E "0/70000 FE, progress [29, 29, 29, 29, 28, 28, 28]/200, cycles [0, 0, 0, 0, 0, 0, 0]"
expect_contains $F "0/90000 FE, progress [23, 23, 22, 22, 22, 22, 22, 22, 22]/200, cycles [0, 0, 0, 0, 0, 0, 0, 0, 0]"

echo "== each tier finishes every process on exactly the energy it has left =="
# The stalled processes need 800 / 1,200 / 1,600 more ticks at 50 FE each, so
# the top-up is exactly 40,000 / 60,000 / 80,000 FE; every buffer ends at 0 and
# every process has produced its one scrap.
say "mekmm energy 40000 $D"
say "mekmm energy 60000 $E"
say "mekmm energy 80000 $F"
ctl wait 4
ctl wait 220
expect_contains $D "0/50000 FE, progress [0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty], out [$SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1]"
expect_contains $E "0/70000 FE, progress [0, 0, 0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty, empty, empty], out [$SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1]"
expect_contains $F "0/90000 FE, progress [0, 0, 0, 0, 0, 0, 0, 0, 0]/200, cycles [1, 1, 1, 1, 1, 1, 1, 1, 1], in [empty, empty, empty, empty, empty, empty, empty, empty, empty], out [$SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1, $SCRAP x1]"

echo "== each tier opens its own grid: 9, 18, 18 and 27 slots =="
# A mod container's menu must show exactly its slot count, and a vanilla grid
# is what a mod block's container opens here (`MOD_MENUS`). The four counts are
# distinct per tier, so a wrong declaration fails.
ctl command "execute in minecraft:the_end run tp @s 16.5 64 4.5" >/dev/null
ctl wait 5
for entry in "$BASIC 8 64 8 9" "$ADVANCED 20 64 8 18" "$ELITE 24 64 8 18" "$ULTIMATE 28 64 8 27"; do
    set -- $entry
    id=$1; x=$2; y=$3; z=$4; want=$5
    got=$(menu_slots "$x" "$y" "$z") || fail "$id did not open a menu"
    echo "  $id menu slots: $got (expected $want)"
    [ "$got" = "$want" ] || fail "$id's menu shows $got slots, expected $want"
done

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
stand_at 8
BEFORE=$(info_once $A)
echo "  before save: $BEFORE"
case "$BEFORE" in
    *"0/30000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"*) ;;
    *) fail "the factory did not stall at progress 60 with 0 FE: $BEFORE" ;;
esac
ctl save | sed 's/^/  /'
stop_game
launch
stand_at 8
ctl wait 40 >/dev/null
AFTER=$(info_once $A)
echo "  after relaunch: $AFTER"
case "$AFTER" in
    *"0/30000 FE, progress [0, 60, 0]/200, cycles [3, 1, 1], in [minecraft:stone x1, minecraft:stone x1, empty], out [$SCRAP x64, $SCRAP x1, $SCRAP x1]"*) ;;
    *) fail "the factory's energy, progress, cycles and slots did not survive the save: $AFTER" ;;
esac
echo "  (the saved compound survives: FE, progress, cycles and slots read back)"

echo "== the carried loot table and tag: a broken factory drops itself =="
# The block is `requires_tool` and sits in the carried `minecraft:mineable/pickaxe`
# tag, and its loot table is the 26.3 singular-condition self-drop. Broken with an
# iron pickaxe in survival it must drop the block; the differential is the same
# break on a stone block, so a harness that cannot read drops fails first.
ctl command "gamemode survival" >/dev/null
ctl give minecraft:iron_pickaxe 1 >/dev/null
ctl wait 6
ctl select minecraft:iron_pickaxe >/dev/null
ctl wait 6
ctl command "execute in minecraft:the_end run setblock 30 64 8 minecraft:stone" >/dev/null
ctl command "execute in minecraft:the_end run setblock 34 64 8 $ULTIMATE" >/dev/null
ctl wait 6
stand_at 32
# A stone block drops cobblestone (vanilla's own loot), so the differential is
# "the break reports a drop at all", not a particular id.
stone_drops=$(ctl break 30 64 8 | jq -r '(.drops // {}) | to_entries | map(.value) | add // 0')
echo "  stone block drop count: $stone_drops (the differential: vanilla's own loot)"
[ "$stone_drops" -ge 1 ] || fail "the differential break dropped nothing, so drops cannot be read"
factory_break=$(ctl break 34 64 8)
factory_drops=$(jq -c '.drops' <<<"$factory_break")
echo "  factory drop: $factory_drops"
[ "$(jq -r --arg id "$ULTIMATE" '.[$id] // 0' <<<"$factory_drops")" -ge 1 ] \
    || fail "the ultimate factory dropped nothing: $factory_drops"

if [ "$SELFTEST" = 1 ]; then
    echo "SELFTEST finished without failing (this is a bug in the test)"
    exit 1
fi
echo "PASS: the four recycling factory tiers place and report their own buffers (30,000/50,000/70,000/90,000 FE); the basic tier refuses to run without energy, turns a stone into a scrap in 200 ticks for 10,000 FE, shares its buffer as 67/67/66 and finishes on a 20,000 FE top-up; a missed 17 % roll still consumes input and energy; a full output slot and an input with no recipe burn nothing; advanced/elite/ultimate stall at 40/29/23 ticks on 10,000 FE and finish every process on exactly the energy they had left; each tier opens its own 9/18/18/27-slot grid; and energy, progress, cycles and slots survive a save and relaunch"
