
# r-nacos dubbo_v2.x 说明

本样例基于dubbo v2.7.23版本，支持jdk1.8版本。



## 使用方式

1. 启动r-nacos
2. 切换到dubbo_v2.x样例目录
3. 本地打包 `mvn package` (如果有问题可以用 `mvn install` )
4. 运行service: `java -jar dubbo-demo-service/target/dubbo-demo-service-1.0-SNAPSHOT.jar`
5. 运行api: `java -jar dubbo-demo-api/target/dubbo-demo-api-1.0-SNAPSHOT.jar`
6. 访问api: `curl "http://127.0.0.1:20795/hi?name=r-nacos"`

```sh
curl "http://127.0.0.1:20795/hi?name=r-nacos"
[dubbo-demo-service] : Hello, r-nacos
```



