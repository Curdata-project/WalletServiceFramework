use crate::message::{Call, CallQuery};

#[macro_export]
macro_rules! call_mod_througth_bus {
    ($bus_addr: ident, $mod_name: expr, $fn_name: expr, $param: expr) => {
        $bus_addr
            .send(CallQuery {
                module: $mod_name.to_string(),
            })
            .await??
            .send(Call {
                method: $fn_name.to_string(),
                args: $param,
            })
            .await??
    };
}

#[macro_export]
macro_rules! call_self {
    ($self_addr: ident, $fn_name: expr, $param: expr) => {
        $self_addr
            .send(Call {
                method: $fn_name.to_string(),
                args: $param,
            })
            .await??
    };
}
