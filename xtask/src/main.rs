use std::fs::File;

mod flags;

fn main() {
	let flags = flags::Localnet::from_env_or_exit();
	match flags.subcommand {
		flags::LocalnetCmd::Run(r) => {
			let path = r.path.unwrap_or_else(|| {
				tempfile::Builder::new()
					.prefix("commune-node-data")
					.tempdir()
					.expect("failed to create tempdir")
					.into_path()
			});
			match (path.exists(), path.is_dir()) {
				(true, false) => panic!("provided path must be a directory"),
				(false, false) => std::fs::create_dir(&path).unwrap(),
				_ => {},
			}

			let base_path = r
				.base_path
				.or_else(|| {
					let path = path.join("data");
					path.is_dir().then_some(path)
				})
				.unwrap_or_else(|| path.join("data"));

			let chain_path = r
				.chain_path
				.or_else(|| {
					let path = path.join("spec.json");
					path.is_file().then_some(path)
				})
				.unwrap_or_else(|| {
					let chain = path.join("spec.json");
					let chain_name = r.chain_name.expect(
						"chain name must be specified if no chain path is provided or found",
					);
					let output = foo::build_chain_spec(&chain_name).output().unwrap();
					std::fs::write(&chain, &output.stdout).unwrap();
					chain
				});

			let secrets_path = r.secrets_path.unwrap_or_else(|| path.clone());

			let key_aura = secrets_path.join("aura.sr25519.key.json");
			if !key_aura.exists() {
				let output = foo::key_generate().output().unwrap();
				std::fs::write(&key_aura, &output.stdout).unwrap();
			}

			let mnemonic = {
				let val: serde_json::Value =
					serde_json::from_reader(File::open(key_aura).unwrap()).unwrap();
				val.get("secretPhrase").unwrap().as_str().unwrap().to_string()
			};

			let key_gran = secrets_path.join("gran.ed25519.key.json");
			if !key_gran.exists() {
				let output = foo::key_inspect_cmd(&mnemonic).output().unwrap();
				std::fs::write(&key_gran, &output.stdout).unwrap();
			}

			let _key_insert_aura = foo::key_insert_cmd(&base_path, &chain_path, &mnemonic, "aura")
				.spawn()
				.unwrap()
				.wait();
			let _key_insert_gran = foo::key_insert_cmd(&base_path, &chain_path, &mnemonic, "gran")
				.spawn()
				.unwrap()
				.wait();

			let bootnodes = (!r.bootnodes.is_empty()).then_some(r.bootnodes);
			let run = foo::run_node(
				&base_path,
				&chain_path,
				r.port.unwrap_or(30333),
				r.rpc_port.unwrap_or(9944),
				r.validator,
				bootnodes,
			)
			.spawn()
			.unwrap()
			.wait();
		},
	}
}

mod foo {
	use std::{ffi::OsStr, process::Command};

	pub fn base_node_run_cmd() -> Command {
		let mut cmd = Command::new("cargo");
		cmd.args(&["run", "--release", "--package", "node-subspace", "--"]);
		cmd
	}

	pub fn build_chain_spec(chain_spec: &str) -> Command {
		let mut cmd = base_node_run_cmd();
		cmd.args(&["build-spec", "--raw"])
			.args(&["--chain", chain_spec])
			.arg("--disable-default-bootnode");
		cmd
	}

	pub fn key_generate() -> Command {
		let mut cmd = base_node_run_cmd();
		cmd.args(&["key", "generate"])
			.args(&["--scheme", "sr25519"])
			.args(&["--output-type", "json"]);
		cmd
	}

	pub fn key_insert_cmd(
		base_path: &dyn AsRef<OsStr>,
		chain_spec: &dyn AsRef<OsStr>,
		mnemonic: &str,
		key_type: &str,
	) -> Command {
		let mut cmd = base_node_run_cmd();
		cmd.args(&["key", "insert"])
			.args(&[&"--base-path" as &(dyn AsRef<_>), base_path])
			.args(&[&"--chain" as &(dyn AsRef<_>), chain_spec])
			.args(&[
				"--scheme",
				match key_type {
					"aura" => "sr25519",
					"gran" => "sr25519",
					_ => panic!(),
				},
			])
			.args(&["--suri", &mnemonic])
			.args(&["--key-type", key_type]);
		cmd
	}

	pub fn key_inspect_cmd(mnemonic: &str) -> Command {
		let mut cmd = base_node_run_cmd();
		cmd.args(&["key", "inspect"])
			.args(&["--scheme", "sr25519"])
			.args(&["--output-type", "json"])
			.arg(mnemonic);
		cmd
	}

	pub fn run_node(
		base_path: &dyn AsRef<OsStr>,
		chain_spec: &dyn AsRef<OsStr>,
		port: u16,
		rpc_port: u16,
		is_validator: bool,
		bootnodes: Option<Vec<String>>,
	) -> Command {
		let mut cmd = base_node_run_cmd();

		cmd.args(&[&"--base-path" as &(dyn AsRef<_>), base_path])
			.args(&[&"--chain" as &(dyn AsRef<_>), chain_spec])
			.args(&["--unsafe-rpc-external", "--rpc-cors", "all"])
			.args(&["--port", &port.to_string(), "--rpc-port", &rpc_port.to_string()]);

		if is_validator {
			cmd.arg("--validator");
		}

		if let Some(bootnodes) = bootnodes {
			cmd.arg("--bootnodes").args(&bootnodes);
		}

		cmd
	}
}
