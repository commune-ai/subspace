use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
};

use frame_remote_externalities::OnlineConfig;
use parity_scale_codec::Encode;
use sc_client_api::StateBackend;

use sc_service::ChainSpec;
use serde_json::Value;
use sp_runtime::{
    generic::{Block, Header},
    traits::BlakeTwo256,
    OpaqueExtrinsic,
};

use crate::flags::MainnetSpec;

pub fn mainnet_spec(flags: MainnetSpec) {
    let mut key = key_name(b"System", b"Account");
    let foo = hex_literal::hex!("a2b6e3b1089b8e233fe38bb9d8028a4b728c4b984f5b07507f2b198192c6e760");
    key.push_str(&hex::encode(
        [
            sp_crypto_hashing::blake2_128(&foo).as_slice(),
            foo.as_slice(),
        ]
        .concat(),
    ));
    dbg!(&key);

    let spec = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(create_mainnet_spec());

    let spec = sc_service::chain_ops::build_spec(spec.as_ref() as &dyn ChainSpec, true).unwrap();
    let mut js: Value = serde_json::from_str(&spec).unwrap();

    let genesis = &mut js["genesis"]["raw"]["top"];

    aura(genesis);
    sudo(genesis);
    // balance(genesis);

    let js = serde_json::to_string_pretty(&js).unwrap();
    std::fs::write(flags.output, js).unwrap();
}

fn sudo(genesis: &mut Value) {
    let key = key_name(b"Sudo", b"Key");
    genesis[&key] = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".into();
}

const KEYS: &[[u8; 32]] = &[
    // Alice
    hex_literal::hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"),
    // Bob
    hex_literal::hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"),
];

fn aura(genesis: &mut Value) {
    let key = key_name(b"Aura", b"Authorities");

    let mut buf = Cursor::new(vec![0; KEYS.size_hint()]);
    KEYS.encode_to(&mut buf);

    let written = &buf.get_ref()[..buf.position() as usize];
    genesis[&key] = Value::String(format!("0x{}", hex::encode(written)));
}

// fn balance(genesis: &mut Value) {
//     let mut key = key_name(b"System", b"Account");
//     key.push_str(&hex::encode(sp_crypto_hashing::blake2_128(&KEYS[0])));
//     dbg!(&key);
// }

fn key_name(pallet: &[u8], key: &[u8]) -> String {
    let mut res = [0; 32];
    res[0..16].copy_from_slice(&sp_crypto_hashing::twox_128(pallet));
    res[16..32].copy_from_slice(&sp_crypto_hashing::twox_128(key));
    format!("0x{}", hex::encode(res))
}

type OpaqueBlock = Block<Header<u32, BlakeTwo256>, OpaqueExtrinsic>;

// #[derive(serde::Deserialize, serde::Serialize)]
// struct DummyStorage;
// impl BuildStorage for DummyStorage {
//     fn assimilate_storage(&self, _: &mut sp_core::storage::Storage) -> Result<(), String> {
//         Ok(())
//     }
// }

async fn create_mainnet_spec() -> Box<dyn ChainSpec> {
    let mut chain_spec = sc_service::GenericChainSpec::<sc_service::NoExtension>::from_json_bytes(include_bytes!(
        "../../node/chain-specs/main.json"
    ))
    .unwrap();

    let api = "wss://api.communeai.net".to_string();

    let mode = frame_remote_externalities::Builder::<OpaqueBlock>::new().mode(
        frame_remote_externalities::Mode::Online(OnlineConfig {
            at: None,
            state_snapshot: None,
            pallets: vec![],
            transport: frame_remote_externalities::Transport::Uri(api),
            child_trie: true,
            hashed_prefixes: vec![],
            hashed_keys: vec![],
        }),
    );

    let ext = mode.build().await.unwrap();

    let mut top = BTreeMap::new();
    let children_default = HashMap::new();

    let mut last_key = vec![];

    while let Some(key) = ext.backend.next_storage_key(&last_key).unwrap() {
        let val = ext.backend.storage(&key).unwrap();
        top.insert(key.clone(), val.unwrap());
        last_key = key;
    }

    chain_spec.set_storage(sp_runtime::Storage {
        top,
        children_default,
    });

    Box::new(chain_spec)
}
