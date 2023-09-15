
# rnacos

## 简介

rnacos是一个用rust实现的nacos服务。

rnacos是一个轻量、 快速、稳定、高性能的服务；包含注册中心、配置中心、web管理控制台功能，支持单机、集群部署。

rnacos设计上完全兼容最新版本nacos面向client sdk 的协议（包含1.x的http OpenApi，和2.x的grpc协议）, 支持使用nacos服务的应用平迁到 rnacos。

rnacos相较于java nacos来说，是一个提供相同功能，启动更快、占用系统资源更小、性能更高、运行更稳定的服务。

详细说明可以看 [rnacos book](https://heqingpan.github.io/rnacos/index.html)

## 开发原由

一方面自己学习 rust 后想，写个中间件实践rust网络并发编程。

另一方面，自己开发应用有用到 nacos，个人云服务部署一个nacos太重，本地开发测试开nacos也比较重。

本人能读java源码又能写rust，分析确认开发可行性后，决定先写一个最小功能集给自己用。

自己写完后用下来整体体验很顺畅，就想开源出来给有需要的人使用。

最开始只兼容v1.x sdk协议，没有配置控制台，后面补上v2.x协议和控制台发才放出来。

## 适用场景

1. 开发测试环境使用nacos，nacos服务可以换成rnacos。启动更快，秒启动。
2. 个人资源云服务部署的 nacos，可以考虑换成rnacos。资源占用率低: 包10M 出头，不依赖 JDK；运行时 cpu 小于0.5% ，小于5M（具体和实例有关）。
3. 使用非订制nacos服务 ，希望能提升服务性能与稳定性，可以考虑迁移到 rnacos。

## 快速开始

### 一、 安装运行 rnacos

【单机部署】

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
#stable是最新正式版本号，也可以指定镜像版本号，如： qingpan/rnacos:v0.2.1
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

测试、试用推荐使用第1、第2种方式，直接下载就可以使用。

在linux下第1、第2种方式默认是musl版本(性能比gnu版本差一些)，在生产服务对性能有要求的可以考虑使用第3、第4种在对应环境编译gnu版本部署。

启动配置可以参考： [运行参数说明](https://heqingpan.github.io/rnacos/deplay_env.html)

【集群部署】

集群部署参考： [集群部署](https://heqingpan.github.io/rnacos/cluster_deploy.html)


### 二、运行nacos 应用

服务启动后，即可运行原有的 nacos 应用。

### 配置中心http api例子

```
# 设置配置
curl -X POST 'http://127.0.0.1:8848/nacos/v1/cs/configs' -d 'dataId=t001&group=foo&content=contentTest'

# 查询
curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=t001&group=foo'

```

### 注册中心http api例子

```
# 注册服务实例
curl -X POST 'http://127.0.0.1:8848/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.11&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"001"}'

curl -X POST 'http://127.0.0.1:8848/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.12&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"002"}'

 curl -X POST 'http://127.0.0.1:8848/nacos/v1/ns/instance' -d 'port=8000&healthy=true&ip=192.168.1.13&weight=1.0&serviceName=nacos.test.001&groupName=foo&metadata={"app":"foo","id":"003"}'

# 查询服务实例

curl "http://127.0.0.1:8848/nacos/v1/ns/instance/list?&namespaceId=public&serviceName=foo%40%40nacos.test.001&groupName=foo&clusters=&healthyOnly=true"

```


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

4、命名空间管理

![](https://user-images.githubusercontent.com/1174480/268299574-4947b9f8-79e1-48e2-97fe-e9767e26ddc0.png)



## 功能说明

这里把 nacos 服务的功能分为三块
1、面向 SDK 的功能
2、面向控制台的功能
3、面向部署、集群的功能

每一块做一个对nacos服务的对比说明。

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

命名空间：

1. 支持管理命名空间列表
2. 支持切换命名空间查询配置、服务数据。

配置中心：

1. 支持配置中心信息管理
1. 支持配置导入、导出,其文件格式与nacos兼容
2. 支持配置历史记录查看与恢复
3. 暂不支持tag的高级查询
4. 暂不支持查询配置监听记录

服务中心：

1. 支持注册中心的服务、服务实例管理
2. 暂不支持查询监听记录

### 三、面向部署、集群的功能

1. 支持单机部署
2. 支持集群部署。集群部署配置中心数据使用raft+节点本地存储组成的分布式存储，不需要依赖mysql。具体参考 [集群部署说明](https://heqingpan.github.io/rnacos/cluster_deploy.html)


## 性能


|模块|场景|单节点qps|集群qps|总结|
|--|--|--|--|--|
|配置中心|配置写入,单机模式|1.5万|1.5万||
|配置中心|配置写入,集群模式|1.8千|1.5千|接入raft后没有充分优化,待优化,理论上可接近单机模式|
|配置中心|配置查询|8万|n*8万|集群的查询总qps是节点的倍数|
|注册中心|服务实例注册,http协议|1.2万|1.0万|注册中心单机模式与集群模式写入的性能一致|
|注册中心|服务实例注册,grpc协议|1.2万|1.2万|grpc协议压测工具没有支持，目前没有实际压测，理论不会比http协议低|
|注册中心|服务实例心跳,http协议|1.2万|1.0万|心跳是按实例计算和服务实例注册一致共享qps|
|注册中心|服务实例心跳,grpc协议|8万以上|n*8万|心跳是按请求链接计算,且不过注册中心处理线程,每个节点只需管理当前节点的心跳，集群总心跳qps是节点的倍数|
|注册中心|查询服务实例|3万|n*3万|集群的查询总qps是节点的倍数|

**注：** 具体结果和压测环境有关


### 性能压测总结

1. rnacos,除了服务服务注册不能稳定在1万以上，其它的接口qps都能稳定在1万以上。
2. java 的查询接口基本能压到1万以上，但不平稳，后继浮动比较大。如果降低压测流程，qps 可以相对平稳。
3. 在多服务查询叠加上多服务注册场景，rnacos  qps能稳定在1.2万左右, java nacos qps 下降明显在0.6万左右。
4. rnacos 综合 qps是 java版的2倍以上，因 java 有 GC，qps水位稳定性上 java较差（相同施压流量，qps 能从峰值1万能降到1百以下）。
5. rnacos 服务,线程数稳定在20个左右，cpu 用例率最大200%左右（相当用个2核），内存在80M 以下
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

1. [x] 配置中心数据支持sled + raft存储
2. [x] 注册中心支持集群
3. 其它
	1. 集群用户认证同步

## rnacos架构图

单实例：

![](https://github.com/heqingpan/rnacos/raw/master/doc/assets/imgs/rnacos_L2_0.1.4.svg)

前端应用因依赖nodejs,所以单独放到另一个项目 [rnacos-console-web](https://github.com/heqingpan/rnacos-console-web) ,再通过cargo 把打包好的前端资源引入到本项目,避免开发rust时还要依赖nodejs。


多实例的raft和distro分布式协议说明待补充
