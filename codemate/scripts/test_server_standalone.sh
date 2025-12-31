#!/bin/bash
# CodeMate REST API Standalone Test Script (handles server lifecycle)
# Usage: bash scripts/test_server_standalone.sh

PORT=8080
DB="/tmp/test_standalone.db"
BASE_URL="http://localhost:$PORT"

# Ensure server is built
if [ ! -f "./target/debug/codemate-server" ]; then
    cargo build -p codemate-server
fi

# Cleanup old server
pkill -9 codemate-server || true

# Start server
./target/debug/codemate-server --database $DB --port $PORT > standalone_server.log 2>&1 &
SERVER_PID=$!

echo "Starting CodeMate server (PID: $SERVER_PID)..."
echo "Starting CodeMate server (PID: $SERVER_PID)..."
# Poll health endpoint with timeout
TIMEOUT=10
COUNT=0
until curl -s $BASE_URL/health > /dev/null || [ $COUNT -eq $TIMEOUT ]; do
    sleep 1
    COUNT=$((COUNT + 1))
    echo "Waiting for server ($COUNT)..."
done

if [ $COUNT -eq $TIMEOUT ]; then
    echo "✗ Server failed to start within ${TIMEOUT}s."
    kill $SERVER_PID
    exit 1
fi

# Run tests
echo "Checking health..."
if ! curl -s $BASE_URL/health > /dev/null; then
    echo "✗ Server failed to start."
    kill $SERVER_PID
    exit 1
fi

echo -n "Testing indexing... "
resp=$(curl -s -X POST $BASE_URL/api/v1/index -H "Content-Type: application/json" -d "{\"path\": \"$(pwd)\"}")
if echo $resp | grep -q "message"; then
    echo "✓ Indexing started"
else
    echo "✗ Indexing failed: $resp"
    kill $SERVER_PID
    exit 1
fi

echo "Waiting for indexing (2s)..."
# Indexing is extremely fast for this small project with our new optimizations
sleep 2

echo -n "Testing search... "
resp=$(curl -s -X POST $BASE_URL/api/v1/search -H "Content-Type: application/json" -d '{"query": "AppState"}')
if echo $resp | grep -q "results"; then
    echo "✓ Search successful"
else
    echo "✗ Search failed: $resp"
    kill $SERVER_PID
    exit 1
fi

echo -n "Testing module graph... "
resp=$(curl -s -X POST $BASE_URL/api/v1/graph/modules -H "Content-Type: application/json" -d '{}')
if echo $resp | grep -q "modules"; then
    echo "✓ Module graph successful"
else
    echo "✗ Module graph failed: $resp"
    kill $SERVER_PID
    exit 1
fi

echo "All standalone REST tests passed."
kill $SERVER_PID
exit 0
