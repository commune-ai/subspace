# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "substrate-interface",
#     "rich",
# ]
# ///

from substrateinterface.keypair import ss58_decode
import binascii
from pathlib import Path
from rich.console import Console
from rich.table import Table

console = Console()

def decode_ss58(target_key: str) -> bytes:
    return ss58_decode(target_key)

def hex_to_bytes(hex_str: str) -> bytes:
    return binascii.unhexlify(hex_str)

def bytes_to_hex(target_bytes: bytes) -> str:
    return binascii.hexlify(target_bytes).decode()

def format_byte_array(target_bytes: bytes) -> str:
    return ", ".join([f"0x{b:02x}" for b in target_bytes])

def get_migrations_file_path() -> Path:
    """Get the path to the migrations.rs file."""
    return Path.cwd() / "pallets" / "governance" / "src" / "migrations.rs"

def extract_migration_bytes_array() -> str:
    """Extract the byte array from the migrations.rs file."""
    target_path = get_migrations_file_path()
    target_lines = target_path.read_text().splitlines()
    
    # Find the line containing the public key bytes declaration
    byte_array_start_idx = -1
    for i, line in enumerate(target_lines):
        if "let public_key_bytes: [u8; 32] = [" in line:
            byte_array_start_idx = i
            break
    
    if byte_array_start_idx == -1:
        console.print("[red]Error: Could not find public key bytes declaration in migrations.rs[/red]")
        return ""
    
    # Extract the two lines containing the byte array
    indent = " " * 12  # Expected indentation
    first_line = target_lines[byte_array_start_idx + 1].strip().removeprefix(indent)
    second_line = target_lines[byte_array_start_idx + 2].strip().removeprefix(indent)
    
    # Combine the lines
    formatted_array_string = first_line + " " + second_line
    return formatted_array_string

def write_bytes_to_migration_file(bytes_str: str) -> bool:
    """Write the byte array to the migrations.rs file."""
    target_path = get_migrations_file_path()
    
    # Read the file
    lines = target_path.read_text().splitlines()
    
    # Find the line containing the public key bytes declaration
    byte_array_start_idx = -1
    for i, line in enumerate(lines):
        if "let public_key_bytes: [u8; 32] = [" in line:
            byte_array_start_idx = i
            break
    
    if byte_array_start_idx == -1:
        console.print("[red]Error: Could not find public key bytes declaration in migrations.rs[/red]")
        return False
    
    # Split the bytes string into two lines (16 bytes per line)
    bytes_list = [b.strip() for b in bytes_str.split(',')]
    first_line = ', '.join(bytes_list[:16]) + ','
    second_line = ', '.join(bytes_list[16:])
    
    # Add indentation
    indent = " " * 12
    first_line = indent + first_line
    second_line = indent + second_line
    
    # Replace the lines
    lines[byte_array_start_idx + 1] = first_line
    lines[byte_array_start_idx + 2] = second_line + ","
    
    # Write back to the file
    target_path.write_text('\n'.join(lines))
    return True

def convert_ss58_to_bytes_array(target_key: str) -> list[str]:
    decoded = decode_ss58(target_key)
    bytes = hex_to_bytes(decoded)
    bytes_array = format_byte_array(bytes)
    return bytes_array


def chunk_bytes(byte_array_str: str, chunk_size: int = 4) -> list[str]:
    """Split a byte array string into chunks for better readability."""
    # Remove any whitespace and split by commas
    bytes_list = [b.strip() for b in byte_array_str.split(',')]
    
    # Group into chunks
    return [', '.join(bytes_list[i:i+chunk_size]) for i in range(0, len(bytes_list), chunk_size)]

