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
        /// Insufficient balance to register a subnet.
        NotEnoughBalanceToRegisterSubnet,
        /// Insufficient stake to withdraw the requested amount.
        NotEnoughStakeToWithdraw,
        /// Insufficient balance in the cold key account to stake the requested amount.
        NotEnoughBalanceToStake,
        /// The weight vectors for keys and values have different sizes.
        WeightVecNotEqualSize,
        /// Duplicate UIDs detected in the weight matrix.
        DuplicateUids,
        /// At least one UID in the weight matrix does not exist in the metagraph.
        InvalidUid,
        /// The number of UIDs in the weight matrix is different from the allowed amount.
        InvalidUidsLength,
        /// The number of registrations in this block exceeds the allowed limit.
        TooManyRegistrationsPerBlock,
        /// The number of registrations in this interval exceeds the allowed limit.
        TooManyRegistrationsPerInterval,
        /// The number of subnet registrations in this interval exceeds the allowed limit.
        TooManySubnetRegistrationsPerInterval,
        /// The module is already registered in the active set.
        AlreadyRegistered,
        /// Failed to convert between u64 and T::Balance.
        CouldNotConvertToBalance,
        /// The specified tempo (epoch) is not valid.
        InvalidTempo,
        /// Attempted to set weights twice within net_epoch/2 blocks.
        SettingWeightsTooFast,
        /// Attempted to set max allowed UIDs to a value less than the current number of registered
        /// UIDs.
        InvalidMaxAllowedUids,
        /// The specified netuid does not exist.
        NetuidDoesNotExist,
        /// A subnet with the given name already exists.
        SubnetNameAlreadyExists,
        /// The subnet name is too short.
        SubnetNameTooShort,
        /// The subnet name is too long.
        SubnetNameTooLong,
        /// The subnet name contains invalid characters.
        InvalidSubnetName,
        /// Failed to add balance to the account.
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
        /// The caller is not the founder of the subnet.
        NotFounder,
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
        /// The trust ratio is invalid.
        InvalidTrustRatio,
        /// The minimum allowed weights value is invalid.
        InvalidMinAllowedWeights,
        /// The maximum allowed weights value is invalid.
        InvalidMaxAllowedWeights,
        /// The minimum delegation fee is invalid.
        InvalidMinDelegationFee,
        /// The module metadata is invalid.
        InvalidModuleMetadata,
        /// The module metadata is too long.
        ModuleMetadataTooLong,
        /// The module metadata is invalid.
        InvalidSubnetMetadata,
        /// The module metadata is too long.
        SubnetMetadataTooLong,
        /// The maximum name length is invalid.
        InvalidMaxNameLength,
        /// The minimum name length is invalid.
        InvalidMinNameLenght,
        /// The maximum allowed subnets value is invalid.
        InvalidMaxAllowedSubnets,
        /// The maximum allowed modules value is invalid.
        InvalidMaxAllowedModules,
        /// The maximum registrations per block value is invalid.
        InvalidMaxRegistrationsPerBlock,
        /// The minimum burn value is invalid, likely too small.
        InvalidMinBurn,
        /// The maximum burn value is invalid.
        InvalidMaxBurn,
        /// The module name is too long.
        ModuleNameTooLong,
        /// The module name is too short.
        ModuleNameTooShort,
        /// The module name is invalid. It must be a UTF-8 encoded string.
        InvalidModuleName,
        /// The module address is too long.
        ModuleAddressTooLong,
        /// The module address is invalid.
        InvalidModuleAddress,
        /// A module with this name already exists in the subnet.
        ModuleNameAlreadyExists,
        /// The founder share is invalid.
        InvalidFounderShare,
        /// The incentive ratio is invalid.
        InvalidIncentiveRatio,
        /// The general subnet application cost is invalid.
        InvalidGeneralSubnetApplicationCost,
        /// The proposal expiration is invalid.
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
        /// There is no subnet that is running with the Rootnet consensus
        RootnetSubnetNotFound,
        /// MinValidatorStake must be lower than 250k
        InvalidMinValidatorStake,
        /// The maximum allowed validators value is invalid, minimum is 10.
        InvalidMaxAllowedValidators,
        /// The encryption period is too short or long, minimum is 360 blocks max is 20_880 blocks
        InvalidMaxEncryptionPeriod,
        /// Subnet is using encrypted weight calls
        SubnetEncrypted,
        /// Subnet is not using encrypted weight calls
        SubnetNotEncrypted,
        /// Uid is not present in LegitWhitelist, it needs to be whitelisted by DAO
        UidNotWhitelisted,
        /// The copier margin must be between 0 and 1
        InvalidCopierMargin,
        /// Floor Founder Share must be between 0 and 100
        InvalidFloorFounderShare,
        /// Subnet Immunity Period has to be more than 0
        InvalidSubnetImmunityPeriod,
        /// Kappa has to be more than 0
        InvalidKappa,
        /// Rho must be more than 0
        InvalidRho,
        /// The maximum allowed set weight calls per epoch must be more than 0
        InvalidMaximumSetWeightCallsPerEpoch,
        /// Some module parameter is invalid
        InvalidModuleParams,
        /// The provided minimum fees are invalid. This can happen when:
        /// - Stake delegation fee is below the system minimum
        /// - Validator weight fee is below the system minimum
        /// - Either fee exceeds 100%
        InvalidMinFees,
        /// Cannot decrease fees below their current values.
        /// Fees can only be increased to prevent economic attacks.
        CannotDecreaseFee,
        /// General error for not having enough balance
        NotEnoughBalance,
        /// Not having enough tokens to bridge back
        NotEnoughBridgedTokens,
        /// User is trying to bridge tokens in closed period
        OutsideValidBlockRange,
    }
}
