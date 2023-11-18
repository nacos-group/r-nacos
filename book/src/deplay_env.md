# 部署参数说明

同一个应用包需要支持不同场景，就需要支持设置自定义参数。

r-nacos 运行参数支持通过环境变量，或指定配置文件方式设置。 如果不设置则按默认参数运行。



## 设置运行参数的方式

### 通过环境变更设置参数

例子：

```sh
RNACOS_HTTP_PORT=8848 ./rnacos
```

这种方式在自定义少量参数时比较方便


### 通过指定配置文件方式设置参数

例子

```sh
# 从0.3.0版本开始支持 -e env_file 运行参数
./rnacos -e env_file
```

如果不指定文件时也会尝试从当前目录下.env文件加载配置参数

env_file内容的格式是

```
KEY1=VALUE1
KEY2=VALUE2
KEY3=VALUE3
```

rnacos 运行时支持的环境变量，如果不设置则按默认配置运行。


## 运行参数说明

| 参数KEY|内容描述|默认值|示例|开始支持的版本|
|--|--|--|--|--|
|RNACOS_HTTP_PORT|rnacos监听http端口|8848|8848|0.1.x|
|RNACOS_GRPC_PORT|rnacos监听grpc端口|默认是 HTTP端口+1000|9848|0.1.x|
|RNACOS_HTTP_WORKERS|http工作线程数|cpu核数|8|0.1.x|
|RNACOS_CONFIG_DB_FILE|配置中心的本地数据库文件地址【0.2.x后不在使用】|config.db|config.db|0.1.x|
|RNACOS_CONFIG_DB_DIR|配置中心的本地数据库sled文件夹, 会在系统运行时自动创建|nacos_db|nacos_db|0.2.x|
|RNACOS_RAFT_NODE_ID|节点id|1|1|0.3.0|
|RNACOS_RAFT_NODE_ADDR|节点地址Ip:GrpcPort,单节点运行时每次启动都会生效；多节点集群部署时，只取加入集群时配置的值|127.0.0.1:GrpcPort|127.0.0.1:9848|0.3.0|
|RNACOS_RAFT_AUTO_INIT|是否当做主节点初始化,(只在每一次启动时生效)|节点1时默认为true,节点非1时为false|true|0.3.0|
|RNACOS_RAFT_JOIN_ADDR|是否当做节点加入对应的主节点,LeaderIp:GrpcPort；只在第一次启动时生效|空|127.0.0.1:9848|0.3.0|
|RUST_LOG|日志等级:debug,info,warn,error;所有http,grpc请求都会打info日志,如果不观注可以设置为error减少日志量|info|error|0.3.0|


注：从v0.3.0开始，默认参数启动的节点会被当做只有一个节点，当前节点是主节点的集群部署。支持其它新增的从节点加入。

