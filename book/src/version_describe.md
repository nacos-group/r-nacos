# 版本说明

## R-NACOS版本说明

|版本|对nacos兼容说明|版本特点说明|适用场景|
|--|--|--|--|
|v0.3.3以上|支持2.x新最版nacos面向sdk的协议|支持集群部署版本，使用1个节点的集群方式当做单机模式,配置中心raft 协议没有充分优化，配置中心集群写入tps在1.5千左右|对配置中心没有太高频写入需求的应用都推荐使用这个版本 (后面会优化配置写入性能)|
|v0.2.3 (不再推荐,新版bugfix暂定不再同步)|支持2.x新最版nacos面向sdk的协议|单机部署版本,配置中心数据库使用sled, 初始内存多占用25M左右,配置中心单机写入tps在1.5万以上|对配置中心有高频写入需求的应用可以考虑使用|
|v0.1.10 (不再推荐,新版bugfix暂定不再同步)|支持1.x服务接口;除了查询服务中心服务列表外，支持2.x大部分接口|单机部署版本,配置中心数据库使用sqlite，内存占用比较低，但配置写入tps不高，7百左右|不使用2.x的注册中心服务，对内存占用敏感的小应用可以考虑使用|

### docker 版本说明

应用每次打包都会同时打对应版本的docker包 ，qingpan/rnacos:$tag 。

每个版本会打两类docker包

|docker包类型|tag 格式| 示例 |说明 |
|--|--|--|--|
|gnu debian包|$version| qingpan/rnacos:v0.4.2 | docker包基于debian-slim,体积比较大(压缩包36M,解压后102M),运行性能相对较高|
|musl alpine包|$version-alpine| qingpan/rnacos:v0.4.2-alpine | docker包基于alpine,体积比较小(压缩包11M,解压后34M),运行性能相对较低|


如果不观注版本，可以使用最新正式版本tag。

+ 最新的gnu正式版本: `qingpan/rnacos:stable`
+ 最新的alpine正式版本: `qingpan/rnacos:stable-alpine`

**MacOS arm系统补充说明** ：目前MacOS arm系统运行`stable`镜像失败，可以先换成`stable-alpine`镜像。等后面解决arm `stable`镜像问题后再把这个注意事项去掉。


## nacos client sdk

### java nacos sdk

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


### go nacos sdk

**nacos-sdk-go**

```
nacos-sdk-go/v2 v2.2.5
```

|协议|验证过版本|推荐版本|
|--|--|--|
|grpc协议(2.x)|2.2.5|>=2.2.5|

### rust nacos sdk

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

