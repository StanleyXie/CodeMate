import subprocess
import json
import sys
import os
import time
import threading
import tempfile

def log_stderr(proc):
    for line in iter(proc.stderr.readline, b''):
        print(f"DEBUG stderr: {line.decode().strip()}", file=sys.stderr)

def send_request(proc, method, params=None, req_id=1):
    request = {
        "jsonrpc": "2.0",
        "type": "request",
        "id": req_id,
        "method": method,
        "params": params or {}
    }
    msg = json.dumps(request) + "\n"
    proc.stdin.write(msg.encode())
    proc.stdin.flush()
    
    line = proc.stdout.readline()
    if not line:
        return None
    print(f"DEBUG raw stdout: {line.decode().strip()}")
    return json.loads(line.decode())

def send_notification(proc, method, params=None):
    notification = {
        "jsonrpc": "2.0",
        "type": "notification",
        "method": method,
        "params": params or {}
    }
    msg = json.dumps(notification) + "\n"
    proc.stdin.write(msg.encode())
    proc.stdin.flush()

def test_mcp():
    # Use built binary for performance
    binary_path = "./target/debug/codemate-server"
    if not os.path.exists(binary_path):
        print("✗ Binary not found. Please run 'cargo build -p codemate-server' first.")
        return

    # Create a fresh temp database for testing
    temp_db = tempfile.mktemp(suffix=".db", prefix="mcp_test_")
    
    print(f"Starting CodeMate MCP server with temp db: {temp_db}...")
    proc = subprocess.Popen(
        [binary_path, "--mcp", "--database", temp_db],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    # Start stderr logging
    threading.Thread(target=log_stderr, args=(proc,), daemon=True).start()

    try:
        # Give server time to initialize (especially model loading)
        print("Waiting for server to initialize (loading embedding model)...")
        time.sleep(5)
        
        print("1. Initializing...")
        resp = send_request(proc, "initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"},
            "implementation": {"name": "test-client", "version": "1.0.0"}
        })
        if resp and "result" in resp:
            print("✓ Initialize successful")
            # Send initialized notification
            send_notification(proc, "initialized")
            print("✓ Sent initialized notification")
        else:
            print(f"✗ Initialize failed: {resp}")
            return

        print("2. Listing tools...")
        resp = send_request(proc, "tools/list", req_id=2)
        if resp and "result" in resp:
            tools = [t["name"] for t in resp["result"]["tools"]]
            print(f"✓ Found tools: {tools}")
        else:
            print(f"✗ Tool list failed: {resp}")
            return

        print("\nMCP server tests passed!")
    except Exception as e:
        print(f"\nTest failed: {e}")
    finally:
        proc.terminate()
        # Clean up temp database
        if os.path.exists(temp_db):
            os.remove(temp_db)

if __name__ == "__main__":
    test_mcp()
