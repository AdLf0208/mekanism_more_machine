// The pure core of the mekmm port (clean-room rewrite).
//
// The **Recycling Factory** (`mekmm:{basic,advanced,elite,ultimate}_recycling_factory`)
// from Mekanism: MoreMachine 1.4.2 (MIT), an add-on of Mekanism. It is the
// Mekanism factory shape: `FactoryTier.processes` parallel processes, each
// spending 50 FE/t for 200 ticks, sharing one buffer of
// `max(storage / 2, usage) * processes` FE, and each turning one item into
// `mekmm:scrap` on a 17 % chance roll.
//
// Nothing here touches the game: the block entity in `basic_recycling_factory.rs`
// reads its saved NBT, calls these functions and writes the result back.
//
// Values and semantics come from the upstream tag `v1.4.2-1.21.1`; see
// `FINDINGS.md` for the files read and where every number came from.

/// Ticks one operation takes (`BASE_TICKS_REQUIRED` = 10 s, the factory base
/// and the recycler both).
pub const RECYCLER_TICKS: i32 = 200;

/// Energy one operating tick of **one process** costs
/// (`UsageConfig.recycler`, 50 FE/t).
pub const RECYCLER_USAGE: i32 = 50;

/// The single recycler's configured buffer (`StorageConfig.recycler`). A
/// factory does not use it directly: `MoreMachineFactory.setMachineData`
/// scales it per tier (see [`factory_capacity`]).
pub const MACHINE_STORAGE: i32 = 20_000;

/// The one recipe's output item, `mekmm:scrap`.
pub const SCRAP: &str = "mekmm:scrap";

/// The four factory tiers the jar registers **without** Evolved Mekanism
/// (`Mekanism` `FactoryTier`, all of `EnumUtils.FACTORY_TIERS`).
///
/// `MoreMachineUtils.getFactoryTier()` appends `MoreMachineEnumUtils.EM_TIERS`
/// (overclocked, quantum, dense, multiversal, creative) only when
/// `evolvedmekanism` is loaded, so those five are not registered here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// `FactoryTier.BASIC`, three processes.
    Basic,
    /// `FactoryTier.ADVANCED`, five processes.
    Advanced,
    /// `FactoryTier.ELITE`, seven processes.
    Elite,
    /// `FactoryTier.ULTIMATE`, nine processes.
    Ultimate,
}

impl Tier {
    /// The tiers the jar registers without Evolved Mekanism, in tier order.
    pub const ALL: [Self; 4] = [Self::Basic, Self::Advanced, Self::Elite, Self::Ultimate];

    /// The tier's registry path, in this mod's namespace.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Basic => "basic_recycling_factory",
            Self::Advanced => "advanced_recycling_factory",
            Self::Elite => "elite_recycling_factory",
            Self::Ultimate => "ultimate_recycling_factory",
        }
    }

    /// The tier's block id (`mekmm:advanced_recycling_factory`).
    #[must_use]
    pub const fn block(self) -> &'static str {
        match self {
            Self::Basic => "mekmm:basic_recycling_factory",
            Self::Advanced => "mekmm:advanced_recycling_factory",
            Self::Elite => "mekmm:elite_recycling_factory",
            Self::Ultimate => "mekmm:ultimate_recycling_factory",
        }
    }

    /// The tier's display name, as the jar's lang file has it.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            Self::Basic => "Basic Recycling Factory",
            Self::Advanced => "Advanced Recycling Factory",
            Self::Elite => "Elite Recycling Factory",
            Self::Ultimate => "Ultimate Recycling Factory",
        }
    }

    /// Parallel processes (`FactoryTier.processes`).
    #[must_use]
    pub const fn processes(self) -> usize {
        match self {
            Self::Basic => 3,
            Self::Advanced => 5,
            Self::Elite => 7,
            Self::Ultimate => 9,
        }
    }

    /// The factory's shared buffer: `max(MACHINE_STORAGE / 2, usage) * processes`,
    /// `MoreMachineFactory.setMachineData`.
    #[must_use]
    pub const fn storage(self) -> i32 {
        factory_capacity(MACHINE_STORAGE, RECYCLER_USAGE, self.processes())
    }

    /// The tier a block id names, `None` for any other block.
    #[must_use]
    pub fn from_block(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tier: &Self| tier.block() == id)
    }
}

