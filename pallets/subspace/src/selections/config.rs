use frame_support::pallet_macros::pallet_section;

#[pallet_section]
pub mod config {
    #[pallet::config(with_default)]
    pub trait Config:
        frame_system::Config
        + pallet_governance_api::GovernanceApi<<Self as frame_system::Config>::AccountId>
        + pallet_emission_api::SubnetEmissionApi<<Self as frame_system::Config>::AccountId>
    {
        /// This pallet's ID, used for generating the treasury account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type that will be used to place deposits on modules.
        type Currency: Currency<Self::AccountId, Balance = u64> + Send + Sync;

        /// The default number of modules that can be registered per interval.
        type DefaultMaxRegistrationsPerInterval: Get<u16>;
        /// The default number of subnets that can be registered per interval.
        type DefaultMaxSubnetRegistrationsPerInterval: Get<u16>;
        /// The default minimum burn amount required for module registration.
        type DefaultModuleMinBurn: Get<u64>;
        /// The default minimum burn amount required for module registration.
        type DefaultSubnetMinBurn: Get<u64>;
        type DefaultMinValidatorStake: Get<u64>;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
        type EnforceWhitelist: Get<bool>;
        type DefaultUseWeightsEncryption: Get<bool>;
    }
}
