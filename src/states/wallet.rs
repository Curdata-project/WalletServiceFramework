use crate::Machine;
use alloc::string::ToString;
use alloc::string::String;

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
        WalletTransition::Starting
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

    fn transition(&mut self, t: String) -> Result<String, ()> {
        let ti: WalletTransition = t.into();
        match (&self.state, ti) {
            (WalletState::Begin, WalletTransition::Starting) => Ok(self.to_string()),
            _ => Err(())
        }
    }
}

