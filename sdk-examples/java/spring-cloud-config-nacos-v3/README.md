
# r-nacos spring-cloud-config-nacos-v3 说明

本样例基于spring-cloud:2025.0.0和com.alibaba.cloud:2025.0.0.0-preview(nacos-client:3.0.3)，支持jdk17以上版本。


## 使用方式

1. 启动r-nacos
2. 切换到spring-cloud-config-nacos-v3样例目录
3. 运行`mvn spring-boot:run` (也可以先打包`mvn package`再运行生成的jar包)
4. 访问api: `curl "http://127.0.0.1:8083/config/get"`查询配置内容
5. 在控制台更新`example.properties`配置的内容后，重新访问api应该可能查询到最新的信息

```sh
curl "http://127.0.0.1:8083/config/get"

#output 
false
```




