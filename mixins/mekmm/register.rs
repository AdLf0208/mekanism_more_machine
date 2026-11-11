// Declare `/mekmm info|energy|chance|progress`.
fn observe_register(commands: &mut Vec<NativeCommand>) {
    crate::game::mekmm_state::on_register_commands(commands);
}
