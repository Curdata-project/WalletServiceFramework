# 货币管理模块

>  货币管理模块，负责管理EWF中存储的货币信息。

## API Reference

### 查询货币信息

RPC 方法名：query_currency_statistics

根据传入的UID查询指定用户的货币统计信息
-  has_avail 是否包含可用部分
-  has_lock 是否包含因交易锁定部分
-  owner_uid 查询用户UID

返回货币信息，每个数组元素存在如下字段：

- id 货币唯一ID
- amount 金额
- status 货币状态 `"Avail"|"Lock"` 可用|交易临时锁定

请求示例：

```json
{
    "jsonrpc":"2.0",
    "method":"currencies.query_currency_statistics",
    "params":{
        "has_avail":true,
        "has_lock":false,
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
            "amount":10000,
            "id":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
            "status":"Avail"
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
    "params":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
    "id":3
}
```

响应示例：
```json
{
    "id":3,
    "jsonrpc":"2.0",
    "result":{
        "amount":10000,
        "currency":{
            "cert":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E",
            "msg_type":"DigitalCurrency",
            "signature":"949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F",
            "t_obj":{
                "addition":[

                ],
                "amount":10000,
                "id":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
                "issue":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E",
                "owner":"03659AE6AFD520C54C48E58E96378B181ACD4CD14A096150281696F641A145864C",
                "script":[

                ]
            }
        },
        "currency_str":"0303534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F40000000000000003143413243363843343634424244434634354636463542314145444236464531303734424141383132363837334344334343374446303544303934383244354642000000000000003033363539414536414644353230433534433438453538453936333738423138314143443443443134413039363135303238313639364636343141313435383634431027000000000000420000000000000030333533344138434638413342304133413331434241383043303745453641354131434635313842364237353838383032373837463133413535453332464336374500000000000000000000000000000000",
        "id":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
        "last_owner_id":"shen",
        "owner_uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a",
        "status":"Avail",
        "txid":"ABCD123",
        "update_time":1600221568000
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

- status: `"Avail"|"Lock"|` 货币状态
- currency 货币结构体，是货币字符串解析后的json表示
- currency_str 货币字符串
- id 货币唯一ID
- last_owner_id 上一持有者
- owner_uid 当前持有者,当前一定是自己
- txid 上次关联交易ID
- update_time 时间戳,单位ms
- amount 金额

请求示例：
```json
{
    "jsonrpc":"2.0",
    "method":"currencies.query_currency_comb",
    "params":{
        "query_param":{
            "page_items":5,
            "page_num":1,
            "order_by":"uid",
            "is_asc_order":true
        },
        "uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a"
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
            "amount":10000,
            "currency":{
                "cert":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E",
                "msg_type":"DigitalCurrency",
                "signature":"949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F",
                "t_obj":{
                    "addition":[

                    ],
                    "amount":10000,
                    "id":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
                    "issue":"03534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E",
                    "owner":"03659AE6AFD520C54C48E58E96378B181ACD4CD14A096150281696F641A145864C",
                    "script":[

                    ]
                }
            },
            "currency_str":"0303534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F40000000000000003143413243363843343634424244434634354636463542314145444236464531303734424141383132363837334344334343374446303544303934383244354642000000000000003033363539414536414644353230433534433438453538453936333738423138314143443443443134413039363135303238313639364636343141313435383634431027000000000000420000000000000030333533344138434638413342304133413331434241383043303745453641354131434635313842364237353838383032373837463133413535453332464336374500000000000000000000000000000000",
            "id":"1CA2C68C464BBDCF45F6F5B1AEDB6FE1074BAA8126873CD3CC7DF05D09482D5F",
            "last_owner_id":"shen",
            "owner_uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a",
            "status":"Avail",
            "txid":"ABCD123",
            "update_time":1600221568000
        }
    ]
}
```

### 解锁货币

RPC 方法名：unlock_currency （内部接口）

根据输入的货币ID列表，将货币从交易锁定状态解除
货币在交易过程会短暂锁定

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

- owner_uid 货币所有者
- currency_str 货币字符串
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
        "owner_uid":"53c28cd429acf7e337f1557b3cf811cbf51aea25e000fd3509d6840fb6fb0b4a",
        "currency_str":"0303534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F40000000000000003143413243363843343634424244434634354636463542314145444236464531303734424141383132363837334344334343374446303544303934383244354642000000000000003033363539414536414644353230433534433438453538453936333738423138314143443443443134413039363135303238313639364636343141313435383634431027000000000000420000000000000030333533344138434638413342304133413331434241383043303745453641354131434635313842364237353838383032373837463133413535453332464336374500000000000000000000000000000000",
        "txid":"160041986642841c981a4e959b",
        "last_owner_id":"shen"
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
            "0303534A8CF8A3B0A3A31CBA80C07EE6A5A1CF518B6B7588802787F13A55E32FC67E949418FCA3E9148AF8D5D7F4575A51BEB00E715824728F2E7675BE48E624E45741C980D38C75ECC1B5FB87C68D4E504B899CA1F3BB9560454F871A315B4CAA9F40000000000000003143413243363843343634424244434634354636463542314145444236464531303734424141383132363837334344334343374446303544303934383244354642000000000000003033363539414536414644353230433534433438453538453936333738423138314143443443443134413039363135303238313639364636343141313435383634431027000000000000420000000000000030333533344138434638413342304133413331434241383043303745453641354131434635313842364237353838383032373837463133413535453332464336374500000000000000000000000000000000"
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
