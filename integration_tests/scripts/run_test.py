#!/usr/bin/env python3
"""
run_test.py  ——  跨平台一键启动 rnacos 三节点集群并跑测试
数据目录 & 日志目录均集中到同一个临时目录，脚本会打印该目录路径
"""

import os
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path


SCRIPT_DIR = Path(__file__).parent.absolute()
SYSTEM = platform.system().lower()
def rnacos_exe_name() -> str:
    return "rnacos.exe" if SYSTEM == "windows" else "rnacos"

# ========== 可调整参数 ==========
INTEGRATION_PROJECT_ROOT_DIR = SCRIPT_DIR.parent.joinpath("rust-client-test")
RNACOS_ROOT_DIR = SCRIPT_DIR.parent.parent;
RNACOS_BIN = RNACOS_ROOT_DIR.joinpath("target").joinpath("debug").joinpath(rnacos_exe_name())
BUILD_CMD= "cargo build" 
TEST_CMD = "cargo test"   # 测试命令
NODE_CNT = 3                   # 节点数
BASE_HTTP_PORT = 8848
BASE_RAFT_PORT = 9848
# =================================

def get_test_data_dir():
    """获取test_data目录路径，确保其存在"""
    test_data_dir = SCRIPT_DIR.parent / "test_data"
    test_data_dir.mkdir(exist_ok=True)
    return test_data_dir

def kill_rnacos():
    exe = rnacos_exe_name()
    if SYSTEM == "windows":
        subprocess.run(["taskkill", "/F", "/IM", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if SYSTEM == "darwin":
        subprocess.run(["pkill", "-f", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    else:
        subprocess.run(["killall", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
def cleanup(work_dir: Path):
    #"""杀掉进程"""
    kill_rnacos()
    # 清理数据库目录
    for node_id in range(1, NODE_CNT + 1):
        dir_path = work_dir / f"db{node_id:02d}"
        try:
            if dir_path.exists():
                if SYSTEM == 'windows':
                    os.system(f'rmdir /s /q "{dir_path}"')
                else:
                    subprocess.run(['rm', '-rf', str(dir_path)], check=True)
        except subprocess.CalledProcessError as e:
            print(f"Error cleaning up {dir_path}: {e}")

def gen_env(node_id: int, work_dir: Path) -> Path:
    http_port = BASE_HTTP_PORT + node_id - 1
    raft_port = BASE_RAFT_PORT + node_id - 1
    lines = [
        f"RNACOS_HTTP_PORT={http_port}",
        f"RNACOS_RAFT_NODE_ADDR=127.0.0.1:{raft_port}",
        f"RNACOS_CONFIG_DB_DIR={work_dir}/db{node_id:02d}",
        f"RNACOS_RAFT_NODE_ID={node_id}",
        "RNACOS_ENABLE_NO_AUTH_CONSOLE=true",
    ]
    if node_id == 1:
        lines.append("RNACOS_RAFT_AUTO_INIT=true")
    else:
        lines.append(f"RNACOS_RAFT_JOIN_ADDR=127.0.0.1:{BASE_RAFT_PORT}")

    env_path = work_dir / f"env{node_id:02d}"
    env_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return env_path

def start_node(env_path: Path, log_path: Path) -> subprocess.Popen:
    log_fp = log_path.open("w", encoding="utf-8")
    return subprocess.Popen([RNACOS_BIN,"-e",str(env_path)],stdout=log_fp, stderr=subprocess.STDOUT)

def main():
    exe = Path(RNACOS_BIN)
    if not exe.exists():
        subprocess.run(BUILD_CMD, shell=True, cwd=RNACOS_ROOT_DIR)
        if not exe.exists():
            print(f"rnacos executable not found: {exe.absolute()}", file=sys.stderr)
            sys.exit(1)

    work_dir = get_test_data_dir()
    print(f"Working directory (data & logs): {work_dir}")
    # 保险起见，先清理
    cleanup(work_dir)
    procs = []

    try:
        for i in range(1, NODE_CNT + 1):
            env_path = gen_env(i, work_dir)
            log_path = work_dir / f"n{i:02d}.log"
            print(f"Starting node {i} ...")
            procs.append(start_node(env_path, log_path))
            time.sleep(3)

        print("Running test command:", TEST_CMD)
        test_ret = subprocess.run(TEST_CMD, shell=True, cwd=INTEGRATION_PROJECT_ROOT_DIR).returncode
    finally:
        print("Stopping rnacos processes...")
        kill_rnacos()
        for p in procs:
            try:
                p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                if SYSTEM == "windows":
                    p.kill()
                else:
                    p.send_signal(signal.SIGKILL)
                p.wait()
            if p.stdout and not p.stdout.closed:
                p.stdout.close()
        try:
            cleanup(work_dir)  
        except:
            pass
    if test_ret != 0:
        print(f"running command error,error_code: {test_ret}")
    sys.exit(test_ret)

if __name__ == "__main__":
    main()

