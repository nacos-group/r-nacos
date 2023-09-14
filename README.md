
# rnacos

## 简介

rnacos是一个用rust实现的nacos服务。

rnacos是一个轻量、 快速、稳定、高性能的服务；包含注册中心、配置中心、web管理控制台功能，支持单机、集群部署。

rnacos设计上完全兼容最新版本nacos面向client sdk 的协议（包含1.x的http OpenApi，和2.x的grpc协议）, 支持使用nacos服务的应用平迁到 rnacos。

详细说明可以看 [rnacos book](https://heqingpan.github.io/rnacos/index.html)

## 开发原由

一方面自己学习 rust 后想，写个中间件实践rust网络并发编程。

另一方面，自己开发应用有用到 nacos，个人云服务部署一个nacos太重，本地开发测试开nacos也比较重。

本人能读java源码又能写rust，分析确认开发可行性后，决定先写一个最小功能集给自己用。

自己写完后用下来整体体验很顺畅，就想开源出来给有需要的人使用。

最开始只兼容sdk 协议，没有配置控制台，后面补上控制台发才放出来。

## 适用场景

1. 开发测试环境使用nacos，nacos服务可以换成rnacos。启动更快，秒启动。
2. 个人资源云服务部署的 nacos，可以考虑换成rnacos。资源占用率低: 包10M 左右，不依赖 JDK；运行时 cpu 小于0.5% ，小于5M（具体和实例有关）。
3. 使用非订制nacos服务 ，希望能提升服务性能与稳定性，理论上都支持迁移到 rnacos。 
4. 目前 rnacos 只支持单机部署，其支持容量在一万个服务实例以上。在一万服务实例场景下压测，qps可以稳定在1.3万左右，内存稳定在50 M以下，cpu稳定在33%左右（和压测环境有关）。


## 快速开始

### 一、 安装运行 rnacos

#### 1) 单机部署

方式1：从 [github release](https://github.com/heqingpan/rnacos/releases) 下载对应系统的应用包，解压后即可运行。

linux 或 mac 

```shell
# 解压
tar -xvf rnacos-x86_64-apple-darwin.tar.gz
# 运行
./rnacos
```

windows 解压后直接运行 rnacos.exe 即可。

方式2:  通过docker 运行

```
docker pull qingpan/rnacos:stable
docker run --name mynacos -p 8848:8848 -p 9848:9848 -d qingpan/rnacos:stable
```

docker 的容器运行目录是 /io，会从这个目录读写配置文件

方式3：通过 cargo 编译安装

```
# 安装
cargo install rnacos
# 运行
rnacos
```

方式4: 下载源码编译运行

```
git clone https://github.com/heqingpan/rnacos.git
cd rnacos
cargo build --release
cargo run
```

推荐使用第1、第2种方式。

**运行参数**

rnacos 运行时支持的环境变量，如果不设置则按默认配置运行。

```
RNACOS_CONFIG_DB_FILE: 配置中心的本地数据库文件地址，默认为运行目录下的 config.db 【标记废弃，0.1.x老版本数据文件配置】
RNACOS_CONFIG_DB_DIR: 配置中心的本地数据库sled文件夹，默认为运行目录下的nacos_db ,会在系统运行时自动创建
RNACOS_HTTP_PORT: rnacos监听http端口，默认是8848
RNACOS_GRPC_PORT: rnacos监听的grpc端口，默认是 HTTP端口+1000
RNACOS_HTTP_WORKERS: http工作线程数，默认是cpu核数
```

也支持从运行目录下的.env读取环境变量
.env 配置格式如下：

```
RNACOS_CONFIG_DB_FILE=config.db
RNACOS_HTTP_PORT=8848
```

#### 2) 集群部署

集群部署由多个节点组成，单个节点的部署方式和单机部署基本一样，只需要额外增加集群的配置信息。

| 参数|内容描述|默认值|示例|
|--|--|--|--|
|RNACOS_RAFT_NODE_ID|节点id|1|1|
|RNACOS_RAFT_NODE_ADDR|节点地址,Ip:GrpcPort|127.0.0.1:GrpcPort|127.0.0.1:9848|
|RNACOS_RAFT_AUTO_INIT|是否当做主节点初始化,(只在每一次启动时生效)|节点1时默认为true\n节点非1时为false|true|
|RNACOS_RAFT_JOIN_ADDR|是否当做节点加入对应的主节点,LeaderIp:GrpcPort(只在每一次启动时生效)|空|127.0.0.1:9848|


##### 一个本地集群样例

1. 配置3个节点的配置信息

env01

```
#file:env01 , Initialize with the leader node role
RNACOS_HTTP_PORT=8848
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9848
RNACOS_CONFIG_DB_DIR=db01
RNACOS_RAFT_NODE_ID=1
RNACOS_RAFT_AUTO_INIT=true
```


env02:

```
#file:env02 , Initialize with the follower node role
RNACOS_HTTP_PORT=8849
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9849
RNACOS_CONFIG_DB_DIR=db02
RNACOS_RAFT_NODE_ID=2
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
```

env03:

```
#file:env03 , Initialize with the follower node role
RNACOS_HTTP_PORT=8850
RNACOS_RAFT_NODE_ADDR=127.0.0.1:9850
RNACOS_CONFIG_DB_DIR=db03
RNACOS_RAFT_NODE_ID=3
RNACOS_RAFT_JOIN_ADDR=127.0.0.1:9848
```


2. 分别依次运行3个节点

```sh
nohup ./rnacos -e env01 > n01.log &
sleep 1
nohup ./rnacos -e env02 > n02.log &
sleep 1
nohup ./rnacos -e env03 > n03.log &
sleep 1
```

可以查询集群的状态


```sh
curl "http://127.0.0.1:8848/nacos/v1/raft/metrics"
curl "http://127.0.0.1:8849/nacos/v1/raft/metrics"
curl "http://127.0.0.1:8850/nacos/v1/raft/metrics"
```

集群运行成功后，就可以开始提供集群服务（目前只支持配置中心，注册中心的集群功能计划开发中）。

具体的运行细节可参考 [test_cluster.sh
](https://github.com/heqingpan/rnacos/blob/master/test_cluster.sh)


### 二、运行nacos 应用

服务启动后，即可运行原有的 nacos 应用。

具体的用法参考 nacos.io 的用户指南。

[JAVA-SDK](https://nacos.io/zh-cn/docs/sdk.html)

[其它语言](https://nacos.io/zh-cn/docs/other-language.html)

[open-api](https://nacos.io/zh-cn/docs/open-api.html)

### 三、控制台管理

启动服务后可以在浏览器通过 `http://127.0.0.1:8848/` 访问rnacos控制台。

主要包含命名空间管理、配置管理、服务管理、服务实例管理。

1、配置管理

配置列表管理

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506155441.png)

新建、编辑配置

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506155545.png)

2、服务列表管理

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506155133.png)

3、服务实例管理

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506155158.png)


## 功能说明

这里把 nacos 服务的功能分为三块
1、面向 SDK 的功能
2、面向控制台的功能
3、面向部署、集群的功能

第一块做一个对nacos服务的对比说明。

### 一、面向 SDK 的功能

访问认证：

1. 有提供获取认证token的接口
2. 实际请求暂不支持认证，都算认证通过。

配置中心：

1. 支持配置中心的基础功能、支持维护配置历史记录
2. 兼容配置中心的SDK协议
3. 暂不支持灰度发布、暂不支持tag隔离

注册中心：

1. 支持注册中心的基础功能
2. 兼容配置中心的SDK协议
3. 暂不支持1.x的 udp 实例变更实时通知，只支持 2.x 版本grpc实例变更实时通知 。最开始的版本也有支持过udp实例变更 通知，后面因支持 grpc 的两者不统一，就暂时去掉，后继可以考虑加回去。

### 二、面向控制台的功能

访问认证：
暂时不开启认证

配置中心：

1. 已支持配置导入、导出,其文件格式与nacos兼容
2. 暂不支持tag的高级查询
3. 暂不支持配置历史记录查看与恢复
4. 暂不支持查询配置监听记录

服务中心：

1. 暂不支持路由类型的设置
2. 暂不支持查询监听记录

### 三、面向部署、集群的功能

1. 目前只支持单机部署，后继考虑支持集群部署。
2. 配置中心的数据存放在本地sled中(0.1.x是sqlite,0.2.x是sled)，后继考虑支持raft支持集群同步。


## 性能


### 压测环境与工具

压测环境:macos i7四核 /16G  ， 施压、受压机器是同一台机器（会拉低压测结果）。
压测工具: 
	* wrk ,qps: 24450左右
	* goose, qps 17000左右 （单进程加限流施压比 wrk低） 
	* 单进程施压请求wrk比goose 输出高

rnacos server版本：v0.1.1 
java nacos server版本: 2.1.0

**因wrk,goose暂时不支持grpc协议，暂时只压测http协议接口**


### 配置中心

配置中心，不会频繁更新，写入不做压测。

#### rust rnacos server：

1. 配置中心单机查询 wrk 压测 qps 在2.4万左右.

#### java nacos server：

1. 配置中心单机查询 wrk 压测, qps 在7700左右



### 注册中心

#### rust rnacos server：

2. naming 注册1000 x 1个实例，每秒200qps，单核cpu: 4.5% 左右
3. naming 单查询1.5万 QPS 左右
	1. wrk  查询单个服务 ，1.65万 qps 
	2. goose 查询1000个服务 ，1.5万 qps 
4. naming 单注册服务
	1. goose,5万到7万实例数  0.7万 qps左右。
4. 查询与注册混合
	1. wrk 查询单个服务（1.5万 qps) + goose 注册（0.075 万qps) 【5千实例】
	2. goose 查询1000个服务（1.3万 qps) + goose 注册（0.07万 qps) 【5千实例】
	3. wrk 查询单个服务（1.5万 qps) + goose 注册（0.15万qps) 【1万实例】
	4. goose 查询1000个服务（1.3万 qps) + goose 注册（0.13万 qps) 【1万实例】