/// A factory's buffer, from the base machine's storage and usage and the tier's
/// process count: `clamp(max(storage * 0.5, usage) * processes)`
/// (`MoreMachineFactory.setMachineData`, the upstream `MathUtils.clampToLong`).
#[must_use]
pub const fn factory_capacity(machine_storage: i32, usage: i32, processes: usize) -> i32 {
    let half = if machine_storage > 0 {
        machine_storage / 2
    } else {
        0
    };
    let base = if half > usage { half } else { usage };
    let total = base as i64 * processes as i64;
    if total > i32::MAX as i64 {
        i32::MAX
    } else {
        total as i32
    }
}

/// How long one operation takes and what one operating tick costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation {
    /// Ticks an operation takes.
    pub ticks: i32,
    /// Energy an operating tick costs, in FE.
    pub usage: i32,
}

/// The recycling factory's operation.
pub const RECYCLING_OPERATION: Operation = Operation {
    ticks: RECYCLER_TICKS,
    usage: RECYCLER_USAGE,
};

/// An item stack as the pure core sees it: an id and a count.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ItemCount {
    /// The item id (`minecraft:stone`).
    pub item: String,
    /// How many of it.
    pub count: u32,
}

impl ItemCount {
    /// A stack of `count` of `item`.
    #[must_use]
    pub fn new(item: &str, count: u32) -> Self {
        Self {
            item: item.to_owned(),
            count,
        }
    }

    /// Whether the stack holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.item.is_empty()
    }
}

/// One recipe as the factory runs it: how many input items one operation
/// consumes, what it may produce and how likely that is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecycleRecipe {
    /// Items taken from the input slot when the operation finishes.
    pub input: u32,
    /// The result, a stack of the recipe's item and count.
    pub output: ItemCount,
    /// The chance the result is produced, in whole percent (upstream `chance`
    /// 0.17 is 17).
    pub chance_percent: u32,
}

/// A factory's saved state: its tier, its energy, each process's progress in
/// ticks and each process's completed operations (the roll's counter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Factory {
    /// The tier the block is: it decides the process count and the buffer.
    pub tier: Tier,
    /// Stored energy in FE.
    pub energy: i32,
    /// Ticks of the operation in progress, per process, `0..operation.ticks`.
    pub progress: Vec<i32>,
    /// Operations completed, per process: the chance roll's counter.
    pub cycles: Vec<u32>,
}

impl Default for Factory {
    /// A basic-tier factory with an empty buffer: the shape a fresh block
    /// entity has before it saves anything.
    fn default() -> Self {
        Self::new(Tier::Basic)
    }
}

impl Factory {
    /// An empty factory of `tier`.
    #[must_use]
    pub fn new(tier: Tier) -> Self {
        let processes = tier.processes();
        Self {
            tier,
            energy: 0,
            progress: vec![0; processes],
            cycles: vec![0; processes],
        }
    }

    /// A factory from what its block entity saves: the `progress` and `cycles`
    /// IntArrays and the `energy` int, padded to the tier's process count and
    /// clamped to its buffer.
    #[must_use]
    pub fn from_saved(tier: Tier, progress: &[i32], cycles: &[i32], energy: i32) -> Self {
        let mut factory = Self {
            energy: clamp_energy(energy, tier.storage()),
            ..Self::new(tier)
        };
        for (slot, &value) in factory.progress.iter_mut().zip(progress) {
            *slot = value.clamp(0, RECYCLER_TICKS - 1);
        }
        for (slot, &value) in factory.cycles.iter_mut().zip(cycles) {
            *slot = u32::try_from(value).unwrap_or(0);
        }
        factory
    }

    /// The tier's buffer capacity.
    #[must_use]
    pub fn capacity(&self) -> i32 {
        self.tier.storage()
    }

    /// The progress array, as the entity's `progress` IntArray.
    #[must_use]
    pub fn progress_saved(&self) -> Vec<i32> {
        self.progress.clone()
    }

    /// The cycle counters, as the entity's `cycles` IntArray.
    #[must_use]
    pub fn cycles_saved(&self) -> Vec<i32> {
        self.cycles
            .iter()
            .map(|cycle| i32::try_from(*cycle).unwrap_or(i32::MAX))
            .collect()
    }
}

/// Why a process did nothing on its tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdleReason {
    /// The input slot is empty (the progress resets).
    NoInput,
    /// The input has no recipe (the progress resets).
    NoRecipe,
    /// The input stack is smaller than the recipe needs (the progress resets;
    /// upstream's `NOT_ENOUGH_INPUT`).
    NotEnoughInput,
    /// Less than one tick's energy is stored (the progress is held).
    NoEnergy,
    /// The output slot has no room for the recipe's result (held).
    NoOutputSpace,
}

