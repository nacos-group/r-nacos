#!/usr/bin/env python3
"""
McpServer Console API Test Runner

This script starts a single r-nacos instance and runs the McpServer console API tests.
It's designed to be simple and focused on testing the McpServer API functionality.
"""

import os
import platform
import subprocess
import sys
import tempfile
import time
import signal
from pathlib import Path


SCRIPT_DIR = Path(__file__).parent.absolute()
SYSTEM = platform.system().lower()


def rnacos_exe_name() -> str:
    return "rnacos.exe" if SYSTEM == "windows" else "rnacos"


# Configuration
RNACOS_ROOT_DIR = SCRIPT_DIR.parent.parent
RNACOS_BIN = RNACOS_ROOT_DIR.joinpath("target").joinpath("debug").joinpath(rnacos_exe_name())
BUILD_CMD = "cargo build"
HTTP_PORT = 8848
RAFT_PORT = 9848


def kill_rnacos():
    """Kill any existing rnacos processes"""
    exe = rnacos_exe_name()
    if SYSTEM == "windows":
        subprocess.run(["taskkill", "/F", "/IM", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    elif SYSTEM == "darwin":
        subprocess.run(["pkill", "-f", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    else:
        subprocess.run(["killall", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def cleanup(work_dir: Path):
    """Clean up processes and data"""
    kill_rnacos()
    # Clean up database directory
    db_dir = work_dir / "db"
    try:
        if db_dir.exists():
            if SYSTEM == 'windows':
                os.system(f'rmdir /s /q "{db_dir}"')
            else:
                subprocess.run(['rm', '-rf', str(db_dir)], check=True)
    except subprocess.CalledProcessError as e:
        print(f"Error cleaning up {db_dir}: {e}")


def create_env_file(work_dir: Path) -> Path:
    """Create environment configuration file"""
    lines = [
        f"RNACOS_HTTP_PORT={HTTP_PORT}",
        f"RNACOS_RAFT_NODE_ADDR=127.0.0.1:{RAFT_PORT}",
        f"RNACOS_CONFIG_DB_DIR={work_dir}/db",
        "RNACOS_RAFT_NODE_ID=1",
        "RNACOS_RAFT_AUTO_INIT=true",
        "RNACOS_ENABLE_NO_AUTH_CONSOLE=true",
    ]
    
    env_path = work_dir / "env"
    env_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return env_path


def start_rnacos(env_path: Path, log_path: Path) -> subprocess.Popen:
    """Start rnacos server"""
    log_fp = log_path.open("w", encoding="utf-8")
    return subprocess.Popen(
        [RNACOS_BIN, "-e", str(env_path)],
        stdout=log_fp,
        stderr=subprocess.STDOUT
    )


def wait_for_server(host: str = "127.0.0.1", port: int = 8848, timeout: int = 30):
    """Wait for server to be ready"""
    import requests
    
    url = f"http://{host}:{port}/rnacos/api/console/v2/mcp/server/list"
    start_time = time.time()
    
    while time.time() - start_time < timeout:
        try:
            response = requests.get(url, timeout=5)
            if response.status_code == 200:
                print(f"✅ Server is ready at {host}:{port}")
                return True
        except requests.RequestException:
            pass
        
        print(f"⏳ Waiting for server to start... ({int(time.time() - start_time)}s)")
        time.sleep(2)
    
    print(f"❌ Server failed to start within {timeout} seconds")
    return False


def run_mcp_server_tests(host: str = "127.0.0.1", port: int = 8848) -> int:
    """Run the McpServer API tests"""
    test_script = SCRIPT_DIR / "mcp_server_console_api_test.py"
    
    if not test_script.exists():
        print(f"❌ Test script not found: {test_script}")
        return 1
    
    print("🧪 Running McpServer Console API tests...")
    
    cmd = [
        sys.executable,
        str(test_script),
        "--host", host,
        "--port", str(port)
    ]
    
    try:
        result = subprocess.run(cmd, check=False)
        return result.returncode
    except Exception as e:
        print(f"❌ Failed to run tests: {e}")
        return 1


def main():
    """Main function"""
    import argparse
    
    parser = argparse.ArgumentParser(description="McpServer Console API Test Runner")
    parser.add_argument("--build", action="store_true", help="Build rnacos before running tests")
    parser.add_argument("--host", default="127.0.0.1", help="Server host")
    parser.add_argument("--port", default=8848, type=int, help="Server port")
    parser.add_argument("--keep-running", action="store_true", help="Keep server running after tests")
    parser.add_argument("--skip-server", action="store_true", help="Skip starting server, test against existing server")
    parser.add_argument("--no-cleanup", action="store_true", help="Skip cleanup of test data and processes")
    
    args = parser.parse_args()
    
    # Skip server setup if requested
    if args.skip_server:
        print(f"⏭️ Skipping server startup, testing against {args.host}:{args.port}")
        
        # Wait for server to be ready
        if not wait_for_server(args.host, args.port):
            print("❌ Target server is not ready")
            sys.exit(1)
        
        # Run tests directly
        test_result = run_mcp_server_tests(args.host, args.port)
        
        if test_result == 0:
            print("🎉 All tests passed!")
        else:
            print("💥 Some tests failed!")
        
        sys.exit(test_result)
    
    # Check if rnacos binary exists
    exe = Path(RNACOS_BIN)
    if not exe.exists() or args.build:
        print("🔨 Building rnacos...")
        result = subprocess.run(BUILD_CMD, shell=True, cwd=RNACOS_ROOT_DIR)
        if result.returncode != 0:
            print("❌ Build failed")
            sys.exit(1)
        
        if not exe.exists():
            print(f"❌ rnacos executable not found: {exe.absolute()}")
            sys.exit(1)
    
    # Create temporary work directory
    work_dir = Path(tempfile.mkdtemp(prefix="rnacos_mcp_server_test_"))
    print(f"📁 Working directory: {work_dir}")
    
    # Clean up any existing processes (unless no-cleanup is specified)
    if not args.no_cleanup:
        cleanup(work_dir)
    
    proc = None
    test_result = 1
    
    try:
        # Create environment file
        env_path = create_env_file(work_dir)
        log_path = work_dir / "rnacos.log"
        
        # Start rnacos server
        print("🚀 Starting rnacos server...")
        proc = start_rnacos(env_path, log_path)
        time.sleep(3)  # Give it a moment to start
        
        # Check if process is still running
        if proc.poll() is not None:
            print("❌ rnacos server failed to start")
            print("📋 Server log:")
            if log_path.exists():
                print(log_path.read_text())
            sys.exit(1)
        
        # Wait for server to be ready
        if not wait_for_server(args.host, args.port):
            print("❌ Server failed to become ready")
            sys.exit(1)
        
        # Run tests
        test_result = run_mcp_server_tests(args.host, args.port)
        
        if test_result == 0:
            print("🎉 All tests passed!")
        else:
            print("💥 Some tests failed!")
        
        if args.keep_running:
            print(f"🔄 Server is running at http://{args.host}:{args.port}")
            print("Press Ctrl+C to stop...")
            try:
                proc.wait()
            except KeyboardInterrupt:
                print("\n🛑 Stopping server...")
        
    except KeyboardInterrupt:
        print("\n🛑 Interrupted by user")
        test_result = 130
    
    finally:
        # Cleanup
        if args.no_cleanup:
            print("⏭️ Skipping cleanup (--no-cleanup specified)")
            if work_dir:
                print(f"📁 Test data preserved in: {work_dir}")
        else:
            print("🧹 Cleaning up...")
            if proc:
                try:
                    if SYSTEM == "windows":
                        proc.kill()
                    else:
                        proc.send_signal(signal.SIGTERM)
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    if SYSTEM == "windows":
                        proc.kill()
                    else:
                        proc.send_signal(signal.SIGKILL)
                    proc.wait()
                except Exception:
                    pass
                
                if proc.stdout and not proc.stdout.closed:
                    proc.stdout.close()
            
            kill_rnacos()
            
            try:
                cleanup(work_dir)
            except Exception as e:
                print(f"⚠️ Cleanup warning: {e}")
    
    sys.exit(test_result)


if __name__ == "__main__":
    main()