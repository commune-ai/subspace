
use frame_support::{assert_ok, parameter_types, traits::{EnsureInherentsAreFirst, Hooks, OnRuntimeUpgrade, StorageMapShim, Contains}, weights::{Weight, IdentityFee, GetDispatchInfo, DispatchInfo}};

use pallet_transaction_payment::{CurrencyAdapter};
use sp_runtime::{
	KeyTypeId,
	CryptoTypeId,
	traits::{
		self,
		BlakeTwo256, 
		IdentityLookup,
		ValidateUnsigned,
		OpaqueKeys,
		Checkable,
		Applyable,
		Dispatchable,
		SignedExtension,
		PostDispatchInfoOf,
		DispatchInfoOf
	}, 
	ApplyExtrinsicResultWithInfo,
	transaction_validity::{TransactionValidity, TransactionSource, TransactionValidityError},
	testing::Header,
	generic::Era,
	codec::{Codec, Encode, Decode}
};

use pallet_subspace::{NeuronMetadata};
use std::net::{Ipv6Addr, Ipv4Addr};
use serde::{Serialize, Serializer, Deserialize, de::Error as DeError, Deserializer};
use std::{fmt::{self, Debug}, ops::Deref, cell::RefCell};
use sp_core::{
	U256,
	crypto::{CryptoType, Dummy, key_types, Public}
};

use frame_support::pallet_prelude::TypeInfo;
use frame_system::{limits};
use frame_system::ChainContext;



type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

const TEST_KEY: &[u8] = &*b":test:key:";
// Will contain `true` when the custom runtime logic was called.
const CUSTOM_ON_RUNTIME_KEY: &[u8] = &*b":custom:on_runtime";

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
		subspace: pallet_subspace::{Pallet, Call, Config, Storage, Event<T>},
	}
);

#[allow(dead_code)]
pub type subspaceCall = pallet_subspace::Call<Test>;

#[allow(dead_code)]
pub type BalanceCall = pallet_balances::Call<Test>;

#[allow(dead_code)]
pub type SudoCall = pallet_sudo::Call<Test>;

#[allow(dead_code)]
pub type TestRuntimeCall = frame_system::Call<Test>;


parameter_types! {
	pub const BlockHashCount: BlockNumber = 640;
	pub BlockWeights: limits::BlockWeights = limits::BlockWeights::simple_max(1024);
	pub const ExistentialDeposit: Balance = 1;
	pub const TransactionByteFee: Balance = 100;
	pub const SDebug:u64 = 1;
	pub const InitialRho: u64 = 10;
	pub const SelfOwnership: u64 = 2;
	pub const InitialImmunityPeriod: u64 = 2;
	pub const InitialMaxAllowedUids: u64 = 100;
	pub const InitialBondsMovingAverage: u64 = 500_000;
	pub const InitialIncentivePruningDenominator: u64 = 1;
	pub const InitialStakePruningDenominator: u64 = 1;
	pub const InitialStakePruningMin: u64 = 1024;
	pub const InitialFoundationDistribution: u64 = 0;

	pub const InitialValidatorBatchSize: u64 = 10;
	pub const InitialValidatorSequenceLen: u64 = 10;
	pub const InitialValidatorEpochLen: u64 = 10;
	pub const InitialValidatorEpochsPerReset: u64 = 10;
	pub const InitialValidatorPruneLen: u64 = 0;
	pub const InitialValidatorLogitsDivergence: u64 = 0;
	pub const InitialValidatorExcludeQuantile: u8 = 10;
	pub const InitialScalingLawPower: u8 = 50;
	pub const InitialSynergyScalingLawPower: u8 = 60;

	pub const InitialMaxWeightLimit: u32 = u32::MAX;
	pub const InitialBlocksPerStep: u64 = 1;
	pub const InitialIssuance: u64 = 548833985028256;
	pub const InitialDifficulty: u64 = 10000;
	pub const MinimumDifficulty: u64 = 10000;
	pub const MaximumDifficulty: u64 = u64::MAX/4;
	pub const InitialMaxRegistrationsPerBlock: u64 = 2;
	pub const InitialTargetRegistrationsPerInterval: u64 = 2;
}

