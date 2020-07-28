use crate::message::{Call, Event, StartNotify};
use actix::prelude::*;
use std::fmt::Debug;

pub trait Module: Debug + Actor + Handler<Call> + Handler<Event> + Handler<StartNotify> {
    fn name(&self) -> String;

    fn version(&self) -> String;
}

// struct OtterModule<T: Module + Sized + Unpin + 'static> {
//     module: T,
// }

// impl<T: Module + Sized + Unpin + 'static> Actor for OtterModule<T> {
//     type Context = Context<Self>;
// }

// impl<T: Module + Sized + Unpin + 'static> Handler<Call> for OtterModule<T> {
//     type Result = ResponseFuture<Result<Value, Error>>;
//     fn handle(&mut self, msg: Call, _ctx: &mut Context<Self>) -> Self::Result {
//         Box::pin(async move {
//             self.module.call(msg).await
//         })
//     }
// }
