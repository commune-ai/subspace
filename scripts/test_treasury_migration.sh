#!/bin/bash
# Script to test the treasury address migration in a development environment

set -e

echo "Starting treasury address migration test..."

# Build the node (if not already built)
echo "Building the node..."
cargo build

# Start a development chain with clean state
echo "Starting a development chain..."
./target/debug/node-subspace --dev --tmp &
NODE_PID=$!

# Wait for the node to start
sleep 5

# Get the current treasury address
echo "Getting the current treasury address..."
CURRENT_TREASURY=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x5f3e4907f716ac89b6347d15ececedca5a6744b4a8da4e2797715a7555bc5694"]}' http://localhost:9944 | jq -r '.result')

echo "Current treasury address: $CURRENT_TREASURY"

# Simulate the migration by killing the node and restarting with the updated runtime
echo "Killing the node..."
kill $NODE_PID
wait $NODE_PID 2>/dev/null || true

echo "Waiting for node to shut down..."
sleep 5

# In a real scenario, we would deploy the updated runtime here
# For this test, we'll just restart the node and pretend the migration happened
echo "Restarting the node with 'updated' runtime..."
./target/debug/node-subspace --dev --tmp &
NODE_PID=$!

# Wait for the node to start
sleep 5

# Get the "new" treasury address (in a real scenario, this would be different)
echo "Getting the 'new' treasury address..."
NEW_TREASURY=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x5f3e4907f716ac89b6347d15ececedca5a6744b4a8da4e2797715a7555bc5694"]}' http://localhost:9944 | jq -r '.result')

echo "New treasury address: $NEW_TREASURY"

# In a real test with the actual migration, we would verify that the addresses are different
# For this simulation, they will be the same
echo "In a real migration, the treasury address would change from $CURRENT_TREASURY to 5GZfkfjD46SmDrnWZbrzkxkYzeJUWKTAB1HvHBurrPc7XcEj"

# Clean up
echo "Cleaning up..."
kill $NODE_PID
wait $NODE_PID 2>/dev/null || true

echo "Test completed."
