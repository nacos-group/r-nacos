
# rnacos

## 简介

rnacos是一个用rust实现的nacos服务。

rnacos是一个轻量、快速、稳定的服务，包含注册中心、配置中心、web管理控制台功能。

rnacos兼容nacos client sdk用到的协议（包含1.x的http OpenApi，和2.x的grpc协议），支持使用nacos服务的应用平迁到 rnacos。

## 开发原由

一方面自己学习 rust 后想，写个中间件实践rust网络并发编程。

另一方面，自己开发应用有用到 nacos，个人云服务部署一个nacos太重，本地开发测试开nacos也比较重。

本人能读java源码又能写rust，分析确认开发可行性后，决定先写一个最小功能集给自己用。

自己写完后用下来整体体验很顺畅，就想开源出来给有需要的人使用。

最开始只兼容sdk 协议，没有配置控制台，后面补上控制台发才放出来。

## 适用场景

1. 开发测试环境使用nacos，nacos服务可以换成rnacos。启动更快，秒启动。
2. 个人资源云服务部署的 nacos，可以考虑换成rnacos。资源占用率低: 包10M 左右，不依赖 JDK；运行时 cpu 小于0.5% ，小于5M（具体和实例有关）。
3. 其它非集群部署的 nacos ，理论上都支持迁移到 rnacos。 
4. 目前 rnacos 只支持单机部署，其支持容量在一万个服务实例以上。在一万服务实例场景下压测，qps 可以稳定在1.2左右，内存稳定在50 M以下，cpu稳定在33%左右（和压测环境有关）。

## 快速开始

### 一、 安装运行 rnacos

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
docker pull qingpan/rnacos:latest
docker run --name mynacos -p 8848:8848 -p 9848:9848 -d qingpan/rnacos:latest
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
RNACOS_CONFIG_DB_FILE: 配置中心的本地数据库文件地址，默认为运行目录下的 config.db
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

1. 暂不支持配置的导入、导出，后继计划支持导入、导出，其格式兼容 nacos 的导出格式
2. 暂不支持tag 的高级查询
3. 暂不支持配置历史记录查看与恢复
4. 暂不支持查询配置监听记录

服务中心：

1. 暂不支持路由类型的设置
2. 暂不支持查询监听记录

### 三、面向部署、集群的功能

1. 目前只支持单机部署，后继考虑支持集群部署。
2. 配置中心的数据存放在本地 sqlite 中，后继考虑支持其它中心数据库。


## 性能

配置中心，不会频繁更新， qps能到2.4万，基本没有性能压力。
性能瓶颈主要在注册中心。

### 配置中心

配置中心单机 qps 支持在2.4万左右，可以参考下面的压测数据。

rnacos版本：v0.1.1
压测环境:macos i7四核 /16G  ， 施压、受压机器是同一台机器（会拉低压测结果）。
压测工具: 
	* wrk ,qps: 24450左右
	* goose, qps 17000左右 （单进程加限流施压比 wrk低） 
	* 单进程施压请求wrk比goose 输出高



### 注册中心

1. naming 注册1000 x 1个实例，每秒100qps，单核cpu: 6% 左右
3. naming 单查询1.5万 QPS 左右
	1. wrk 1.65万 qps 左右
	2. goose 1.5万 qps 左右
4. naming 单注册服务
	1. goose,5万到7万实例数，能稳定压到0.7万 qps左右；长时间超过上限会出现请求积压，qps慢慢降低。
4. 查询与注册混合 （两个进程同时压测）
	1. wrk 查询（1.5万 qps） + goose 注册（0.075 万qps） 【5千实例】
	2. goose 查询（1.3万 qps） + goose 注册（0.07万 qps） 【5千实例】
	3. 1. wrk 查询（1.5万 qps） + goose 注册（0.15万qps） 【1万实例】
	2. goose 查询（1.3万 qps） + goose 注册（0.13万 qps） 【1万实例】


压测环境： 施压、受压机器是同一台机器（可能会拉低压测结果）

rnacos版本：v0.1.1
压测环境:macos i7四核 /16G  。
压测工具: goose（用于处理动态压测数据） ，wrk（只做静态查询）

以下是前期一次通过 goose单进程 的混合压测记录，有图表可以参考。

在一万服务实例(1000个服务，每个服务10个实例）场景下对单个 rnacos服务做限流压测（110个用户，每个用户限流100) 。
qps 稳定在1.1万左右，95%的 rt 在13ms 内；服务进程内存稳定在50 M以内，服务进程cpu稳定在总体30%左右(施压进程 cpu在总体35%左右)


qps 稳定在1.1万左右

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506173839.png)

95%的 rt 在13ms 内（rt突刺和内部时间器检查有关,有优化空间，目前还可接受）

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506173946.png)

压测时的服务列表与实例

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506173200.png)

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/20230506173351.png)

