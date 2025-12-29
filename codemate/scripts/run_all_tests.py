#!/usr/bin/env python3
import subprocess
import sys
import os
import time
import signal

# Configuration
TESTS = [
    {
        "name": "Rust Unit & E2E Tests",
        "command": ["cargo", "test", "--all"],
        "timeout": 60,
        "critical": True
    },
    {
        "name": "Module Detection Tests",
        "command": ["python3", "scripts/test_module_detection.py"],
        "timeout": 30,
        "critical": True
    },
    {
        "name": "REST API Tests",
        "command": ["bash", "scripts/test_server_standalone.sh"],
        "timeout": 60,
        "critical": False
    },
    {
        "name": "MCP Server Tests",
        "command": ["python3", "scripts/test_mcp.py"],
        "timeout": 30,
        "critical": False
    }
]

def run_test(test):
    print(f"\n>>> Running: {test['name']}...")
    try:
        start_time = time.time()
        # Using shell=False for security, assuming command is a list
        process = subprocess.Popen(
            test['command'],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            preexec_fn=os.setsid # For timeout handling on Unix
        )
        
        try:
            stdout, _ = process.communicate(timeout=test['timeout'])
            duration = time.time() - start_time
            
            if process.returncode == 0:
                print(f"✅ {test['name']} passed ({duration:.2f}s)")
                return True
            else:
                print(f"❌ {test['name']} failed with exit code {process.returncode}")
                if stdout:
                    print("-" * 20)
                    print(stdout.strip()[-500:]) # Show last 500 chars
                    print("-" * 20)
                return False
                
        except subprocess.TimeoutExpired:
            os.killpg(os.getpgid(process.pid), signal.SIGKILL)
            print(f"⏰ {test['name']} timed out after {test['timeout']}s")
            return False
            
    except Exception as e:
        print(f"⚠️ Error running {test['name']}: {e}")
        return False

def main():
    print("=" * 40)
    print("CodeMate Unified Automated Testing Suite")
    print("=" * 40)
    
    # Ensure binary exists for server tests
    if not os.path.exists("./target/debug/codemate-server"):
        print("Building codemate-server...")
        subprocess.run(["cargo", "build", "-p", "codemate-server"], check=True)

    results = []
    for test in TESTS:
        success = run_test(test)
        results.append((test['name'], success, test['critical']))
        
        if not success and test['critical']:
            print(f"\n🛑 Critical test '{test['name']}' failed. Aborting remaining tests.")
            break

    print("\n" + "=" * 40)
    print("TEST SUMMARY")
    print("=" * 40)
    
    all_passed = True
    for name, success, critical in results:
        status = "PASS" if success else ("FAIL (SKIPPED)" if not critical else "FAIL (ABORTED)")
        icon = "✅" if success else "❌"
        print(f"{icon} {name}: {status}")
        if not success and critical:
            all_passed = False
    
    if all_passed:
        print("\n✨ All critical tests passed!")
        sys.exit(0)
    else:
        print("\n🚫 Some critical tests failed.")
        sys.exit(1)

if __name__ == "__main__":
    main()
