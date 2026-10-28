#!/usr/bin/env python3
"""
run_openapi_privilege_test.py —— 启动 openapi 数据权限集成测试所需实例并执行测试

自动完成:
1. cargo build 编译 rnacos(如 debug 二进制不存在)
2. 启动鉴权实例(RNACOS_ENABLE_OPEN_API_AUTH=true, http=18848 grpc=19848 console=20848)
3. 启动默认鉴权关闭实例(http=28848 grpc=29848 console=30848), 用于用例10回归
4. 执行 openapi_privilege_test.py 全量用例(用例4-10)
5. 停止实例并清理临时数据目录

数据目录: integration_tests/test_data/openapi_privilege/
"""

import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

import requests

SCRIPT_DIR = Path(__file__).parent.absolute()
SYSTEM = platform.system().lower()
RNACOS_ROOT_DIR = SCRIPT_DIR.parent.parent
RNACOS_BIN = RNACOS_ROOT_DIR.joinpath("target", "debug", ("rnacos.exe" if SYSTEM == "windows" else "rnacos"))
BUILD_CMD = "cargo build"
WORK_DIR = SCRIPT_DIR.parent.joinpath("test_data", "openapi_privilege")
TEST_SCRIPT = SCRIPT_DIR.joinpath("openapi_privilege_test.py")

AUTH_HTTP_PORT = 18848
NOAUTH_HTTP_PORT = 28848

PROCS = []


def start_instance(name: str, http_port: int, extra_env: dict, db_dir: Path, log_path: Path):
    env_lines = [
        f"RNACOS_HTTP_PORT={http_port}",
        f"RNACOS_DATA_DIR={db_dir}",
        "RNACOS_RAFT_NODE_ID=1",
        "RNACOS_RAFT_AUTO_INIT=true",
        # disable console captcha so the test can login through the console api
        "RNACOS_CONSOLE_ENABLE_CAPTCHA=false",
    ]
    for key, value in extra_env.items():
        env_lines.append(f"{key}={value}")
    env_path = WORK_DIR / f"env_{name}"
    env_path.write_text("\n".join(env_lines) + "\n", encoding="utf-8")
    log_fp = log_path.open("w", encoding="utf-8")
    proc = subprocess.Popen([str(RNACOS_BIN), "-e", str(env_path)], stdout=log_fp, stderr=subprocess.STDOUT)
    PROCS.append(proc)
    return proc


def wait_ready(base_url: str, timeout_seconds: int = 60) -> bool:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        try:
            resp = requests.get(f"{base_url}/health", timeout=2)
            if resp.status_code == 200:
                return True
        except requests.RequestException:
            pass
        time.sleep(1)
    return False


def stop_all():
    for proc in PROCS:
        if proc.poll() is None:
            proc.terminate()
    deadline = time.time() + 10
    for proc in PROCS:
        while proc.poll() is None and time.time() < deadline:
            time.sleep(0.2)
        if proc.poll() is None:
            proc.kill()
    PROCS.clear()


def main():
    if not RNACOS_BIN.exists():
        print("rnacos binary not found, building...")
        ret = subprocess.run(BUILD_CMD, shell=True, cwd=RNACOS_ROOT_DIR).returncode
        if ret != 0 or not RNACOS_BIN.exists():
            print("cargo build failed", file=sys.stderr)
            return 1

    if WORK_DIR.exists():
        shutil.rmtree(WORK_DIR, ignore_errors=True)
    WORK_DIR.mkdir(parents=True, exist_ok=True)

    exit_code = 1
    try:
        print("Starting auth-enabled instance (RNACOS_ENABLE_OPEN_API_AUTH=true) ...")
        start_instance(
            "auth",
            AUTH_HTTP_PORT,
            {"RNACOS_ENABLE_OPEN_API_AUTH": "true"},
            WORK_DIR / "db_auth",
            WORK_DIR / "auth.log",
        )
        print("Starting default (auth disabled) instance ...")
        start_instance(
            "noauth",
            NOAUTH_HTTP_PORT,
            {},
            WORK_DIR / "db_noauth",
            WORK_DIR / "noauth.log",
        )

        if not wait_ready(f"http://127.0.0.1:{AUTH_HTTP_PORT}"):
            print("auth instance not ready, check log:", WORK_DIR / "auth.log", file=sys.stderr)
            return 1
        if not wait_ready(f"http://127.0.0.1:{NOAUTH_HTTP_PORT}"):
            print("no-auth instance not ready, check log:", WORK_DIR / "noauth.log", file=sys.stderr)
            return 1
        print("instances are ready\n")

        ret = subprocess.run([sys.executable, str(TEST_SCRIPT)])
        exit_code = ret.returncode
    finally:
        print("\nStopping instances ...")
        stop_all()
        print(f"(data & logs kept at {WORK_DIR})")
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
