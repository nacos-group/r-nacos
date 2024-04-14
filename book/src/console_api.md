# 控制台接口

## 全局说明

### 公共对象

> 通用返回值

```json
{
    "success": boolean, //调用结果是否成功
    "data: object, //返回对像
    "code": string, //返回的错误码，在success为false时有值
    "message": string , //返回的错误信息,在success为false时有值
}
```

> 通用分页对像

```json
{
    "totalCount": int, //总记录数
    "list": [], //本次分页查询结果列表
}
```

对应的分页查询返回值为

```json
{
    "success": boolean, //调用结果是否成功
    "data: {
        "totalCount": int, //总记录数
        "list": [], //本次分页查询结果列表
    },
    "code": string, //返回的错误码，在success为false时有值
    "message": string , //返回的错误信息,在success为false时有值
}

```

### 特殊错误码定义


|错误码|错误信息|备注|
|--|--|--|
|SYSTEM_ERROR|系统错误|未明确的系统错误|
|NO_LOGIN|用户没有登陆|需要跳转到登陆页|
|NO_PERMISSION|用户没有权限||
|CAPTCHA_CHECK_ERROR|验证码校验不通过||
|LOGIN_LIMITE_ERROR|用户频繁密码校验不通过被限制登陆||

### 其它

重构后规范化的接口都放到`/rnacos/api/console/v2/`下

## 接口

### 1. 登陆模块

> 登陆接口

'''

'''