/// What one process did on its tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A tick of the operation was made; energy was spent.
    Working,
    /// The operation finished: the input was consumed and the chance rolled.
    /// `output` is the scrap produced, or `None` when the roll missed.
    Produced {
        /// The result the roll produced, `None` on a miss.
        output: Option<ItemCount>,
    },
    /// Nothing happened.
    Idle(IdleReason),
}

/// What one tick of a factory needs besides its own state.
pub struct Tick<'a> {
    /// The operation's ticks and FE/t.
    pub operation: Operation,
    /// The result item's stack size, for the output-room check.
    pub max_stack: u32,
    /// A block position's seed; mixed with the process and its cycle so a
    /// factory's rolls are reproducible without a world RNG.
    pub seed: u64,
    /// A harness override: `Some(true)` always produces, `Some(false)` never.
    pub forced_chance: Option<bool>,
    /// The recipe table.
    pub lookup: &'a dyn Fn(&str) -> Option<RecycleRecipe>,
}

/// `energy` clamped into `0..=capacity`, as a buffer reads its saved int.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// SplitMix64: the port's own deterministic random for the chance roll.
#[must_use]
pub fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Whether a `chance_percent` roll succeeds for `seed`: an exact percentage
/// (0 never, 100 always) drawn uniformly from SplitMix64.
#[must_use]
pub fn roll(chance_percent: u32, seed: u64) -> bool {
    let chance = chance_percent.min(100);
    chance > 0 && (splitmix64(seed) % 100) < u64::from(chance)
}

/// The roll's seed for one process's next operation: the block's seed mixed
/// with the process index and how many operations it has completed, so two
/// processes and two cycles never share a roll.
#[must_use]
pub fn roll_seed(seed: u64, process: usize, cycle: u32) -> u64 {
    let process = (process as u64)
        .wrapping_add(1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let cycle = u64::from(cycle)
        .wrapping_add(1)
        .wrapping_mul(0xD1B5_4A32_D192_ED03);
    splitmix64(seed ^ process ^ cycle)
}

/// One tick of all of a factory's processes (`CachedRecipe.process`, one per
/// process).
///
/// Each process reads its own input and output, spends `operation.usage` for
/// one progress tick, and on `operation.ticks` consumes the recipe's input and
/// rolls the recipe's chance for its output. No energy or no output room holds
/// a process's progress; an empty input, an input without a recipe or too few
/// items resets it (upstream's `mismatchedRecipe` and `NOT_ENOUGH_INPUT`).
///
/// The number of processes is the factory's tier, so `inputs` and `outputs`
/// must hold at least [`Tier::processes`] entries: the block entity builds one
/// per process from its container.
///
/// # Panics
///
/// Panics when `inputs` or `outputs` is shorter than the tier's process count.
pub fn factory_tick(
    factory: &mut Factory,
    tick: &Tick<'_>,
    inputs: &mut [ItemCount],
    outputs: &mut [ItemCount],
) -> Vec<Outcome> {
    let processes = factory.tier.processes();
    assert!(
        inputs.len() >= processes && outputs.len() >= processes,
        "a {:?} factory needs {processes} input and output slots, got {} and {}",
        factory.tier,
        inputs.len(),
        outputs.len()
    );
    let mut outcomes = Vec::with_capacity(processes);
    for process in 0..processes {
        outcomes.push(process_tick(
            factory,
            tick,
            process,
            &mut inputs[process],
            &mut outputs[process],
        ));
    }
    outcomes
}

/// One process of a factory's tick. Split out so the processes are visibly the
/// same machine, sharing only the buffer in `factory.energy`.
fn process_tick(
    factory: &mut Factory,
    tick: &Tick<'_>,
    process: usize,
    input: &mut ItemCount,
    output: &mut ItemCount,
) -> Outcome {
    let operation = tick.operation;
    if input.is_empty() {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NoInput);
    }
    let Some(recipe) = (tick.lookup)(&input.item).filter(|recipe| !recipe.output.is_empty()) else {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NoRecipe);
    };
    if input.count < recipe.input.max(1) {
        factory.progress[process] = 0;
        return Outcome::Idle(IdleReason::NotEnoughInput);
    }
    // Upstream checks the *maximum* chance output's room before it spends
    // energy (`MoreMachineOutputHelper.calculateOperationsCanSupport`).
    let room = if output.is_empty() {
        tick.max_stack
    } else if output.item == recipe.output.item {
        tick.max_stack.saturating_sub(output.count)
    } else {
        0
    };
    if room < recipe.output.count {
        return Outcome::Idle(IdleReason::NoOutputSpace);
    }
    if factory.energy < operation.usage {
        return Outcome::Idle(IdleReason::NoEnergy);
    }
    factory.energy -= operation.usage;
    factory.progress[process] += 1;
    if factory.progress[process] < operation.ticks {
        return Outcome::Working;
    }
    factory.progress[process] = 0;
    input.count -= recipe.input.max(1);
    let produced = match tick.forced_chance {
        Some(hit) => hit,
        None => roll(
            recipe.chance_percent,
            roll_seed(tick.seed, process, factory.cycles[process]),
        ),
    };
    factory.cycles[process] = factory.cycles[process].wrapping_add(1);
    let output_stack = produced.then(|| recipe.output.clone());
    if let Some(stack) = &output_stack {
        if output.is_empty() {
            *output = stack.clone();
        } else {
            output.count += stack.count;
        }
    }
    Outcome::Produced {
        output: output_stack,
    }
}

