# 集群部署

r-nacos 在v0.3.0版本后支持集群部署。

## 集群功能机制简介

集群部署的目标是通过多实例部署的方式，支持服务的水平扩容，支持部分节点异常后继续提供服务，提升稳定性。

### 配置中心

配置中心使用raft集群协议+本地存储，持久化数据，不需要再依赖 mysql 存储配置。其持久化机制类似`etcd`。

|请求方式|说明|性能|
|--|--|--|
|写入|只有主节点能写入，其它节点收到写入请求后转发到主节点写入|集群2千tps左右，有优化空间|
|读取|每个节点都能读取全量数据|单节点8万qps左右,集群总容量为n*8万|

### 注册中心

注册中心使用类distor协议，同步集群间的数据。

注册中心复用配置中心节点列表信息，两者协议是分别单独实现的。

|请求方式|说明|性能|
|--|--|--|
|写入|注册中心每个节点平等，按hash划分每个节点负责的内容；节点对负责的服务可写，否则转发到对应负责的节点处理。|集群1万tps左右|
|读取|每个节点都能读取全量数据|单节点3万qps左右,集群总容量为n*3万|

## 集群部署

集群部署和单机部署步骤一致，只是对应的运行参数不同，增加了集群节点的配置。

### 一、取得r-nacos安装包

安装包的获取方式与 [快速开始](./quick-started.md)一致

### 二、配置集群规则


集群部署相关的配置参数有四个:RNACOS_RAFT_NODE_ID,RNACOS_RAFT_NODE_ADDR,RNACOS_RAFT_AUTO_INIT,RNACOS_RAFT_JOIN_ADDR。

具体参数说明参考 [运行参数说明](./deplay_env.md) 

集群配置规则： 

1. 所有的集群节点都需要设置RNACOS_RAFT_NODE_ID,RNACOS_RAFT_NODE_ADDR ,其中不同节点的node_id和 node_addr不能相同；node_id为一个正整数，node_addr为`ip:grpc_port` 
2. 集群主节点： 初始设置RNACOS_RAFT_AUTO_INIT为true （如果节点为1，默认是 true,不用额外设置）。
3. 集群从节点： 初始设置RNACOS_RAFT_AUTO_INIT为false (节点非1,默认就是false,不用额外设置)；另外需要设置RNACOS_RAFT_JOIN_ADDR为当前主节点的地址，以方便启动时自动加入集群中。
4. 第2、3点只是为了初始化组建集群。集群运行起来之后，后继启动配置从raft db中加载。
5. 集群节点数量不要求，可以是1、2、3、4、... ； 不过raft协议只支持小于集群半数节点异常后继续提供写入服务(查询不影响)。例如：3个节点集群支持1个节点异常后提供写入服务，2个节点集群可以正常运行，不支持节点异常后提供服务。
6. 从节点可以在使用过程中按需加入。比如原来3个节点，可能在使用一段时间后增加2个节点扩容。
   
#### 实例：规划集群节点信息并编写对应的配置文件


按上面的配置规则，下面我们配置一个3节点集群例子。

初始化节信息

1. 主节点id为1，地址为127.0.0.1:9848
2. 第一个从节点id为2，地址为127.0.0.1:9849
3. 第二个从节点id为3，地址为127.0.0.1:9849

正式集群部署的log等级建议设置为`warn`,不打正常的请求日志，只打报警或异常日志，减少日志量。

**配置信息如下**

env01

```
#file:env01 , Initialize with the leader node role
RUST_LOG=warn
#RNACOS_INIT_ADMIN_USERNAME=admin
#RNACOS_INIT_ADMIN_PASSWORD=admin
RNACOS_HTTP_PORT=8848
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9848
RNACOS_CONFIG_DB_DIR=db01
RNACOS_RAFT_NODE_ID=1
RNACOS_RAFT_AUTO_INIT=true
```


env02:

