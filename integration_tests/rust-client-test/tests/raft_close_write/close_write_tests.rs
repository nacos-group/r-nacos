//! close-write 端到端集成测试。
//!
//! 覆盖 `POST /nacos/v1/raft/close-write` 的完整行为链路：标记文件安全闸门、
//! 免 token 鉴权、FileStore apply 入口拦截、gRPC 入站复制拦截、禁写幂等性。
//!
//! 由于 raft 禁写是进程级单向破坏性操作（禁写 node1 后该节点在当前进程生命
//! 周期内不可恢复），会破坏其它依赖 node1 正常参与 raft 的集群测试，因此本
//! 测试标注 `#[ignore]`，由 `integration_tests/scripts/run_test.py` 在常规
//! 测试通过后单独串行执行：
//! `cargo test close_write -- --ignored --test-threads=1`。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use nacos_rust_client::client::config_client::{ConfigClient, ConfigKey};
use serde_json::Value;

use crate::common::{ClientFactory, Protocol, generate_test_group, wait_for};

const BASE_HTTP_PORT: u16 = 8848;
const NODE_IDS: [u64; 3] = [1, 2, 3];
const MARK_FILE: &str = "close_raft_mark";

fn node_http_port(node_id: u64) -> u16 {
    BASE_HTTP_PORT + (node_id as u16) - 1
}

fn node_http_base(node_id: u64) -> String {
    format!("http://127.0.0.1:{}", node_http_port(node_id))
}

/// 集群数据根目录：优先取 run_test.py 注入的 `RNACOS_TEST_WORK_DIR`，
/// 否则回退到相对 cargo test 工作目录（rust-client-test）的 `../test_data`。
fn work_dir() -> PathBuf {
    std::env::var("RNACOS_TEST_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../test_data"))
}

fn node_data_dir(node_id: u64) -> PathBuf {
    work_dir().join(format!("db{:02}", node_id))
}

fn mark_path(node_id: u64) -> PathBuf {
    node_data_dir(node_id).join(MARK_FILE)
}

fn ensure_mark(node_id: u64) {
    let p = mark_path(node_id);
    std::fs::create_dir_all(p.parent().expect("mark parent exists")).ok();
    std::fs::write(&p, b"").expect("create close_raft_mark");
}

fn remove_mark(node_id: u64) {
    let p = mark_path(node_id);
    if p.exists() {
        std::fs::remove_file(&p).ok();
    }
}

async fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build reqwest client")
}

/// 调用 close-write 接口（不带 token），返回 (HTTP 状态码, 响应体 JSON)。
async fn post_close_write(node_id: u64) -> (reqwest::StatusCode, Value) {
    let url = format!("{}/nacos/v1/raft/close-write", node_http_base(node_id));
    let resp = http_client()
        .await
        .post(&url)
        .send()
        .await
        .expect("post close-write should succeed at transport level");
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body)
}

/// 获取节点 raft metrics（`GET /nacos/v1/raft/metrics`）。
async fn get_metrics(node_id: u64) -> Value {
    let url = format!("{}/nacos/v1/raft/metrics", node_http_base(node_id));
    let resp = http_client()
        .await
        .get(&url)
        .send()
        .await
        .expect("get metrics should succeed");
    resp.json().await.unwrap_or(Value::Null)
}

fn metrics_u64(metrics: &Value, key: &str) -> u64 {
    metrics.get(key).and_then(|v| v.as_u64()).unwrap_or(0)
}

fn metrics_leader(metrics: &Value) -> Option<u64> {
    metrics.get("current_leader").and_then(|v| v.as_u64())
}

/// node1 (RaftMetrics.last_applied) 应用到状态机的最后日志索引。
async fn last_applied(node_id: u64) -> u64 {
    metrics_u64(&get_metrics(node_id).await, "last_applied")
}

/// node1 (RaftMetrics.last_log_index) 本地 raft 日志最后索引。
async fn last_log_index(node_id: u64) -> u64 {
    metrics_u64(&get_metrics(node_id).await, "last_log_index")
}

/// 取任一健康节点上观察到的集群 leader。
async fn current_leader() -> Option<u64> {
    for nid in NODE_IDS {
        let m = get_metrics(nid).await;
        if let Some(l) = metrics_leader(&m) {
            if l > 0 {
                return Some(l);
            }
        }
    }
    None
}

