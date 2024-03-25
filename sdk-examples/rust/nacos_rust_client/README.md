# nacos_rust_client 说明

## 介绍

rust实现的nacos客户端sdk。

nacos_rust_client是在写[r-nacos](https://github.com/r-nacos/r-nacos) 过程中实现的客户端，方便服务端与客户端都相互验证，目前已功能已比较稳定，推荐使用。

0.3.x版本开始同时支持nacos `1.x`版http协议和`2.x`版本协议，支持在创建client时指定使用协议类型。

0.2.x版本只支持nacos `1.x`版http协议.

0.3.x版本兼容0.2版本api，建议使用nacos_rust_client都升级到0.3.x版本。

特点：

1. 使用 actix + tokio 实现。
2. 支持配置中心的推送、获取、监听。
3. 支持注册中心的服务实例注册(自动维护心跳)、服务实例获取(自动监听缓存实例列表)。
4. 创建的客户端后台处理，都放在同一个actix环境线程; 高性能，不会有线程膨胀，稳定可控。


## 样例运行说明

1. 运行sdk样例前，需要先在本地运行rnacos服务。默认使用`127.0.0.1:8848`服务地址,如果不一致，需要调整sdk中的nacos服务地址。
2. cd到样例目录,通过`cargo run`即可运行对应样例应用。


