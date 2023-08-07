#!/bin/sh

set -o errexit

cargo build --release

app_name='rnacos'

kill() {
    if [ "$(uname)" = "Darwin" ]; then
        if pgrep -xq -- "${app_name}"; then
            pkill -f "${app_name}"
        fi
    else
        set +e # killall will error if finds no process to kill
        killall ${app_name}
        set -e
    fi
}

echo "Killing all running ${app_name}"
kill
sleep 1

test_dir='cluster_example'

echo "init cluster dir: $test_dir"
rm -rf $test_dir
mkdir -p $test_dir

start_cluster() {
    echo "start node:1"
    local env_file="$test_dir/env_01"
    cat > $env_file <<EOF
#file:env01
RNACOS_HTTP_PORT=8848
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9848
RNACOS_CONFIG_DB_DIR=cluster_example/db_01
RNACOS_RAFT_NODE_ID=1
RNACOS_RAFT_AUTO_INIT=true
EOF
    nohup ./target/release/${app_name}  -e $env_file  > "$test_dir/node_01.log" &
    sleep 1

    echo "start node:2"
    local env_file="$test_dir/env_02"
    cat > $env_file <<EOF
RNACOS_HTTP_PORT=8849
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9849
RNACOS_CONFIG_DB_DIR=cluster_example/db_02
RNACOS_RAFT_NODE_ID=2
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
EOF
    nohup ./target/release/${app_name}  -e $env_file  > "$test_dir/node_02.log" &
    sleep 1

    echo "start node:3"
    local env_file="$test_dir/env_03"
    cat > $env_file <<EOF
RNACOS_HTTP_PORT=8850
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9850
RNACOS_CONFIG_DB_DIR=cluster_example/db_03
RNACOS_RAFT_NODE_ID=3
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
EOF
    nohup ./target/release/${app_name}  -e $env_file  > "$test_dir/node_03.log" &
    sleep 1
}

query_node_metrics() {
    echo "\n the node1 raft metrics"
    curl "http://127.0.0.1:8848/nacos/v1/raft/metrics"

    echo "\n the node2 raft metrics"
    curl "http://127.0.0.1:8849/nacos/v1/raft/metrics"

    echo "\n the node3 raft metrics"
    curl "http://127.0.0.1:8850/nacos/v1/raft/metrics"
}

start_cluster

query_node_metrics

echo "\npublish config t001:contentTest to node 1"
curl -X POST 'http://127.0.0.1:8848/nacos/v1/cs/configs' -d 'dataId=t001&group=foo&content=contentTest'
sleep 1

echo "\nget config info t001 from node 1, value:"
curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=t001&group=foo'

echo "\nget config info t001 from node 2, value:"
curl 'http://127.0.0.1:8849/nacos/v1/cs/configs?dataId=t001&group=foo'

echo "\nget config info t001 from node 3, value:"
curl 'http://127.0.0.1:8850/nacos/v1/cs/configs?dataId=t001&group=foo'
sleep 1

echo "\npublish config t002:contentTest02 to node 2"
curl -X POST 'http://127.0.0.1:8849/nacos/v1/cs/configs' -d 'dataId=t002&group=foo&content=contentTest02'
sleep 1

echo "\nget config info t002 from node 1, value:"
curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=t002&group=foo'

echo "\nget config info t002 from node 2, value:"
curl 'http://127.0.0.1:8849/nacos/v1/cs/configs?dataId=t002&group=foo'

echo "\nget config info t002 from node 3, value:"
curl 'http://127.0.0.1:8850/nacos/v1/cs/configs?dataId=t002&group=foo'

query_node_metrics


restart_cluster() {

    kill 

    echo "\nrestart cluster"

    sleep 1

    start_cluster

    sleep 1

    echo "\ncluster restart metrics:"

    query_node_metrics

    echo "\n\nwait until the clusters restart (need 30 seconds)"

    # the async-raft clusters restart need 30 seconds

    sleep 32

    query_node_metrics

    echo "\n\nquery the contents before restart"

    echo "\nget config info t002 from node 1, value:"
    curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=t002&group=foo'

    echo "\nget config info t002 from node 2, value:"
    curl 'http://127.0.0.1:8849/nacos/v1/cs/configs?dataId=t002&group=foo'

    echo "\nget config info t002 from node 3, value:"
    curl 'http://127.0.0.1:8850/nacos/v1/cs/configs?dataId=t002&group=foo'


    echo "\n\npublish config t003:contentTest03 to node 3"
    curl -X POST 'http://127.0.0.1:8850/nacos/v1/cs/configs' -d 'dataId=t003&group=foo&content=contentTest03'
    sleep 1

    echo "\nget config info t003 from node 1, value:"
    curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=t003&group=foo'

    echo "\nget config info t003 from node 2, value:"
    curl 'http://127.0.0.1:8849/nacos/v1/cs/configs?dataId=t003&group=foo'

    echo "\nget config info t002 from node 3, value:"
    curl 'http://127.0.0.1:8850/nacos/v1/cs/configs?dataId=t003&group=foo'

    query_node_metrics

}

restart_cluster

kill

echo "\n==== end ===="
