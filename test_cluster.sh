#!/bin/sh

set -o errexit

action=$1

usage() {
    echo "cmd args invalid"
    echo "usage: $0 start | start_debug | restart | restart_debug  | kill | clean | test_naming"
    exit 2
}

app_name='rnacos'

test_dir='cluster_example'

#defuat path
app_path="./target/release/$app_name"

kill() {
    echo "Killing all running ${app_name}"
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

#kill
#sleep 1

clean_cluster_dir() {
    echo "init cluster dir: $test_dir"
    rm -rf $test_dir
    mkdir -p $test_dir
}

start_cluster() {
    echo "start node:1"
    local env_file="$test_dir/env_01"
    cat >$env_file <<EOF
#file:env01
#RUST_LOG=debug|info|warn|error, default is info
RNACOS_HTTP_PORT=8848
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9848
RNACOS_CONFIG_DB_DIR=cluster_example/db_01
RNACOS_RAFT_NODE_ID=1
RNACOS_RAFT_AUTO_INIT=true
RNACOS_ENABLE_NO_AUTH_CONSOLE=true
EOF
    nohup ${app_path} -e $env_file >"$test_dir/node_01.log" &
    sleep 1

    echo "start node:2"
    local env_file="$test_dir/env_02"
    cat >$env_file <<EOF
#file:env02
#RUST_LOG=debug|info|warn|error, default is info
RNACOS_HTTP_PORT=8849
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9849
RNACOS_CONFIG_DB_DIR=cluster_example/db_02
RNACOS_RAFT_NODE_ID=2
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
RNACOS_ENABLE_NO_AUTH_CONSOLE=true
EOF
    nohup ${app_path} -e $env_file >"$test_dir/node_02.log" &
    sleep 1

    echo "start node:3"
    local env_file="$test_dir/env_03"
    cat >$env_file <<EOF
#file:env03
#RUST_LOG=debug|info|warn|error, default is info
#RUST_LOG=warn
RNACOS_HTTP_PORT=8850
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9850
RNACOS_CONFIG_DB_DIR=cluster_example/db_03
RNACOS_RAFT_NODE_ID=3
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
RNACOS_ENABLE_NO_AUTH_CONSOLE=true
EOF
    nohup ${app_path} -e $env_file >"$test_dir/node_03.log" &
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

#start_cluster

#query_node_metrics

test_config_cluster() {

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
}

test_restart_config_cluster() {
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
}

test_naming_cluster() {
    echo "\nregister instance nacos.test.001 to node 1"
    curl -X POST 'http://127.0.0.1:8848/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.11&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"001"}'
    echo "\nregister instance nacos.test.001 to node 2"
    curl -X POST 'http://127.0.0.1:8849/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.12&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"002"}'
    echo "\nregister instance nacos.test.001 to node 3"
    curl -X POST 'http://127.0.0.1:8850/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.13&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"003"}'
    sleep 1
    echo "\n\nquery service instance nacos.test.001 from node 1, value:"
    curl "http://127.0.0.1:8848/nacos/v1/ns/instance/list?&namespaceId=public&serviceName=foo%40%40nacos.test.001&groupName=foo&clusters=&healthyOnly=true"
    echo "\n\nquery service instance nacos.test.001 from node 2, value:"
    curl "http://127.0.0.1:8849/nacos/v1/ns/instance/list?&namespaceId=public&serviceName=foo%40%40nacos.test.001&groupName=foo&clusters=&healthyOnly=true"
    echo "\n\nquery service instance nacos.test.001 from node 3, value:"
    curl "http://127.0.0.1:8850/nacos/v1/ns/instance/list?&namespaceId=public&serviceName=foo%40%40nacos.test.001&groupName=foo&clusters=&healthyOnly=true"
    echo "\n"
}

#query_node_metrics

restart_cluster() {

    kill

    echo "\nrestart cluster"

    sleep 1

    start_cluster

    sleep 1

    echo "\ncluster restart metrics:"

    query_node_metrics

    echo "\n\nwait until the clusters restart (need 5 seconds)"

    # the async-raft-ext all clusters restart need 5 seconds
    sleep 6
    query_node_metrics
}

#restart_cluster

#kill

start() {
    kill
    sleep 1
    cargo build --release
    app_path="./target/release/$app_name"
    clean_cluster_dir
    start_cluster
    query_node_metrics
    test_config_cluster
    test_naming_cluster
    query_node_metrics
}

start_debug() {
    kill
    sleep 1
    cargo build
    app_path="./target/debug/$app_name"
    clean_cluster_dir
    start_cluster
    query_node_metrics
    test_config_cluster
    test_naming_cluster
    query_node_metrics
}

restart() {
    cargo build --release
    app_path="./target/release/$app_name"
    restart_cluster
    test_restart_config_cluster
    test_naming_cluster
    query_node_metrics
}

restart_debug() {
    cargo build
    app_path="./target/debug/$app_name"
    restart_cluster
    test_restart_config_cluster
    test_naming_cluster
    query_node_metrics
}

main() {
    case $action in
    start)
        start
        ;;
    start_debug)
        start_debug
        ;;
    restart)
        restart
        ;;
    restart_debug)
        restart_debug
        ;;
    clean)
        kill
        sleep 1
        clean_cluster_dir
        ;;
    test_naming)
        test_naming_cluster
        ;;
    kill)
        kill
        ;;
    *)
        usage
        ;;
    esac
}
main
echo "\n==== end ===="
