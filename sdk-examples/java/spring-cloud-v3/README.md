
# r-nacos spring-cloud-v3 说明

本样例基于spring-boot:3.2.8和spring-cloud:2023.0.3，支持jdk17版本。



## 使用方式

1. 启动r-nacos
2. 切换到spring-cloud-v3样例目录
3. 本地打包 `mvn package` (如果有问题可以用 `mvn install` )
4. 运行service: `java -jar demo-service/target/demo-service-1.0-SNAPSHOT.jar`
5. 运行api: `java -jar demo-api/target/demo-api-1.0-SNAPSHOT.jar`
6. 访问api: `curl "http://127.0.0.1:20885/hi?name=r-nacos"`

```sh
curl "http://127.0.0.1:20885/hi?name=r-nacos"

[springcloud-demo-service]: Hello, r-nacos
```



