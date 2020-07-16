use crate::Error;
use crate::Machine;
use alloc::string::String;
use alloc::string::ToString;

enum WalletState {
    Begin,
    Start,
    StoreUninital,
    StoreInitaled,
    Unregistered,
    Ready,
    Close,
    Destory,
}

enum WalletTransition {
    Starting,
    EmptyWallet,
    InitalSuccess,
    StoreInitaled,
    Unregistered,
    Registered,
    RegisterComplete,
    CloseWallet,
    ClearWallet,
}

impl From<String> for WalletTransition {
    fn from(t: String) -> WalletTransition {
        let tmp: &str = &t;
        match tmp {
            "Starting" => WalletTransition::Starting,
            "EmptyWallet" => WalletTransition::EmptyWallet,
            "InitalSuccess" => WalletTransition::InitalSuccess,
            "StoreInitaled" => WalletTransition::StoreInitaled,
            "Unregistered" => WalletTransition::Unregistered,
            "Registered" => WalletTransition::Registered,
            "RegisterComplete" => WalletTransition::RegisterComplete,
            "CloseWallet" => WalletTransition::CloseWallet,
            "ClearWallet" => WalletTransition::ClearWallet,
            _ => WalletTransition::Starting,
        }
    }
}

pub struct WalletMachine {
    state: WalletState,
}

impl Default for WalletMachine {
    fn default() -> Self {
        Self {
            state: WalletState::Begin,
        }
    }
}

impl Machine for WalletMachine {
    fn name(&self) -> String {
        "wallet".to_string()
    }

    fn to_string(&self) -> String {
        match self.state {
            WalletState::Begin => "Begin".to_string(),
            WalletState::Start => "Start".to_string(),
            WalletState::StoreUninital => "StoreUninital".to_string(),
            WalletState::StoreInitaled => "StoreInitaled".to_string(),
            WalletState::Unregistered => "Unregistered".to_string(),
            WalletState::Ready => "Ready".to_string(),
            WalletState::Close => "Close".to_string(),
            WalletState::Destory => "Destory".to_string(),
        }
    }

    fn transition(&mut self, t: String) -> Result<String, Error> {
        let ti: WalletTransition = t.into();
        match (&self.state, ti) {
            (WalletState::Begin, WalletTransition::Starting) => {
                self.state = WalletState::Start;
                Ok(self.to_string())
            }
            (WalletState::Start, WalletTransition::EmptyWallet) => {
                self.state = WalletState::StoreUninital;
                Ok(self.to_string())
            }
            (WalletState::StoreUninital, WalletTransition::InitalSuccess) => {
                self.state = WalletState::StoreInitaled;
                Ok(self.to_string())
            }
            (WalletState::Start, WalletTransition::StoreInitaled) => {
                self.state = WalletState::StoreInitaled;
                Ok(self.to_string())
            }
            (WalletState::StoreInitaled, WalletTransition::Unregistered) => {
                self.state = WalletState::Unregistered;
                Ok(self.to_string())
            }
            (WalletState::Unregistered, WalletTransition::RegisterComplete) => {
                self.state = WalletState::Ready;
                Ok(self.to_string())
            }
            (WalletState::StoreInitaled, WalletTransition::Registered) => {
                self.state = WalletState::Ready;
                Ok(self.to_string())
            }
            (WalletState::Ready, WalletTransition::CloseWallet) => {
                self.state = WalletState::Close;
                Ok(self.to_string())
            }
            (WalletState::Ready, WalletTransition::ClearWallet) => {
                self.state = WalletState::Destory;
                Ok(self.to_string())
            }
            _ => Err(Error::TransitionError),
        }
    }
}
