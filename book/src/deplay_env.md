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
|RNACOS_INIT_ADMIN_USERNAME|初始化管理员用户名，只在主节点第一次启动时生效|admin|rnacos|0.5.11|
|RNACOS_INIT_ADMIN_PASSWORD|初始化管理员密码，只在主节点第一次启动时生效|admin|rnacos123456|0.5.11|
|RNACOS_ENABLE_METRICS|是否开启监控指标功能|true|true|0.5.13|
|RNACOS_METRICS_LOG_INTERVAL_SECOND|监控指标采集打印到日志的间隔,单位秒,最小间隔为5秒|30|10|0.5.13|
|RNACOS_CONSOLE_ENABLE_CAPTCHA| 验证码的开关| true|true|0.5.14|
|RNACOS_MCP_HTTP_TIMEOUT_SECOND|MCP服务HTTP请求超时时间，单位为秒|30|60|0.7.3|


注：从v0.3.0开始，默认参数启动的节点会被当做只有一个节点，当前节点是主节点的集群部署。支持其它新增的从节点加入。