thread_local!{
	pub static RUNTIME_VERSION: std::cell::RefCell<sp_version::RuntimeVersion> =
		Default::default();
}

/// Balance of an account.
#[allow(dead_code)]
pub type Balance = u128;

/// An index to a block.
#[allow(dead_code)]
pub type BlockNumber = u64;

pub struct TestBaseCallFilter;
impl Contains<Call> for TestBaseCallFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			// Transfer works. Use `transfer_keep_alive` for a call that doesn't pass the filter.
			Call::Balances(pallet_balances::Call::transfer { .. }) => true,
			// For benchmarking, this acts as a noop call
			Call::System(frame_system::Call::remark { .. }) => true,
			_ => false,
		}
	}
}
impl frame_system::Config for Test {
	type BaseCallFilter = TestBaseCallFilter;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type Version = RuntimeVersion;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[allow(dead_code)]
pub type AccountId = u64;

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type Event = ();
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test>,
		frame_system::Provider<Test>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type MaxLocks = ();
	type WeightInfo = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
}

impl pallet_subspace::Config for Test {
	type Event = ();
	type Currency = Balances;
	type TransactionByteFee = TransactionByteFee;
	type SDebug = SDebug;
	type InitialRho = InitialRho;
	type SelfOwnership = SelfOwnership;
	
	type InitialValidatorBatchSize = InitialValidatorBatchSize;
	type InitialValidatorSequenceLen = InitialValidatorSequenceLen;
	type InitialValidatorEpochLen = InitialValidatorEpochLen;
	type InitialValidatorEpochsPerReset = InitialValidatorEpochsPerReset;
	type InitialValidatorPruneLen = InitialValidatorPruneLen;
	type InitialValidatorLogitsDivergence = InitialValidatorLogitsDivergence;
	type InitialValidatorExcludeQuantile = InitialValidatorExcludeQuantile;
	type InitialScalingLawPower = InitialScalingLawPower;
	type InitialSynergyScalingLawPower = InitialSynergyScalingLawPower;

	type InitialImmunityPeriod = InitialImmunityPeriod;
	type InitialMaxAllowedUids = InitialMaxAllowedUids;
	type InitialBondsMovingAverage = InitialBondsMovingAverage;
	type InitialMaxWeightLimit = InitialMaxWeightLimit;
	type InitialStakePruningDenominator = InitialStakePruningDenominator;
	type InitialStakePruningMin = InitialStakePruningMin;
	type InitialIncentivePruningDenominator = InitialIncentivePruningDenominator;
	type InitialFoundationDistribution = InitialFoundationDistribution;
	type InitialIssuance = InitialIssuance;
	type InitialDifficulty = InitialDifficulty;
	type MinimumDifficulty = MinimumDifficulty;
	type MaximumDifficulty = MaximumDifficulty;
	type InitialBlocksPerStep = InitialBlocksPerStep;
	type InitialMaxRegistrationsPerBlock = InitialMaxRegistrationsPerBlock;
	type InitialTargetRegistrationsPerInterval = InitialTargetRegistrationsPerInterval;

}

impl pallet_sudo::Config for Test {
	type Event = ();
	type Call = Call;
}

impl pallet_transaction_payment::Config for Test {
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type OperationalFeeMultiplier = frame_support::traits::ConstU8<5>;
}


pub struct RuntimeVersion;
impl frame_support::traits::Get<sp_version::RuntimeVersion> for RuntimeVersion {
	fn get() -> sp_version::RuntimeVersion {
		RUNTIME_VERSION.with(|v| v.borrow().clone())
	}
}


type SignedExtra = (
	frame_system::CheckEra<Test>,
	frame_system::CheckNonce<Test>,
	frame_system::CheckWeight<Test>,
	pallet_subspace::SubspaceSignedExtension<Test>,
	//pallet_transaction_payment::ChargeTransactionPaymentOld<Test>
);


