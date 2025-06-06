# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "substrate-interface>=1.4.2",
#     "rich",
#     "scalecodec>=1.2.0"
# ]
# ///
"""
Derive a multi-signature address from the provided senate keys.

This script creates a multi-signature address from the specified senate keys
with a configurable threshold.

Usage:
    python derive_senate_multisig.py [--threshold THRESHOLD]

Example:
    python derive_senate_multisig.py --threshold 4
"""

import argparse
import binascii
from substrateinterface import SubstrateInterface, Keypair
from rich.console import Console
from rich.table import Table
from scalecodec.utils.ss58 import ss58_decode, ss58_encode

console = Console()

# Senate keys provided
SENATE_KEYS = [
    "5H47pSknyzk4NM5LyE6Z3YiRKb3JjhYbea2pAUdocb95HrQL", 
    "5EkM3FpJWZQ6pL7khr16aNWwv5HFMpQ4BWUj7bWehWkb7rXa", 
    "5CMNEDouxNdMUEM6NE9HRYaJwCSBarwr765jeLdHvWEE15NH", 
    "5FZsiAJS5WMzsrisfLWosyzaCEQ141rncjv55VFLHcUER99c", 
    "5DyPNNRLbrLWgPZPVES45LfEgFKyfmPbrtJkFLiSbmWLumYj", 
    "5DPSqGAAy5ze1JGuSJb68fFPKbDmXhfMqoNSHLFnJgUNTPaU", 
    "5HmjuwYGRXhxxbFz6EJBXpAyPKwRsQxFKdZQeLdTtg5UEudA"
]

def derive_senate_multisig(threshold=4, node_url="wss://api.communeai.net", ss58_format=42):
    """
    Derive a multi-signature address from the senate keys.
    
    Args:
        threshold: Number of signatures required (default: 4)
        node_url: URL of the Substrate node (default: wss://api.communeai.net)
        ss58_format: SS58 format to use (default: 42 for Subspace)
        
    Returns:
        Dictionary containing the multi-sig address, signatories, and threshold
    """
    try:
        # Connect to the node
        substrate = SubstrateInterface(url=node_url, ss58_format=ss58_format)
        
        # Validate the threshold
        if threshold < 1 or threshold > len(SENATE_KEYS):
            raise ValueError(f"Threshold must be between 1 and {len(SENATE_KEYS)}")
            
        # Sort the public keys (required for deterministic multisig generation)
        # First convert SS58 addresses to public keys
        public_keys = [ss58_decode(address) for address in SENATE_KEYS]
        # Sort the public keys
        sorted_public_keys = sorted(public_keys)
        # Convert back to SS58 addresses
        sorted_addresses = [ss58_encode(pk, ss58_format=ss58_format) for pk in sorted_public_keys]
        
        # Generate the multisig address
        multi_account = substrate.generate_multisig_account(
            signatories=sorted_addresses,
            threshold=threshold
        )
        
        # Extract the SS58 address from the multi_account object
        if hasattr(multi_account, 'ss58_address'):
            multi_address = multi_account.ss58_address
        elif isinstance(multi_account, dict) and 'ss58_address' in multi_account:
            multi_address = multi_account['ss58_address']
        elif isinstance(multi_account, str):
            multi_address = multi_account
        else:
            # If we can't get the address directly, try to generate it manually
            try:
                multi_address = str(multi_account)
            except:
                raise ValueError("Could not extract multisig address from result")
        
        # Create a table for better visualization
        table = Table(title=f"Senate Multi-Signature ({threshold} of {len(SENATE_KEYS)})")
        table.add_column("Component", style="cyan")
        table.add_column("Value", style="green")
        
        table.add_row("Multi-signature Address", multi_address)
        table.add_row("Threshold", str(threshold))
        
        # Add signatories to the table
        for i, key in enumerate(SENATE_KEYS, 1):
            table.add_row(f"Signatory {i}", key)
        
        console.print(table)
        
        return {
            "address": multi_address,
            "signatories": SENATE_KEYS,
            "threshold": threshold
        }
    except Exception as e:
        console.print(f"Error: {e}", style="bold red")
        return None

def main():
    parser = argparse.ArgumentParser(description="Derive a multi-signature address from senate keys")
    parser.add_argument("--threshold", type=int, default=4, help="Number of signatures required (default: 4)")
    parser.add_argument("--node-url", type=str, default="wss://api.communeai.net", help="URL of the Substrate node")
    parser.add_argument("--ss58-format", type=int, default=42, help="SS58 format to use (default: 42 for Subspace)")
    
    args = parser.parse_args()
    
    derive_senate_multisig(threshold=args.threshold, node_url=args.node_url, ss58_format=args.ss58_format)

if __name__ == "__main__":
    main()
