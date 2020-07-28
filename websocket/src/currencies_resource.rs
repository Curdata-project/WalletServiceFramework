use serde::{Serialize, Deserialize};
use crate::error::WalletError;
use jsonrpc_core::Data;
use ewf_core::{Call, Event, Module, Transition, Bus, CallQuery};
use currencies::CurrencyEntity;
use currencies::Error as CurrenciesError;
use actix::prelude::*;
use serde_json::{json, Value};
use std::collections::hash_map::HashMap;
use crate::spawn_call_task;


#[derive(Serialize, Deserialize)]
pub struct CurrencyDetail {
    value: u64,
    id: String,
    dcds: String,
    locked: bool,
    owner: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetDetailParam {
    ids: Vec<String>,
}

pub async fn get_detail_by_ids(
    currencies_resource: Data<CurrencyResource>,
    req: GetDetailParam,
) -> Result<HashMap<String, Value>, WalletError> {

    let mut data = HashMap::new();

    for each in req.ids {
        let bus_addr = currencies_resource.get_ref().addr.clone();
        let currencies = bus_addr.send(CallQuery{module: "currencies".to_string()}).await??;

        let each_clone = each.clone();
        let ans = spawn_call_task(async move{
            currencies.send(Call{
                addr: bus_addr.clone(),
                method: "find_currency_by_id".to_string(),
                args: json!(each_clone),
            })
        }).await.await.map_err(|_| WalletError::ServerModuleCallError)??;

        let currency_entity: Result<CurrencyEntity, CurrenciesError> = serde_json::from_value(ans).map_err(|_| WalletError::ModuleParamTypeError)?;
        data.insert(each.clone(), match currency_entity {
            Ok(CurrencyEntity::AvailEntity{id,
                currency,
                txid,
                update_time,
                last_owner_id}) => {
                    Value::Null
                },
            Ok(CurrencyEntity::LockEntity{id,
                transaction,
                txid,
                update_time,
                last_owner_id}) => {
                    Value::Null
                },
            Err(CurrenciesError::CurrencyByidNotFound) => Value::Null,
            Err(error) =>{
                log::error!("module currencies return expect error {:?}", error);
                return Err(WalletError::ExpectError);
            },
        });
    }

    Ok(data)
}

pub struct CurrencyResource{
    addr: Addr<Bus>
}

impl CurrencyResource {
    pub fn new(addr: Addr<Bus>) -> Self {
        Self {
            addr
        }
    }
}