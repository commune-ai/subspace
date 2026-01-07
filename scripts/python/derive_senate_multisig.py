# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "substrate-interface>=1.4.2",
#     "rich",
#     "scalecodec>=1.2.0",
#     "base58",
# ]
# ///
"""
Derive a multi-signature address from the provided senate keys.

This script creates a multi-signature address from the specified senate keys
with a configurable threshold. Supports both Substrate SS58 and Solana base58 key formats.

Usage:
    python derive_senate_multisig.py [--threshold THRESHOLD]

Example:
    python derive_senate_multisig.py --threshold 4
"""

import argparse
import binascii
import base58
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

def is_solana_key(key: str) -> bool:
    """
    Check if a key is in Solana format (plain base58, ~43-44 chars).
    Solana keys are typically 32 bytes encoded in base58 without SS58 format.
    SS58 keys typically start with '5' and are longer.
    """
    try:
        # Try to decode as plain base58
        decoded = base58.b58decode(key)
        # Solana keys are exactly 32 bytes
        return len(decoded) == 32 and not key.startswith('5')
    except Exception:
        return False

def decode_key_to_hex(key: str) -> str:
    """
    Decode a key in either Solana (base58) or Substrate (SS58) format.
    Returns the hex-encoded 32-byte public key.
    """
    if is_solana_key(key):
        # Solana key: plain base58 encoding
        decoded = base58.b58decode(key)
        return decoded.hex()
    else:
        # Substrate key: SS58 encoding
        return ss58_decode(key)

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
        # First convert addresses to public keys (hex format)
        public_keys = [decode_key_to_hex(address) for address in SENATE_KEYS]
        # Sort the public keys
        sorted_public_keys = sorted(public_keys)
        # Convert back to SS58 addresses for Substrate multisig
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
