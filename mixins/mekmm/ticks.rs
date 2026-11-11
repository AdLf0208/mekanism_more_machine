// The factory ticks.
fn observe_block_entity_ticks(block: &str, ticks: &mut bool) {
    crate::game::mekmm_state::on_block_entity_ticks(block, ticks);
}
