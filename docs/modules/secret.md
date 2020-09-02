# 密钥管理模块

>  简介

## API

### 生成密钥并完成注册-gen_and_register

> 随机生成密钥并在传入的注册服务url注册
    url: 注册服务url
    timeout: 超时时间，单位s
    info
        account: 账户名
        password: 密码

> 返回的用户信息
    uid: 用户UID
    account: 账户名
    cert: 证书
    last_tx_time: 上次关联交易时间

> 密钥对生成错误

> 服务器请求失败: ...

> 注册失败: ...

请求示例：
```json
{"jsonrpc": "2.0", "method": "secret.gen_and_register", "params": {"url": "http://ahb1r0-o-node.curdata.cn:8808/api/wallet", "timeout": 5, "info": {"account":"test", "password":"passwd"}}, "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":{"account":"test","cert":"02193A43D455207A7C4B5FC78C448725DFDA051DBE5C96A259B15E1DEE575DAE85","last_tx_time":0,"uid":"58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7"}}
```
```json
{"error":{"code":1001,"message":"密钥对生成错误"},"id":3,"jsonrpc":"2.0"}
```
```json
{"error":{"code":1003,"message":"服务器请求失败: ..."},"id":3,"jsonrpc":"2.0"}
```
```json
{"error":{"code":1004,"message":"注册失败: ..."},"id":3,"jsonrpc":"2.0"}
```


### 查询密钥列表信息-query_secret_comb 内部接口

> 查询密钥列表信息
    query_param
        page_items 每页条目数
        page_num 页数
        order_by 预留，暂不使用，填none
        is_asc_order  预留，暂不使用，填false

> 返回密钥列表信息，单个列表元素格式为
    uid: 用户UID
    secret_type: "sm2"
    keypair: 私钥结构体
    cert: 证书结构体

请求示例：
```json
{"jsonrpc": "2.0", "method": "secret.query_secret_comb", "params": {"page_items":5,"page_num":1,"order_by":"none","is_asc_order":false}, "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":[{"cert":...,"keypair":...,"secret_type":"sm2","uid":"58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7"}]}
```


### 查询用户密钥信息-get_secret 内部接口

> 根据传入的用户UID查询密钥信息
    用户UID

> 返回用户信息，格式为
    uid: 用户UID
    secret_type: "sm2"
    keypair: 私钥结构体
    cert: 证书结构体

> 未发现可用密钥对

请求示例：
```json
{"jsonrpc": "2.0", "method": "secret.get_secret", "params": "58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7", "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":{"cert":...,"keypair":...,"secret_type":"sm2","uid":"58f03cdc99d826c8710eaa937912cea0e388551cb4010da4ada97f1a0ce98ae7"}}
```
```json
{"error":{"code":1002,"message":"未发现可用密钥对"},"id":3,"jsonrpc":"2.0"}
```


### 签名交易结构体-sign_transaction 内部接口

> 根据传入的用户UID查询密钥信息
    uid: 用户UID,
    oppo_cert: 交易对方证书,
    datas: 交易货币列表,

> 返回用户信息，格式为
    datas: 交易体字符串列表

> 未发现可用密钥对

> 签名失败

请求示例：
```json
{"jsonrpc": "2.0", "method": "secret.sign_transaction", "params": {"uid":"a95e72eb0fc76263c04edd279609a8f7edb259ac04860bb8e881ad611edb701f","oppo_cert":"02193A43D455207A7C4B5FC78C448725DFDA051DBE5C96A259B15E1DEE575DAE85","datas":[{"cert":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E","msg_type":"DigitalCurrency","signature":{"r":"70F0AF45C6106766D5C983F3942747B779406C328AEDE49AE4D98F6790287B9E","s":"D04E53BDDE5D226A8635D0151D370FD7BA9901FCCF994076946673883B273F33"},"t_obj":{"quota_info":{"cert":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E","msg_type":"QuotaControlField","signature":{"r":"9FE439D0E7D635D009387B60C320780EF303C61EDF613222465B1F4F86805C42","s":"A63CDB945E2845F7EAEC1E5FF8DDA8115852875816DC4D33BCC572607A6DE3D4"},"t_obj":{"delivery_system":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E","id":"343372267F27B5A9B5519A86ED3EFC3D7A4F2A4199A907DD1D92011E875F4B98","timestamp":1595558506691,"trade_hash":"18EE7102187A30D9AFF17BA95D6A7F7994E444A841FB5BC7F3698459089D8B36","value":10}},"wallet_cert":"03659AE6AFD520C54C48E58E96378B181ACD4CD14A096150281696F641A145864C"}}]}, "id": 3}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":{"datas":["0603a9a9200528def6db0413363b39ed9bbb89bd1ed4a3ecacdd422ce175ef437ac6f15020d87c7bd7eaad41d6c11c03107f3b0e969a3c3792d1bc09de44c56a0269d123c7bfdd99f7f404e25513d83daff9c8222b61fac8a55a2d079391c4b2e4d80b1b528ee08d6317b6891b2079c23a47763465bfe1a1f52886d1cb62501a91a002193a43d455207a7c4b5fc78c448725dfda051dbe5c96a259b15e1dee575dae850303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c"]}}
```
```json
{"error":{"code":1002,"message":"未发现可用密钥对"},"id":3,"jsonrpc":"2.0"}
```
```json
{"error":{"code":1005,"message":"签名失败"},"id":3,"jsonrpc":"2.0"}
```