def main(target_key: str):
    # Get the byte arrays
    target_bytes_str = convert_ss58_to_bytes_array(target_key)
    migration_bytes_str = extract_migration_bytes_array()
    
    # Check if they match
    match = target_bytes_str == migration_bytes_str
    match_status = "[green]MATCH[/green]" if match else "[red]MISMATCH[/red]"
    
    # Create a header for the table
    console.print(f"\nVerifying bytes for SS58 address: {target_key}")
    console.print(f"Match status: {match_status}\n")
    
    # Create a table for side-by-side comparison
    table = Table(title="Byte Comparison (4-byte chunks)")
    table.add_column("Chunk", justify="right", style="cyan")
    table.add_column("Target Key Bytes", style="green")
    table.add_column("Migration Code Bytes", style="yellow")
    table.add_column("Match", justify="center")
    
    # Split the byte arrays into chunks for better readability
    target_chunks = chunk_bytes(target_bytes_str)
    migration_chunks = chunk_bytes(migration_bytes_str)
    
    # Add rows to the table for each chunk
    for i, (target_chunk, migration_chunk) in enumerate(zip(target_chunks, migration_chunks)):
        chunk_match = target_chunk == migration_chunk
        match_indicator = "✓" if chunk_match else "✗"
        match_style = "green" if chunk_match else "red"
        
        table.add_row(
            f"Chunk {i+1}", 
            target_chunk, 
            migration_chunk,
            f"[{match_style}]{match_indicator}[/{match_style}]"
        )
    
    # Print the table
    console.print(table)

    
    

def print_custom_help():
    """Print a custom, rich-formatted help menu"""
    from rich.panel import Panel
    from rich.text import Text
    from rich.padding import Padding
    
    title = Text("Treasury Key Validator", style="bold cyan")
    subtitle = Text("A tool to validate and update treasury key bytes in migration code", style="italic yellow")
    
    usage = Text("\nUsage:", style="bold green")
    usage_cmd = Text("  uv run scripts/python/crypto/validate_replacement_key.py [OPTIONS]\n", style="blue")
    
    options_title = Text("Options:", style="bold green")
    options = [
        ("--key KEY", "The SS58 address to validate or write to the migration code"),
        ("--write", "Write the correct bytes to the migration code file"),
        ("-h, --help", "Show this help message and exit")
    ]
    
    options_text = ""
    for opt, desc in options:
        options_text += f"  [bold blue]{opt:<20}[/bold blue] [white]{desc}[/white]\n"
    
    examples_title = Text("\nExamples:", style="bold green")
    examples = [
        ("Validate default key:", "uv run scripts/python/crypto/validate_replacement_key.py"),
        ("Validate custom key:", "uv run scripts/python/crypto/validate_replacement_key.py --key YOUR_SS58_ADDRESS"),
        ("Update migration code:", "uv run scripts/python/crypto/validate_replacement_key.py --write")
    ]
    
    examples_text = ""
    for ex_desc, ex_cmd in examples:
        examples_text += f"  [bold yellow]{ex_desc:<25}[/bold yellow] [blue]{ex_cmd}[/blue]\n"
    
    content = f"{title}\n{subtitle}\n{usage}{usage_cmd}{options_title}\n{options_text}{examples_title}\n{examples_text}"
    panel = Panel(content, border_style="green", title="[bold white]Treasury Key Validator[/bold white]", subtitle="[italic]v1.0.0[/italic]")
    
    console.print(panel)

if __name__ == "__main__":
    import argparse
    import sys
    
    # Check if help flag is present
    if "-h" in sys.argv or "--help" in sys.argv:
        print_custom_help()
        sys.exit(0)
    
    # Use standard argparse for actual argument parsing
    parser = argparse.ArgumentParser(description="Validate and update treasury key bytes in migration code")
    parser.add_argument(
        "--key", 
        required=False, 
        default="5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj", 
        type=str,
        help="The SS58 address to validate or write to the migration code"
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write the correct bytes to the migration code file"
    )
    args = parser.parse_args()
    
    # Always run the validation
    main(args.key)
    
    # If write flag is set, update the migration code
    if args.write:
        target_bytes_str = convert_ss58_to_bytes_array(args.key)
        if write_bytes_to_migration_file(target_bytes_str):
            console.print(f"\n[green]Successfully updated migration code with bytes for {args.key}[/green]")
        else:
            console.print(f"\n[red]Failed to update migration code[/red]")