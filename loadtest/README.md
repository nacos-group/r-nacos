
# rnacos 压测工程

基于[goose](https://book.goose.rs/)写的压测工具，支持对rnacos服务接口做性能压测。

主要目地是为了确认rnacos性能基线，辅助性能优化，大的版本变更可以用此工具测试对比性能结果。

工具使用的是 nacos sdk 压测，所以也支持对java nacos server做压测。

支持比较对应场景下rnacos与nacos的性能。

## 压测内容

主要压测4个场景接口:

1. 配置中心设置
   1. http_config_set
2. 配置中心查询
   1. http_config_query
3. 注册中心注册
   1. http_naming_beat http注册压测。
   2. grpc_naming_register  grpc注册。grpc注册后，只要链路不断开，实例会一直保持。这个非压测，可以配合做查询压测
4. 注册中心查询
   1. http_naming_query http 实例查询压测

计划每个场景分别对http协议与grpc协议做测试，目前只支持http。
goose不能直接支持grpc，grcp 压测待补充。

## 压测命令


```shell
# cargo run --bin $filename --release --  --host $host -r 10 -u $user_count -t time --report-file $out_report.html
# --host 服务地址
# -u 用户数量,代码中每个用户限流100 qps,想要压测的总qps除以100就是压测的用户数量
# -r 用户每秒增量数量
# -t 运行时间，不指定会一直运行
# --report-file 运行结束后生成的压测报告
# 其它参数可以参考 goose book
# http_config_set
cargo run --bin http_config_set --release --  --host http://127.0.0.1:8848 -r 10 -u 100 -t 60 --report-file report_config_set.html

# http_config_query
cargo run --bin http_config_query --release --  --host http://127.0.0.1:8848 -r 10 -u 100 -t 60 --report-file report_config_query.html

# http_naming_beat
cargo run --bin http_naming_beat --release --  --host http://127.0.0.1:8848 -r 10 -u 100 -t 60 --report-file report_naming_beat.html

# http_naming_query
cargo run --bin http_naming_query --release --  --host http://127.0.0.1:8848 -r 10 -u 100 -t 60 --report-file report_naming_query.html

# grpc register
cargo run --bin grpc_naming_register --release

```

