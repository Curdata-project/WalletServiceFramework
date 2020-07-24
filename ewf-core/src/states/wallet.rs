use crate::error::Error;
use crate::Machine;

enum WalletState {
    Begin,
    Start,
    Ready,
    Failed,
    Close,
    Destory,
}

enum WalletTransition {
    Starting,
    InitalSuccess,
    InitalFailed,
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
    fn name(&self) -> String {
        "wallet".to_string()
    }

    fn to_string(&self) -> String {
        match self.state {
            WalletState::Begin => "Begin".to_string(),
            WalletState::Start => "Start".to_string(),
            WalletState::Ready => "Ready".to_string(),
            WalletState::Failed => "Failed".to_string(),
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
            (WalletState::Start, WalletTransition::InitalSuccess) => {
                self.state = WalletState::Ready;
                Ok(self.to_string())
            }
            (WalletState::Start, WalletTransition::InitalFailed) => {
                self.state = WalletState::Failed;
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
            _ => Err(Error::TransitionNotFound),
        }
    }
}