use crate::error::Error;
use crate::message::{Call, Event};
use async_trait::async_trait;
use serde_json::Value;
use actix::prelude::*;

#[async_trait]
pub trait Module {
    fn notify(&self, event: &Event) -> Result<(), Error>;

    async fn call(&self, call: Call) -> Result<Value, Error>;

    fn name(&self) -> String;

    fn version(&self) -> String;
}

struct OtterModule<T: Module + Sized + Unpin + 'static> {
    module: T,
}


impl<T: Module + Sized + Unpin + 'static> Actor for OtterModule<T> {
    type Context = Context<Self>;
}

impl<T: Module + Sized + Unpin + 'static> Handler<Call> for OtterModule<T> {
    type Result = ResponseFuture<Result<Value, Error>>;
    fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            self.module.call(msg).await
        })
    }
}
