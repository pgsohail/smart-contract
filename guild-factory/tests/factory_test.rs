pub mod factory_setup;

use factory_setup::*;

#[test]
fn all_setup_test() {
    let _ = FarmStakingSetup::new(
        guild_sc::contract_obj,
        energy_factory::contract_obj,
        guild_sc_config::contract_obj,
        guild_factory::contract_obj,
    );
}