/// The recycling recipe table: the jar's `mekmm:recycler` recipes over items
/// this port registers.
///
/// Source: `data/mekmm/recipe/recycler/{from_stone,from_dirt,from_substrate}.json`.
/// `from_stone` matches the `c:stones` tag; 26.3 ships no convention (`c`)
/// tags, so the tag's membership is this port's definition (the same one
/// `ports/cookingforblockheads` carries). `from_substrate` needs
/// `mekanism:substrate`, which this port does not register, so it is dropped.
pub mod recycling {
    /// The rows: `(input item, output item, output count, chance percent)`.
    static TABLE: &[(&str, &str, u32, u32)] = &[
        // `c:stones` (the port's definition of the convention tag).
        ("minecraft:stone", "mekmm:scrap", 1, 17),
        ("minecraft:granite", "mekmm:scrap", 1, 17),
        ("minecraft:diorite", "mekmm:scrap", 1, 17),
        ("minecraft:andesite", "mekmm:scrap", 1, 17),
        ("minecraft:deepslate", "mekmm:scrap", 1, 17),
        ("minecraft:tuff", "mekmm:scrap", 1, 17),
        ("minecraft:calcite", "mekmm:scrap", 1, 17),
        ("minecraft:dripstone_block", "mekmm:scrap", 1, 17),
        ("minecraft:smooth_stone", "mekmm:scrap", 1, 17),
        // `from_dirt`: dirt or grass_block.
        ("minecraft:dirt", "mekmm:scrap", 1, 17),
        ("minecraft:grass_block", "mekmm:scrap", 1, 17),
    ];

    /// The recipe for `item`, `None` when the factory cannot recycle it.
    #[must_use]
    pub fn lookup(item: &str) -> Option<super::RecycleRecipe> {
        TABLE.iter().find(|(input, ..)| *input == item).map(
            |&(_, output, count, chance_percent)| super::RecycleRecipe {
                input: 1,
                output: super::ItemCount::new(output, count),
                chance_percent,
            },
        )
    }
}

/// How far a player without operator permission may be from a factory to read it.
pub const INSPECT_RANGE: f64 = 8.0;

/// `Commands.LEVEL_ADMINS`: the permission level that bypasses the reach check.
pub const LEVEL_ADMINS: u8 = 2;

/// Whether a caller may read a factory's contents: an operator, or a player in
/// the same level within [`INSPECT_RANGE`] of the block.
pub fn may_inspect(op: bool, same_level: bool, distance: f64) -> bool {
    op || (same_level && distance <= INSPECT_RANGE)
}

