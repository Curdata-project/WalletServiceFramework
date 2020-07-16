use wallet_service_framework::{Module, Event, Bus};
use wallet_service_framework::Error;
use std::path::Path;
use serde_json::{Value, json};


static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency" (
    "id" VARCHAR(255) NOT NULL,
    "quota_control_field" TEXT NOT NULL,
    "explain_info" TEXT NOT NULL,
    "state" VARCHAR(255) NOT NULL,
    "owner" VARCHAR(255) NOT NULL,
    "create_time" TIMESTAMP NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

pub struct CurrenciesMgr {
    path: String,
}

impl CurrenciesMgr{
    pub fn new(path: String) -> Self {
        Self{
            path,
        }
    }
     
    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }

    pub fn create(&mut self) -> Result<(), Error> {
        self.open()
    }

    /// url 形如 sqlite:///home/lee/rustorm/file.db
    pub fn open(&mut self) -> Result<(), Error> {
        Ok(())
    }
}


impl Module for CurrenciesMgr {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), Error> {
        let event: &str = &event.event;
        match event {
            "Start" => {
                let exists = self.exists();
                if exists {
                    unsafe{ 
                        let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                        mut_bus.transition(0, "StoreInitaled".to_string())?;
                    }
                }
                else{
                    unsafe{ 
                        let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                        mut_bus.transition(0, "EmptyWallet".to_string())?;
                    }
                }
            }
            "StoreUninital" => {
                unsafe{ 
                    let self_ = &mut * ( self as *const Self as * mut Self);
                    self_.create();

                    let mut_bus = &mut * ( bus as *const Bus as * mut Bus);
                    mut_bus.transition(0, "InitalSuccess".to_string())?;
                }
            }
            // no care this event, ignore
            _ => return Ok(()),
        }
        Ok(())
    }

    fn call(&self, method: &str, _intput: Value) -> Result<Value, Error> {
        match method {
            _ => Err(Error::NotMethodCallError),
        }
    }

    fn name(&self) -> String {
        "currencies".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}

