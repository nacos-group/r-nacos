# 快速开始


## 一、 安装运行 rnacos

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

启动配置可以参考： [运行参数说明](./deplay_env.md)

## 二、运行nacos 应用

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


## 三、使用rnacos控制台

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