#### java nacos server：

1. 配置中心查询 wrk 压测, 7700 qps 左右
2. naming 注册1000 x 1个实例，每秒200qps，单核cpu: 17% 左右
3. naming 单查询
	1. wrk 查询单个服务 ，1.35万 qps 。
	2. goose 查询1000个服务，1万 qps（前期应该还能上去一些）。前30秒能稳定在1万左右，30秒后，跌到200左右之后再上下浮动，可能受 GC 影响。
4. naming 单注册
	1. goose,5万到7万实例数  0.45万 qps左右。
5. 查询与注册混合
	1. wrk 查询单个服务（1.3万 qps) + goose 注册（0.07 万qps) 【5千实例】
	2. goose 查询1000个服务（1万 qps) + goose 注册（0.07万 qps) 【5千实例】;  前期能保持，后期 qps 上下浮动比较大，最低小于50。
	3.  wrk 查询单个服务（0.9万 qps) + goose 注册（0.12万qps) 【1万实例】
	4. goose 查询1000个服务（0.6万 qps) + goose 注册（0.08万 qps) 【1万实例】

### 性能压测总结

1. rnacos,除了服务服务注册不能稳定在1万以上，其它的接口qps都能稳定在1万以上。

2. java 的查询接口基本能压到1万以上，但不平稳，后继浮动比较大。如果降低压测流程，qps 可以相对平稳。
3. 在多服务查询叠加上多服务注册场景，rnacos  qps能稳定在1.3万左右, java nacos qps 下降明显在0.6万左右。
4. rnacos 综合 qps是 java版的2倍以上，因 java 有 GC，qps水位稳定性上 java较差（相同施压流量，qps 能从峰值1万能降到1百以下）。
5. rnacos 服务,线程数稳定在7，cpu 用例率最大200%左右（相当用个2核），内存在50M 以下
6. java nacos 服务，线程数最大300左右， cpu 用例率最大500%左右，内存600M到900M。


## 后继计划

### 一、 对单机功能补全

1. 配置中心
	1.  [x] 控制台支持导出
	2.  [x] 查询配置历史变更记录
	3.  [x] 支持历史记录回滚。
	4.  [x] 回滚时支持配置内容比较。
	5.  [ ] 支持恢复发布，与 tag配置隔离 (优先级暂定晚于支持集群部署功能)
	6.  [ ] 支持查询服务监听列表
2. 注册中心
	7.  [ ] 支持服务路由类型的设置(这部分涉及插件,暂定不支持)
	8.  [ ] 支持查询服务监听列表

### 二、支持集群部署

1. 配置中心数据支持sled + raft存储
2. 注册中心支持集群
	1. 写路由
	2. 集群间的数据同步
3. 其它
	1. 集群用户认证同步

## rnacos架构图

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/rnacos_L2_0.1.4.svg)

前端应用因依赖nodejs,所以单独放到另一个项目 [rnacos-console-web](https://github.com/heqingpan/rnacos-console-web) ,再通过cargo 把打包好的前端资源引入到本项目,避免开发rust时还要依赖nodejs。
