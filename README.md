# Guilds

## Setup

For the complete setup, you need 3 contracts deployed: A factory, a global config, and a guild which will be used as the source in the factory.

### Guild SC

The guild which will be used as source for the user guilds. The arguments don't matter too much, since this will be paused at deploy time. Just make sure they're valid values.

```
#[init]
fn init(
    &self,
    farming_token_id: TokenIdentifier,
    division_safety_constant: BigUint,
    config_sc_address: ManagedAddress,
    guild_master: ManagedAddress,
    first_week_start_epoch: Epoch,
    per_block_reward_amount: BigUint,
    mut admins: MultiValueEncoded<ManagedAddress>,
)
```

Note: For the config SC, just use any SC address.

### Factory SC

This SC will be used to deploy all the guilds by the users.

```
#[init]
fn init(
    &self,
    guild_sc_source_address: ManagedAddress,
    max_guilds: usize,
    farming_token_id: TokenIdentifier,
    division_safety_constant: BigUint,
    per_block_reward_amount: BigUint,
    boosted_yields_factors: BoostedYieldsFactors<Self::Api>,
    admins: MultiValueEncoded<ManagedAddress>,
)
```

`guild_sc_source_address` - The address of the above guild SC.
`max_guilds` - Maximum number of guilds that can be deployed at any given time.
`farming_token_id` - The farming token for all the deployed guilds.
`division_safety_constant` - Used in guilds. Recommeded values is 10^18.
`per_block_reward_amount` - The rewards per block used by all the guilds.
`boosted_yields_factors` - A struct of all the factors used for boosted yields. Fields described below.
`admins` - List of addresses that can perform admin-only actions on the guild factory.

```
pub struct BoostedYieldsFactors<M: ManagedTypeApi> {
    pub max_rewards_factor: BigUint<M>,
    pub user_rewards_energy_const: BigUint<M>,
    pub user_rewards_farm_const: BigUint<M>,
    pub min_energy_amount: BigUint<M>,
    pub min_farm_amount: BigUint<M>,
}
```

We recommed using the values used in the already existing farm-staking contracts.

Guild masters can set their boosted yields percentage through the following endpoint:
```
#[endpoint(setBoostedYieldsRewardsPercentage)]
fn set_boosted_yields_rewards_percentage(&self, percentage: Percentage)
```

By default, percentage is set to 0, meaning boosted yields are disabled.

### Global config SC

The global config SC contains the variables for all the guilds. It has to be deployed through the factory endpoint:

```
#[only_admin]
#[endpoint(deployConfigSc)]
fn deploy_config_sc(
    &self,
    max_staked_tokens: BigUint,
    user_unbond_epochs: Epoch,
    guild_master_unbond_epochs: Epoch,
    min_stake_user: BigUint,
    min_stake_guild_master: BigUint,
    config_sc_code: ManagedBuffer,
)
```

`max_staked_tokens` - The maximum amount of staked tokens in a guild. This is not a per user amount, but a global amount.
`user_unbond_epochs` - The number of epochs until the user can claim his original tokens after unstake.
`guild_master_unbond_epochs` - Same as above, but for guild master.
`min_stake_user` - The minimum amount of tokens the user must stake.
`min_stake_guild_master` - Same as above, but for guild master.
`config_sc_code` - The code of the config SC.

The above values will be used by all the user-deployed guilds.

Additionally, in this contract we have all the tiers. To keep the consistency in all the guild contracts, all the tiers have to be added at the same time, and no tier can be added afterwards.

To be able to call the following endpoints, use this endpoint from the factory SC:
```
#[only_admin]
#[endpoint(callConfigFunction)]
fn call_config_function(
    &self,
    function_name: ManagedBuffer,
    args: MultiValueEncoded<ManagedBuffer>,
)
```

User tiers are added through the following endpoint:
```
#[only_owner]
#[endpoint(addUserTiers)]
fn add_user_tiers(&self, tiers: MultiValueEncoded<RewardTierMultiValue<Self::Api>>)
```

Each argument is as follows:
```
pub type RewardTierMultiValue<M> = MultiValue4<BigUint<M>, BigUint<M>, BigUint<M>, BigUint<M>>;
```

Where each value is: `min_stake, max_stake, apr, compounded_apr`

Similarly as above, guild master tiers are added through this endpoint:
```
#[only_owner]
#[endpoint(addGuildMasterTiers)]
fn add_guild_master_tiers(&self, tiers: MultiValueEncoded<RewardTierMultiValue<Self::Api>>)
```

APRs can be modified at any time afterwards through the following endpoints:
```
#[only_owner]
#[endpoint(setUserTierApr)]
fn set_user_tier_apr(
    &self,
    min_stake: BigUint,
    max_stake: BigUint,
    new_apr: BigUint,
    new_compounded_apr: BigUint,
)

#[only_owner]
#[endpoint(setGuildMasterTierApr)]
fn set_guild_master_tier_apr(
    &self,
    min_stake: BigUint,
    max_stake: BigUint,
    new_apr: BigUint,
    new_compounded_apr: BigUint,
)
```

## Users deploying guilds through factory

After the setup is complete, any user can deploy their own guild through the factory SC. They first have to use the following endpoint:

```
#[endpoint(deployGuild)]
fn deploy_guild(&self) -> ManagedAddress
```

Note that only one guild per user can be deployed.

Next, they have to issue and set the roles for the Farm token and Unbond token. This can be done through the following endpoints:
```
#[payable("EGLD")]
#[endpoint(registerFarmToken)]
fn register_farm_token(
    &self,
    token_display_name: ManagedBuffer,
    token_ticker: ManagedBuffer,
    num_decimals: usize,
)

#[endpoint(setTransferRoleFarmToken)]
fn set_transfer_role_farm_token(&self)

#[payable("EGLD")]
#[endpoint(registerUnbondToken)]
fn register_unbond_token(
    &self,
    token_display_name: ManagedBuffer,
    token_ticker: ManagedBuffer,
    num_decimals: usize,
)

#[endpoint(setTransferRoleUnbondToken)]
fn set_transfer_role_unbond_token(&self)
```

Once all the tokens are issued and their roles set, they can resume their guild through this endpoint:
```
#[endpoint(resumeGuild)]
fn resume_guild_endpoint(&self, guild: ManagedAddress)
```

Note that only the guild master may call this endpoint.
