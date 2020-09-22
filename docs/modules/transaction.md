# 连接交易模块

>  进行交易过程的接口

## API Reference

### 发起交易

RPC方法名：tx_send

向另一个用户发起交易

- uid: 用户uid
- oppo_peer_uid: 交易对方uid
- input: 收入
- output: 支出

> 成功返回交易流id

- txid: 交易流id
		由(ms时间戳, hex格式的随机8bit)构成

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"transaction.tx_send",
    "params":{
        "uid":"8cb5451bff19b83d2ee0b820526d5de84b45f2c9182fa89a56913b4b36244d92",
        "oppo_peer_uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a",
        "output":0,
        "input":10
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":{
        "txid":"160041986642841c981a4e959b"
    }
}
```


### 接收交易 内部函数

RPC方法名：on_connect（内部接口）

收到交易发起连接
	拉起一个执行此次交易的状态机

- uid: 用户uid
- oppo_peer_uid: 交易对方uid
- txid: 对方指定的交易流id，同时也唯一标记了交易使用的连接通道

> 成功返回交易流id

- txid: 交易流id

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"transaction.tx_send",
    "params":{
        "uid":"8cb5451bff19b83d2ee0b820526d5de84b45f2c9182fa89a56913b4b36244d92",
        "oppo_peer_uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a",
        "output":0,
        "input":10
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":{
        "txid":"160041986642841c981a4e959b"
    }
}
```