/// Whether an output slot must be written: only when the tick actually changed
/// it. An untouched slot keeps whatever it held, so a foreign stack is never
/// rewritten into scrap (the review finding).
#[must_use]
pub fn output_changed(before: &ItemCount, after: &ItemCount) -> bool {
    before != after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> ItemCount {
        ItemCount::new(item, count)
    }

    fn empty(processes: usize) -> Vec<ItemCount> {
        vec![ItemCount::default(); processes]
    }

    fn inputs(items: &[&str]) -> Vec<ItemCount> {
        items.iter().map(|item| stack(item, 1)).collect()
    }

    fn tick_with<'a>(
        forced: Option<bool>,
        lookup: &'a dyn Fn(&str) -> Option<RecycleRecipe>,
    ) -> Tick<'a> {
        Tick {
            operation: RECYCLING_OPERATION,
            max_stack: 64,
            seed: 0x1234_5678,
            forced_chance: forced,
            lookup,
        }
    }

    fn run(
        factory: &mut Factory,
        tick: &Tick<'_>,
        inputs: &mut [ItemCount],
        outputs: &mut [ItemCount],
        ticks: i32,
    ) {
        for _ in 0..ticks {
            factory_tick(factory, tick, inputs, outputs);
        }
    }

    /// A tier's progress array as the strings the in-game readout prints.
    fn progresses(factory: &Factory) -> Vec<i32> {
        factory.progress.clone()
    }

    #[test]
    fn the_constants_are_the_jars_numbers() {
        assert_eq!(RECYCLER_TICKS, 200, "BASE_TICKS_REQUIRED = 10 s");
        assert_eq!(RECYCLER_USAGE, 50, "UsageConfig.recycler");
        assert_eq!(MACHINE_STORAGE, 20_000, "StorageConfig.recycler");
        assert_eq!(RECYCLING_OPERATION.ticks, RECYCLER_TICKS);
        assert_eq!(RECYCLING_OPERATION.usage, RECYCLER_USAGE);
    }

    #[test]
    fn the_four_tiers_are_the_jars_process_counts() {
        assert_eq!(
            Tier::ALL.map(Tier::processes),
            [3, 5, 7, 9],
            "FactoryTier.BASIC/ADVANCED/ELITE/ULTIMATE"
        );
        assert_eq!(
            Tier::ALL.map(Tier::id),
            [
                "basic_recycling_factory",
                "advanced_recycling_factory",
                "elite_recycling_factory",
                "ultimate_recycling_factory"
            ]
        );
        assert_eq!(
            Tier::ALL.map(Tier::block),
            [
                "mekmm:basic_recycling_factory",
                "mekmm:advanced_recycling_factory",
                "mekmm:elite_recycling_factory",
                "mekmm:ultimate_recycling_factory"
            ]
        );
        assert_eq!(Tier::Advanced.display(), "Advanced Recycling Factory");
    }

    #[test]
    fn the_factory_buffer_is_half_the_machine_storage_times_its_processes() {
        // `MoreMachineFactory.setMachineData`: max(storage * 0.5, usage) * processes.
        assert_eq!(
            Tier::ALL.map(Tier::storage),
            [30_000, 50_000, 70_000, 90_000]
        );
        assert_eq!(
            Tier::ALL.map(Tier::storage),
            Tier::ALL.map(|tier| factory_capacity(
                MACHINE_STORAGE,
                RECYCLER_USAGE,
                tier.processes()
            )),
        );
        // The usage floor: a base storage under twice the usage is not used.
        assert_eq!(factory_capacity(40, 50, 3), 150, "max(20, 50) * 3");
        assert_eq!(factory_capacity(20_000, 0, 3), 30_000, "half the storage");
        assert_eq!(
            factory_capacity(0, 0, 9),
            0,
            "nothing configured, nothing to store"
        );
    }

    #[test]
    fn every_tier_block_id_resolves_back_to_its_tier() {
        for tier in Tier::ALL {
            assert_eq!(Tier::from_block(tier.block()), Some(tier));
        }
        for other in [
            "mekmm:recycler",
            "mekmm:scrap",
            "minecraft:stone",
            "mekmm:basic_recycling_factory_extra",
            "",
        ] {
            assert_eq!(Tier::from_block(other), None, "{other} is not a factory");
        }
    }

    #[test]
    fn an_operation_costs_ten_thousand_fe() {
        assert_eq!(
            i64::from(RECYCLER_TICKS) * i64::from(RECYCLER_USAGE),
            10_000,
            "200 ticks at 50 FE/t"
        );
    }

    #[test]
    fn no_energy_holds_the_operation_and_spends_nothing() {
        let mut factory = Factory::new(Tier::Basic);
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoEnergy));
        assert_eq!(factory.progress, [0, 0, 0], "the progress is held");
        assert_eq!(factory.energy, 0);
        assert_eq!(in_slots[0].count, 1, "nothing was consumed");
    }

    #[test]
    fn one_tick_spends_fifty_fe_and_advances_one_tick() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Working);
        assert_eq!(factory.energy, 29_950);
        assert_eq!(factory.progress, [1, 0, 0]);
    }

    #[test]
    fn an_operation_takes_two_hundred_ticks_and_ten_thousand_fe() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        let mut last = Outcome::Working;
        for i in 0..RECYCLER_TICKS {
            last = factory_tick(
                &mut factory,
                &tick_with(Some(true), &recycling::lookup),
                &mut in_slots,
                &mut out_slots,
            )[0]
            .clone();
            if i + 1 < RECYCLER_TICKS {
                assert_eq!(last, Outcome::Working, "tick {} is not the finish", i + 1);
                assert_eq!(
                    in_slots[0].count, 1,
                    "the input is consumed at the end only"
                );
            }
        }
        assert_eq!(
            last,
            Outcome::Produced {
                output: Some(stack("mekmm:scrap", 1))
            }
        );
        assert_eq!(
            factory.energy,
            Tier::Basic.storage() - 10_000,
            "exactly one operation's energy"
        );
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(in_slots[0].count, 0);
        assert_eq!(out_slots[0], stack("mekmm:scrap", 1));
        assert_eq!(factory.cycles, [1, 0, 0]);
    }

    #[test]
    fn three_processes_each_cost_fifty_fe_from_the_one_buffer() {
        let mut factory = Factory {
            energy: 10_000,
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone"; 3]);
        let mut out_slots = empty(3);
        run(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            67,
        );
        // 67 x 150 = 10,050 wanted from 10,000: the first two processes take
        // the last 100 FE on tick 67, the third finds none.
        assert_eq!(factory.energy, 0);
        assert_eq!(factory.progress, [67, 67, 66]);
        assert_eq!(
            in_slots.iter().map(|s| s.count).collect::<Vec<_>>(),
            [1, 1, 1],
            "no operation finished"
        );
    }

    #[test]
    fn five_processes_stall_after_forty_ticks_on_ten_thousand_fe() {
        assert_eq!(advanced_stall(), [40, 40, 40, 40, 40]);
    }

    #[test]
    fn seven_processes_stall_after_twenty_eight_ticks_on_ten_thousand_fe() {
        // 28 x 350 = 9,800 spent; the last 200 FE advance four processes to 29.
        assert_eq!(elite_stall(), [29, 29, 29, 29, 28, 28, 28]);
    }

    #[test]
    fn nine_processes_stall_after_twenty_two_ticks_on_ten_thousand_fe() {
        // 22 x 450 = 9,900 spent; the last 100 FE advance two processes to 23.
        assert_eq!(ultimate_stall(), [23, 23, 22, 22, 22, 22, 22, 22, 22]);
    }

    /// The stall a tier reaches from all inputs filled and 10,000 FE.
    fn stall(tier: Tier) -> (Vec<i32>, i32) {
        let mut factory = Factory {
            energy: 10_000,
            ..Factory::new(tier)
        };
        let mut in_slots = inputs(&vec!["minecraft:stone"; tier.processes()]);
        let mut out_slots = empty(tier.processes());
        run(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            200,
        );
        (progresses(&factory), factory.energy)
    }

    fn advanced_stall() -> Vec<i32> {
        let (progress, energy) = stall(Tier::Advanced);
        assert_eq!(energy, 0, "the buffer is empty at the stall");
        progress
    }

    fn elite_stall() -> Vec<i32> {
        let (progress, energy) = stall(Tier::Elite);
        assert_eq!(energy, 0, "the buffer is empty at the stall");
        progress
    }

    fn ultimate_stall() -> Vec<i32> {
        let (progress, energy) = stall(Tier::Ultimate);
        assert_eq!(energy, 0, "the buffer is empty at the stall");
        progress
    }

    #[test]
    fn a_higher_tier_finishes_on_exactly_the_energy_it_has_left() {
        for (tier, top_up) in [
            (Tier::Advanced, 40_000),
            (Tier::Elite, 60_000),
            (Tier::Ultimate, 80_000),
        ] {
            let processes = tier.processes();
            let mut factory = Factory {
                energy: 10_000,
                ..Factory::new(tier)
            };
            let mut in_slots = inputs(&vec!["minecraft:stone"; processes]);
            let mut out_slots = empty(processes);
            run(
                &mut factory,
                &tick_with(Some(true), &recycling::lookup),
                &mut in_slots,
                &mut out_slots,
                200,
            );
            assert_eq!(factory.energy, 0, "{tier:?} stalls empty");
            // The remaining ticks of the stalled processes cost exactly this.
            let remaining: i32 = factory
                .progress
                .iter()
                .map(|ticks| RECYCLER_TICKS - ticks)
                .sum();
            assert_eq!(
                remaining * RECYCLER_USAGE,
                top_up,
                "{tier:?} remaining ticks x 50 FE/t"
            );
            factory.energy = top_up;
            run(
                &mut factory,
                &tick_with(Some(true), &recycling::lookup),
                &mut in_slots,
                &mut out_slots,
                200,
            );
            assert_eq!(factory.energy, 0, "{tier:?} spends exactly the top-up");
            assert_eq!(
                factory.progress,
                vec![0; processes],
                "{tier:?} finished every process"
            );
            assert_eq!(
                factory.cycles,
                vec![1; processes],
                "{tier:?} completed one operation per process"
            );
            assert_eq!(
                out_slots,
                vec![stack("mekmm:scrap", 1); processes],
                "{tier:?} produced one scrap per process"
            );
            assert!(
                in_slots.iter().all(|slot| slot.count == 0),
                "{tier:?} consumed every input"
            );
        }
    }

    #[test]
    fn each_process_runs_its_own_recipe() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "minecraft:dirt", "minecraft:diamond"]);
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Working);
        assert_eq!(outcomes[1], Outcome::Working);
        assert_eq!(outcomes[2], Outcome::Idle(IdleReason::NoRecipe));
        assert_eq!(factory.progress, [1, 1, 0]);
        assert_eq!(factory.energy, 29_900, "two processes spent");
        assert_eq!(in_slots[2].count, 1, "the diamond is left alone");
    }

    #[test]
    fn an_input_without_a_recipe_resets_the_progress() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            progress: vec![50, 0, 0],
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:iron_ingot", "", ""]);
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoRecipe));
        assert_eq!(factory.progress, [0, 0, 0], "the progress resets");
        assert_eq!(factory.energy, Tier::Basic.storage(), "no energy was spent");
        assert_eq!(in_slots[0].count, 1);
    }

    #[test]
    fn an_empty_input_resets_the_progress() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            progress: vec![50, 0, 0],
            ..Factory::default()
        };
        let mut in_slots = empty(3);
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoInput));
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(factory.energy, Tier::Basic.storage());
    }

    #[test]
    fn too_few_items_holds_nothing_and_resets() {
        // A row whose recipe needs more than the slot holds would reset; the
        // recycling table needs one, so exercise the guard through a custom
        // table instead of guessing a second upstream recipe.
        fn three(_item: &str) -> Option<RecycleRecipe> {
            Some(RecycleRecipe {
                input: 3,
                output: stack("mekmm:scrap", 1),
                chance_percent: 100,
            })
        }
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            progress: vec![50, 0, 0],
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        in_slots[0].count = 2;
        let mut out_slots = empty(3);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(None, &three),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NotEnoughInput));
        assert_eq!(factory.progress, [0, 0, 0]);
        assert_eq!(factory.energy, Tier::Basic.storage());
    }

    #[test]
    fn a_full_output_holds_the_progress_without_spending() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            progress: vec![10, 0, 0],
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        out_slots[0] = stack("mekmm:scrap", 64);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoOutputSpace));
        assert_eq!(factory.progress, [10, 0, 0], "held, not reset");
        assert_eq!(factory.energy, Tier::Basic.storage(), "no energy was spent");
        assert_eq!(in_slots[0].count, 1);
    }

    #[test]
    fn another_item_in_the_output_slot_blocks_the_result() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        out_slots[0] = stack("minecraft:iron_ingot", 1);
        let outcomes = factory_tick(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
        );
        assert_eq!(outcomes[0], Outcome::Idle(IdleReason::NoOutputSpace));
        assert_eq!(factory.energy, Tier::Basic.storage());
    }

    #[test]
    fn a_missed_roll_still_consumes_the_input_and_the_energy() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = inputs(&["minecraft:stone", "", ""]);
        let mut out_slots = empty(3);
        run(
            &mut factory,
            &tick_with(Some(false), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            RECYCLER_TICKS,
        );
        assert_eq!(factory.energy, Tier::Basic.storage() - 10_000);
        assert_eq!(in_slots[0].count, 0, "the input is consumed either way");
        assert_eq!(
            out_slots[0],
            ItemCount::default(),
            "and nothing is produced"
        );
        assert_eq!(factory.cycles, [1, 0, 0]);
    }

    #[test]
    fn the_roll_is_the_recipes_chance() {
        assert_eq!(
            recycling::lookup("minecraft:stone").map(|r| r.chance_percent),
            Some(17)
        );
        assert!(!roll(0, 12345), "0 % never succeeds");
        assert!(roll(100, 12345), "100 % always succeeds");
        let hits = (0..10_000u64).filter(|seed| roll(17, *seed)).count();
        assert!(
            (1_550..=1_850).contains(&hits),
            "17 % of 10,000 seeds, got {hits}"
        );
    }

    #[test]
    fn two_processes_and_two_cycles_never_share_a_roll_seed() {
        let mut seeds = std::collections::BTreeSet::new();
        for process in 0..Tier::Ultimate.processes() {
            for cycle in 0..64 {
                seeds.insert(roll_seed(0xABCD, process, cycle));
            }
        }
        assert_eq!(
            seeds.len(),
            Tier::Ultimate.processes() * 64,
            "every seed is distinct"
        );
    }

    #[test]
    fn stone_dirt_and_grass_block_recycle_and_other_items_do_not() {
        for item in [
            "minecraft:stone",
            "minecraft:granite",
            "minecraft:diorite",
            "minecraft:andesite",
            "minecraft:deepslate",
            "minecraft:tuff",
            "minecraft:calcite",
            "minecraft:dripstone_block",
            "minecraft:smooth_stone",
            "minecraft:dirt",
            "minecraft:grass_block",
        ] {
            let recipe = recycling::lookup(item).unwrap_or_else(|| panic!("{item} recycles"));
            assert_eq!(recipe.input, 1);
            assert_eq!(recipe.output, stack("mekmm:scrap", 1));
            assert_eq!(recipe.chance_percent, 17);
        }
        for item in [
            "minecraft:iron_ingot",
            "minecraft:diamond",
            "minecraft:cobblestone",
            "mekanism:substrate",
            "",
        ] {
            assert_eq!(recycling::lookup(item), None, "{item} has no recipe");
        }
    }

    #[test]
    fn a_stack_runs_one_operation_at_a_time() {
        let mut factory = Factory {
            energy: Tier::Basic.storage(),
            ..Factory::default()
        };
        let mut in_slots = empty(3);
        in_slots[0] = stack("minecraft:stone", 5);
        let mut out_slots = empty(3);
        run(
            &mut factory,
            &tick_with(Some(true), &recycling::lookup),
            &mut in_slots,
            &mut out_slots,
            RECYCLER_TICKS,
        );
        assert_eq!(in_slots[0].count, 4, "one item per operation");
        assert_eq!(out_slots[0].count, 1);
    }

    #[test]
    fn energy_is_clamped_into_the_buffer() {
        assert_eq!(clamp_energy(-5, MACHINE_STORAGE), 0);
        assert_eq!(clamp_energy(25_000, MACHINE_STORAGE), 20_000);
        assert_eq!(clamp_energy(4_321, MACHINE_STORAGE), 4_321);
        assert_eq!(clamp_energy(4_321, 0), 0);
    }

    #[test]
    fn saved_arrays_are_padded_and_clamped_per_tier() {
        let factory = Factory::from_saved(Tier::Basic, &[3, -1], &[7], 45_000);
        assert_eq!(factory.progress, [3, 0, 0], "a short array pads and clamps");
        assert_eq!(factory.cycles, [7, 0, 0]);
        assert_eq!(factory.energy, 30_000, "the buffer caps the int");
        let round_trip = Factory::from_saved(
            Tier::Basic,
            &factory.progress_saved(),
            &factory.cycles_saved(),
            factory.energy,
        );
        assert_eq!(round_trip, factory);

        let advanced = Factory::from_saved(Tier::Advanced, &[1; 7], &[2; 9], 60_000);
        assert_eq!(advanced.progress.len(), 5, "five processes, not three");
        assert_eq!(advanced.cycles.len(), 5);
        assert_eq!(advanced.energy, 50_000, "the advanced buffer is 50,000 FE");
    }

    #[test]
    fn a_tier_s_factory_runs_exactly_its_process_count() {
        for tier in Tier::ALL {
            let processes = tier.processes();
            let mut factory = Factory {
                energy: tier.storage(),
                ..Factory::new(tier)
            };
            let mut in_slots = inputs(&vec!["minecraft:stone"; processes]);
            let mut out_slots = empty(processes);
            let outcomes = factory_tick(
                &mut factory,
                &tick_with(Some(true), &recycling::lookup),
                &mut in_slots,
                &mut out_slots,
            );
            assert_eq!(outcomes.len(), processes, "{tier:?} ticks its processes");
            assert!(
                outcomes.iter().all(|outcome| *outcome == Outcome::Working),
                "{tier:?}: every process works"
            );
            assert_eq!(
                factory.energy,
                tier.storage() - 50 * processes as i32,
                "{tier:?} spends 50 FE per process"
            );
            assert_eq!(factory.progress, vec![1; processes]);
        }
    }

    #[test]
    fn an_untouched_output_slot_is_not_rewritten() {
        let scrap = ItemCount::new("mekmm:scrap", 4);
        let foreign = ItemCount::new("minecraft:diamond", 4);
        assert!(
            !output_changed(&foreign, &foreign),
            "the recipe rejected it"
        );
        assert!(!output_changed(&scrap, &scrap), "the slot did not move");
        assert!(output_changed(&foreign, &scrap), "an actual change");
        assert!(
            output_changed(&scrap, &ItemCount::default()),
            "a drained slot"
        );
    }

    #[test]
    fn a_readout_needs_an_operator_reach_or_the_same_level() {
        assert!(may_inspect(true, false, f64::MAX), "an operator");
        assert!(may_inspect(false, true, 4.0), "a player in reach");
        assert!(!may_inspect(false, true, 9.0), "out of reach");
        assert!(!may_inspect(false, false, 1.0), "another level");
    }
}