#[allow(dead_code)]
pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	fn on_runtime_upgrade() -> Weight {
		sp_io::storage::set(TEST_KEY, "custom_upgrade".as_bytes());
		sp_io::storage::set(CUSTOM_ON_RUNTIME_KEY, &true.encode());
		0
	}
}

impl EnsureInherentsAreFirst<XtBlock<TestXt<Call, SignedExtra>>> for Test {
    fn ensure_inherents_are_first(_: &XtBlock<TestXt<Call, SignedExtra>>) -> Result<(), u32> {
        Ok(Default::default())
    }
}

#[allow(dead_code)]
pub type Executive = frame_executive::Executive<
	Test,
	XtBlock<TestXt<Call, SignedExtra>>,
	ChainContext<Test>,
	Test,
	AllPallets,
	CustomOnRuntimeUpgrade
>;

#[allow(dead_code)]
fn extra(nonce: u64) -> SignedExtra {
	(
		frame_system::CheckEra::from(Era::Immortal),
		frame_system::CheckNonce::from(nonce),
		frame_system::CheckWeight::new(),
		pallet_subspace::SubspaceSignedExtension::new(),
		// pallet_transaction_payment::ChargeTransactionPayment::from(0)
	)
}

#[allow(dead_code)]
pub fn sign_extra(who: u64, nonce: u64) -> Option<(u64, SignedExtra)> {
	Some((who, extra(nonce)))
}


// Build genesis storage according to the mock runtime.
#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

#[allow(dead_code)]
pub fn test_ext_with_balances(balances : Vec<(u64, u128)>) -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	pallet_balances::GenesisConfig::<Test> { balances }
		.assimilate_storage(&mut t)
		.unwrap();

	t.into()
}


// #[allow(dead_code)]
// pub fn test_ext_with_stake(stake : Vec<(u64, u64)>) -> sp_io::TestExternalities {
// 	let mut t = frame_system::GenesisConfig::default()
// 	.build_storage::<Test>()
// 	.unwrap();
// 	pallet_subspace::GenesisConfig {
// 		stake: vec![]
// 	}.assimilate_storage::<Test>(&mut t)
// 		.unwrap();
// 	t.into()
// }

#[allow(dead_code)]
pub fn register_ok_neuron( key_account_id: u64, key_account_id: u64) -> NeuronMetadata<u64> {
	let result = subspace::register( <<Test as frame_system::Config>::Origin>::signed(key_account_id), block_number, nonce, work, key_account_id, key_account_id );
	assert_ok!(result);
	let neuron = subspace::get_neuron_for_key(&key_account_id);
	neuron
}
#[allow(dead_code)]
pub fn register_ok_neuron_with_nonce( key_account_id: u64, key_account_id: u64, nonce: u64 ) -> NeuronMetadata<u64> {
	let block_number: u64 = subspace::get_current_block_as_u64();
	let (nonce2, work): (u64, Vec<u8>) = subspace::create_work_for_block_number( block_number, nonce );
	let result = subspace::register( <<Test as frame_system::Config>::Origin>::signed(key_account_id), block_number, nonce2, work, key_account_id, key_account_id );
	assert_ok!(result);
	let neuron = subspace::get_neuron_for_key(&key_account_id);
	neuron
}

#[allow(dead_code)]
pub fn serve_module( key_account_id : u64, name: Vec<u8>, ip: u128, port: u16,  ) -> NeuronMetadata<u64> {
	let result = subspace::serve_module(<<Test as frame_system::Config>::Origin>::signed(key_account_id), version, ip, port );
	assert_ok!(result);
	let neuron = subspace::get_neuron_for_key(&key_account_id);
	neuron
}

// #[allow(dead_code)]
// pub fn n_subscribe_ok_neuron(n: usize) -> Vec<NeuronMetadata<u64>> {
// 	let mut neurons: Vec<NeuronMetadata<u64>> = vec![];
// 	for i in 0..n {
// 		let neuron: NeuronMetadata<u64> = register_ok_neuron(0, i as u8, i as u64, i as u64);
// 		neurons.push(neuron);
// 	}
// 	return neurons;
// }

