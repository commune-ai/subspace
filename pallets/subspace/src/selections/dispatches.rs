use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod dispatches {
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(1)]
        #[pallet::weight((T::WeightInfo::add_stake(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_add_stake(origin, module_key, amount)
        }

        #[pallet::call_index(2)]
        #[pallet::weight((T::WeightInfo::remove_stake(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_remove_stake(origin, module_key, amount)
        }

        #[pallet::call_index(3)]
        #[pallet::weight((T::WeightInfo::add_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn add_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_add_stake_multiple(origin, module_keys, amounts)
        }

        #[pallet::call_index(4)]
        #[pallet::weight((T::WeightInfo::remove_stake_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn remove_stake_multiple(
            origin: OriginFor<T>,
            module_keys: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_remove_stake_multiple(origin, module_keys, amounts)
        }

        #[pallet::call_index(5)]
        #[pallet::weight((T::WeightInfo::transfer_stake(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_stake(
            origin: OriginFor<T>,
            module_key: T::AccountId,
            new_module_key: T::AccountId,
            amount: u64,
        ) -> DispatchResult {
            Self::do_transfer_stake(origin, module_key, new_module_key, amount)
        }

        #[pallet::call_index(6)]
        #[pallet::weight((T::WeightInfo::transfer_multiple(), DispatchClass::Normal, Pays::No))]
        pub fn transfer_multiple(
            origin: OriginFor<T>,
            destinations: Vec<T::AccountId>,
            amounts: Vec<u64>,
        ) -> DispatchResult {
            Self::do_transfer_multiple(origin, destinations, amounts)
        }

        #[pallet::call_index(7)]
        #[pallet::weight((T::WeightInfo::register(), DispatchClass::Normal, Pays::No))]
        pub fn register(
            origin: OriginFor<T>,
            network_name: Vec<u8>,
            name: Vec<u8>,
            address: Vec<u8>,
            module_key: T::AccountId,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register(origin, network_name, name, address, module_key, metadata)
        }

        #[pallet::call_index(8)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
        pub fn deregister(origin: OriginFor<T>, netuid: u16) -> DispatchResult {
            Self::do_deregister(origin, netuid)
        }

        #[pallet::call_index(9)]
        #[pallet::weight((T::WeightInfo::deregister(), DispatchClass::Normal, Pays::No))]
        pub fn update_module(
            origin: OriginFor<T>,
            netuid: u16,
            name: Vec<u8>,
            address: Vec<u8>,
            stake_delegation_fee: Option<Percent>,
            validator_weight_fee: Option<Percent>,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            let key = ensure_signed(origin.clone())?;
            ensure!(
                Self::is_registered(Some(netuid), &key),
                Error::<T>::ModuleDoesNotExist
            );

            let uid = Self::get_uid_for_key(netuid, &key).ok_or(Error::<T>::ModuleDoesNotExist)?;
            let params = Self::module_params(netuid, &key, uid);

            let fees = match (stake_delegation_fee, validator_weight_fee) {
                (None, None) => None,
                (stake_fee, weight_fee) => {
                    let current_fees = ValidatorFeeConfig::<T>::get(&key);
                    Some(ValidatorFees {
                        stake_delegation_fee: stake_fee
                            .unwrap_or(current_fees.stake_delegation_fee),
                        validator_weight_fee: weight_fee
                            .unwrap_or(current_fees.validator_weight_fee),
                    })
                }
            };

            let changeset = ModuleChangeset::update(&params, name, address, fees, metadata);
            Self::do_update_module(origin, netuid, changeset)
        }

        #[pallet::call_index(10)]
        #[pallet::weight((T::WeightInfo::update_subnet(), DispatchClass::Normal, Pays::No))]
        pub fn update_subnet(
            origin: OriginFor<T>,
            netuid: u16,
            founder: T::AccountId,
            founder_share: u16,
            name: BoundedVec<u8, ConstU32<256>>,
            metadata: Option<BoundedVec<u8, ConstU32<120>>>,
            immunity_period: u16,
            incentive_ratio: u16,
            max_allowed_uids: u16,
            max_allowed_weights: u16,
            min_allowed_weights: u16,
            max_weight_age: u64,
            tempo: u16,
            maximum_set_weight_calls_per_epoch: Option<u16>,
            vote_mode: VoteMode,
            bonds_ma: u64,
            module_burn_config: GeneralBurnConfiguration<T>,
            min_validator_stake: u64,
            max_allowed_validators: Option<u16>,
            use_weights_encryption: bool,
            copier_margin: I64F64,
            max_encryption_period: Option<u64>,
        ) -> DispatchResult {
            let params = SubnetParams {
                founder,
                founder_share,
                immunity_period,
                incentive_ratio,
                max_allowed_uids,
                max_allowed_weights,
                min_allowed_weights,
                max_weight_age,
                name,
                tempo,
                maximum_set_weight_calls_per_epoch,
                bonds_ma,
                module_burn_config,
                min_validator_stake,
                max_allowed_validators,
                governance_config: GovernanceConfiguration {
                    vote_mode,
                    ..T::get_subnet_governance_configuration(netuid)
                },
                metadata,
                use_weights_encryption,
                copier_margin,
                max_encryption_period,
            };

            let changeset = SubnetChangeset::update(netuid, params)?;
            Self::do_update_subnet(origin, netuid, changeset)
        }

        #[pallet::call_index(12)]
        #[pallet::weight((T::WeightInfo::register(), DispatchClass::Normal, Pays::No))]
        pub fn register_subnet(
            origin: OriginFor<T>,
            name: Vec<u8>,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            Self::do_register_subnet(origin, name, metadata)
        }

        #[pallet::call_index(13)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn bridge(origin: OriginFor<T>, amount: u64) -> DispatchResult {
            let key = ensure_signed(origin)?;

            ensure!(
                Self::has_enough_balance(&key, amount),
                Error::<T>::NotEnoughBalance
            );

            // 1. Remove the balance from the account
            let Some(removed_balance_as_currency) = Self::u64_to_balance(amount) else {
                return Err(Error::<T>::CouldNotConvertToBalance.into());
            };

            Self::remove_balance_from_account(&key, removed_balance_as_currency)?;

            Bridged::<T>::mutate(&key, |bridged| *bridged = bridged.saturating_add(amount));

            Self::deposit_event(Event::Bridged(key, amount));

            Ok(())
        }

        #[pallet::call_index(14)]
        #[pallet::weight((Weight::zero(), DispatchClass::Normal, Pays::No))]
        pub fn bridge_withdraw(origin: OriginFor<T>, amount: u64) -> DispatchResult {
            let key = ensure_signed(origin)?;

            // Check if user has enough bridged tokens
            let bridged_amount = Bridged::<T>::get(&key);
            ensure!(
                bridged_amount >= amount && amount > 0,
                Error::<T>::NotEnoughBridgedTokens
            );

            // Convert amount to balance
            let Some(amount_as_currency) = Self::u64_to_balance(amount) else {
                return Err(Error::<T>::CouldNotConvertToBalance.into());
            };

            // Add balance back to account
            Self::add_balance_to_account(&key, amount_as_currency);

            // Reduce bridged amount
            Bridged::<T>::mutate(&key, |bridged| *bridged = bridged.saturating_sub(amount));

            Self::deposit_event(Event::BridgeWithdrawn(key, amount));

            Ok(())
        }
    }
}
