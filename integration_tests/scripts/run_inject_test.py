#!/usr/bin/env python3
"""
run_inject_test.py —— raft 日志写入错误注入端到端复现脚本（纯 python 驱动）

一键启动 3 节点 debug 集群 -> 向 leader 发布配置 -> 对从节点(node2)注入
discard_log(丢弃 1 次 WriteBatch) -> 多次更新配置触发 raft 在从节点上产生后续
WriteBatch -> 轮询从节点日志判定是否复现 "logfile index != record.index"。

仅复用 run_test.py 的集群启停基础设施（gen_env / start_node / kill_rnacos /
cleanup / get_test_data_dir），不运行任何 rust 测试用例。

前置依赖:
  - 已安装 python3 与 nacos python sdk（pip install nacos）
  - 以 cargo build --features debug 构建 target/debug/rnacos（含 inject-error 接口）

用法:
  python3 run_inject_test.py                # 默认: 复现后自动关停集群并清理临时目录
  python3 run_inject_test.py --keep-cluster # 复现后保留集群, 打印各节点端口与日志路径
  python3 run_inject_test.py --only-stop    # 仅关停并清理已存在的测试集群, 不运行测试

关于更新次数: raftlog 的 index_interval 默认为 128（见 raft/filestore/model.rs）,
"logfile index != record.index" 告警仅在每 128 条记录边界处检查一次。丢弃 1 条记录后,
需要继续写入约一个 index_interval 的记录才稳定命中边界, 故脚本采用"循环更新 + 轮询日志"
的方式, 命中告警即提前停止, 最多更新 UPDATE_MAX 次。
"""

import argparse
import json
import os
import platform
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent.absolute()
SYSTEM = platform.system().lower()

# 复用 run_test.py 的路径与集群参数定义
RNACOS_ROOT_DIR = SCRIPT_DIR.parent.parent
RNACOS_BIN = RNACOS_ROOT_DIR.joinpath(
    "target", "debug", "rnacos.exe" if SYSTEM == "windows" else "rnacos"
)
BUILD_CMD = "cargo build --features debug"
NODE_CNT = 3
BASE_HTTP_PORT = 8848
BASE_RAFT_PORT = 9848

# 注入与配置参数（与 testcase.md 用例1 对齐）
INJECT_TARGET_NODE = 2          # 被注入的从节点 node_id
INJECT_TIMES = 1                # 丢弃后续 WriteBatch 的次数
DATA_ID = "inject.test"
GROUP = "default"
NAMESPACE = ""                  # 默认命名空间
UPDATE_MAX = 200                # 注入后最多更新配置的次数（需覆盖 index_interval=128）
UPDATE_INTERVAL = 0.3           # 每次更新之间的间隔（秒）
ALERT_KEYWORD = "logfile index != record.index"


def get_test_data_dir():
    """获取集中存放数据/日志的临时目录，确保其存在。"""
    test_data_dir = SCRIPT_DIR.parent / "test_data"
    test_data_dir.mkdir(exist_ok=True)
    return test_data_dir