#[allow(dead_code)]
pub(crate) fn run_to_block(n: u64) {
    while System::block_number() < n {
		subspace::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
		subspace::on_initialize(System::block_number());
    }
}

#[allow(dead_code)]
pub(crate) fn step_block(n: u64) {
	for _ in 0..n {
		subspace::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		subspace::on_initialize(System::block_number());
	}
}

// Generates an ipv6 address based on 8 ipv6 words and returns it as u128
#[allow(dead_code)]
pub fn ipv6(a: u16, b : u16, c : u16, d : u16, e : u16 ,f: u16, g: u16,h :u16) -> u128 {
	return Ipv6Addr::new(a,b,c,d,e,f,g,h).into();
}

// Generate an ipv4 address based on 4 bytes and returns the corresponding u128, so it can be fed
// to the module::subscribe() function
#[allow(dead_code)]
pub fn ipv4(a: u8 ,b: u8,c : u8,d : u8) -> u128 {
	let ipv4 : Ipv4Addr =  Ipv4Addr::new(a, b, c, d);
	let integer : u32 = ipv4.into();
	return u128::from(integer);
}


/************************************************************
	TEST EXTRINSIC
************************************************************/



/// A dummy type which can be used instead of regular cryptographic primitives.
///
/// 1. Wraps a `u64` `AccountId` and is able to `IdentifyAccount`.
/// 2. Can be converted to any `Public` key.
/// 3. Implements `RuntimeAppPublic` so it can be used instead of regular application-specific
///    crypto.
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, Debug, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct UintAuthorityId(pub u64);

impl From<u64> for UintAuthorityId {
	fn from(id: u64) -> Self {
		UintAuthorityId(id)
	}
}

impl From<UintAuthorityId> for u64 {
	fn from(id: UintAuthorityId) -> u64 {
		id.0
	}
}

impl UintAuthorityId {
	/// Convert this authority id into a public key.
	pub fn to_public_key<T: Public>(&self) -> T {
		let bytes: [u8; 32] = U256::from(self.0).into();
		T::from_slice(&bytes).unwrap()
	}
}

impl CryptoType for UintAuthorityId {
	type Pair = Dummy;
}

impl AsRef<[u8]> for UintAuthorityId {
	fn as_ref(&self) -> &[u8] {
		// Unsafe, i know, but it's test code and it's just there because it's really convenient to
		// keep `UintAuthorityId` as a u64 under the hood.
		unsafe {
			std::slice::from_raw_parts(&self.0 as *const u64 as *const _, std::mem::size_of::<u64>())
		}
	}
}

thread_local! {
	/// A list of all UintAuthorityId keys returned to the runtime.
	static ALL_KEYS: RefCell<Vec<UintAuthorityId>> = RefCell::new(vec![]);
}

impl UintAuthorityId {
	/// Set the list of keys returned by the runtime call for all keys of that type.
	pub fn set_all_keys<T: Into<UintAuthorityId>>(keys: impl IntoIterator<Item=T>) {
		ALL_KEYS.with(|l| *l.borrow_mut() = keys.into_iter().map(Into::into).collect())
	}
}

impl sp_application_crypto::RuntimeAppPublic for UintAuthorityId {
	const ID: KeyTypeId = key_types::DUMMY;
	const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"dumm");

	type Signature = TestSignature;

	fn all() -> Vec<Self> {
		ALL_KEYS.with(|l| l.borrow().clone())
	}

	fn generate_pair(_: Option<Vec<u8>>) -> Self {
		use rand::RngCore;
		UintAuthorityId(rand::thread_rng().next_u64())
	}

	fn sign<M: AsRef<[u8]>>(&self, msg: &M) -> Option<Self::Signature> {
		Some(TestSignature(self.0, msg.as_ref().to_vec()))
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		traits::Verify::verify(signature, msg.as_ref(), &self.0)
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		AsRef::<[u8]>::as_ref(self).to_vec()
	}
}

