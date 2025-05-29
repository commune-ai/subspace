# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "substrate-interface",
#     "rich",
#     "scalecodec",
# ]
# ///

from substrateinterface.keypair import ss58_decode
from substrateinterface import SubstrateInterface
import binascii
from pathlib import Path
from rich.console import Console
from rich.table import Table
from typing import List, Dict, Optional, Tuple, Union
import re

console = Console()

def decode_ss58(target_key: str) -> bytes:
    return ss58_decode(target_key)

def hex_to_bytes(hex_str: str) -> bytes:
    return binascii.unhexlify(hex_str)

def get_migrations_file_path() -> Path:
    """Get the path to the migrations.rs file."""
    return Path.cwd() / "pallets" / "governance" / "src" / "migrations.rs"

def extract_senate_keys_from_comments() -> List[str]:
    """Extract the senate keys from the commented section in the migrations.rs file."""
    target_path = get_migrations_file_path()
    target_lines = target_path.read_text().splitlines()
    
    senate_keys = []
    in_senate_keys_section = False
    
    for line in target_lines:
        line = line.strip()
        
        # Start of senate keys section
        if "// senate_keys = [" in line:
            in_senate_keys_section = True
            continue
            
        # End of senate keys section
        if in_senate_keys_section and "//" in line and "]" in line:
            in_senate_keys_section = False
            continue
            
        # Extract key
        if in_senate_keys_section and "//" in line and '"' in line:
            # Remove comment prefix and extract the key
            clean_line = line.strip().replace('//', '').strip()
            # Extract the key from the line (removing quotes and commas)
            if '"' in clean_line:
                # Extract the content between quotes
                key = clean_line.split('"')[1].strip()
                if key and len(key) > 10:  # Basic validation to ensure it's a key
                    senate_keys.append(key)
    
    return senate_keys

def extract_senate_key_bytes() -> List[List[int]]:
    """Extract the senate key bytes from the migrations.rs file."""
    target_path = get_migrations_file_path()
    target_lines = target_path.read_text().splitlines()
    
    senate_bytes = []
    current_bytes = []
    in_senate_keys = False
    in_key_array = False
    
    for line in target_lines:
        line = line.strip()
        
        # Start of senate keys section
        if "let senate_keys: [[u8; 32];" in line:
            in_senate_keys = True
            continue
        
        # End of senate keys section
        if in_senate_keys and line == "];":
            in_senate_keys = False
            # Add the last key if we have one
            if current_bytes and len(current_bytes) == 32:
                senate_bytes.append(current_bytes)
            continue
        
        # Start of a new key array
        if in_senate_keys and line == "[":
            in_key_array = True
            current_bytes = []
            continue
        
        # End of a key array
        if in_senate_keys and in_key_array and line == "],":
            in_key_array = False
            if current_bytes and len(current_bytes) == 32:
                senate_bytes.append(current_bytes)
            continue
        
        # Process byte values within a key array
        if in_senate_keys and in_key_array and "0x" in line:
            # Extract all hex values from the line
            hex_values = [part.strip() for part in line.split(',')]
            for hex_val in hex_values:
                if hex_val.startswith('0x'):
                    try:
                        byte_val = int(hex_val, 16)
                        current_bytes.append(byte_val)
                    except ValueError:
                        pass
    
    return senate_bytes

def validate_senate_keys():
    """Validate the senate keys from comments against the byte arrays in the code."""
    # Extract senate keys from comments and byte arrays from code
    comment_keys = extract_senate_keys_from_comments()
    byte_arrays = extract_senate_key_bytes()
    
    if not comment_keys:
        console.print("[red]Error: Could not find senate keys in comments[/red]")
        return
    
    if not byte_arrays:
        console.print("[red]Error: Could not find senate key byte arrays in code[/red]")
        return
    
    console.print(f"\n[bold]Found {len(comment_keys)} senate keys in comments and {len(byte_arrays)} byte arrays in code[/bold]\n")
    
    # Create a table for results
    table = Table(title="Senate Keys Validation")
    table.add_column("#", justify="right", style="cyan")
    table.add_column("Comment Key (SS58)", style="green")
    table.add_column("Byte Array Match", style="yellow")
    table.add_column("Match Index", style="blue")
    
    # Convert each comment key to bytes and check if it matches any byte array
    for i, key in enumerate(comment_keys):
        try:
            # Convert SS58 key to bytes
            key_bytes = decode_ss58(key)
            key_bytes_array = list(hex_to_bytes(key_bytes))
            
            # Check if this key matches any byte array
            match_found = False
            match_index = -1
            for j, byte_array in enumerate(byte_arrays):
                if key_bytes_array == byte_array:
                    match_found = True
                    match_index = j
                    break
            
            match_status = "[green]✓ MATCH[/green]" if match_found else "[red]✗ NO MATCH[/red]"
            match_idx_str = f"[blue]Index {match_index}[/blue]" if match_found else ""
            table.add_row(f"{i+1}", key, match_status, match_idx_str)
            
        except Exception as e:
            table.add_row(f"{i+1}", key, f"[red]Error: {str(e)}[/red]", "")
    
    console.print(table)

def main():
    """Main function to validate senate keys."""
    validate_senate_keys()

    
    

def print_custom_help():
    """Print a custom, rich-formatted help menu"""
    from rich.panel import Panel
    from rich.text import Text
    
    title = Text("Senate Keys Validator", style="bold cyan")
    subtitle = Text("A tool to validate senate keys in migration code", style="italic yellow")
    
    usage = Text("\nUsage:", style="bold green")
    usage_cmd = Text("  uv run scripts/python/validate_replacement_key.py [OPTIONS]\n", style="blue")
    
    options_title = Text("Options:", style="bold green")
    options = [
        ("-h, --help", "Show this help message and exit")
    ]
    
    options_text = ""
    for opt, desc in options:
        options_text += f"  [bold blue]{opt:<25}[/bold blue] [white]{desc}[/white]\n"
    
    examples_title = Text("\nExamples:", style="bold green")
    examples = [
        ("Validate senate keys:", "uv run scripts/python/validate_replacement_key.py")
    ]
    
    examples_text = ""
    for ex_desc, ex_cmd in examples:
        examples_text += f"  [bold yellow]{ex_desc:<30}[/bold yellow] [blue]{ex_cmd}[/blue]\n"
    
    content = f"{title}\n{subtitle}\n{usage}{usage_cmd}{options_title}\n{options_text}{examples_title}\n{examples_text}"
    panel = Panel(content, border_style="green", title="[bold white]Senate Keys Validator[/bold white]", subtitle="[italic]v1.0.0[/italic]")
    
    console.print(panel)

if __name__ == "__main__":
    import sys
    
    # Check if help flag is present
    if "-h" in sys.argv or "--help" in sys.argv:
        print_custom_help()
        sys.exit(0)
    
    # Run the validation
    main()