/// 等待一个非 `excluded` 的节点成为 leader（禁写 node1 后新 leader 应落在 node2/node3）。
async fn wait_for_leader_not(excluded: u64) -> u64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        for nid in [2u64, 3u64] {
            let m = get_metrics(nid).await;
            if let Some(l) = metrics_leader(&m) {
                if l != excluded && l > 0 {
                    return l;
                }
            }
        }
        if tokio::time::Instant::now() > deadline {
            panic!("no leader (other than {}) elected within 30s", excluded);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

fn config_client_for_node(cluster_index: usize) -> Arc<ConfigClient> {
    ClientFactory::new().create_config_client(Protocol::Http, true, Some(cluster_index))
}

fn config_client_for_leader(leader_id: u64) -> Arc<ConfigClient> {
    config_client_for_node((leader_id - 1) as usize)
}

async fn wait_cluster_ready() {
    let ready = wait_for(
        || async { current_leader().await.is_some() },
        Duration::from_secs(30),
    )
    .await;
    assert!(ready, "cluster should elect a leader within 30s");
    tokio::time::sleep(Duration::from_secs(1)).await;
}

/// close-write 端到端集成测试（串行覆盖 5 个用例）。
///
/// 标注 `#[ignore]` 以避免与常规集群测试并行：禁写 node1 是破坏性单向操作，
/// 需由 run_test.py 在常规测试之后单独触发。
#[tokio::test]
#[ignore]
async fn test_close_write_end_to_end() {
    wait_cluster_ready().await;

    println!("[case1] close-write rejected when mark absent, raft stays healthy");
    remove_mark(1);
    let before = last_applied(1).await;
    let leader = current_leader().await.expect("leader exists");
    let writer = config_client_for_leader(leader);
    let warm_key = ConfigKey::new(&format!("cw_warm_{}", unique_suffix()), &generate_test_group(), "");
    writer.set_config(&warm_key, "v1").await.expect("warm write ok");
    let grew = wait_for(
        || async { last_applied(1).await > before },
        Duration::from_secs(10),
    )
    .await;
    assert!(
        grew,
        "node1 last_applied should grow when raft healthy (before={}, after={})",
        before,
        last_applied(1).await
    );

    let (status, body) = post_close_write(1).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body.get("ok").and_then(|v| v.as_i64()), Some(0));
    let msg = body.get("msg").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        msg.contains(MARK_FILE),
        "msg should mention {}, got: {}",
        MARK_FILE,
        msg
    );

    let before2 = last_applied(1).await;
    let warm2 = ConfigKey::new(&format!("cw_warm2_{}", unique_suffix()), &generate_test_group(), "");
    writer.set_config(&warm2, "v1").await.expect("warm2 write ok");
    let still_growing = wait_for(
        || async { last_applied(1).await > before2 },
        Duration::from_secs(10),
    )
    .await;
    assert!(
        still_growing,
        "node1 last_applied should keep growing after rejected close-write"
    );

    println!("[case2] create mark then close-write succeeds, token-free (IGNORE_PATH)");
    ensure_mark(1);
    let (status, body) = post_close_write(1).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_ne!(status.as_u16(), 401);
    assert_ne!(status.as_u16(), 403);
    assert_eq!(body.get("ok").and_then(|v| v.as_i64()), Some(1));

    println!("[case3] apply blocked after close-write: node1 last_applied frozen");
    let frozen = last_applied(1).await;
    let leader = wait_for_leader_not(1).await;
    let leader_writer = config_client_for_leader(leader);
    let probe_group = generate_test_group();
    let probe_key = ConfigKey::new("close_write_probe", &probe_group, "");
    leader_writer
        .set_config(&probe_key, "probe_value")
        .await
        .expect("leader should accept write (majority without node1)");

    let probe_applied_on_leader = wait_for(
        || async {
            let m = get_metrics(leader).await;
            metrics_u64(&m, "last_applied") > metrics_u64(&get_metrics(1).await, "last_applied")
        },
        Duration::from_secs(10),
    )
    .await;
    assert!(
        probe_applied_on_leader,
        "leader should have applied the probe while node1 stays frozen"
    );

    tokio::time::sleep(Duration::from_secs(2)).await;
    let now = last_applied(1).await;
    assert_eq!(
        now, frozen,
        "node1 last_applied must not grow after close-write (apply entry blocked)"
    );

    let node1_reader = config_client_for_node(0);
    let res = node1_reader.get_config(&probe_key).await;
    assert!(
        res.is_err(),
        "node1 should NOT have probe config (apply blocked, state machine unchanged)"
    );

    println!("[case4] gRPC replication blocked: node1 last_log_index frozen");
    let log_frozen = last_log_index(1).await;
    let leader = wait_for_leader_not(1).await;
    let leader_writer = config_client_for_leader(leader);
    for i in 0..5u64 {
        let k = ConfigKey::new(
            &format!("close_write_repl_{}_{}", i, unique_suffix()),
            &probe_group,
            "",
        );
        leader_writer
            .set_config(&k, &format!("v{}", i))
            .await
            .expect("leader should accept continuous writes");
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
    let log_now = last_log_index(1).await;
    assert_eq!(
        log_now, log_frozen,
        "node1 last_log_index must not grow (gRPC append_entries blocked)"
    );
    let leader_log = last_log_index(leader).await;
    assert!(
        leader_log > log_now,
        "leader last_log_index ({}) should be ahead of node1 ({})",
        leader_log,
        log_now
    );

    println!("[case5] idempotent: repeat close-write still ok=1, state preserved");
    assert!(mark_path(1).exists(), "mark file still present");
    let frozen5 = last_applied(1).await;
    let (status, body) = post_close_write(1).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body.get("ok").and_then(|v| v.as_i64()), Some(1));
    tokio::time::sleep(Duration::from_secs(1)).await;
    let now5 = last_applied(1).await;
    assert_eq!(
        now5, frozen5,
        "node1 should remain in close-write state after repeat call"
    );
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let c = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", ts, c)
}
