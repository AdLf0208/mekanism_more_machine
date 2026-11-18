//! Mekanism: MoreMachine for Horizon — slice 1, the Basic Recycling Factory.
//!
//! One flat SDK package (mod API 0.2): the carried block, its item, models,
//! textures, lang, loot table, creative tab and tags are the `assets/` and
//! `data/` here, and the behaviour is the one `impl Hooks` below. The rules
//! themselves live in [`runtime`], byte-identical to the retired `.hnmod`
//! bundle's `runtime.rs`; [`basic_recycling_factory`] is the glue.
//!
//! What the retired bundle expressed as eight injected helpers plus an embedded
//! content file is here three shapes: the factory's block entity is a **Part**
//! (`basic_recycling_factory::FactoryPart`, attached to the type declared for
//! the block), its nine-slot container and its FE buffer are **declarations** on
//! the block key, and the commands are the typed `register_commands`/
//! `run_mod_command` pair. Its embedded-content helper has no successor: one
//! `.hmod` carries the content and the code, which is what that helper existed to
//! work around.
use mod_api::{
    self, Command, CommandArgumentValue, Flow, Hooks, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, Text,
};

mod basic_recycling_factory;

pub mod runtime;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the factory: one block with a block entity, nine slots of which
    /// the first six are the three inputs and three outputs, and one input-only
    /// 20,000 FE buffer.
    ///
    /// The retired bundle wrote the buffer into the `block_entity_energy` hook's
    /// shared output and had to be careful to leave a foreign block's entry
    /// alone. A declaration is keyed by the block, so that hazard cannot recur:
    /// another mod's block is another key.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        let block = registrar.block_key(basic_recycling_factory::FACTORY);
        registrar.block_container(&block, basic_recycling_factory::container());
        registrar.block_energy(&block, basic_recycling_factory::battery());
        basic_recycling_factory::remember_scrap(registrar);
        let kind = basic_recycling_factory::declare(registrar);
        if let Err(error) = registrar.block_entity_part(&kind, basic_recycling_factory::FactoryPart)
        {
            context.refuse(&error.to_string());
        }
    }

    /// `/mekmm info|energy|chance|progress`: the harness controls the in-game
    /// check uses, since the slice has no GUI of its own.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("mekmm info <pos:block_pos>"));
        event.commands.push(Command::op(
            "mekmm energy <amount:integer> <pos:block_pos>",
            2,
        ));
        event.commands.push(Command::op(
            "mekmm chance <forced:integer> <pos:block_pos>",
            2,
        ));
        event
            .commands
            .push(Command::new("mekmm progress <pos:block_pos>"));
    }

    /// The command bodies. The arguments are parsed and resolved by the server's
    /// dispatcher, so this only reads the block entity and answers; a factory
    /// that is not there answers "No block entity there." as it did in 0.1.0.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
            return Flow::Continue;
        };
        let line = match literals.as_slice() {
            ["mekmm", "info"] => basic_recycling_factory::info_line(host, *pos)
                .unwrap_or_else(|| "No block entity there.".to_owned()),
            ["mekmm", "energy"] => {
                let Some(CommandArgumentValue::Integer(amount)) = event.command.argument("amount")
                else {
                    return Flow::Continue;
                };
                match basic_recycling_factory::set_energy(host, *pos, *amount) {
                    Some(after) => format!("Energy set to {after} FE."),
                    None => "No block entity there.".to_owned(),
                }
            }
            ["mekmm", "chance"] => {
                let Some(CommandArgumentValue::Integer(forced)) = event.command.argument("forced")
                else {
                    return Flow::Continue;
                };
                match basic_recycling_factory::set_chance(host, *pos, *forced) {
                    Some(state) => format!("Chance is now {state}."),
                    None => "No block entity there.".to_owned(),
                }
            }
            ["mekmm", "progress"] => basic_recycling_factory::progress_line(host, *pos)
                .unwrap_or_else(|| "No block entity there.".to_owned()),
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}
