
# nacos-spring-boot-config-example 说明

本样例基于 https://github.com/nacos-group/nacos-examples/tree/master/nacos-spring-boot-example/nacos-spring-boot-config-example  修改



## 使用方式

1. 启动r-nacos  
2. 在public命名空间下增加一个配置， 配置Id: `example`  配置组: `DEFAULT_GROUP` 配置内容：`useLocalCache=true`
3. 启动spring-boot应用，在本项目目录运行 `mvn spring-boot:run`
4. 访问应用接口 `curl "http://127.0.0.1:8080/config/get",`请求结果为`true`; 或者直接用浏览器打开`http://127.0.0.1:8080/config/get`
5. 验证动态修改配置能力，在r-nacos控制台中修改第2步配置内容为`useLocalCache=false`，在第4步请求的结果变更为`false`;



