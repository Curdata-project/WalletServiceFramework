# 用户管理模块

>  简介

## API

### 添加用户-add_user 内部接口

> 添加用户到存储
    uid: 用户ID
    cert: 注册证书
    last_tx_time: 上次交易时间
    account: 账户名

> 成功返回空

请求示例：
```json
{"jsonrpc": "2.0", "method": "user.add_user", "params": {"uid": "58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae9", "cert": "02193A43D455207A7C4B5FC78C448725DFDA051DBE5C96A259B15E1DEE575DAE85", "last_tx_time": 0, "account": "test"}, "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":null}
```


### 查询用户列表-query_user

> 查询用户列表
    用户ID

> 成功返回用户信息
    uid: 用户UID
    account: 账户名
    cert: 证书
    last_tx_time: 上次关联交易时间

> 未发现该用户

请求示例：
```json
{"jsonrpc": "2.0", "method": "user.query_user", "params": "58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae9", "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":{"account":"test","cert":"02193A43D455207A7C4B5FC78C448725DFDA051DBE5C96A259B15E1DEE575DAE85","last_tx_time":0,"uid":"58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7"}}
```
```json
{"error":{"code":3001,"message":"未发现该用户"},"id":3,"jsonrpc":"2.0"}
```


### 查询用户列表-query_user_comb

> 查询用户列表
    query_param
        page_items 每页条目数
        page_num 页数
        order_by 预留，暂不使用，填none
        is_asc_order  预留，暂不使用，填false

> 返回密钥列表信息，单个列表元素格式为
    uid: 用户UID
    account: 账户名
    cert: 证书
    last_tx_time: 上次关联交易时间

请求示例：
```json
{"jsonrpc": "2.0", "method": "user.query_user_comb", "params": {"page_items":5,"page_num":1,"order_by":"none","is_asc_order":false}, "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":[{"account":"test","cert":"02193A43D455207A7C4B5FC78C448725DFDA051DBE5C96A259B15E1DEE575DAE85","last_tx_time":0,"uid":"58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7"}]}
```