```
#file:env02 , Initialize with the follower node role
RUST_LOG=warn
#RNACOS_INIT_ADMIN_USERNAME=admin
#RNACOS_INIT_ADMIN_PASSWORD=admin
RNACOS_HTTP_PORT=8849
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9849
RNACOS_CONFIG_DB_DIR=db02
RNACOS_RAFT_NODE_ID=2
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
```

env03:

```
#file:env03 , Initialize with the follower node role
RUST_LOG=warn
#RNACOS_INIT_ADMIN_USERNAME=admin
#RNACOS_INIT_ADMIN_PASSWORD=admin
RNACOS_HTTP_PORT=8850
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9850
RNACOS_CONFIG_DB_DIR=db03
RNACOS_RAFT_NODE_ID=3
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
```

**注：** 上面的地址是本机运行多实例的地址，实际使用时换成具体的服务ip和port即可。

### 三、启动集群

#### 第一次启动

分别运行三个节点，需要先运行主节点成功后再运行

先运行主节点

```sh
nohup ./rnacos -e env01 > n01.log &
```

主节点功能启动后，再运行从节点

```sh
nohup ./rnacos -e env02 > n02.log &
nohup ./rnacos -e env03 > n03.log &
```

实例过程中不同的节点需要在不同的服务器运行服务。

#### 集群重启

节点重启和第一次启动的配置和启动方式不变。

集群启动后，集群的节点信息已久化节点本地数据库中。
在节点重启时后直接从本地数据库加载集群节点的信息。这时就不需要读取需要加入的集群地址，RNACOS_RAFT_JOIN_ADDR不会再被使用(留在配置中也不影响)。

部分节点重启，在重启一个心跳时间(0.5秒)就会被重新加入集群。

全部节点重启， raft需要启动静默5秒+选举超时3秒后才重新选举主节点；10秒左右集群才提供配置写入服务。 期间配置查询，和注册中心的读写可以正常使用。


### 四、运行应用使用集群

集群服务启动后，即可运行原有的 nacos 应用。

#### 配置中心http api例子

```sh
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

```

#### 注册中心http api例子

```sh
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

```

如果在本地源码编译，可使用或参考[test_cluster.sh](https://github.com/heqingpan/rnacos/blob/master/test_cluster.sh) 创建、测试集群。


具体的用法参考 nacos.io 的用户指南。

[JAVA-SDK](https://nacos.io/zh-cn/docs/sdk.html)

[其它语言](https://nacos.io/zh-cn/docs/other-language.html)

[open-api](https://nacos.io/zh-cn/docs/open-api.html)


## 集群管理工具

### 通过控制台查看集群状态

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230915000345.png)

在控制台页面主要观注集群节点列表状态与raft主角点是否正常

### 通过接口查看集群状态

1. 查询指定节点的raft 集群状态

```sh
curl "http://127.0.0.1:8848/nacos/v1/raft/metrics"
# {"id":1,"state":"Leader","current_term":1,"last_log_index":10,"last_applied":10,"current_leader":1,"membership_config":{"members":[1,2,3],"members_after_consensus":null}}
# 主要关注 current_leader和members
```

2. 增加节点


```sh
curl -X POST "http://127.0.0.1:8848/nacos/v1/raft/add-learner" -H "Content-Type: application/json" -d '[2, "127.0.0.1:9849"]'
```

推荐通过启动新节点时设置`RNACOS_RAFT_JOIN_ADDR`加入集群。
如果配置时主节点不确定，可以启动后再调用主节点接口把新节点加入集群。

此接口也可以用于在集群运行期更新集群节点地址。

3. 更新集群节点列表


```sh
curl -X POST "http://127.0.0.1:8848/nacos/v1/raft/change-membership" -H "Content-Type: application/json" -d '[1, 2, 3]'
```

如果通过手动方式增加节点，需要调用本接口更新集群节点列表。

此接口可以用于对集群缩容，下线指定节点。



## 附录介绍

[rnacos实现raft和类distro协议，支持集群部署](https://www.cnblogs.com/shizioo/p/17710328.html)
