use crate::error::Error;
use crate::Machine;

enum TransactionState {
    Begin,
    Start,
    PaymentPlanSyn,
    PaymentPlanDone,
    PaymentPlanDone,
    WaitCurrencyStat,
    SendCurrencyStat,
    CurrencyPlanDone,
    SenderExchangeCurrency,
    CurrencyPlanDone,
    ComputePlan,
    ExchangeCurrency,
    SendCurrencyPlan,
    HalfTranscation,
    EndTranscation,
}

enum PayTransaction {
}

impl From<String> for PayTransaction {
    fn from(t: String) -> PayTransaction {
        let t: &str = &t;
        match t {
            _ => PayTransaction::Starting,
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
            TransactionState::Failed => "Failed".to_string(),
            TransactionState::Close => "Close".to_string(),
            TransactionState::Destory => "Destory".to_string(),
        }
    }

    fn transition(&mut self, t: String) -> Result<String, Error> {
        let ti: PayTransaction = t.into();
        match (&self.state, ti) {
            (TransactionState::Begin, PayTransaction::Starting) => {
                self.state = TransactionState::Start;
                Ok(self.to_string())
            }
            (TransactionState::Start, PayTransaction::SendTransaction) => {
                self.state = TransactionState::Ready;
                Ok(self.to_string())
            }
            (TransactionState::Begin, PayTransaction::RecvTransaction) => {
                self.state = TransactionState::Ready;
                Ok(self.to_string())
            }
            (TransactionState::PaymentPlanSyn, PayTransaction::InitalFailed) => {
                self.state = TransactionState::Failed;
                Ok(self.to_string())
            }
            (TransactionState::PaymentPlanDone, PayTransaction::CloseTransaction) => {
                self.state = TransactionState::Close;
                Ok(self.to_string())
            }
            (TransactionState::WaitCurrencyStat, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::SendCurrencyStat, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::CurrencyPlanDone, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::SenderExchangeCurrency, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::CurrencyPlanDone, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::ExchangeCurrency, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::SendCurrencyPlan, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::HalfTranscation, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            (TransactionState::EndTranscation, PayTransaction::ClearTransaction) => {
                self.state = TransactionState::Destory;
                Ok(self.to_string())
            }
            _ => Err(Error::TransitionNotFound),
        }
    }
}
