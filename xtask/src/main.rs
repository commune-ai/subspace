use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr},
};

mod flags;

fn main() {
    let flags = flags::Localnet::from_env_or_exit();

    match flags.subcommand {
        flags::LocalnetCmd::Run(r) => localnet_run(r),
    }
}

fn localnet_run(mut r: flags::Run) {
    let (mut node, mut account) = match (r.alice, r.bob) {
        (true, false) => {
            if r.bootnodes.is_empty() {
                r.bootnodes.push(BOB_NODE.bootnode_uri(Ipv4Addr::LOCALHOST.into()));
            }
            (ALICE_NODE.clone(), ALICE_ACCOUNT.clone())
        }
        (false, true) => {
            if r.bootnodes.is_empty() {
                r.bootnodes.push(ALICE_NODE.bootnode_uri(Ipv4Addr::LOCALHOST.into()));
            }
            (BOB_NODE.clone(), BOB_ACCOUNT.clone())
        }
        (false, false) => (Node::default(), Account::default()),
        _ => panic!("select only one of: --alice, --bob"),
    };

    if let Some(node_name) = r.node_name {
        node.name = Some(node_name.into());
    }
    if let Some(node_validator) = r.node_validator {
        node.validator = node_validator;
    }
    if let Some(node_key) = r.node_key {
        let node_id = ops::key_inspect_node_cmd(&node_key);
        node.key = Some(node_key.into());
        node.id = Some(node_id.into());
    }
    if let Some(account_suri) = r.account_suri {
        account.suri = account_suri.into();
    }

    let path = r.path.unwrap_or_else(|| {
        tempfile::Builder::new()
            .prefix("commune-node-data")
            .suffix(node.name.as_ref().unwrap_or(&Cow::Borrowed("")).as_ref())
            .tempdir()
            .expect("failed to create tempdir")
            .into_path()
    });

    match (path.exists(), path.is_dir()) {
        (true, false) => panic!("provided path must be a directory"),
        (false, false) => std::fs::create_dir(&path).unwrap(),
        _ => {}
    }

    let chain_path = r
        .chain_spec
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("spec.json"));

    ops::key_insert_cmd(&path, &chain_path, &account.suri, "aura");
    ops::key_insert_cmd(&path, &chain_path, &account.suri, "gran");

    let _run = ops::run_node(&path, &chain_path, &node, &r.bootnodes).spawn().unwrap().wait();
}

#[derive(Clone)]
struct Node<'a> {
    name: Option<Cow<'a, str>>,
    id: Option<Cow<'a, str>>,
    key: Option<Cow<'a, str>>,
    tcp_port: u16,
    rpc_port: u16,
    validator: bool,
}

impl<'a> Node<'a> {
    fn bootnode_uri(&self, addr: IpAddr) -> String {
        format!(
            "/{}/{addr}/tcp/{}/p2p/{}",
            match addr {
                IpAddr::V4(_) => "ip4",
                IpAddr::V6(_) => "ip6",
            },
            self.tcp_port,
            self.id.as_ref().unwrap()
        )
    }
}

impl Default for Node<'_> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            id: Default::default(),
            key: Default::default(),
            tcp_port: 30333,
            rpc_port: 9944,
            validator: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Default)]
struct Account<'a> {
    suri: Cow<'a, str>,
    aura_address: Cow<'a, str>,
    grandpa_address: Cow<'a, str>,
}

static ALICE_NODE: Node<'static> = Node {
    name: Some(Cow::Borrowed("Alice")),
    id: Some(Cow::Borrowed(
        "12D3KooWBorpca6RKiebVjeFJA5o9iVWnZpg98yQbYqRC6f8CnLw",
    )),
    key: Some(Cow::Borrowed(
        "2756181a3b9bca683a35b51a0a5d75ee536738680bcb9066c68be1db305a1ac5",
    )),
    tcp_port: 30341,
    rpc_port: 9951,
    validator: true,
};
static ALICE_ACCOUNT: Account<'static> = Account {
    suri: Cow::Borrowed(
        "bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice",
    ),
    aura_address: Cow::Borrowed("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"),
    grandpa_address: Cow::Borrowed("5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"),
};

