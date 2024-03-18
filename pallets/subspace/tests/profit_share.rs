mod mock;

use frame_support::assert_ok;
use log::info;
use mock::*;
use sp_core::U256;
use sp_std::vec;

/* TO DO SAM: write test for LatuUpdate after it is set */
#[test]
fn test_add_profit_share() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let miner_key = U256::from(0);
        let voter_key = U256::from(1);
        register_module(netuid, miner_key, 1_000_000_000).expect("register miner module failed");
        register_module(netuid, voter_key, 1_000_000_000).expect("register voter module failed");
        let miner_uid = SubspaceModule::get_uid_for_key(netuid, &miner_key);
        let _voter_uid = SubspaceModule::get_uid_for_key(netuid, &voter_key);

        SubspaceModule::set_min_allowed_weights(netuid, 1);

        let profit_sharer_keys = vec![U256::from(2), U256::from(3)];
        let shares = vec![2, 2];
        let result = SubspaceModule::add_profit_shares(
            get_origin(miner_key),
            profit_sharer_keys.clone(),
            shares.clone(),
        );
        assert_ok!(result);
        let profit_shares = SubspaceModule::get_profit_shares(miner_key);
        assert_eq!(profit_shares.len(), shares.len(), "profit shares not added");
        info!("founder profit shares: {profit_shares:?}");
        let result =
            SubspaceModule::set_weights(get_origin(voter_key), netuid, vec![miner_uid], vec![1]);

        assert_ok!(result);
        let params = SubspaceModule::subnet_params(netuid);
        info!("params: {params:?}");
        let miner_emission = SubspaceModule::get_emission_for_key(netuid, &miner_key);
        let voter_emission = SubspaceModule::get_emission_for_key(netuid, &voter_key);
        assert_eq!(miner_emission, voter_emission, "emission not equal");
        assert!(miner_emission == 0, "emission not equal");
        assert!(voter_emission == 0, "emission not equal");
        let miner_stake = SubspaceModule::get_stake_for_key(netuid, &miner_key);
        let voter_stake = SubspaceModule::get_stake_for_key(netuid, &voter_key);
        info!("miner stake before: {miner_stake:?}");
        info!("voter stake before: {voter_stake:?}");
        step_epoch(netuid);
        let miner_emission = SubspaceModule::get_emission_for_key(netuid, &miner_key);
        let voter_emission = SubspaceModule::get_emission_for_key(netuid, &voter_key);
        assert!(miner_emission > 0, "emission not equal");
        assert!(voter_emission > 0, "emission not equal");
        assert_eq!(miner_emission, voter_emission, "emission not equal");

        info!("miner emission: {miner_emission:?}");
        info!("voter emission: {voter_emission:?}");
        let miner_balance = SubspaceModule::get_balance_u64(&miner_key);
        let voter_balance = SubspaceModule::get_balance_u64(&voter_key);
        info!("miner balance: {miner_balance:?}");
        info!("voter balance: {voter_balance:?}");
        let miner_stake = SubspaceModule::get_stake_for_key(netuid, &miner_key);
        let voter_stake = SubspaceModule::get_stake_for_key(netuid, &voter_key);
        info!("miner stake after: {miner_stake:?}");
        info!("voter stake after: {voter_stake:?}");

        let _emission_for_subnet = SubspaceModule::get_subnet_emission(netuid);
        let profit_share_emissions = SubspaceModule::get_profit_shares(miner_key);
        info!("profit share emissions: {profit_share_emissions:?}");

        // check the profit sharers
        let mut profit_share_balances: Vec<u64> = Vec::new();
        for profit_sharer_key in profit_sharer_keys.iter() {
            let profit_share_balance =
                SubspaceModule::get_stake_to_total(netuid, profit_sharer_key);
            let stake_to_vector = SubspaceModule::get_stake_to_vector(netuid, profit_sharer_key);
            info!("profit share balance: {stake_to_vector:?}");
            info!("profit share balance: {profit_share_balance:?}");
            profit_share_balances.push(profit_share_balance);
            assert!(profit_share_balances[0] > 0, "profit share balance is zero");
        }

        // sum of profit shares should be equal to the emission
        let sum_profit_share_balances: u64 = profit_share_balances.iter().sum();
        let delta = 1000;
        assert!(
            sum_profit_share_balances > miner_emission - delta
                || sum_profit_share_balances < miner_emission + delta,
            "profit share balances do not add up to the emission"
        );
    })
}
