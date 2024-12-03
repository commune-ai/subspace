use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod hooks {
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 =
                block_number.try_into().ok().expect("blockchain won't pass 2 ^ 64 blocks");

            // Adjust costs to reflect the demand
            Self::adjust_registration_parameters(block_number);

            // Clears the root net weights daily quota
            Self::clear_rootnet_daily_weight_calls(block_number);

            // TODO: fix later
            Weight::default()
        }

        fn on_idle(_n: BlockNumberFor<T>, _remaining: Weight) -> Weight {
            log::info!("running on_idle");
            Weight::zero()
        }
    }
}
