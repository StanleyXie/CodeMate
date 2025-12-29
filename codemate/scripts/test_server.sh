#!/bin/bash
# CodeMate REST API Test Script

BASE_URL="http://localhost:8080"

echo "Checking if server is running on $BASE_URL..."
if ! curl -s $BASE_URL/health > /dev/null; then
    echo "✗ Server is not running. Please start it with 'cargo run -p codemate-server'"
    exit 1
fi

echo "✓ Health check passed"

echo -n "Testing indexing... "
resp=$(curl -s -X POST $BASE_URL/api/v1/index -H "Content-Type: application/json" -d '{"path": "."}')
if echo $resp | grep -q "message"; then
    echo "✓ Indexing started"
else
    echo "✗ Indexing failed: $resp"
    exit 1
fi

echo "Waiting for indexing (5s)..."
sleep 5

echo -n "Testing search... "
resp=$(curl -s -X POST $BASE_URL/api/v1/search -H "Content-Type: application/json" -d '{"query": "AppState"}')
if echo $resp | grep -q "results"; then
    echo "✓ Search successful"
else
    echo "✗ Search failed: $resp"
    exit 1
fi

echo -n "Testing tree rendering... "
resp=$(curl -s -X POST $BASE_URL/api/v1/graph/tree -H "Content-Type: application/json" -d '{"symbol": "AppState"}')
if echo $resp | grep -q "tree"; then
    echo "✓ Tree rendering successful"
else
    echo "✗ Tree rendering failed: $resp"
    exit 1
fi

echo -n "Testing module graph... "
resp=$(curl -s -X POST $BASE_URL/api/v1/graph/modules -H "Content-Type: application/json" -d '{}')
if echo $resp | grep -q "modules"; then
    echo "✓ Module graph successful"
else
    echo "✗ Module graph failed: $resp"
    exit 1
fi

echo -e "\nAll REST API tests passed!"
