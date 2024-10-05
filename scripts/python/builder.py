"""
Builds a genesis snapshot of current mainnet state
"""

import json
import argparse
import os
import codecs
from typing import Any
import logging

from substrateinterface.base import SubstrateInterface

QUERY_URL = "wss://commune-api-node-1.communeai.net"
STANDARD_MODULE = "SubspaceModule"

EXISTENTIAL_DEPOSIT = 500
MAX_NAME_LENGTH = 32

SUDO = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"

logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)

def query_map_values(
    client: SubstrateInterface, module: str, storage_function: str, params: list = []
) -> dict:
    logging.info(f"Querying {module}.{storage_function} with params {params}")
    result = client.query_map(
        module=module, storage_function=storage_function, params=params
    )
    return {k.value: v.value for k, v in result}


def get_subnets(client: SubstrateInterface) -> dict[str, Any]:
    logging.info("Fetching subnet information")
    N = query_map_values(
        client=client, storage_function="N", module=STANDARD_MODULE)
    subnets: dict[Any, Any] = {"subnets": []}

    subnet_names = query_map_values(
        client=client, storage_function="SubnetNames", module=STANDARD_MODULE
    )
    founder_addys = query_map_values(
        client=client, storage_function="Founder", module=STANDARD_MODULE
    )

    # due to potential name cutting we might repeat module names, which is not allowed.
    encountered_names = set()
    for netuid in N:
        logging.info(f"Processing subnet with netuid: {netuid}")
        subnet = {
            "name": subnet_names[netuid],
            "founder": founder_addys[netuid],
            "modules": [],
        }

        keys = query_map_values(
            client=client,
            storage_function="Keys",
            module=STANDARD_MODULE,
            params=[netuid],
        )
        names = query_map_values(
            client=client,
            storage_function="Name",
            module=STANDARD_MODULE,
            params=[netuid],
        )
        addresses = query_map_values(
            client=client,
            storage_function="Address",
            module=STANDARD_MODULE,
            params=[netuid],
        )
        stake_froms = query_map_values(
            client=client,
            storage_function="StakeFrom",
            module=STANDARD_MODULE,
            params=[netuid],
        )

        for index, key in keys.items():
            name = names[index][:MAX_NAME_LENGTH]
            if name in encountered_names:
                continue
            encountered_names.add(name)
            module = {
                "key": key,
                "name": name,
                "address": addresses[index][:MAX_NAME_LENGTH],
                "stake_from": {stake[0]: stake[1] for stake in stake_froms.get(key, 0)},
            }
            subnet["modules"].append(module)

        subnets["subnets"].append(subnet)
    return subnets


def get_balances(client: SubstrateInterface) -> dict[str, dict[str, int]]:
    logging.info("Fetching account balances")
    balances = query_map_values(
        client=client, module="System", storage_function="Account", params=[]
    )
    result = {
        k: v["data"]["free"]
        for k, v in balances.items()
        if v["data"]["free"] > EXISTENTIAL_DEPOSIT
    }
    return {"balances": result}


def get_code(client: SubstrateInterface) -> dict[str, str]:
    logging.info("Fetching code")
    return {"code": str(client.query("Substrate", "Code"))}


def get_sudo(key: str) -> dict[str, str]:
    return {"sudo": key}


def build_snap(
    code: dict[str, str], balances: dict[str, dict[str, int]], subnets: dict[str, Any]
) -> dict[str, Any]:
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
        description="Generate a snapshot of balances and subnets."
    )
    parser.add_argument(
        "-o",
        "--output",
        default="local.json",
        help="Output file name (default: local.json)",
    )
    parser.add_argument(
        "-d",
        "--directory",
        default=".",
        help="Output directory (default: current directory)",
    )
    parser.add_argument(
        "-c",
        "--code",
        default=False,
        help="If the generated spec file should contain the mainnet runtime code (default: false)",
    )
    args = parser.parse_args()

    output_path = os.path.join(args.directory, args.output)

    logging.info("Starting snapshot generation, might take up to 10 minutes.")
    client = SubstrateInterface(QUERY_URL)
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
