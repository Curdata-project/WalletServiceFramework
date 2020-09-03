# 货币管理模块

>  货币管理模块，负责管理EWF中存储的货币信息。

## API Reference

### 查询货币统计信息

RPC 方法名：query_currency_statistics

根据传入的UID查询指定用户的货币统计信息
-  has_avail 是否包含可用部分
-  has_lock 是否包含因交易锁定部分
-  has_wait_confirm 是否包含待交易见证部分
-  owner_uid 查询用户UID

返回的货币统计信息由包含value,num两个字段的array组成

请求示例：

```json
{
    "jsonrpc":"2.0",
    "method":"currencies.query_currency_statistics",
    "params":{
        "has_avail":true,
        "has_lock":false,
        "has_wait_confirm":false,
        "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d"
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":[
        {
            "num":1,
            "value":10
        }
    ]
}
```

### 查询货币详细信息

RPC 方法名：find_currency_by_id

根据传入的货币ID查询持有货币的详细信息

- 货币ID

返回货币详细信息，格式同query_currency_comb结果的单个元素

> 指定货币未发现

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.find_currency_by_id",
    "params":"343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":{
        "AvailEntity":{
            "currency":"...",
            "currency_str":"0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c",
            "id":"343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
            "last_owner_id":"shen",
            "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
            "txid":"zxzxc",
            "update_time":1597723771000,
            "value":10
        }
    }
}
```

错误示例：

```json
{
    "error":{
        "code":2002,
        "message":"指定货币未发现"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```

### 查询货币列表详细信息

RPC 方法名：query_currency_comb

根据传入的用户UID查询持有货币列表的详细信息

- uid 用户UID
- query_param
- page_items 每页条目数
- page_num 页数
- order_by 预留，暂不使用，填none
- is_asc_order  预留，暂不使用，填false

返回货币数组，每个数组元素存在如下字段：

- status: `"Avail"|"Lock"|"WaitConfirm"` 货币状态
- currency 货币结构体，是货币字符串解析后的json表示
- currency_str 货币字符串
- id 货币唯一ID
- last_owner_id 上一持有者
- owner_uid 当前持有者,当前一定是自己
- txid 上次关联交易ID
- update_time 时间戳,单位ms
- value 面值

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.query_currency_comb",
    "params":{
        "query_param":{
            "page_items":5,
            "page_num":1,
            "order_by":"none",
            "is_asc_order":false
        },
        "uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d"
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":[
        {
            "AvailEntity":{
                "currency":"...",
                "currency_str":"0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c",
                "id":"343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
                "last_owner_id":"shen",
                "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
                "txid":"zxzxc",
                "update_time":1597723771000,
                "value":10
            }
        }
    ]
}
```

### 挑选指定数目面值的货币

RPC 方法名：pick_specified_num_currency （内部接口）

根据传入信息挑选指定数目面值的货币，被选中的货币进入交易锁定

- owner_uid 用户UID
- items
- value 面值
- num 数量

被选中的货币信息

> 格式同query_currency_comb

可能存在的错误：

- 可用货币不足

- 取可用货币失败

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.pick_specified_num_currency",
    "params":{
        "items":[
            {
                "value":10,
                "num":1
            }
        ],
        "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d"
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":[
        {
            "AvailEntity":{
                "currency":"...",
                "currency_str":"0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c",
                "id":"343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98",
                "last_owner_id":"shen",
                "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
                "txid":"zxzxc",
                "update_time":1597723771000,
                "value":10
            }
        }
    ]
}
```
```json
{
    "error":{
        "code":2004,
        "message":"可用货币不足"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```
```json
{
    "error":{
        "code":2005,
        "message":"取可用货币失败"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```

### 解锁货币

RPC 方法名：unlock_currency （内部接口）

根据输入的货币ID列表，将货币从交易锁定状态解除

- ids: 包含货币ID的数组

成功返回

- 无

可能存在的错误：

- 解锁交易货币失败

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.unlock_currency",
    "params":{
        "ids":[
            "343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98"
        ]
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":null
}
```
```json
{
    "error":{
        "code":2006,
        "message":"解锁交易货币失败"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```

### 货币交易见证

RPC 方法名：confirm_currency

根据输入的货币ID列表，将货币从等待交易见证状态解除，新状态为可用状态

- owner_uid: 货币所有者, 
- currency_str: 货币字符串,

成功返回

- 无

可能存在的错误：

- 货币交易见证失败

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.confirm_currency",
    "params":{
        "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
        "currency_str":"0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c"
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":null
}
```
```json
{
    "error":{
        "code":2001,
        "message":"货币交易见证失败"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```

### 添加货币到存储

RPC 方法名：add_currency（内部接口）

添加输入的一张货币到钱包

- status: `"Avail"|"Lock"|"WaitConfirm"` 货币状态
- owner_uid 货币所有者
- currency_str 货币字符串
- transaction_str 交易字符串
- txid 关联交易ID
- last_owner_id 上一所有者

成功返回

- 无

可能存在的错误：

- 输入货币未通过校验

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.add_currency",
    "params":{
        "AvailEntity":{
            "owner_uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
            "currency_str":"0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c",
            "txid":"zxzxc",
            "last_owner_id":"shen"
        }
    },
    "id":3
}
```

响应示例：
```json
{"id":3,"jsonrpc":"2.0","result":null}
```
```json
{"error":{"code":2003,"message":"输入货币未通过校验"},"id":3,"jsonrpc":"2.0"}
```

### 充值货币到存储

> 此接口通过DCGS充值

RPC 方法名：deposit

充值一组可用货币到存储

- uid: 货币所有者,
- bank_num: 转出银行账号,
- amount: 金额,
- currencys: 货币字符串数组,

成功返回

- 无

可能存在的错误：

- 输入货币未通过校验

请求示例：

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.deposit",
    "params":{
        "uid":"e4d6b948e1efe0c8cff6d0cdc9aa9f7c6e54150b08d51f4039ad498cc3e5509d",
        "bank_num":"1234567890",
        "amount":10,
        "currencys":[
            "0303534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e70f0af45c6106766d5c983f3942747b779406c328aede49ae4d98f6790287b9ed04e53bdde5d226a8635d0151d370fd7ba9901fccf994076946673883b273f330203534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e9fe439d0e7d635d009387b60c320780ef303c61edf613222465b1f4f86805c42a63cdb945e2845f7eaec1e5ff8dda8115852875816dc4d33bcc572607a6de3d4343372267f27b5a9b5519a86ed3efc3d7a4f2a4199a907dd1d92011e875f4b98c3b0b27e730100000a0000000000000003534a8cf8a3b0a3a31cba80c07ee6a5a1cf518b6b7588802787f13a55e32fc67e18ee7102187a30d9aff17ba95d6a7f7994e444a841fb5bc7f3698459089d8b3603659ae6afd520c54c48e58e96378b181acd4cd14a096150281696f641a145864c"
        ]
    },
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":null
}
```
```json
{
    "error":{
        "code":2003,
        "message":"输入货币未通过校验"
    },
    "id":3,
    "jsonrpc":"2.0"
}
```