def kill_rnacos():
    """杀掉本机所有 rnacos 进程；unix 下 pkill 发 SIGTERM 后节点优雅退出需要数秒，
    故轮询等待其真正退出，确保函数返回时进程已消失（超时则强制 SIGKILL）。
    匹配 "/rnacos" 而非 "rnacos"，避免误伤工作目录名含 "r-nacos" 的无关进程。"""
    exe = "rnacos.exe" if SYSTEM == "windows" else "rnacos"
    if SYSTEM == "windows":
        subprocess.run(["taskkill", "/F", "/IM", exe],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return
    subprocess.run(["pkill", "-f", exe],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.time() + 10
    while time.time() < deadline:
        ret = subprocess.run(["pgrep", "-f", "/rnacos"],
                             stdout=subprocess.DEVNULL)
        if ret.returncode != 0:
            return
        time.sleep(0.5)
    subprocess.run(["pkill", "-9", "-f", "/rnacos"],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def cleanup(work_dir, procs=None):
    """杀掉节点进程并清理各节点数据目录；procs 为本次启动的进程句柄用于回收。"""
    kill_rnacos()
    if procs:
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
    for node_id in range(1, NODE_CNT + 1):
        dir_path = work_dir / f"db{node_id:02d}"
        try:
            if dir_path.exists():
                if SYSTEM == "windows":
                    os.system(f'rmdir /s /q "{dir_path}"')
                else:
                    subprocess.run(["rm", "-rf", str(dir_path)], check=True)
        except subprocess.CalledProcessError as e:
            print(f"Error cleaning up {dir_path}: {e}")


def gen_env(node_id, work_dir):
    """生成单个节点的 env 文件（复用 run_test.py 的配置方式）。"""
    http_port = BASE_HTTP_PORT + node_id - 1
    raft_port = BASE_RAFT_PORT + node_id - 1
    lines = [
        f"RNACOS_HTTP_PORT={http_port}",
        f"RNACOS_RAFT_NODE_ADDR=127.0.0.1:{raft_port}",
        f"RNACOS_DATA_DIR={work_dir}/db{node_id:02d}",
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


def start_node(env_path, log_path):
    """启动单个节点，stdout/stderr 重定向到 log_path。"""
    log_fp = log_path.open("w", encoding="utf-8")
    return subprocess.Popen(
        [str(RNACOS_BIN), "-e", str(env_path)],
        stdout=log_fp, stderr=subprocess.STDOUT,
    )


def node_http_port(node_id):
    return BASE_HTTP_PORT + node_id - 1


def node_log_path(work_dir, node_id):
    return work_dir / f"n{node_id:02d}.log"


def http_get_json(url, timeout=2):
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def wait_node_up(node_id, timeout=40):
    """轮询节点 metrics 接口直到其可访问。"""
    url = f"http://127.0.0.1:{node_http_port(node_id)}/nacos/v1/raft/metrics"
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            http_get_json(url)
            return True
        except Exception:
            time.sleep(1)
    return False


def wait_for_leader(timeout=60):
    """轮询各节点 metrics，返回当前 leader 的 node_id。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        for node_id in range(1, NODE_CNT + 1):
            url = f"http://127.0.0.1:{node_http_port(node_id)}/nacos/v1/raft/metrics"
            try:
                data = http_get_json(url)
            except Exception:
                continue
            leader = data.get("current_leader")
            if leader is None:
                continue
            try:
                return int(leader)
            except (TypeError, ValueError):
                continue
        time.sleep(1)
    raise RuntimeError(f"等待集群选出 leader 超时({timeout}s)")


def inject_discard_log(node_id, times):
    """向指定节点 POST inject-error，返回 (http_status, body_json_or_None)。"""
    url = f"http://127.0.0.1:{node_http_port(node_id)}/nacos/v1/raft/inject-error"
    payload = json.dumps({"scene": "discard_log", "times": times}).encode("utf-8")
    req = urllib.request.Request(
        url, data=payload, headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            body = json.loads(resp.read().decode("utf-8"))
            return resp.status, body
    except urllib.error.HTTPError as e:
        return e.code, None


def check_log_alert(log_path):
    """检查节点日志文件是否已出现目标告警关键字。"""
    if not log_path.exists():
        return False
    try:
        with log_path.open("r", encoding="utf-8", errors="ignore") as f:
            return ALERT_KEYWORD in f.read()
    except Exception:
        return False


def build_debug_binary():
    """以 debug feature 构建 rnacos 二进制。"""
    print(f"Building debug binary: {BUILD_CMD}")
    ret = subprocess.run(BUILD_CMD, shell=True, cwd=RNACOS_ROOT_DIR).returncode
    if ret != 0:
        print(f"[ERROR] 构建失败: {BUILD_CMD}", file=sys.stderr)
        sys.exit(1)


def ensure_binary():
    """确保带 debug feature 的二进制存在；缺失则构建。"""
    exe = Path(RNACOS_BIN)
    if not exe.exists():
        build_debug_binary()
    if not exe.exists():
        print(f"[ERROR] rnacos 二进制不存在: {exe.absolute()}", file=sys.stderr)
        sys.exit(1)


def start_cluster(work_dir):
    """启动 3 节点集群，返回进程句柄列表。"""
    procs = []
    for i in range(1, NODE_CNT + 1):
        env_path = gen_env(i, work_dir)
        log_path = node_log_path(work_dir, i)
        print(f"Starting node {i} (http 127.0.0.1:{node_http_port(i)}) ...")
        procs.append(start_node(env_path, log_path))
        time.sleep(3)
    return procs


def run_reproduce(work_dir):
    """执行完整复现流程，返回 (是否复现, 进程句柄列表)。"""
    import nacos  # 惰性导入: 仅默认/--keep-cluster 模式需要 nacos sdk

    ensure_binary()
    cleanup(work_dir)
    procs = start_cluster(work_dir)
    for i in range(1, NODE_CNT + 1):
        if not wait_node_up(i, timeout=40):
            print(f"[WARN] node {i} 未在预期时间内就绪")

    leader_id = wait_for_leader(timeout=60)
    print(f"Cluster leader elected: node {leader_id} "
          f"(http 127.0.0.1:{node_http_port(leader_id)})")

    inject_node = INJECT_TARGET_NODE
    if leader_id == inject_node:
        inject_node = 2 if inject_node != 2 else 3
        print(f"[WARN] 注入目标 node{INJECT_TARGET_NODE} 当前是 leader, "
              f"改为注入 node {inject_node}")

    leader_addr = f"http://127.0.0.1:{node_http_port(leader_id)}"
    client = nacos.NacosClient(leader_addr, namespace=NAMESPACE)
    print(f"Publish config data_id={DATA_ID} to leader {leader_addr}")
    client.publish_config(DATA_ID, GROUP, "inject init value 0")
    payload = {"name": "pre-inject", "value": 0}
    for i in range(60):
        payload["value"] = i
        client.publish_config(DATA_ID, GROUP, json.dumps(payload))
        time.sleep(0.05)

    status, body = inject_discard_log(inject_node, INJECT_TIMES)
    if status == 404:
        print("[ERROR] inject-error 路由返回 404, 当前二进制未启用 debug feature。\n"
              "        请重新构建: cargo build --features debug", file=sys.stderr)
        return False, procs
    print(f"Inject discard_log to node{inject_node} times={INJECT_TIMES} "
          f"-> http {status}, body={body}")

    reproduced = False
    payload = {"name": "inject", "value": 0}
    for i in range(1, UPDATE_MAX + 1):
        payload["value"] = i
        client.publish_config(DATA_ID, GROUP, json.dumps(payload))
        if i % 10 == 0 or i == 1:
            print(f"  update config {i}/{UPDATE_MAX} ...")
        if check_log_alert(node_log_path(work_dir, inject_node)):
            reproduced = True
            print(f"Alert detected after {i} update(s).")
            break
        time.sleep(UPDATE_INTERVAL)

    return reproduced, procs


def main():
    parser = argparse.ArgumentParser(
        description="raft 日志写入错误注入端到端复现脚本"
    )
    parser.add_argument(
        "--keep-cluster", action="store_true",
        help="判定结束后不关停集群, 打印各节点端口与日志路径供人工复核",
    )
    parser.add_argument(
        "--only-stop", action="store_true",
        help="仅关停并清理已存在的测试集群, 不启动/发布/注入/更新",
    )
    args = parser.parse_args()

    work_dir = get_test_data_dir()
    print(f"Work directory (data & logs): {work_dir}")

    if args.only_stop:
        print("Only stopping existing cluster ...")
        cleanup(work_dir)
        print("Done.")
        return

    try:
        reproduced, procs = run_reproduce(work_dir)
    except Exception as e:
        print(f"[ERROR] 复现流程异常: {e}", file=sys.stderr)
        cleanup(work_dir)
        sys.exit(1)

    if reproduced:
        print("[已复现] logfile index != record.index")
    else:
        print(f"[未复现] 更新 {UPDATE_MAX} 次后未在从节点日志中发现目标告警")

    if args.keep_cluster:
        print("Cluster kept for manual inspection:")
        for i in range(1, NODE_CNT + 1):
            print(f"  node{i}: http 127.0.0.1:{node_http_port(i)}, "
                  f"log {node_log_path(work_dir, i).absolute()}")
    else:
        print("Stopping cluster and cleaning up ...")
        cleanup(work_dir, procs)
        print(f"Work directory cleaned: {work_dir}")


if __name__ == "__main__":
    main()