static BOB_NODE: Node<'static> = Node {
    name: Some(Cow::Borrowed("Bob")),
    id: Some(Cow::Borrowed(
        "12D3KooWQh3CeSp2rpUVvPb6jqvmHVCUieoZmKbkUhZ8rPR77vmA",
    )),
    key: Some(Cow::Borrowed(
        "e83fa0787cb280d95c666ead866a2a4bc1ee1e36faa1ed06623595eb3f474681",
    )),
    tcp_port: 30342,
    rpc_port: 9952,
    validator: true,
};
static BOB_ACCOUNT: Account<'static> = Account {
    suri: Cow::Borrowed(
        "bottom drive obey lake curtain smoke basket hold race lonely fit walk//Bob",
    ),
    aura_address: Cow::Borrowed("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"),
    grandpa_address: Cow::Borrowed("5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E"),
};

#[allow(dead_code)]
mod ops {
    use super::*;
    use std::{
        ffi::OsStr,
        io::Write,
        process::{Command, Stdio},
    };

    pub fn base_node_run_cmd() -> Command {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--release", "--package", "node-subspace", "--"]);
        cmd
    }

    pub fn build_chain_spec(chain_spec: &str) -> Command {
        let mut cmd = base_node_run_cmd();
        cmd.args(["build-spec", "--raw"])
            .args(["--chain", chain_spec])
            .arg("--disable-default-bootnode");
        cmd
    }

    pub fn key_generate() -> Command {
        let mut cmd = base_node_run_cmd();
        cmd.args(["key", "generate"])
            .args(["--scheme", "sr25519"])
            .args(["--output-type", "json"]);
        cmd
    }

    pub fn key_insert_cmd(
        base_path: &dyn AsRef<OsStr>,
        chain_spec: &dyn AsRef<OsStr>,
        suri: &str,
        key_type: &str,
    ) {
        let mut cmd = base_node_run_cmd();
        let scheme = match key_type {
            "aura" => "sr25519",
            "gran" => "ed25519",
            _ => panic!(),
        };
        cmd.args(["key", "insert"])
            .args([&"--base-path" as &(dyn AsRef<_>), base_path])
            .args([&"--chain" as &(dyn AsRef<_>), chain_spec])
            .args(["--scheme", scheme])
            .args(["--suri", &suri])
            .args(["--key-type", key_type])
            .spawn()
            .unwrap()
            .wait()
            .expect("failed to run key insert");
    }

    pub fn key_inspect_cmd(suri: &str) -> Command {
        let mut cmd = base_node_run_cmd();
        cmd.args(["key", "inspect"])
            .args(["--scheme", "ed25519"])
            .args(["--output-type", "json"])
            .arg(suri);
        cmd
    }

    pub fn key_inspect_node_cmd(key: &str) -> String {
        let mut child = base_node_run_cmd()
            .args(["key", "inspect-node-key"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to inspect node key");
        child
            .stdin
            .as_mut()
            .expect("missing stdin")
            .write_all(key.as_bytes())
            .expect("failed to write node key");
        let output = child.wait_with_output().expect("inspect-node-key failed");
        String::from_utf8(output.stdout).expect("invalid node id")
    }

    pub fn run_node(
        base_path: &dyn AsRef<OsStr>,
        chain_spec: &dyn AsRef<OsStr>,
        node: &Node<'_>,
        bootnodes: &[String],
    ) -> Command {
        let mut cmd = base_node_run_cmd();

        cmd.args([&"--base-path" as &(dyn AsRef<_>), base_path])
            .args([&"--chain" as &(dyn AsRef<_>), chain_spec])
            .args(["--unsafe-rpc-external", "--rpc-cors", "all"])
            .args(["--port", &node.tcp_port.to_string()])
            .args(["--rpc-port", &node.rpc_port.to_string()])
            .arg("--bootnodes")
            .args(bootnodes)
            .args(["--force-authoring"]);

        if node.validator {
            cmd.arg("--validator");
        }

        if let Some(name) = &node.name {
            cmd.args(["--name", &name]);
        }

        if let Some(node_key) = &node.key {
            cmd.args(["--node-key", &node_key]);
        }

        cmd
    }
}
