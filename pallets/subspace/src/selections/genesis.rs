use crate::*;
use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod genesis {
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub subnets: Vec<ConfigSubnet<Vec<u8>, T::AccountId>>,
        pub block: u32,
        _phantom: PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                subnets: Vec::new(),
                block: 0,
                _phantom: PhantomData,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let def = DefaultSubnetParams::<T>::get();

            for (netuid, subnet) in self.subnets.iter().enumerate() {
                let netuid = netuid as u16;

                let params: SubnetParams<T> = SubnetParams {
                    name: subnet.name.clone().try_into().expect("subnet name is too long"),
                    founder: subnet.founder.clone(),
                    tempo: subnet.tempo.unwrap_or(def.tempo),
                    immunity_period: subnet.immunity_period.unwrap_or(def.immunity_period),
                    min_allowed_weights: subnet
                        .min_allowed_weights
                        .unwrap_or(def.min_allowed_weights),
                    max_allowed_weights: subnet
                        .max_allowed_weights
                        .unwrap_or(def.max_allowed_weights),
                    max_allowed_uids: subnet.max_allowed_uids.unwrap_or(def.max_allowed_uids),
                    ..def.clone()
                };

                log::info!("registering subnet {netuid} with params: {params:?}");

                let fee = DelegationFee::<T>::get(&params.founder);
                let changeset: SubnetChangeset<T> =
                    SubnetChangeset::new(params).expect("genesis subnets are valid");
                let _ = self::Pallet::<T>::add_subnet(changeset, Some(netuid))
                    .expect("Failed to register genesis subnet");

                for (module_uid, module) in subnet.modules.iter().enumerate() {
                    let module_uid = module_uid as u16;

                    let changeset = ModuleChangeset::new(
                        module.name.clone(),
                        module.address.clone(),
                        fee,
                        None,
                    );
                    self::Pallet::<T>::append_module(netuid, &module.key, changeset)
                        .expect("genesis modules are valid");
                    T::set_weights(netuid, module_uid, module.weights.clone());

                    for (staker, stake) in module.stake_from.iter().flatten() {
                        Pallet::<T>::increase_stake(staker, &module.key, *stake);
                    }
                }
            }
            log::info!("{:?}", SubnetGaps::<T>::get());
        }
    }
}
