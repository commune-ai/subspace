"""
Builds a genesis snapshot of current mainnet state
"""
import json
import argparse
import os
import codecs
from typing import Any
import logging

from communex.client import CommuneClient

QUERY_URL = "wss://api.communeai.net"
STANDARD_MODULE = "SubspaceModule"

EXISTENTIAL_DEPOSIT = 500
MAX_NAME_LENGTH = 32

SUDO = "5Dy6aBqv2MQEVpSAKqB147uQUZrAqK18JjFWs2jnzSXHn6Lh"

logging.basicConfig(level=logging.INFO,
                    format='%(asctime)s - %(levelname)s - %(message)s')


def get_subnets(client: CommuneClient) -> dict[str, Any]:
    logging.info("Fetching subnet information")
    subnets: dict[Any, Any] = {
        "subnets": []
    }

    netuids = client.query_map("N", extract_value=False)["N"]
    founder_addys = client.query_map_founder()
    subnet_names = client.query_map_subnet_names()
    stake_froms = client.query_map_stakefrom()

    encountered_names = set()
    for netuid in netuids:
        logging.info(f"Processing subnet with netuid: {netuid}")
        subnet = {
            "name": subnet_names[netuid],
            "founder": founder_addys[netuid],
            "modules": []
        }
        name = subnet_names[netuid]

        keys = client.query_map_key(netuid=netuid)
        names = client.query_map_name(netuid=netuid)
        addresses = client.query_map_address(netuid=netuid)

        for index, key in keys.items():
            name = names[index][:MAX_NAME_LENGTH]
            if name in encountered_names:
                continue
            encountered_names.add(name)

            # Convert the list of tuples to a dictionary
            stake_from_list = stake_froms.get(key, [])
            stake_from_dict = {addr: amount for addr, amount in stake_from_list}

            module = {
                "key": key,
                "name": name,
                "address": addresses[index][:MAX_NAME_LENGTH],
                "stake_from": stake_from_dict
            }
            subnet["modules"].append(module)

        subnets["subnets"].append(subnet)
    return subnets

def get_balances(client: CommuneClient) -> dict[str, dict[str, int]]:
    logging.info("Fetching account balances")
    balances = client.query_map_balances()
    result = {k: v["data"]["free"] for k, v in balances.items(  # type: ignore
    ) if v["data"]["free"] > EXISTENTIAL_DEPOSIT}  # type: ignore
    return {"balances": result}  # type: ignore

def get_code(client: CommuneClient) -> dict[str, str]:
    logging.info("Fetching code")
    return {"code": str(client.query(module="Substrate", name="Code"))}

def get_sudo(key: str) -> dict[str, str]:
    return {"sudo": key}

def build_snap(code: dict[str, str], balances: dict[str, dict[str, int]], subnets: dict[str, Any]) -> dict[str, Any]:
    """
    Returns:
    snapshot spec with keys, in the following order:
    - sudo: str
    - balances: dict[str, int]
    - subnets: dict[str, Any]
    """
    spec: dict[str, Any] = {}
    spec.update(code)
    spec.update(get_sudo(SUDO))
    spec.update(balances)
    spec.update(subnets)
    return spec

def main():
    parser = argparse.ArgumentParser(
        description="Generate a snapshot of balances and subnets.")
    parser.add_argument("-o", "--output", default="local.json",
                        help="Output file name (default: local.json)")
    parser.add_argument("-d", "--directory", default=".",
                        help="Output directory (default: current directory)")
    parser.add_argument("-c", "--code", default=False,
                        help="If the generated spec file should contain the mainnet runtime code (default: false)")
    args = parser.parse_args()

    output_path = os.path.join(args.directory, args.output)

    logging.info("Starting snapshot generation, might take up to 10 minutes.")
    client = CommuneClient(QUERY_URL)
    logging.info(f"Connected to {QUERY_URL}")

    if args.code:
        code = get_code(client)
    else:
        code = {}
    balances = get_balances(client)
    subnets = get_subnets(client)

    logging.info("Building snapshot")
    spec = build_snap(code, balances, subnets)

    logging.info(f"Writing snapshot to {output_path}")
    os.makedirs(args.directory, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(spec, f, indent=4)

    logging.info("Snapshot generation complete")

if __name__ == "__main__":
    main()
