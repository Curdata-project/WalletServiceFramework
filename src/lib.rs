// #![no_std]
#![feature(async_closure)]

extern crate alloc;


pub mod error;

pub mod transtion_caller;

pub mod module_bus;

pub mod storage;


pub mod wallet_mgr;

pub mod keypair_mgr;

pub mod currency_mgr;

pub mod pay_part;


#[macro_use]
extern crate rustorm;