impl OpaqueKeys for UintAuthorityId {
	type KeyTypeIdProviders = ();

	fn key_ids() -> &'static [KeyTypeId] {
		&[key_types::DUMMY]
	}

	fn get_raw(&self, _: KeyTypeId) -> &[u8] {
		self.as_ref()
	}

	fn get<T: Decode>(&self, _: KeyTypeId) -> Option<T> {
		self.using_encoded(|mut x| T::decode(&mut x)).ok()
	}
}

impl sp_runtime::BoundToRuntimeAppPublic for UintAuthorityId {
	type Public = Self;
}

impl traits::IdentifyAccount for UintAuthorityId {
	type AccountId = u64;

	fn into_account(self) -> Self::AccountId {
		self.0
	}
}

/// A dummy signature type, to match `UintAuthorityId`.
#[derive(Eq, PartialEq, Clone, Debug, Hash, Serialize, Deserialize, Encode, Decode, TypeInfo)]
pub struct TestSignature(pub u64, pub Vec<u8>);

impl sp_runtime::traits::Verify for TestSignature {
	type Signer = UintAuthorityId;

	fn verify<L: sp_runtime::traits::Lazy<[u8]>>(&self, mut msg: L, signer: &u64) -> bool {
		signer == &self.0 && msg.get() == &self.1[..]
	}
}


/// An opaque extrinsic wrapper type.
#[derive(PartialEq, Eq, Clone, Debug, Encode, Decode, parity_util_mem::MallocSizeOf)]
pub struct ExtrinsicWrapper<Xt>(Xt);

impl<Xt> sp_runtime::traits::Extrinsic for ExtrinsicWrapper<Xt>
where Xt: parity_util_mem::MallocSizeOf
{
	type Call = ();
	type SignaturePayload = ();

	fn is_signed(&self) -> Option<bool> {
		None
	}
}

impl<Xt: Encode> serde::Serialize for ExtrinsicWrapper<Xt> {
	fn serialize<S>(&self, seq: S) -> Result<S::Ok, S::Error> where S: ::serde::Serializer {
		self.using_encoded(|bytes| seq.serialize_bytes(bytes))
	}
}

impl<Xt> From<Xt> for ExtrinsicWrapper<Xt> {
	fn from(xt: Xt) -> Self {
		ExtrinsicWrapper(xt)
	}
}

impl<Xt> Deref for ExtrinsicWrapper<Xt> {
	type Target = Xt;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Testing block
#[derive(PartialEq, Eq, Clone, Serialize, Debug, Encode, Decode, parity_util_mem::MallocSizeOf)]
pub struct XtBlock<Xt> {
	/// Block header
	pub header: Header,
	/// List of extrinsics
	pub extrinsics: Vec<Xt>,
}

impl<Xt: 'static + Codec + Sized + Send + Sync + Serialize + Clone + Eq + Debug + sp_runtime::traits::Extrinsic> sp_runtime::traits::Block
	for XtBlock<Xt>
{
	type Extrinsic = Xt;
	type Header = Header;
	type Hash = <Header as sp_runtime::traits::Header>::Hash;

	fn header(&self) -> &Self::Header {
		&self.header
	}
	fn extrinsics(&self) -> &[Self::Extrinsic] {
		&self.extrinsics[..]
	}
	fn deconstruct(self) -> (Self::Header, Vec<Self::Extrinsic>) {
		(self.header, self.extrinsics)
	}
	fn new(header: Self::Header, extrinsics: Vec<Self::Extrinsic>) -> Self {
		XtBlock { header, extrinsics }
	}
	fn encode_from(header: &Self::Header, extrinsics: &[Self::Extrinsic]) -> Vec<u8> {
		(header, extrinsics).encode()
	}
}

impl<'a, Xt> Deserialize<'a> for XtBlock<Xt> where XtBlock<Xt>: Decode {
	fn deserialize<D: Deserializer<'a>>(de: D) -> Result<Self, D::Error> {
		let r = <Vec<u8>>::deserialize(de)?;
		Decode::decode(&mut &r[..])
			.map_err(|e| DeError::custom(format!("Invalid value passed into decode: {}", e)))
	}
}


