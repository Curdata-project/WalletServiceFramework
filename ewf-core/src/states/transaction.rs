use crate::error::Error;
use crate::Machine;

enum TransactionState {
    Begin,
    Start,
    Ready,
}

enum TransactionTransition {
    Starting,
    TransactionSuccess,
}

impl From<String> for TransactionTransition {
    fn from(t: String) -> TransactionTransition {
        let t: &str = &t;
        match t {
            "Starting" => TransactionTransition::Starting,
            "TransactionSuccess" => TransactionTransition::TransactionSuccess,
            _ => TransactionTransition::Starting,
        }
    }
}

pub struct TransactionMachine {
    state: TransactionState,
}

impl Default for TransactionMachine {
    fn default() -> Self {
        Self {
            state: TransactionState::Begin,
        }
    }
}

impl Machine for TransactionMachine {
    fn name(&self) -> String {
        "wallet".to_string()
    }

    fn to_string(&self) -> String {
        match self.state {
            TransactionState::Begin => "Begin".to_string(),
            TransactionState::Start => "Start".to_string(),
            TransactionState::Ready => "Ready".to_string(),
        }
    }

    fn transition(&mut self, t: String) -> Result<String, Error> {
        let ti: TransactionTransition = t.into();
        match (&self.state, ti) {
            (TransactionState::Begin, TransactionTransition::Starting) => {
                self.state = TransactionState::Start;
                Ok(self.to_string())
            }
            (TransactionState::Start, TransactionTransition::TransactionSuccess) => {
                self.state = TransactionState::Ready;
                Ok(self.to_string())
            }
            _ => Err(Error::TransitionNotFound),
        }
    }
}
