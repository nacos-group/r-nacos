
# rnacos 压测工程

基于[goose](https://book.goose.rs/)写的压测工具，支持对rnacos服务接口做性能压测。

主要目地是为了确认rnacos性能基线，辅助性能优化，大的版本变更可以用此工具测试对比性能结果。

工具使用的是 nacos sdk 压测，所以也支持对java nacos server做压测。

支持比较对应场景下rnacos与nacos的性能。

## 压测内容

主要压测4个场景接口:

1. 配置中心设置
2. 配置中心查询
3. 注册中心注册
4. 注册中心查询

每个场景分别对http协议与grpc协议做测试。

## 压测命令


```shell
# 具体参数可以参考 goose book
cargo run --bin http_config_query --release --  --host http://127.0.0.1:8848 -r 10 -u 120 -t 60 --report-file report/report_config_06.html
```

待补充
