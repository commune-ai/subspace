use frame_support::pallet_macros::pallet_section;

// TODO: 2. For a doc comment specifically, you can use intra-doc links:
// ```rust
// /// The encryption period is too short or long, minimum is [`MIN_ENCRYPTION_PERIOD`] blocks max is [`MAX_ENCRYPTION_PERIOD`] blocks
#[pallet_section]
pub mod errors {
    #[pallet::error]
    pub enum Error<T> {
        /// The specified network does not exist.
        NetworkDoesNotExist,
        /// The specified module does not exist.
        ModuleDoesNotExist,
        /// The network is immune to changes.
        NetworkIsImmuned,
        /// Insufficient balance in the cold key account to stake the requested amount.
        NotEnoughBalanceToStake,
        /// The weight vectors for keys and values have different sizes.
        WeightVecNotEqualSize,
        /// The number of registrations in this block exceeds the allowed limit.
        TooManyRegistrationsPerBlock,
        /// The number of registrations in this interval exceeds the allowed limit.
        TooManyRegistrationsPerInterval,
        /// The module is already registered in the active set.
        AlreadyRegistered,
        /// Failed to convert between u64 and T::Balance.
        CouldNotConvertToBalance,
        /// The specified tempo (epoch) is not valid.

        /// The specified netuid does not exist.
        BalanceNotAdded,
        /// Failed to remove stake from the account.
        StakeNotRemoved,
        /// The key is already registered.
        KeyAlreadyRegistered,
        /// No keys provided (empty key set).
        EmptyKeys,
        /// Too many keys provided.
        TooManyKeys,
        /// Invalid shares distribution.
        InvalidShares,
        /// Insufficient stake to set weights.
        NotEnoughStakeToSetWeights,
        /// Insufficient stake to start a network.
        NotEnoughStakeToStartNetwork,
        /// Insufficient stake per weight.
        NotEnoughStakePerWeight,
        /// No self-weight provided.
        NoSelfWeight,
        /// Vectors have different lengths.
        DifferentLengths,
        /// Insufficient balance to register.
        NotEnoughBalanceToRegister,
        /// Failed to add stake to the account.
        StakeNotAdded,
        /// Failed to remove balance from the account.
        BalanceNotRemoved,
        /// Balance could not be removed from the account.
        BalanceCouldNotBeRemoved,
        /// Insufficient stake to register.
        NotEnoughStakeToRegister,
        /// The entity is still registered and cannot be modified.
        StillRegistered,
        /// Attempted to set max allowed modules to a value less than the current number of
        /// registered modules.
        MaxAllowedModules,
        /// Insufficient balance to transfer.
        NotEnoughBalanceToTransfer,
        /// The system is not in vote mode.
        NotVoteMode,
        /// The maximum allowed weights value is invalid.
        InvalidModuleMetadata,
        /// The module metadata is too long.
        ModuleMetadataTooLong,
        /// The module metadata is invalid.
        InvalidMaxNameLength,
        /// The minimum name length is invalid.
        InvalidMinNameLenght,
        /// The maximum allowed modules value is invalid.
        InvalidMaxAllowedModules,
        /// The maximum registrations per block value is invalid.
        InvalidMaxRegistrationsPerBlock,
        /// The module name is too long.
        ModuleNameTooLong,
        /// The module name is too short.
        ModuleNameTooShort,
        /// The module name is invalid. It must be a UTF-8 encoded string.
        InvalidModuleName,
        /// The module url is too long.
        ModuleUrlTooLong,
        /// The module url is invalid.
        InvalidModuleUrl,
        /// A module with this name already exists in the subnet.
        ModuleNameAlreadyExists,
        /// The incentive ratio is invalid.
        InvalidProposalExpiration,
        /// The maximum weight age is invalid.
        InvalidMaxWeightAge,
        /// The maximum number of set weights per epoch has been reached.
        MaxSetWeightsPerEpochReached,
        /// An arithmetic error occurred during calculation.
        ArithmeticError,
        /// The target registrations per interval is invalid.
        InvalidTargetRegistrationsPerInterval,
        /// The maximum registrations per interval is invalid.
        InvalidMaxRegistrationsPerInterval,
        /// The adjustment alpha value is invalid.
        InvalidAdjustmentAlpha,
        /// The target registrations interval is invalid.
        InvalidTargetRegistrationsInterval,
        /// The minimum immunity stake is invalid.
        InvalidMinImmunityStake,
        /// The extrinsic panicked during execution.
        ExtrinsicPanicked,
        /// A step in the process panicked.
        StepPanicked,
        /// The stake amount to add or remove is too small. Minimum is 0.5 unit.
        StakeTooSmall,
        /// The validator is delegating weights to another validator
        DelegatingControl,
        /// The validator is not delegating weights to another validator
        NotDelegatingControl,
        /// Some module parameter is invalid
        InvalidModuleParams,
  
    }
}
