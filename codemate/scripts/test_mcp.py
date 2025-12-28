import subprocess
import json
import sys
import os
import time
import threading

def log_stderr(proc):
    for line in iter(proc.stderr.readline, b''):
        print(f"DEBUG stderr: {line.decode().strip()}", file=sys.stderr)

def send_request(proc, method, params=None, req_id=1):
    request = {
        "jsonrpc": "2.0",
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
    return json.loads(line.decode())

def test_mcp():
    # Use built binary for performance
    binary_path = "./target/debug/codemate-server"
    if not os.path.exists(binary_path):
        print("✗ Binary not found. Please run 'cargo build -p codemate-server' first.")
        return

    print("Starting CodeMate MCP server...")
    proc = subprocess.Popen(
        [binary_path, "--mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    # Start stderr logging
    threading.Thread(target=log_stderr, args=(proc,), daemon=True).start()

    try:
        # Give server time to initialize (especially model loading)
        time.sleep(2)
        
        print("1. Initializing...")
        resp = send_request(proc, "initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"},
            "implementation": {"name": "test-client", "version": "1.0.0"}
        })
        if resp and "result" in resp:
            print("✓ Initialize successful")
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

if __name__ == "__main__":
    test_mcp()
