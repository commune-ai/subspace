use node_subspace_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig,
	SystemConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_core::crypto::Ss58Codec;
use std::str::FromStr;
use sc_service::config::MultiaddrWithPeerId;


// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn authority_keys_from_ss58(s_aura :&str, s_grandpa : &str) -> (AuraId, GrandpaId) {
	(
		get_aura_from_ss58_addr(s_aura),
		get_grandpa_from_ss58_addr(s_grandpa),
	)
}

pub fn get_aura_from_ss58_addr(s: &str) -> AuraId {
	AuraId::from_ss58check(s).unwrap()
}

pub fn get_grandpa_from_ss58_addr(s: &str) -> GrandpaId {
	GrandpaId::from_ss58check(s).unwrap()
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		subspace_module: Default::default(),
	}
}

pub fn nobunaga_stagenet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        "Nobunaga bittensor stage net",
        "nobunaga_stagenet",
        ChainType::Live,
        move || network_genesis(
            wasm_binary,
            vec![
                authority_keys_from_ss58("5HpYFp6rxbwyP6XTo1P9JW66MG4GLGSDJXzyqBBkp9izwixs", "5GCPtsbg2B6MfzittNrYEavQRcNkrsuhEPLLQknqNSEa94QZ"), // Berthier
                authority_keys_from_ss58("5G4AFSEocmTEJifjTftjMeJwKxtGSsf7u8jvzKei7Zu5yPf3", "5DfELfHmiBBj8pJ2oAn7t5FYb7sJ3aourqGRJ8w4b7Nm713C"), // Davout
                authority_keys_from_ss58("5HWMSqJGuaWuLAftrkc3NJTYGFAWSZqxxJLHuP1Zfs4usWMV", "5DyGjGqKdsdwMB3wA8PhP1xhQPEJubxZ7ytLg7w7HVVjdjBZ"), // Junot
                authority_keys_from_ss58("5E7WNsyUVP3KDVaCGWA35qrqij17wauQrbejGatN7bSvHQJA", "5EdvJs8mVYoargVafkKtbWxTzMmKcu1RF8JX7KpKsTEkzQKb"), // Moncey
                authority_keys_from_ss58("5G7LZAXo1AfF3xt8XgkvhpcfZXiRNrfzPMgWqkJsrfwwnoVG", "5CUm3sJVZB1Wbd5oyfqddPvp313bxT363eq64BjN4A2XRagC"), // Marmont
                authority_keys_from_ss58("5F7DdihUa1TDC6aH1M2ReJdrUU8Tp11Az2fUYeUYw8Bu29sv", "5Du5u4b43dxj396gQfB3bSDs3CXvgBUQACUVyaNAFaKdF8XU"), // Massena
            ],
            AccountId::from_ss58check("5HoqMpw98Ys7MiF7vxN28a8KGyU1jbReTQJrddwXY9QpRpm1").unwrap(), // Sudo
            vec![
                AccountId::from_ss58check("5HoqMpw98Ys7MiF7vxN28a8KGyU1jbReTQJrddwXY9QpRpm1").unwrap(), // Sudo
            ],
            true,
        ),
        vec![
            MultiaddrWithPeerId::from_str("/dns4/soult.nobunaga.opentensor.ai/tcp/30333/ws/p2p/12D3KooWS2CcPSkKKguw5vvzKHMroh33h523H88aAgNos7UCEyjh").unwrap(),
        ],
        None,
        None,
        None,
        None,
		None
    ))
}

