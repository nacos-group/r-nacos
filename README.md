
# r-nacos

## 简介

r-nacos是一个用rust实现的nacos服务。

r-nacos是一个轻量、 快速、稳定、高性能的服务；包含注册中心、配置中心、web管理控制台功能，支持单机、集群部署。

r-nacos设计上完全兼容最新版本nacos面向client sdk 的协议（包含1.x的http OpenApi，和2.x的grpc协议）, 支持使用nacos服务的应用平迁到 r-nacos。

r-nacos相较于java nacos来说，是一个提供相同功能，启动更快、占用系统资源更小、性能更高、运行更稳定的服务。

详细说明可以看 [r-nacos docs](https://r-nacos.github.io/docs/)和[deepwiki](https://deepwiki.com/nacos-group/r-nacos/)

![Docker Pulls](https://img.shields.io/docker/pulls/qingpan/rnacos)  
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/r-nacos/r-nacos/total)

## 适用场景

1. 开发测试环境使用nacos，nacos服务可以换成r-nacos。启动更快，秒启动。
2. 个人资源云服务部署的 nacos，可以考虑换成r-nacos。资源占用率低: 包10M 出头，不依赖 JDK；运行时 cpu 小于0.5% ，小于5M（具体和实例有关）。
3. 使用非订制nacos服务 ，希望能提升服务性能与稳定性，可以考虑迁移到 r-nacos。


## 演示

控制台演示地址： [https://www.bestreven.top/rnacos/](https://www.bestreven.top/rnacos/) 
(演示服务与网址由一位热心用户提供）

用户名: `dev` ,密码: `dev`

演示内容：

+ 配置中心：接近5千个配置
+ 服务中心：30个服务，每个服务有15个实例，共450个服务实例。

*注：* 以上演示内容，服务使用的内存在15M左右

## 快速开始

### 一、 安装运行 r-nacos

【单机部署】

#### 方式1：下载二进制包运行

从 [github release](https://github.com/r-nacos/r-nacos/releases) 下载对应系统的应用包，解压后即可运行。

linux 或 MacOS

```shell
# 解压
tar -xvf rnacos-x86_64-apple-darwin.tar.gz
# 运行
./rnacos
```

windows 解压后直接运行 rnacos.exe 即可。

#### 方式2: 通过docker 运行


```
#stable是最新正式版本号，也可以指定镜像版本号，如： qingpan/rnacos:v0.4.0
docker pull qingpan/rnacos:stable  
docker run --name mynacos -p 8848:8848 -p 9848:9848 -p 10848:10848 -d qingpan/rnacos:stable
```

docker 的容器运行目录是 /io，会从这个目录读写配置文件

##### docker 版本说明

应用每次打包都会同时打对应版本的docker包 ，qingpan/rnacos:$tag 。

每个版本会打两类docker包

|docker包类型|tag 格式| 示例 |说明 |
|--|--|--|--|
|gnu debian包|$version| qingpan/rnacos:v0.4.0 | docker包基于debian-slim,体积比较大(压缩包36M,解压后102M),运行性能相对较高;|
|musl alpine包|$version-alpine| qingpan/rnacos:v0.4.0-alpine | docker包基于alpine,体积比较小(压缩包11M,解压后34M),运行性能相对较低;|


如果不观注版本，可以使用最新正式版本tag: 

+ 最新的gnu正式版本: `qingpan/rnacos:stable`
+ 最新的alpine正式版本: `qingpan/rnacos:stable-alpine`


#### 方式3: 通过docker-compose 运行

单机部署样列:

[docker-compose.yaml](https://github.com/nacos-group/r-nacos/blob/master/docker/docker-compose/r-nacos-simple/docker-compose.yaml)

```yaml
# 集群部署样例,数据目录: ./data
version: '3.8'

services:
  nacos:
    image: qingpan/rnacos:stable
    container_name: nacos
    ports:
      - "8848:8848"
      - "9848:9848"
      - "10848:10848"
    volumes:
      - ./data:/io:rw
    environment:
      - RNACOS_INIT_ADMIN_USERNAME=admin
      - RNACOS_INIT_ADMIN_PASSWORD=admin
      - RNACOS_HTTP_PORT=8848
    restart: always

```

集群部署样列: [docker-compose.yaml](https://github.com/nacos-group/r-nacos/blob/master/docker/docker-compose/r-nacos-cluster/docker-compose.yaml)

#### 方式4：通过 cargo 编译安装

```
# 安装
cargo install rnacos
# 运行
rnacos
```

#### 方式5: 下载源码编译运行

```
git clone https://github.com/r-nacos/r-nacos.git
cd r-nacos
cargo build --release
cargo run --release
```

#### 方式6: MacOS支持通过brew安装

```shell
# 把r-nacos加入taps
brew tap r-nacos/r-nacos 

# brew 安装 r-nacos
brew install r-nacos

# 运行
rnacos

# 后续可以直接通过以下命令更新到最新版本
# brew upgrade r-nacos 
```

#### 方式7: 部署到k8s

k8s支持使用 [helm](https://github.com/nacos-group/r-nacos/tree/master/deploy/k8s/helm) 部署。


测试、试用推荐使用第1、第2、第3种方式，直接下载运行就可以使用。

在linux下第1种方式默认是musl版本(性能比gnu版本差一些)，在生产服务对性能有要求的可以考虑使用第2、第3、第4、第5种在对应环境编译gnu版本部署。

#### 启动配置: 

| 参数KEY|内容描述|默认值|示例|开始支持的版本|
|--|--|--|--|--|
|RNACOS_HTTP_PORT|r-nacos监听http端口|8848|8848|0.1.x|
|RNACOS_GRPC_PORT|r-nacos监听grpc端口|默认是 HTTP端口+1000|9848|0.1.x|
|RNACOS_HTTP_CONSOLE_PORT|r-nacos独立控制台端口|默认是 HTTP端口+2000;设置为0可不开启独立控制台|10848|0.4.x|
|RNACOS_CONSOLE_LOGIN_ONE_HOUR_LIMIT|r-nacos控制台登录1小时失败次数限制|默认是5,一个用户连续登陆失败5次，会被锁定1个小时|5|0.4.x|
|RNACOS_HTTP_WORKERS|http工作线程数|cpu核数|8|0.1.x|
|RNACOS_CONFIG_DB_FILE|配置中心的本地数据库文件地址【0.2.x后不在使用】|config.db|config.db|0.1.x|
|RNACOS_CONFIG_DB_DIR|配置中心的本地数据库文件夹, 会在系统运行时自动创建【因语义原因，v0.6.x后推荐使用RNACOS_DATA_DIR】|nacos_db|nacos_db|0.2.x|
|RNACOS_DATA_DIR|本地数据库文件夹, 会在系统运行时自动创建【与RNACOS_CONFIG_DB_DIR等价，用于替代RNACOS_CONFIG_DB_DIR】|linux,MacOS默认为~/.local/share/r-nacos/nacos_db;windows,docker默认为nacos_db|nacos_db|0.6.x|
|RNACOS_RAFT_NODE_ID|节点id|1|1|0.3.0|
|RNACOS_RAFT_NODE_ADDR|节点地址Ip:GrpcPort,单节点运行时每次启动都会生效；多节点集群部署时，只取加入集群时配置的值|127.0.0.1:GrpcPort|127.0.0.1:9848|0.3.0|
|RNACOS_RAFT_AUTO_INIT|是否当做主节点初始化,(只在每一次启动时生效)|节点1时默认为true,节点非1时为false|true|0.3.0|
|RNACOS_RAFT_JOIN_ADDR|是否当做节点加入对应的主节点,LeaderIp:GrpcPort；只在第一次启动时生效|空|127.0.0.1:9848|0.3.0|
|RNACOS_RAFT_SNAPSHOT_LOG_SIZE|raft打包snapshot镜像的日志数量;即变更日志超过这个值则会触发一次打包镜像|默认值10000|10000|0.5.0|
|RUST_LOG|日志等级:debug,info,warn,error;所有http,grpc请求都会打info日志,如果不观注可以设置为error减少日志量|info|error|0.3.0|
|RNACOS_ENABLE_NO_AUTH_CONSOLE|是否开启无鉴权控制台|false|false|0.5.2|
|RNACOS_CONSOLE_LOGIN_TIMEOUT|控制台登陆有效时长(单位为秒)|一天,86400秒|86400|0.5.0|
|RNACOS_GMT_OFFSET_HOURS|日志时间的时区，单位小时；默认为本机时区，运行在docker时需要指定|local|8(东8区),-5(西5区)|0.5.7|
|RNACOS_ENABLE_OPEN_API_AUTH|是否对openapi开启鉴权；（注：nacos切换到r-nacos过程中不要开启鉴权）|false|true|0.5.8|
|RNACOS_API_LOGIN_TIMEOUT|open api鉴权有效时长，单位为秒；(注：从不鉴权到开启鉴权，需要间隔对应时长以保证客户端token能更新生效)|一小时,3600秒|3600|0.5.8|
|RNACOS_CLUSTER_TOKEN|集群间的通信请求校验token，空表示不开启校验，设置后只有相同token的节点间才可通讯|空字符串|1234567890abcdefg|0.5.8|
|RNACOS_BACKUP_TOKEN|数据备份接口请求校验token，空或长度小于32位表示不开启备份接口|空字符串|1234567890abcdefg1234567890abcdefg|0.6.6|
|RNACOS_INIT_ADMIN_USERNAME|初始化管理员用户名，只在主节点第一次启动时生效|admin|rnacos|0.5.11|
|RNACOS_INIT_ADMIN_PASSWORD|初始化管理员密码，只在主节点第一次启动时生效|admin|rnacos123456|0.5.11|
|RNACOS_ENABLE_METRICS|是否开启监控指标功能|true|true|0.5.13|
|RNACOS_METRICS_ENABLE_LOG|是否开启打印监控指标日志|false|false|0.5.21|
|RNACOS_METRICS_COLLECT_INTERVAL_SECOND|监控指标采集指标间隔,单位秒,最小间隔为1秒,不能小于RNACOS_METRICS_LOG_INTERVAL_SECOND|15|5|0.5.14|
|RNACOS_METRICS_LOG_INTERVAL_SECOND|监控指标采集打印到日志的间隔,单位秒,最小间隔为5秒|60|30|0.5.13|
|RNACOS_CONSOLE_ENABLE_CAPTCHA| 验证码的开关| true|true|0.5.14|

启动配置方式可以参考： [运行参数说明](https://r-nacos.github.io/docs/notes/env_config/)

【集群部署】

集群部署参考文档： 

+ [集群部署](https://r-nacos.github.io/docs/notes/cluster_deploy)
+ [集群部署样例](https://r-nacos.github.io/docs/notes/deploy_example/docker_cluster_deploy/)


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


具体的用法参考 [nacos.io](https://nacos.io/zh-cn/docs/sdk.html) 的用户指南。

### nacos client sdk

#### java nacos sdk

**nacos-client**

```xml
<dependency>
    <groupId>com.alibaba.nacos</groupId>
    <artifactId>nacos-client</artifactId>
    <version>${nacos.version}</version>
</dependency>
```

|协议|验证过版本|推荐版本|
|--|--|--|
|grpc协议(2.x)|2.1.0|>2.1.x|
|http协议(1.x)|1.4.1|>1.4.x|


#### go nacos sdk

**nacos-sdk-go**

```
nacos-sdk-go/v2 v2.2.5
```

|协议|验证过版本|推荐版本|
|--|--|--|
|grpc协议(2.x)|2.2.5|>=2.2.5|

#### rust nacos sdk

**nacos-sdk-rust**

```
nacos-sdk = "0.3.3"
```

|协议|验证过版本|推荐版本|
|--|--|--|
|grpc协议|0.3.3|>=0.3.3|


**nacos_rust_client**

```
nacos_rust_client = "0.3.0"
```

|协议|验证过版本|推荐版本|
|--|--|--|
|同时支持http协议与grpc协议|0.3.0|>=0.3.0|
|http协议|0.2.2|>=0.2.2|


[其它语言](https://nacos.io/zh-cn/docs/other-language.html)

[open-api](https://nacos.io/zh-cn/docs/open-api.html)

### 三、控制台管理

从0.4.0版本开始，支持独立端口号的新控制台。新控制台有完备的用户管理、登陆校验、权限控制，支持对外网暴露。

启动服务后可以在浏览器通过 `http://127.0.0.1:10848/rnacos/` 访问r-nacos新控制台。 

老控制台`http://127.0.0.1:8848/rnacos/` 标记废弃，默认不开启，可通过配置开启。老控制台不需要登陆鉴权、不支持用户管理。

控制台主要包含用户管理、命名空间管理、配置管理、服务管理、服务实例管理。

> 控制台线上演示

地址： [https://www.bestreven.top/rnacos/](https://www.bestreven.top/rnacos/) 
(演示服务与网址由一位热心用户提供）

演示用户：

+ 开发者:
    + 用户名: `dev` ,密码: `dev`
+ 访客:
    + 用户名: `guest`, 密码: `guest`

演示内容：

+ 配置中心：接近5千个配置
+ 服务中心：30个服务，每个服务有15个实例，共450个服务实例。




> 1、用户登陆

在新控制台打开一个地址，如果检测到没有登陆，会自动跳转到登陆页面。
一个用户连续登陆失败5次，会被锁定1个小时。这个次数可以通过启动参数配置。

<img style="width: 400px;" width="400" src="https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20231223220425.png" />

> 2、用户管理

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20231223222325.png)

系统会默认创建一个名为`admin`的用户，密码为`admin`(也可以通过环境变量 RNACOS_INIT_ADMIN_USERNAME 和 RNACOS_INIT_ADMIN_PASSWORD 修改默认账号的账户名和密码)。 

进去控制台后可按需管理用户。 

用户角色权限说明：

1. 管理员: 所有控制台权限
2. 开发者：除了用户管理的所有控制台权限
3. 访客：只能查询配置中心与注册中心的数据，没有编辑权限。


**注意：** 对外暴露的nacos控制台端口前，建议增加一个自定义管理员，把admin用户删除或禁用。


> 3、配置管理

配置列表管理

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20230506155441.png)

新建、编辑配置

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20230506155545.png)

> 4、服务列表管理

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20230506155133.png)

> 5、服务实例管理

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20230506155158.png)

> 6、命名空间管理

![](https://user-images.githubusercontent.com/1174480/268299574-4947b9f8-79e1-48e2-97fe-e9767e26ddc0.png)

> 7、系统监控

![](https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/20240722075241.png)



## 功能说明

这里把 nacos 服务的功能分为三块
1、面向 SDK 的功能
2、面向控制台的功能
3、面向部署、集群的功能

每一块做一个对nacos服务的对比说明。

### 一、面向 SDK 的功能

访问认证：

1. 有提供获取认证token的接口

配置中心：

1. 支持配置中心的基础功能、支持维护配置历史记录
2. 兼容配置中心的SDK协议
3. 暂不支持灰度发布、暂不支持tag隔离

注册中心：

1. 支持注册中心的基础功能
2. 兼容配置中心的SDK协议
3. 暂不支持1.x的 udp 实例变更实时通知，只支持 2.x 版本grpc实例变更实时通知 。最开始的版本也有支持过udp实例变更 通知，后面因支持 grpc 的两者不统一，就暂时去掉，后继可以考虑加回去。

### 二、面向开发、管理员的控制台的功能

控制台：
1. 支持使用独立的控制台端口提供对外网服务。

用户管理：

1. 支持管理用户列表
2. 支持用户角色权限管理
3. 支持用户密码重置

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
2. 支持集群部署。集群部署配置中心数据使用raft+节点本地存储组成的分布式存储，不需要依赖mysql。具体参考 [集群部署说明](https://r-nacos.github.io/docs/notes/deploy_example/docker_cluster_deploy/)


## 性能


|模块|场景|单节点qps/tps|集群qps/tps|总结/备注|
|--|--|--|--|--|
|配置中心|配置写入,http协议|1.76万|7.6千|集群写入压测是在同一台电脑运行3个节点,如果换成多个机器部署,tps应该还能有所提升。|
|配置中心|配置查询,http协议|8万|n*8万|集群的查询总qps是节点的倍数|
|注册中心|服务实例注册,http协议|4.8万|2.4万|集群写入压测是在同一台电脑运行3个节点,如果换成多个机器部署,tps应该还能有所提升。|
|注册中心|服务实例注册,grpc协议|4.8万|2.4万|grpc协议压测工具没有支持，目前没有实际压测，理论不会比http协议低|
|注册中心|服务实例心跳,http协议|4.8万|2.4万|心跳是按实例计算和服务实例注册一致共享qps|
|注册中心|服务实例心跳,grpc协议|8万以上|n*8万|心跳是按请求链接计算,且不过注册中心处理线程,每个节点只需管理当前节点的心跳，集群总心跳qps是节点的倍数|
|注册中心|查询服务实例|5.4万|n*5.4万|集群的查询总qps是节点的倍数|

**注：** 具体结果和压测环境有关

详细信息可以参考
[性能与容量说明](https://r-nacos.github.io/docs/notes/performance/)


## r-nacos架构图


![架构图](https://raw.githubusercontent.com/r-nacos/r-nacos/master/doc/assets/imgs/r-nacos_L2_0.3.7.svg)

说明：

+ r-nacos默认支持集群部署，单机就相当于一个节点的集群，后续有需要可以按需加入新的节点；
+ 数据持久化使用raft协议分布式数据库(raft协议+节点文件存储),类似etcd; 
+ 只需对`RNACOS_CONFIG_DB_DIR:nacos_db`目录下的文件备份与恢复，即可实现数据的备份与恢复；
+ r-nacos控制台使用前后端分离架构；前端应用因依赖nodejs,所以单独放到另一个项目 [r-nacos-console-web](https://github.com/r-nacos/rnacos-console-web) ,再通过cargo 把打包好的前端资源引入到本项目,避免开发rust时还要依赖nodejs。

r-nacos架构设计参考： 

+ [架构](https://r-nacos.github.io/docs/notes/architecture/)
+ [deepwiki architecture](https://deepwiki.com/nacos-group/r-nacos/2-system-architecture)


## 使用登记

使用r-nacos的同学，欢迎在[登记地址](https://github.com/r-nacos/r-nacos/issues/32) 登记，登记只是为了方便产品推广。

## 联系方式

> R-NACOS微信沟通群：先加微信(添加好友请备注'r-nacos')，再拉进群。

<img style="width: 200px;" width="200" src="https://github.com/r-nacos/r-nacos/raw/master/doc/assets/imgs/wechat.jpg" alt="qingpan2014" />

