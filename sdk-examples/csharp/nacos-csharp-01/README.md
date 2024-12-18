# nacos-csharp 说明

配置中心使用样例

## 使用方式

1. 启动r-nacos  
2. 运行应用, 在本项目目录运行: `dotnet run --urls 'http://*:5215'`
3. 访问应用接口:
    + 设置配置 `curl -i 'http://127.0.0.1:5215/api/config/set?key=abc'`
    + 获取配置 `curl -i 'http://127.0.0.1:5215/api/config/set?key=abc'`
    + 获取指定应用配置 `curl -i 'http://127.0.0.1:5215/api/config/get_config_name`
4. 验证动态修改配置能力，在r-nacos控制台中修改第3步对应的配置内容再查询配置内容信息会更新;