pub fn nakamoto_mainnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		"Nakamoto Bittensor Mainnet",
		"nakamoto_mainnet",
		ChainType::Live,
		move || network_genesis(
			wasm_binary,
			vec![
				authority_keys_from_ss58("5D2tbCvaKpNyvnM1D272MHnjFXMfmR1A4aJ24XfSzH9zcm9t", "5Coe1QXmsMoXZMrQjYoQsUhYvVHDWze29HtrtLNEucdSY2qY"), // Gundam
				authority_keys_from_ss58("5EekXHmQUjBrFQPct6Zwbg39E26mFrVwXm4HoGkSGpJZNr3A", "5FaU1wfZA4L6ob7c7U6c4HxLx96VECFQiporuo4QMiNMsfj9"), // Connor
				authority_keys_from_ss58("5HEAoUKQcvH2GfXnNDrDcfFxAjuHvW7S14TFkfz6qYX3VsQy", "5CgXTNLmj3M7SMY232atsRweCRVoLKCaNFRfnYLuUBxojcpU"), // Miyagi
				authority_keys_from_ss58("5Dq2nwHZCCaSiwYzBVhVp4fusmLh8eZ4nmzMzesqUnvW4DAs", "5DuyMR9XcD9JFN79YeGvGQDLVrE9ofcXWpGjmdbaggv9ZiWt"), // McFly
				authority_keys_from_ss58("5H4PTNoyHZH2V8HkSamM9s4CE4SwJBmkq2KDF6zXrntTxwHs", "5FMeJZFHKjEPkoXS7svhfcasuAbvZFJnB8E2Ms1MTytPcFk3"), // Bodhi
				authority_keys_from_ss58("5FePgMvuGPsHwaNKGvcC3eU7mv9uHcYwxWNaeYbgBs2RSFft", "5Eh9RhakCQATG3uvvvhcPv1xAdpFc31giDWK962bgFLD4vHz"), // Vader
			],
			AccountId::from_ss58check("5GLKGJdjCwBYgtim7F4eZCxDC3bMe9VvhCnpG2k9ihdyPX9p").unwrap(), // Sudo
			vec![
				AccountId::from_ss58check("5GLKGJdjCwBYgtim7F4eZCxDC3bMe9VvhCnpG2k9ihdyPX9p").unwrap(), // Sudo
			],
			true,
		),
		vec![
			MultiaddrWithPeerId::from_str("/dns4/skywalker.nakamoto.opentensor.ai/tcp/30333/ws/p2p/12D3KooWDWw2Ph2JLHFxNUAhLpgGf2HAE2BHjXkoaj7HS9HapKnc").unwrap(),
			MultiaddrWithPeerId::from_str("/dns4/kenobi.nakamoto.opentensor.ai/tcp/30333/ws/p2p/12D3KooWASPUokJTdXKKYvBhkcX4JxNVfaN9WUqZuoL33NJQEu7A").unwrap()
	    ],
		None,
		None,
		None,
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn network_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	_endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			balances: vec![ 
				(Ss58Codec::from_ss58check("5FCJNwo2MSnHBEgoydnXt1aLGdFL6pCmpte476nFQ4X5vmxe").unwrap(),6058535716465),
				(Ss58Codec::from_ss58check("5GnAUFDCvHWVETq96c23CBpQhbn11nNpPJ6jPynXdtvjcxUP").unwrap(),1361471253587),
				(Ss58Codec::from_ss58check("5EWuhvnio1pAMX83tTk8FBXBx4NYGWHKotJLmEXEfJj3u9Cj").unwrap(),12169330936148),
				(Ss58Codec::from_ss58check("5DJ6UmSrhYkyMzLB5ZVdNv8SDX23Hdx72XUEkFcb2Y3wq3cU").unwrap(),1329405857621),
				(Ss58Codec::from_ss58check("5G9uJMBHiZXnQua4wk1B2dFtCxGaP96rekmmYb8Y4kLUnKAQ").unwrap(),4737003367291),
				(Ss58Codec::from_ss58check("5H5xtaUkbNWTzDzAcMGh9Swt7RHgxwXPzxSRN1VGBxJ1Wbsh").unwrap(),6053808614933),
				(Ss58Codec::from_ss58check("5Hjb7tT48v9GsQftP4vVv3QswZaB848AmSWsW43c4MqBDi2D").unwrap(),2178729414305),
				(Ss58Codec::from_ss58check("5DtCTKk27a1g3NC91QsbEvqpf1A4LFWLjEj2f3FEPy8N8xB9").unwrap(),487427554332),
				(Ss58Codec::from_ss58check("5FnDcv3ntNcNugfDncs9HMavrFJu3vFaBVHFwqA4VL35gdUT").unwrap(),288556411810),
				(Ss58Codec::from_ss58check("5F4e5zE5cVwXPUygmWTf2fnf5PgkkHSpGpqWRLn6Rq47nnG9").unwrap(),7007711172010),
				(Ss58Codec::from_ss58check("5CUZmEGKAkacgYkhkVGyMLArXZ8YzYWuv7GafiKQbu9Cr84N").unwrap(),83652486479),
				(Ss58Codec::from_ss58check("5CkDjgxJ8wHWJXGDtXaSo2WZchqC3x8eAXuXNK22WoiSCLsW").unwrap(),5245312473),
				(Ss58Codec::from_ss58check("5HVk87EYtPdxcMdpBGFDjFUcUReAaoP6nwSXNDywZrJfwwA5").unwrap(),33305099562),
				(Ss58Codec::from_ss58check("5D84qo6Q4CR7QnZt7uNtnnMa1ieEyMRB7UAWn2wGQWCrE6z6").unwrap(),6887459691),
				(Ss58Codec::from_ss58check("5GjFoWA7V3QRLuFFr9arZMvYVrqvcZvVxoMtcQHTnCoFEQWs").unwrap(),3404807014720),
				(Ss58Codec::from_ss58check("5DerVC33R61QG32TyVD6GFqBhgyq9RHDu94YTsE9TPdseDoX").unwrap(),52554103968),
				(Ss58Codec::from_ss58check("5GR7WqcpS9SUxBsEUG2mr8HHb3x7vNEsVPPq6dDXri2cTmTb").unwrap(),83334970226068),
				(Ss58Codec::from_ss58check("5DeWrUYvYdegBtWcKMUMvCmDpsMUxfSG1gxHtiBEFMhfSS64").unwrap(),31642191325),
				(Ss58Codec::from_ss58check("5EPYbmMFu5axn8RxHxTXeNkgt5RAGA7G6NMcKKR8pdkcd938").unwrap(),17245685911445),
				(Ss58Codec::from_ss58check("5CPzftp7YLS2D9ycFyN7Gta7AtvSApHPwFbu5pmxBSenQ2jy").unwrap(),23521025615),
				(Ss58Codec::from_ss58check("5E7sLXbV7KfW8kbZ1caLVSX4tR24Ze41jCZ1JTtn7LCkCbbL").unwrap(),10467648816),
				(Ss58Codec::from_ss58check("5D7Q7Vg8L9SvACN4C3BPwpzLmfscJQi3h7fw2PTxmfamC1s6").unwrap(),17575545933),
				(Ss58Codec::from_ss58check("5EnFFZvE1JYpKJQbmYzorhNiB7Cn5Hc5YtMYhTaD7F88eGBC").unwrap(),2542826207),
				(Ss58Codec::from_ss58check("5H6Yc5AxjgdCxxVSt9XWBe7XKBxJ7cZUcvBxazHzqQKzpuaJ").unwrap(),47113887399),
				(Ss58Codec::from_ss58check("5DLDP6ZzARGucxjFW6bnZ3LwWsGtZ9aW1rxqFxe3z12G5R4y").unwrap(),223826458145),
				(Ss58Codec::from_ss58check("5Ec4vzPHibBpmNDpgUr3YLctnQo4yUwDQuUUowkjBuA29XPj").unwrap(),31218201951),
				(Ss58Codec::from_ss58check("5ELJHdcUDj8P5M7Zihbf2huomfDzczBtn91BB9RNDLvPE8x7").unwrap(),2763151048794),
				(Ss58Codec::from_ss58check("5EvxHFhqgcpiTwRn46dG9JoyNmtpr6T2YhWJYkP57CuUwHTe").unwrap(),11233306078),
				(Ss58Codec::from_ss58check("5GYYC6c31EjQwDFdRULdxGv2UqDGQ3TAMyqBEnUH5g9W4G53").unwrap(),60410035075),
				(Ss58Codec::from_ss58check("5DDBm3sDeWjnDJWPahPb5APmhmePoHVnsYRWJcPZMC4FAJNw").unwrap(),2483656161),
				(Ss58Codec::from_ss58check("5HR7J9oH73naotLbAXc3rB8ib6Seocd3uMVbP5166nJk6ibg").unwrap(),10806953990),
				(Ss58Codec::from_ss58check("5G9UTRXYWmDws2oDPnwyrwV1jCBqXLi4nmH3wn91nfe23VwC").unwrap(),7085630875734),
				(Ss58Codec::from_ss58check("5CzNSYGqdoJGWuR2rubJfnmE7N5G7hDJ7xc6FxbfGkLjC9WD").unwrap(),6294060755),
				(Ss58Codec::from_ss58check("5F6qQRSjA94vzfAyzJtteinQKpX2Dyz7sdnnFFNNK2k9e5q2").unwrap(),44536681840),
				(Ss58Codec::from_ss58check("5DfKNex3C47REM48CdHzmZ5MtrdNgSVVV6W3DBSgWQyihKoz").unwrap(),56784795576128),
				(Ss58Codec::from_ss58check("5FnCFXXAG2z6x2fJVcjX2FsSj5pEdwWsmnUbc2fz5aBrGHCz").unwrap(),72188648231),
				(Ss58Codec::from_ss58check("5HNQqtmwdF8qxttxQZZy1qsizv9Affu5TWYjXP78Kc9BJmXc").unwrap(),43898494074),
				(Ss58Codec::from_ss58check("5DChW2gxVs8QR4iWHB1qrHABqViMe1cHRDXwfBnDACrMvT8b").unwrap(),8680203905),
				(Ss58Codec::from_ss58check("5Ef7mnoLxfHqNxHNDkRrXLAoffzjb3sE1UAnWsoQnr8sBBW1").unwrap(),304850397668),
				(Ss58Codec::from_ss58check("5FKWrQVyiyXibpXrjhwNFtWN4tKu8Cf9BLRENJfAjt1wUDiP").unwrap(),2365594857),
				(Ss58Codec::from_ss58check("5GKyqqiQy9u7XwhDM9yHztqr7xezTkp1juf6pb6FV45ckyuC").unwrap(),1140568050),
				(Ss58Codec::from_ss58check("5G9eYJ35v1ok32pfsu9DeNYZTY5LXVsC4eUo47RLe4D9Kz9W").unwrap(),190311652503),
				(Ss58Codec::from_ss58check("5F6zds2aQgGMcEVum6waFyoJLiUAbK4nnwq7B1fwwmbzqAE9").unwrap(),2138832677),
				(Ss58Codec::from_ss58check("5EbyKoerE14yGV6WkqsNEgny6okUv5zmyxbFssen2NLhiXwD").unwrap(),2482165557),
				(Ss58Codec::from_ss58check("5FEToRY3vHQii3FjmkvJYBY7FQKXgAosoS14BGnXDoobNhXz").unwrap(),1581251482),
				(Ss58Codec::from_ss58check("5H9CdM4uURTdBPGUdyJfSg3tTBQFxpa6i4UCqu7xx1G1Gtd6").unwrap(),1630363615),
				(Ss58Codec::from_ss58check("5DhYzJpZa9TN4hfLc7ouwQ4sCMTGFPrK4s2YvBcbwrzZpe1Q").unwrap(),60240430840),
				(Ss58Codec::from_ss58check("5HgoAEcxrP8F5t4qwKjBAXjxaehv959fwkBkRQbmz8tr5ers").unwrap(),4068155752),
				(Ss58Codec::from_ss58check("5EZHW7ZK7EYe4TZXFzruob57kCgA9jRkyNPP1YWLmNaoecuV").unwrap(),5388550182),
				(Ss58Codec::from_ss58check("5HQQDfJpEw9sJjyL7y7n95JaEsvLDFUGXtLnYcMtX8fbGxHe").unwrap(),2076075000),
				(Ss58Codec::from_ss58check("5FpdqHyazvQ2X73zrKRuorBSmChBefAH5QNT6FoPneTnJEiA").unwrap(),62172953321),
				(Ss58Codec::from_ss58check("5ESNdmSSCYhDWniQwNDimWdgaQcB8JVHS7bjhsKJWASLbUMg").unwrap(),1920513902),
				(Ss58Codec::from_ss58check("5FKaTiBGSGwMND4MZyBoTUEaz2R8RJDeVyWjZwBKGs4dHam5").unwrap(),1676557423108),
				(Ss58Codec::from_ss58check("5FTRRGxLnjhtPSyGamBcEGpzR5TmErgff8uMLphpv4L1JY83").unwrap(),1387376275),
				(Ss58Codec::from_ss58check("5HgDq3PFNUZnS6tZ6E3eHkrW2YUm53DZ7WYty6otkEKAgUhs").unwrap(),3844563095),
				(Ss58Codec::from_ss58check("5Do3tQ9H933HmomU1b5prZRyyKJgwHV4YR5YGRKufFAuJR4h").unwrap(),162588287560178),
				(Ss58Codec::from_ss58check("5EAPe28Y6P7vxWtLqcQynrrcCAjwmeFHLqgS6cEGEQf3HMai").unwrap(),58599001218),
				(Ss58Codec::from_ss58check("5GWntCvQnUZNHBsmyTwr1yybEnco8SsHwfg3HCRhkmnyzK5a").unwrap(),1998484999),
				(Ss58Codec::from_ss58check("5F9Ay1jccuqeXe3mSpn4mVUK4KCrDVR3HD6dwfRYQBqxJRfS").unwrap(),808269682),
				(Ss58Codec::from_ss58check("5H3UyBZ2xZsT67X1sHGZ44Z8ypu37jYPdW7oHyHzt36cMLMD").unwrap(),85117481303),
				(Ss58Codec::from_ss58check("5E2FxTujz7jkvhLKsnDpWeuvcighX1iLmbTirTGvsVvEBgN7").unwrap(),744495696),
				(Ss58Codec::from_ss58check("5CZpGBEXjGqFRVQuRbwdqj1xVs7RmbYNACFKs5kEp3SvVacf").unwrap(),1723310073),
				(Ss58Codec::from_ss58check("5G73fJzBkR9fBMkDs8KLyMbfo113Z3bZiz9Xu4e6vTZf7exf").unwrap(),1683214511),
				(Ss58Codec::from_ss58check("5FnLasufyU45ugD4DyycZos3qqnuVziygGJD7cBUKmwBnFmb").unwrap(),2067289966),
				(Ss58Codec::from_ss58check("5C5hahvA4C99jQYgLVeWd9an3qipR89uza9VzbnTe2C64LA5").unwrap(),50441000841884),
				(Ss58Codec::from_ss58check("5G6zgsBzRstVRQbPqcHT9CDcJM2TiNzT5JYTkSz2wnkTSGXp").unwrap(),117409283297),
				(Ss58Codec::from_ss58check("5H6WBY4zBUsE6DLXm3btLV5L1jg4tJbEY42cUnZYoeQpaCv9").unwrap(),2213280512),
				(Ss58Codec::from_ss58check("5Ei7HBTN6TTuNMgLMvnQq7yjdoJHRwf3QvBYVhZrVXiGuSXq").unwrap(),3584564884068),
				(Ss58Codec::from_ss58check("5FESee31XdRqGvehxhDFDcFqCohs6GzDSWBKLMX1e9Z7DqeG").unwrap(),15331520056),
				(Ss58Codec::from_ss58check("5HGmuSuPhLDu17W2J6UyghJc7Mq93p5kUP3vhcNRx9Q7GQNe").unwrap(),166571643954),
				(Ss58Codec::from_ss58check("5DPZoVx9njcDM8gXWoR4TvWYWtwhQGx3kb6yX325CoNRVUcA").unwrap(),36774794268),
				(Ss58Codec::from_ss58check("5H6gP3Ma4aiD7aZjwF8TtV68qJM9hRfi8T7GS1EDnmNjda3n").unwrap(),11577681307),
				(Ss58Codec::from_ss58check("5EUzHvF1PnumC941HUoEQmtYtatfztZbLyxV4a2zDGfrVVoQ").unwrap(),13992338646),
				(Ss58Codec::from_ss58check("5H1k74pjguqzMSyssoH8R1hEfvBxqXVHS7XdPqy7YssfDQUa").unwrap(),9635759754),
				(Ss58Codec::from_ss58check("5FF8MMhvVXsdMNsiLb3dr48zqhMtCcq5CnqqA3M1mg26LQiU").unwrap(),46145297921507),
				(Ss58Codec::from_ss58check("5CMjXsAYM3LnkiFTwri7KX5RPSgc649NLXQRZ8uTLBCUwEjg").unwrap(),1226403883262),
				(Ss58Codec::from_ss58check("5DD8H46dVWTEhGfDXeurLPSFZRHvidgQfJBfYEyorcZgyJTk").unwrap(),21356325812),
				(Ss58Codec::from_ss58check("5EfPMwaXKjxyz9Vkwdzu6SZgqWGx8c1sXJkdCsPnLgvgbd9U").unwrap(),144751749326),
				(Ss58Codec::from_ss58check("5FZaDwf3U9E8jhDuJS4DM3yPsUmruwJ88RpnR9pJsGKBGkiq").unwrap(),10741684461),
				(Ss58Codec::from_ss58check("5DqnuTsPdHUExKQ8w9WKpnUspP7cmdrdXvSasE1vvrEjB9qn").unwrap(),7689022928),
				(Ss58Codec::from_ss58check("5D57qEJ9HDZM7uZnhvQYbtgWXVSyas9Ufj7H6nhCBAvNWMTu").unwrap(),3074777681),
				(Ss58Codec::from_ss58check("5H1FCbM9rFEMZXAGLaiDdViUuxd3v9hsWrtwi1PZm14Pao4C").unwrap(),8381985802),
				(Ss58Codec::from_ss58check("5FhPQJS2kaBwrNekW7e2xsj5CS3gEknn46assUCmreafoMor").unwrap(),845908867535),
				(Ss58Codec::from_ss58check("5GgPDhBvztUwGzscqYsekSBTcLjFDCjiJpkfv113XHSFiJYf").unwrap(),24453162475),
				(Ss58Codec::from_ss58check("5HB3ek7FC6nKA2Wnc32FpuyMKSxh15DvP7rwzyW7PLMKEPtn").unwrap(),339046172042),
				(Ss58Codec::from_ss58check("5DchgrpU6AMXTqzJic4mvTuyXRNZZ5ryJPXx3dWYexbWvw6Q").unwrap(),44273077636),
				(Ss58Codec::from_ss58check("5HbaohCTous1t4AYAgARXbR3PRME1JYG7H38jmr7o69Ct2UX").unwrap(),11287309691),
				(Ss58Codec::from_ss58check("5G6kVnU3wBhZQWF9hPWCcPZvfndh9NfRNf2cGQof4dZZC9Yb").unwrap(),96731129531),
				(Ss58Codec::from_ss58check("5C8KTfrqiudgkxhx3p4SKdBpaoJPXehwJ87R9njBkQDZP58t").unwrap(),694946850),
				(Ss58Codec::from_ss58check("5EZntQEpV2eQ81PawaQt5Gbmn8KK8Gjqhkh8hCGtU4TGYvQe").unwrap(),854321393),
				(Ss58Codec::from_ss58check("5DqJfdMXLQASQwetfbiq7Jp1eNbuzc8vy47KTZ83ydXminK5").unwrap(),37799552779),
				(Ss58Codec::from_ss58check("5Da2AQZTDCGryghMUddHjB6HUMU9NfpLSLjMwLBm84JeD4x7").unwrap(),1668906229),
				(Ss58Codec::from_ss58check("5Guz1Buq5qifxoZXaP79BXh9sK4Vve5kyLLRQskrAKKDDqZz").unwrap(),67635636409),
				(Ss58Codec::from_ss58check("5HQMXSFr8j6nJWPrk6Zd5TGvhHB8vJDzUd9NRgh5bTrFvaHx").unwrap(),3233668397822),
				(Ss58Codec::from_ss58check("5CfZCocFJrKodzeuaC7AZKZYSX9Lb1z92BUnYo3i3rGnp6yK").unwrap(),158432635265),
				(Ss58Codec::from_ss58check("5Ei4X89AGEY7gThjHNMy7VLytFff1FXowCo8DHax5ENFDqbq").unwrap(),68955551345),
				(Ss58Codec::from_ss58check("5CvsizhizFZ7ApDM3izqtDuJBYHPRUJfnjhiC9i5WQnccxwb").unwrap(),9661544717),
				(Ss58Codec::from_ss58check("5C5JFw1yfvdJSE6u4KiWJfd5KihahdqihFiGoNnSBZSkVLKB").unwrap(),141244441207),
				(Ss58Codec::from_ss58check("5HDkLBH66bNVBbAVrQHXV8CQQnnvjVWwWeY51pwoYJqfwbFy").unwrap(),369296176156),
				(Ss58Codec::from_ss58check("5GUD3qi4tqa9J67hkhhfLVmaMHysw1W5fk5kNuBb9pexPH61").unwrap(),3848919393319),
				(Ss58Codec::from_ss58check("5H8W6sCQE3CjyK3XjKtia3KGjmngsKUkLAMyWbfmNFsupCbz").unwrap(),13878875307),
				(Ss58Codec::from_ss58check("5FKrf2dESBKTpN4KtAEVRraYNvhwTEVzJvqE1oRiiUbrPLTs").unwrap(),278717975498),
				(Ss58Codec::from_ss58check("5Fj9YAfZA1an5YS5JGREaq5WJKQzXzxWf8ZWoyPavjsjVZzj").unwrap(),147622075351),
				(Ss58Codec::from_ss58check("5Gj6PV1GNkx4eRRWxufcq85Ln9FKHNUHDwy2ZxcFYRJDsE14").unwrap(),299212555085),
				(Ss58Codec::from_ss58check("5DywEmwKpZ2PMupGjwFrTw5nGHfHZoDUAg2KQ1pLDFULHKvu").unwrap(),2007849670),
				(Ss58Codec::from_ss58check("5FhJtBpBHdQ5T6HBgQiZDh49Gwx2qbT6Ar6bX3JmMbMRGAcE").unwrap(),32470702319),
				(Ss58Codec::from_ss58check("5CJzPh1dfx1aHcxthxNXT1MeF4u9xAkr9t24UtwgUBbASdWD").unwrap(),278955265684),
				(Ss58Codec::from_ss58check("5DwTMekK5Jdzemt1qog4JzmWrpL1LPZy7Xrv8VUJbVCw1bnH").unwrap(),622417932235),
				(Ss58Codec::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap(),1040631877269),
				(Ss58Codec::from_ss58check("5EhghokF2zVxEoZMPb2nxzw197ZGjeZvF516SsjkW1yGtpvT").unwrap(),561390712888),
				(Ss58Codec::from_ss58check("5DCn5pGPgLKiydV1HngArCJq7p8KLkD4PFeCFtCrEtAQWQE2").unwrap(),228554348603),
				(Ss58Codec::from_ss58check("5FCtfesvGbVmEBn6zFVT7xk7pGQttzv6GwqLYwc7gM6c5Q36").unwrap(),27393352551),
				(Ss58Codec::from_ss58check("5DvqTBhAVefrahWPkQ4pGAE4USu3ooSJzP1rizJpYwnx6cmr").unwrap(),212420350000),
				(Ss58Codec::from_ss58check("5GQnsc8xmdbTp48FoyqHdWTZRf7NDgMPqLHU3RL6KGxRmyEo").unwrap(),785226202660),
				(Ss58Codec::from_ss58check("5F75S7Ku9Q8hw81hhNXfJkKKumdrAR4LvCWPVmdqFUvTiXq7").unwrap(),62948051459),
				(Ss58Codec::from_ss58check("5H4QnD7581Vrf3gRdQxnjr2bF4x1ipfL9aavi1ntKPQjoy39").unwrap(),44323661005),
				(Ss58Codec::from_ss58check("5CtUzBjrhiM1tBWYnbLHNYzLkykAYpggokW5pLam4585ttGZ").unwrap(),556656340565),
				(Ss58Codec::from_ss58check("5EbjA9DecXPDRKB54z2Qz3WHjERvVquV9W23MpDec3W8d394").unwrap(),5905645907),
				(Ss58Codec::from_ss58check("5H8ukzMrAdA7FjRDBqmaUiAFfWxKeFRGupMbRkoeG98vTwfD").unwrap(),388999787121),
				(Ss58Codec::from_ss58check("5HTxRWcXDc6T7L714Lmn8o6ZnmnqtWpCFWZ55QtHzJCvzvbB").unwrap(),26097303850),
				(Ss58Codec::from_ss58check("5GHXY4njngDKuVrF8WUKBPsmwwb3eB1c29gJ7YQ6CAs5w747").unwrap(),298289228408),
				(Ss58Codec::from_ss58check("5Gxoeiccdpc4L3MoEnVhe9v3eHBmzNzibsVqdtym8SaYVjvi").unwrap(),199436838007),
				(Ss58Codec::from_ss58check("5HK8fz6k9435fc4yn92fcxV3BgyWTQtypA4FzV2mdffwgQqr").unwrap(),194686025505),
				(Ss58Codec::from_ss58check("5Cyqi5hEgjEe87hwCLpqMrdzfWxQc42N54Kntw5316zHepQZ").unwrap(),48445479309),
				(Ss58Codec::from_ss58check("5D4kKDfNEkHMjnjgdb37N457SCC8t5kn4fzYG9MVVhhsKyCj").unwrap(),191663985539),
				(Ss58Codec::from_ss58check("5GKvroHmENQUJygnviunVB4UZNDfmdibMD4msKnmZYJ1jDYi").unwrap(),200569261180),
				(Ss58Codec::from_ss58check("5FeZAvdC9a1qMVW6JLvCM52ESHf9uXm6JrUJYLfJpL2YedXy").unwrap(),4088658973465),
				(Ss58Codec::from_ss58check("5GBfJjQQndeF6YNtuVbcanBEPdazW5sKmZJBEnBz74Q5Nbzn").unwrap(),1526047522245),
				(Ss58Codec::from_ss58check("5Gb7fP8kCbdTJmX21tSnBjZXtJDKZsojYizm3Q3wKibzRfy9").unwrap(),2651799226),
				(Ss58Codec::from_ss58check("5CUZckMDQPe138UWy4bDo8LopuUA87f3L82d8hDCgd6MoxtK").unwrap(),1569922774058),
				(Ss58Codec::from_ss58check("5FgkcyWK5JX4CNJJXhcQJxJokWs1DEQ7CrtqgRVZcpfWD1de").unwrap(),5954212274),
				(Ss58Codec::from_ss58check("5Gn1FKPQzq3S1icXXkXhHfQgDSL7XYff8HQCnYhaW4jCNaFq").unwrap(),2320657109036),
				(Ss58Codec::from_ss58check("5HVypJhQPkste1S3A3b1g3K36bx1kdBjGH8ZLQk3UPEZhkHG").unwrap(),53058578924),
				(Ss58Codec::from_ss58check("5HT2bx1JZRQMPaCHqiLX8nmNnRriSQjyc8Wau1RgCkpMpjFP").unwrap(),160025154425),
				(Ss58Codec::from_ss58check("5CCbQ67eaWmonRJrevUoimAv2bc895PagRMX3UPMRyc2MknG").unwrap(),1792295283),
				(Ss58Codec::from_ss58check("5Et8EjoKmnKG85Q8wJDYNg7V1g3MTEwwPizT3w2mtusNXs9Q").unwrap(),58091686109),
				(Ss58Codec::from_ss58check("5GbDgrvSNaAgERBKxC29vvQy2f9iXNofN9oUqbwwidxJCLzn").unwrap(),9512824500),
				(Ss58Codec::from_ss58check("5DwMRWiPdMXEePxg8bZmAjMqfPwKjztRuncRSybdHyLRzBUM").unwrap(),45849129981),
				(Ss58Codec::from_ss58check("5DFyv1GBTxTU5QL4pgPXuqzXdc23E6LfsSMHdCNMbB2x4HEw").unwrap(),16981911713),
				(Ss58Codec::from_ss58check("5DkvAVfngiy4wC2mJfQWDwYCAChyaYuymyok58Ezd2cgwdta").unwrap(),56808838217),
				(Ss58Codec::from_ss58check("5F9y4SvJGwoBWCPLZ7ozUQbj9bvevdHLVvvd2VsjDsMuyH2t").unwrap(),3358208730),
				(Ss58Codec::from_ss58check("5ENqHN7pf5B6ws85i8iy53RGMrjw7CRfYe5j6PxdVjqMi3hr").unwrap(),409406512),
				(Ss58Codec::from_ss58check("5HgXo7ia6cwzyzpxnZtaZKPTZR5MhEHxC27hsaw7692gVLfk").unwrap(),36794333157),
				(Ss58Codec::from_ss58check("5ENw9NfLbwK6SEKuk1Jd8x32xHCbyu52rzjm3zbC2jVtYAK6").unwrap(),4816657720),
				(Ss58Codec::from_ss58check("5EqWCxF9DgLweQWUs6VR9FM3iydP72bqJ3aCmhVcg5N9HDvK").unwrap(),969932583),
				(Ss58Codec::from_ss58check("5DNogT2HynwJJmjWjEkweJxgTfF4LMhimdDUdx3hgagSUn3k").unwrap(),205552967813),
				(Ss58Codec::from_ss58check("5D5Dwkd7Vc7kkrKDm4jhcZPqQ5LBzkrjFhg3huBEDQvoDa96").unwrap(),2127828365193),
				(Ss58Codec::from_ss58check("5GWVhXLwkpAVxZCk68NW6zJd2ivREnLmeef2BBAiLsxMQfGA").unwrap(),27108635848),
				(Ss58Codec::from_ss58check("5C8DuNYhmsmMoN4LwvQzB58gocPiHbhMGYHvU8jM31oXsHhW").unwrap(),2051502308),
				(Ss58Codec::from_ss58check("5GGwMkegWj1xfv6MqNjYXubFMkDdfAZDz4qEMcB3CLJKAX33").unwrap(),11956774340),
				(Ss58Codec::from_ss58check("5H9FA9ETMt4dUMocTVq6srjV5RRPJRHm9WsdYktk1A4DVjhf").unwrap(),3237346816),
				(Ss58Codec::from_ss58check("5Ea6wBBChJTet7R7vpM6eg8ssSJDq31sz8j37Tw9GqgUGJXe").unwrap(),141356285019),
				(Ss58Codec::from_ss58check("5Da4g5UDVmcJ2nz7bQmFiHCKg22Gsk6V9GJvsM1jLkF2uo12").unwrap(),7552451121),
				(Ss58Codec::from_ss58check("5GKLDhSPJQriWx5k2SAeEraREGpc9hvkdJHv3XMFqqqFx8Zi").unwrap(),990527937),
				(Ss58Codec::from_ss58check("5DD26kC2kxajmwfbbZmVmxhrY9VeeyR1Gpzy9i8wxLUg6zxm").unwrap(),140859696687),
				(Ss58Codec::from_ss58check("5Gpew32KS1ykrwsiuCZWw5vcgFPFC1o7DWpmdYzXVfgdm8Fg").unwrap(),6525130288),
				(Ss58Codec::from_ss58check("5HibYJC1tmtCnSjJAca1PUcL6L4cGovyemenx4SYBN1KphLw").unwrap(),2196983062),
				(Ss58Codec::from_ss58check("5GZLLDRP2tTQvJYvZKHUdoYXsDaRpWZFK3Dz5oeduQjPU8ou").unwrap(),113511427332),
				(Ss58Codec::from_ss58check("5ERNyLsMxRnZwoQ6yXYTNGbCb5Ha2umCbeKxjjrp6J6hmr6H").unwrap(),40223995072),
				(Ss58Codec::from_ss58check("5CPxqLgVPKGR2V67yQnReBi82FbMwNxyqeb1Gztg1EEVwMNj").unwrap(),6255835010),
				(Ss58Codec::from_ss58check("5FR4uAsGtDbgUDCrtrqehKYDv1cy5TnWGRQXHGBJdoWi4aTE").unwrap(),1604961490),
				(Ss58Codec::from_ss58check("5ESBRggDbfFgpdPWrZcLMYf6Xi24T6mqLaqErQ8Pr9qjAzuY").unwrap(),634296609587),
				(Ss58Codec::from_ss58check("5GqHrX3hjQF4tDSi1uMCmJfLZ2mQSo41LkXxE8rYACM2gtSr").unwrap(),5425910412),
				(Ss58Codec::from_ss58check("5Fxgmo5vwH7mgvHmNbJCLeykMZmEoGpqNFTyXZeJGSPzJHzy").unwrap(),1227731663),
				(Ss58Codec::from_ss58check("5GEaYuXfcaowNs1fesCivrWMYzAiUt87iBbEJmwVAfDMnv6m").unwrap(),3021936160686),
				(Ss58Codec::from_ss58check("5FZ4diU828dAMakdNYEZCChu32HmdK5y5j5sCir5krUURfVn").unwrap(),63820835424),
				(Ss58Codec::from_ss58check("5CWwYyCNthU86KyJWy63z4bmjRdJsZjhXfTaYDXBsW6Gok8W").unwrap(),429819494),
				(Ss58Codec::from_ss58check("5G6YTd5Wn8bjpMM45JZw7N8EmzRzpZt6rSvsb5gSaLUWqbVg").unwrap(),1556715443814),
				(Ss58Codec::from_ss58check("5E1cTrWQe5dVYQH6hfdFgMcMSeNvaydh4JZT1nwPjGGowqnk").unwrap(),10783372859),
				(Ss58Codec::from_ss58check("5CSAoJkLs76kApouczJmXkx3yycqg9hW9vBtB6NyUWf2sTBa").unwrap(),1155376760385),
				(Ss58Codec::from_ss58check("5FCLgwNCXC3pfyBt9M96x7WopNEPTeHbzpBU5SMFjiFrfkrc").unwrap(),645615871724),
				(Ss58Codec::from_ss58check("5DnQGgjghexD2r1s99AYEKEdmm3Wffqig9bypBZPfexpX8Sj").unwrap(),561874382034),
				(Ss58Codec::from_ss58check("5EsjbrqWbvhfdAYLJq5f5PZg7UQNDRS6QRkskZC1eVmwAzZQ").unwrap(),424253407871),
				(Ss58Codec::from_ss58check("5Dz7MnHJnp9wjDzwLjiKjN7PHR2XcSQDEc78r4cKKxqGED1d").unwrap(),1859437099633),
				(Ss58Codec::from_ss58check("5H6YjDdKQswuuNwr6ALwRcokfW22mz7HVTP4jpMMRkT8V722").unwrap(),2297219975672),
				(Ss58Codec::from_ss58check("5FEgCWDTTvsovk4BSahAHg7x9kw6JCXJoRejjwjCKASanjR5").unwrap(),366617147756),
				(Ss58Codec::from_ss58check("5DULtvexBxLWhTGrDAbDcj5Cdp3hNCKFC6B1B2jZwaTdTuH1").unwrap(),84747252981),
				(Ss58Codec::from_ss58check("5EFmEZjiCF9PxSf83vN1E5GoeDC3ZBBDMFtfQ5pJWkcnMe8w").unwrap(),1579634234512),
				(Ss58Codec::from_ss58check("5CP4ct5pokEdEweSbgrFoSCkynzip2bgr7QXU5nWhgQV4X5w").unwrap(),418758637949),
				(Ss58Codec::from_ss58check("5G9C4hNPhctPXdMupe8eUJbyyeeoo2LjXPZU2m2JfK2p7KpM").unwrap(),985315269623),
				(Ss58Codec::from_ss58check("5F4mfSBYsxgMA69c4tP7AKkZKqnrdNaTH1kEtiLrMoubWgzt").unwrap(),1331164829789),
				(Ss58Codec::from_ss58check("5HbMbLjAJFuW4CVjRw2AVY8KKTqz7iV5mYBxqq2hxKCwh3oB").unwrap(),2050860078105),
				(Ss58Codec::from_ss58check("5Ehq9h1E9XY4w8nmPWDMjz3hGo2L9d6ZyvHZE5xCtf845EJ6").unwrap(),281522286947),
				(Ss58Codec::from_ss58check("5CUY9ZSE3ybVou184kqqYG22LBGhyNxwMfpgwkCmUGysRfJp").unwrap(),1695483274651),
				(Ss58Codec::from_ss58check("5GU5vAkS8nknEWyFVRVJ8Kf79NTVvavhPGs7S4Ce9Xpjpneg").unwrap(),3066247084862),
				(Ss58Codec::from_ss58check("5Haf8ZBfQW737ZtDyeeeokc5BSBPmd1pftSHzpiUmBojGvkL").unwrap(),1363711197677),
				(Ss58Codec::from_ss58check("5GBTNtdAc8MF54Azkhh3Cc3eqjTmnFrVptpTugiiRuh2tgYD").unwrap(),95853263039),
				(Ss58Codec::from_ss58check("5HR9egmceuADJMWp72q6uhNRwt7m1yYZyLRh6gBZqDURERnH").unwrap(),963756220),
				(Ss58Codec::from_ss58check("5DRuVrWm59smqB5GBKdFq3BuEBLqo8DSQoUt7L6mgTwYAeaU").unwrap(),1003016645593),
				(Ss58Codec::from_ss58check("5CLmyfZZWsTKAPejwV1Ua4qN6XD7hcu7HXUKu5scnye6pJJM").unwrap(),5603542799),
				(Ss58Codec::from_ss58check("5GZQ8EqNZqk2KiomwFsUST6P96CojNaEuaAa8x388ibKPy1k").unwrap(),60654825118),
				(Ss58Codec::from_ss58check("5GTduSpShRMoJsaAynBfQN83xFzgKBanxh17w2v4mFv7nqLh").unwrap(),64600224383),
				(Ss58Codec::from_ss58check("5Ggyk8u7QwGeUkjWBf4uYnZHiYjHZ5ryN8zNxVg6f9f3beYD").unwrap(),134228856432),
				(Ss58Codec::from_ss58check("5DhyXrY19WxunGoZecLWMcTBMcNV6WLcVouaw2X2ToqWQTm1").unwrap(),507994255),
				(Ss58Codec::from_ss58check("5DhzMTVrSWps3FxXDQsw4vFs9VWJJ6sDSp2Vh9mSpgr6w4ar").unwrap(),65064092857),
				(Ss58Codec::from_ss58check("5EjUHh1rn99SaTUMfp8kdgnCPKkUZjy51tRqCE4MUdPKffcS").unwrap(),887697241),
				(Ss58Codec::from_ss58check("5FsBKfm5Rmh7VGdhch67vrsBXjdZXGzgW1voyXcGxBQ2UzmM").unwrap(),161764564786),
				(Ss58Codec::from_ss58check("5EPczjiqmCBCo3cJf8gwGhR5WYz6oLTn2VWoXpuriaNeXDmf").unwrap(),1113865669315),
				(Ss58Codec::from_ss58check("5Gb6WTKvw2xK1GxCnfFn3QuNb64CKyow2SBXPm1Z259CKM9G").unwrap(),113668499458),
				(Ss58Codec::from_ss58check("5H3C7qRSJYj3TFuxCPNtaRAMJfUtjxmoLYq8PpS9Jsp3hDpA").unwrap(),400835252),
				(Ss58Codec::from_ss58check("5Ctb5J2RYizcuQdkUPcN6KhVg5rRT2YwwfrEBjCa5bFcnHK7").unwrap(),5581126797),
				(Ss58Codec::from_ss58check("5FWXNybFmFzrTNvD2rA4EQRiaRWweezoX9V3iBGxFytXNh1U").unwrap(),24382004682),
				(Ss58Codec::from_ss58check("5FRPBzk1Aud4pUsnjMFbxsqjZBY7i7jcVsWGWNbWjxCK3Yd8").unwrap(),1664351673),
				(Ss58Codec::from_ss58check("5DHdSJSuzCiV341C9S2Kg6Ju7dKRShzDMhZjjp9jH9vwNyYP").unwrap(),1211168830688),
				(Ss58Codec::from_ss58check("5CfMf3sy6yjtFJTE7pNzxEKRhX48vbx85PxQKUsuqPsD58yC").unwrap(),18058743632),
				(Ss58Codec::from_ss58check("5EWUyjdMu1HaFZb93geFauhaNvkaMq2mz99zbzgwj19sKnqK").unwrap(),29535677414),
				(Ss58Codec::from_ss58check("5Hbve8aDRPJnpXvZwiUm8fmeck751NanHaUZTFnUQxZqr1g7").unwrap(),6283253236),
				(Ss58Codec::from_ss58check("5EHazCJw2g2jdW9C7x32YarjB2W34sG5D3BAkK3qv91mnBtS").unwrap(),32324384726),
				(Ss58Codec::from_ss58check("5DJA8EvfKYm2uySwRuDrht4sDwMUC2Y8rJfZRmUfHA86aBAa").unwrap(),590253372589),
				(Ss58Codec::from_ss58check("5CG2xNv394USV8zGzcvkQ56uWo2kCGsLyJrJ6SwZdoTvV5W3").unwrap(),43780103362),
				(Ss58Codec::from_ss58check("5DRTVVgwvvvPXtY5waf4BKNB3xAGrFDr3GC5vzP8yPTN3SVZ").unwrap(),1106640312877),
				(Ss58Codec::from_ss58check("5Ggza7RRB6ixA4F8ja3JKEM6VjtKa7Bxk5maBEi6eKMgKTHk").unwrap(),837696644261),
				(Ss58Codec::from_ss58check("5HTKjbx7KfmK9A1xXc1AsmsLrvVh9dxggLcRCyyVjQ1fgc5X").unwrap(),345567449524),
				(Ss58Codec::from_ss58check("5EEgRYqMB5feyqiUSGLD8g2S8CMxT69XgCUMMt6GBZBtaJTR").unwrap(),1168933608988),
				(Ss58Codec::from_ss58check("5Et6LTRw9vqLR5FNcZp3Sphut2sU4ZsdZnsSpc6A8fEA9r48").unwrap(),763123219926),
				(Ss58Codec::from_ss58check("5GEdGHpvDdbRBinWaXiYLfGQQjPquRSDBoGGVTeK9SpyrZw2").unwrap(),269955432),
				(Ss58Codec::from_ss58check("5CZa9yz6tix2oQUNmSHjD59cCKjFByxS9DpBUyFh5P7HRPq9").unwrap(),716614885626),
				(Ss58Codec::from_ss58check("5FQu3TsfE3c4LANLHQQ5FMRWWcKfJCvJeFedEPCVSTvtTc7c").unwrap(),1071414904),
				(Ss58Codec::from_ss58check("5ESNYy5jmbe6t7GzAaSuZxvmCPbag45HzjvGcB3kzkTAgrnk").unwrap(),61944783048),
				(Ss58Codec::from_ss58check("5Fbm5DeN8LSV6zFF6NkkTMt2VdCtHcThJjK9Ue4DSNcEPU1x").unwrap(),2029174636),
				(Ss58Codec::from_ss58check("5F97m2GptHLV3f3169yikV6cRUNFxH7FgnLx8KvpGwtcdUgk").unwrap(),434629940561),
				(Ss58Codec::from_ss58check("5DSBKDdQm6DA1BWYXoyZD4ta1HKAws2raVrfzjg22cy4QRXb").unwrap(),281813471985),
				(Ss58Codec::from_ss58check("5Dt5Lp2qcFn1UBUvuRvYuJsEhTNZDtNUUvuHRKtLoZkKkbH3").unwrap(),1242621834),
				(Ss58Codec::from_ss58check("5Cqeypgt9vC7u9b9F8tQUT8Dx7Q9KnCyZrQVsY9gPdKrQcJS").unwrap(),804438525),
				(Ss58Codec::from_ss58check("5F2P4tekJrSmcfAJAYjpvi9QFsL96veey4SA3qFyVM1ABYrd").unwrap(),10320863752),
				(Ss58Codec::from_ss58check("5CPVWeySQmkm9DCLv4aJR8UND8Aa96mNvFVBhUgGHyDxX4xN").unwrap(),10897034002),
				(Ss58Codec::from_ss58check("5GHGrWHavwyuQdJAahkEGvrEGzed42sGyk9d7fs5JaP4ntRb").unwrap(),2131287586),
				(Ss58Codec::from_ss58check("5HKRaFfowhp1HHdBCUQ7fmKajR7tozT7TaSaL6sXzrMvU4uD").unwrap(),910821523),
				(Ss58Codec::from_ss58check("5DG54h7Y4yH4rhVEayFgLFUttD9W7XAQe8ZjfBAQcehxj4YA").unwrap(),2102583483),
				(Ss58Codec::from_ss58check("5EsbXr3aTEG341ptz2LJcuLzC9FKScv2YePeAov789K2kFn6").unwrap(),800798564),
				(Ss58Codec::from_ss58check("5HYKJjNCVQChu8k5JR2cPFBsbzLXxs7nWCLgJyu9PmMiWYWs").unwrap(),16906924235),
				(Ss58Codec::from_ss58check("5HjWm8cVkHgsL9eYBtFqxtSVkN4sQTAdtMGvZTkmXLCSnAKD").unwrap(),4787633616),
				(Ss58Codec::from_ss58check("5DU5AuEsJvjhvtMpuFkQuGJYpZ4G5LvEUFmJicMTxM4EfKjf").unwrap(),2438091576),
				(Ss58Codec::from_ss58check("5Fh7n8WNeCoTQqHWSijM6Sfa1CUgkvqxsEBpoY72VntTd6m6").unwrap(),98417573203),
				(Ss58Codec::from_ss58check("5GW1BHMAyauzdsC8bX8UtxcSEikiePehaLYpgTqfYQYFTs2j").unwrap(),3316067812),
				(Ss58Codec::from_ss58check("5GU7sVfAyApwEc2r8SKS8jkTpWLkMMdHZnfxesoM9pMVTAjY").unwrap(),6684300399),
				(Ss58Codec::from_ss58check("5Hq3WKeDpZHRgEnktKkem91pPgJT8PiZ8qEuvbuRfEcMqRkF").unwrap(),824463576),
				(Ss58Codec::from_ss58check("5DCs1YnDpkykejCD3PfoTM3cnX4DksUnp57FKiJPidFBizXa").unwrap(),1328792863),
				(Ss58Codec::from_ss58check("5G1ACMDwBwHgJDaxVp3qUwcPDcJuo2z3P6FQtCwwDpHYjgQe").unwrap(),1163460757),
				(Ss58Codec::from_ss58check("5GBbF4iL7tis5uUY6XDinP9BoRQEoBeFzwKbwuQXEpNvaA5a").unwrap(),4780509053),
				(Ss58Codec::from_ss58check("5CRzdPD9sNgSe65QPbRH5UUAhvCdctVsMzMqXkvFiDozmWTD").unwrap(),6088354219),
				(Ss58Codec::from_ss58check("5He5o8oDDBuLrnLB7cwqUvdczv8VXfFbtALVk9ofFfJgE2ur").unwrap(),1599541064),
				(Ss58Codec::from_ss58check("5GehpxwRj9EjMzybqgKnrxkBrSi8iTyw7zL5i6QG2Rp5SNmw").unwrap(),1464825038),
				(Ss58Codec::from_ss58check("5DkPTTV97kKTSvgaH9akX3XHwjpdhcJjGLBJV6MEHUS3pGdw").unwrap(),1280927955),
				(Ss58Codec::from_ss58check("5F3vSZ8k3ZG8fzpu918UjU8mR6pWEStEFJEVf56g6p3qMvAd").unwrap(),1914724914),
				(Ss58Codec::from_ss58check("5CVPcHoyfSMUJQVMt6FYGf25673gNZgGmdLoZT69ZUh2EsFs").unwrap(),4239601057),
				(Ss58Codec::from_ss58check("5CDRsm7KJpdLywVXp9F2Kq8fm5mCmA9LDU6sqe6Fo4f3xtxJ").unwrap(),2303602943),
				(Ss58Codec::from_ss58check("5CPyMUQVb1Gzv3KiMDcPkrm2v2twHE3mEmwj6WmcPR97rm2N").unwrap(),3418223293),
				(Ss58Codec::from_ss58check("5C8YUdKNMCyKXXBCRR2hy8vd7P7udijJVd8tJxGHYRvq47Fj").unwrap(),2725220291),
				(Ss58Codec::from_ss58check("5CSXo5p3aeRMA7mG1TnqQneA3xYSryGGFg3zLjgcBx8ST9Lt").unwrap(),10083502055),
				(Ss58Codec::from_ss58check("5H6RiKV43C1MUAYavgFEtmXxFJLheMbTGLzTPSp2NSiHUDS8").unwrap(),3729369254),
				(Ss58Codec::from_ss58check("5EFfZJ2ZxhAYsWYa9VhJZZqVPwTufvbKuVmVtwb3Gr5BytX5").unwrap(),99744508270),
				(Ss58Codec::from_ss58check("5GL6gSkRUqvUef3Gwvae5zUA8uYN4pM3NeFTrAwwX3g2PfM9").unwrap(),3152970353),
				(Ss58Codec::from_ss58check("5HKgMTc7xb3v9e7Sy6NiEFRz8Xd8k19HNdcr7RWGRhJUSm6V").unwrap(),3960467235),
				(Ss58Codec::from_ss58check("5HnEPNC3MWjxtRJhBEEMRnNyCfFxqa9HURuRgYHqo5qrnehZ").unwrap(),2280048642),
				(Ss58Codec::from_ss58check("5C5CnE71KdBJidnG2i5LXi3rXQW8NLPxidXJrVx6bMvzdK5W").unwrap(),1721808197),
				(Ss58Codec::from_ss58check("5GgfCd5qB6zG79d2a6QmHyVGN3e7ESBUVhExDN1fA5EeqjU9").unwrap(),1201510968),
				(Ss58Codec::from_ss58check("5GHXaxUSXi5R7kGv9QBEk7iU7DiL324biDrmpWzL4cC7oniQ").unwrap(),2603278456),
				(Ss58Codec::from_ss58check("5GRRTkVmXPty7Fw3ibHhwjZF45VV7C1STyHQr4LLRqLds6V2").unwrap(),878496577),
				(Ss58Codec::from_ss58check("5GjJHnzdy63gHEo9PjHF6ertW29KZZqZJm1d4BMEJr3awrRf").unwrap(),650260583),
				(Ss58Codec::from_ss58check("5EZwam2JjsgVefziRmzDfW1tykTNefoBriRFMWvBCgHffBQg").unwrap(),800711043),
				(Ss58Codec::from_ss58check("5FqZ4ajrgJR8Lzw2VacsR23qVyuRZo9yYcknAWnRxwmfHxeT").unwrap(),923514187),
				(Ss58Codec::from_ss58check("5EnpNXJwkM6S6Gk9Uj7FVVPRiFW8udBwybEFmuLbEN4P7owW").unwrap(),6809616295),
				(Ss58Codec::from_ss58check("5FFYKaPJn7tFWMACTV39R19hbmakyD7QvEZ7zLn5h71yrFBS").unwrap(),2172015609732),
				(Ss58Codec::from_ss58check("5Cf7sTRz9uJwb9xBwVqeNWbeq32zcRk4mVsnpeckvcit8PC6").unwrap(),3122327508),
				(Ss58Codec::from_ss58check("5HQP1BmE6BPNaa9PJM1osKgHmDiUHZTmNEMWH6Cktb9Ngehw").unwrap(),19668767170),
				(Ss58Codec::from_ss58check("5Hgw6EVX4w74dRJfhVnWcGwmGy7T1sjHm3r5PGNKQKpStyZ7").unwrap(),4151568714),
				(Ss58Codec::from_ss58check("5DZGJDz2sXr7SnkMuTZhqXnDXJvetvG69bu9YoxZQRerrGL3").unwrap(),510653383177),
				(Ss58Codec::from_ss58check("5FX8zE9R52hpJmMwsKsDhc7tVEXy1XbCzWZx6DLDkkfpi6ER").unwrap(),124103969),
				(Ss58Codec::from_ss58check("5ChcNztMZHcGzKyYSh1YTCQistSEMSuDdkmqT2T9b4xXW3jm").unwrap(),84593083903),
				(Ss58Codec::from_ss58check("5F4VqZ8ECCtfQRH2S4K5oTVxiD1i8vH5Y8kJVr4UZtNDpmuz").unwrap(),455022221),
				(Ss58Codec::from_ss58check("5HbRcp8CheU7iv8BW5AkfQQBeFUPk9nJAKm4jVxc8Vs29qpC").unwrap(),296130561840),
				(Ss58Codec::from_ss58check("5CiKpAEPmLTusDsxQSor7ED7P9Y598aG3cxcSFgRxCPJh53c").unwrap(),661503638),
				(Ss58Codec::from_ss58check("5FTxEuLVS4yokmcFd9x6bjByWT1oNoWnQqmt7wZH9YyWCKcq").unwrap(),15584144138),
				(Ss58Codec::from_ss58check("5ELP2SFD3DgpxwZyacELZiqA7pkUndyoutvVtsDF9y2bLE2K").unwrap(),17706631605),
				(Ss58Codec::from_ss58check("5Dki8aVM2Wvr9PUkTiJyynsncr8vDLHNL2K3m6b8DSrZSviJ").unwrap(),11607766321),
				(Ss58Codec::from_ss58check("5Gxs2tmaCTpMJq3UrRHKWWLSxujXR5CUBzegZ3egJWSgv16F").unwrap(),393467619678),
				(Ss58Codec::from_ss58check("5EZ5HyfvD7Cfm5D9sJZGNofXdV29pbqYWrDDHfUoazYZ4cYp").unwrap(),21000000000),
				(Ss58Codec::from_ss58check("5FRodKLnzFsa8kZsNbB5s2wy5Dmc66bpSc1CNwFiosj1KWbo").unwrap(),999000000000),
				(Ss58Codec::from_ss58check("5CVKoSiF7AdcxnLv6iJsDnXw6EE51xRPeoJzr4q2nZ8ywpwf").unwrap(),290630219731),
				(Ss58Codec::from_ss58check("5GbSmaoza9rzDViaLTmFS2vhjobEQdv93cekXYAJ6XPstMej").unwrap(),199624999589),
				(Ss58Codec::from_ss58check("5HRPxma1TuTY4gPdAs4bPgyTcen13P43rucRiHppV6geJrqM").unwrap(),20184746050866),
				(Ss58Codec::from_ss58check("5GKhCgLGHgYJwkBui3evAWHTgDfB7smWpiKdT81o8VrakTtX").unwrap(),500000000000),
				(Ss58Codec::from_ss58check("5Hpqi1Hcq2Mih9oE2ykjT4umiwu63KcBNQaa7a8U9HZyk5rh").unwrap(),13649874999857)

			],
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		subspace_module: Default::default()
	}
}
