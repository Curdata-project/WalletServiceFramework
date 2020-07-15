use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type SerialType = Vec<u8>;

fn deserial_func<T>(serial_data: Vec<u8>) -> Result<T, ()>
where
    T: for<'de> Deserialize<'de>,
{
    bincode::deserialize(&serial_data).map_err(|_| ())
}

fn serial_func<T>(t: T) -> Result<Vec<u8>, ()>
where
    T: Serialize,
{
    bincode::serialize(&t).map_err(|_| ())
}

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

type TransitionHandlerType = Box<dyn Fn(SerialType) -> BoxFuture<Result<SerialType, SerialType>>>;

struct TransitionHandler {
    pub(crate) fn_call: TransitionHandlerType,
    pub(crate) i_type: TypeId,
    pub(crate) o_type: TypeId,
    pub(crate) e_type: TypeId,
}

unsafe impl Send for TransitionHandler {}

unsafe impl Sync for TransitionHandler {}

pub struct TranstionCaller {
    handlers: HashMap<String, BTreeMap<u16, TransitionHandler>>,
}

impl TranstionCaller {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn registe_event<I, O, E, F, FN>(mut self, key: String, priority: u16, fn_call: FN) -> Self
    where
        I: for<'de> Deserialize<'de> + 'static + Send,
        O: Serialize + 'static,
        E: Serialize + 'static,
        F: Future<Output = Result<O, E>> + 'static + Send,
        FN: Fn(I) -> F + 'static + Clone + Send + Sync,
    {
        let inner = move |intput_any: SerialType| -> Pin<
            Box<dyn Future<Output = Result<SerialType, SerialType>> + Send>,
        > {
            async fn inner_call<I, O, E, F, FN>(
                intput_any: SerialType,
                call: FN,
            ) -> Result<SerialType, SerialType>
            where
                I: for<'de> Deserialize<'de> + 'static + Send,
                O: Serialize + 'static,
                E: Serialize + 'static,
                F: Future<Output = Result<O, E>> + 'static + Send,
                FN: Fn(I) -> F + 'static + Clone + Send + Sync,
            {
                let input: I = deserial_func(intput_any).unwrap();
                let call_ = call.clone();
                match call_(input).await {
                    Ok(output) => Ok(serial_func(output).unwrap()),
                    Err(err) => Err(serial_func(err).unwrap()),
                }
            }

            let fn_call_ = fn_call.clone();
            Box::pin(inner_call(intput_any, fn_call_))
        };
        let handle = TransitionHandler {
            fn_call: Box::new(inner),
            i_type: TypeId::of::<I>(),
            o_type: TypeId::of::<O>(),
            e_type: TypeId::of::<E>(),
        };

        if let Some(mut bucket) = self.handlers.get_mut(&key) {
            bucket.insert(priority, handle);
        }
        else{
            let mut bucket = BTreeMap::new();
            bucket.insert(priority, handle);

            self.handlers.insert(key, bucket);
        }

        self
    }

    pub fn event_once<I, O, E>(&self, key: &str, data: I) -> BoxFuture<Result<O, E>>
    where
        I: Serialize + for<'de> Deserialize<'de> + 'static + Send + Sized,
        O: Serialize + for<'de> Deserialize<'de> + 'static + Send + Sized,
        E: Serialize + for<'de> Deserialize<'de> + 'static + Send + Sized,
    {
        let handle = self.handlers.get(key).unwrap().iter().next().unwrap().1;
        let call: &TransitionHandlerType = &handle.fn_call;

        let input = serial_func(data).unwrap();
        let middle_async = call(input);

        let inner =
            move |fn_call: BoxFuture<Result<SerialType, SerialType>>| -> BoxFuture<Result<O, E>> {
                async fn inner_call<O, E>(
                    fn_call: BoxFuture<Result<SerialType, SerialType>>,
                ) -> Result<O, E>
                where
                    O: Serialize + for<'de> Deserialize<'de> + 'static + Send + Sized,
                    E: Serialize + for<'de> Deserialize<'de> + 'static + Send + Sized,
                {
                    match fn_call.await {
                        Ok(output_any) => Ok(deserial_func::<O>(output_any).unwrap()),
                        Err(err_any) => Err(deserial_func::<E>(err_any).unwrap()),
                    }
                }

                Box::pin(inner_call(fn_call))
            };

        inner(middle_async)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tokio;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum ExampleError {
        // websock 错误
        ParamIsNone,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Detail {
        value: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct GetDetailParam {
        ids: Vec<String>,
    }

    async fn store_detail(param: GetDetailParam) -> Result<Detail, ExampleError> {
        if param.ids.len() == 0 {
            return Err(ExampleError::ParamIsNone);
        }
        Ok(Detail{ value: 99u64 })
    }

    #[test]
    fn test_transition_caller() {
        let mut caller = Arc::new(
            TranstionCaller::new().registe_event("store.detail".to_string(), 0, store_detail),
        );

        let task = async move {
            let ans: Result<Detail, ExampleError> = caller
                .event_once(
                    &"store.detail",
                    GetDetailParam {
                        ids: vec![],
                    },
                )
                .await;

            println!("{:?}", ans);

            let ans: Result<Detail, ExampleError> = caller
                .event_once(
                    &"store.detail",
                    GetDetailParam {
                        ids: vec!["id_001".to_string()],
                    },
                )
                .await;

            println!("{:?}", ans);
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(task);
    }
}