/// Test transaction, tuple of (sender, call, signed_extra)
/// with index only used if sender is some.
///
/// If sender is some then the transaction is signed otherwise it is unsigned.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct TestXt<Call, Extra> {
	/// Signature of the extrinsic.
	pub signature: Option<(u64, Extra)>,
	/// Call of the extrinsic.
	pub call: Call,
}

#[allow(dead_code)]
impl<Call, Extra> TestXt<Call, Extra> {
	/// Create a new `TextXt`.
	pub fn new(call: Call, signature: Option<(u64, Extra)>) -> Self {
		Self { call, signature }
	}
}

// Non-opaque extrinsics always 0.
parity_util_mem::malloc_size_of_is_0!(any: TestXt<Call, Extra>);

impl<Call, Extra> Serialize for TestXt<Call, Extra> where TestXt<Call, Extra>: Encode {
	fn serialize<S>(&self, seq: S) -> Result<S::Ok, S::Error> where S: Serializer {
		self.using_encoded(|bytes| seq.serialize_bytes(bytes))
	}
}

impl<Call, Extra> Debug for TestXt<Call, Extra> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "TestXt({:?}, ...)", self.signature.as_ref().map(|x| &x.0))
	}
}

impl<Call: Codec + Sync + Send, Context, Extra> Checkable<Context> for TestXt<Call, Extra> {
	type Checked = Self;
	fn check(self, _: &Context) -> Result<Self::Checked, TransactionValidityError> { Ok(self) }
}

impl<Call: Codec + Sync + Send, Extra> traits::Extrinsic for TestXt<Call, Extra> {
	type Call = Call;
	type SignaturePayload = (u64, Extra);

	fn is_signed(&self) -> Option<bool> {
		Some(self.signature.is_some())
	}

	fn new(c: Call, sig: Option<Self::SignaturePayload>) -> Option<Self> {
		Some(TestXt { signature: sig, call: c })
	}

}

impl<Origin, Call, Extra> Applyable for TestXt<Call, Extra> where
	Call: 'static + Sized + Send + Sync + Clone + Eq + Codec + Debug + Dispatchable<Origin=Origin>,
	Extra: SignedExtension<AccountId=u64, Call=Call>,
	Origin: From<Option<u64>>,
{
	type Call = Call;

	/// Checks to see if this is a valid *transaction*. It returns information on it if so.
	fn validate<U: ValidateUnsigned<Call=Self::Call>>(
		&self,
		_source: TransactionSource,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		Ok(Default::default())
	}

	/// Executes all necessary logic needed prior to dispatch and deconstructs into function call,
	/// index and sender.
	fn apply<U: ValidateUnsigned<Call=Self::Call>>(
		self,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> ApplyExtrinsicResultWithInfo<PostDispatchInfoOf<Self::Call>> {
        let maybe_who = if let Some((who, extra)) = self.signature {
			Extra::pre_dispatch(extra, &who, &self.call, info, len)?;
			Some(who)
		} else {
			Extra::pre_dispatch_unsigned(&self.call, info, len)?;
			U::pre_dispatch(&self.call)?;
			None
		};
        
		let res = self.call.dispatch(Origin::from(maybe_who));
		let post_info = match res {
			Ok(info) => info,
			Err(err) => err.post_info,
		};
		// Extra::post_dispatch(info, &post_info, len, &res.map(|_| ()).map_err(|e| e.error))?;
		Ok(res)
	}
}

/// Implementation for unchecked extrinsic.
impl<Call, Extra> GetDispatchInfo
	for TestXt<Call, Extra>
where
	Call: GetDispatchInfo,
	Extra: SignedExtension,
{
	fn get_dispatch_info(&self) -> DispatchInfo {
		self.call.get_dispatch_info()
	}
}




