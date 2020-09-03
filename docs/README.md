# Documentation

EWF采用JSONRPC over Websocket进行通讯，所有的网络通讯通过Websocket完成，具体在Websocket上运行的数据采用 [JSONRPC2.0](https://www.jsonrpc.org/specification) 作为数据传输的payload。利用Websocket，EWF可以对外进行事件推送等功能。

## Module

EWF由多个功能相对独立的模块组成，每个模块负责不同的功能。每一个模块可以对外暴露多个方法。每一个模块对外暴露的方法名规则为：`<模块名>.<方法名>`。每一个模块的功能是独立的，模块与模块间通过模块间过程调用（IMC）来进行数据传递，也就意味着EWF可以自由的替换任何一个模块的代码实现，从而适配不同的环境，增加EWF的功能。

EWF当前版本包含如下模块：

- [currencies](./modules/currencies.md) 货币管理
- [secret](./modules/secret.md) 密钥管理
- [user](./modules/user.md) 用户管理
- [history](./modules/history.md) 交易历史
- [transaction](./modules/transaction.md) 交易模块

