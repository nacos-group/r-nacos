
# r-nacos spring-cloud-v2 说明

本样例基于spring-boot:2.7.18和spring-cloud:2021.0.9，支持jdk1.8版本。



## 使用方式

1. 启动r-nacos
2. 切换到spring-cloud-v2样例目录
3. 本地打包 `mvn package` (如果有问题可以用 `mvn install` )
4. 运行service: `java -jar demo-service/target/demo-service-1.0-SNAPSHOT.jar`
5. 运行api: `java -jar demo-api/target/demo-api-1.0-SNAPSHOT.jar`
6. 访问api: `curl "http://127.0.0.1:20895/hi?name=r-nacos"`

```sh
curl "http://127.0.0.1:20895/hi?name=r-nacos"

[springcloud-demo-service]: Hello, r-nacos
```



