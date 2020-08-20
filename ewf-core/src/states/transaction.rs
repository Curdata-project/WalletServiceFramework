use crate::error::Error;
use crate::Machine;

enum TransactionState {
    Begin,
    Start,
    PaymentPlanSend,
    PaymentPlanRecv,
    PaymentPlanDone,
    WaitCurrencyStat,
    ComputePlan,
    ExchangeCurrency,
    SendCurrencyPlan,
    SendExchangeCurrency,
    SendCurrencyStat,
    CurrencyPlanDone,
    SenderExchangeCurrency,
    HalfTransaction,
    EndTransaction,
}

enum TransactionTransition {
    Starting,
    PaymentPlanSyn,
    PaymentPlanAck,
    RecvPaymentPlanSyn,
    RecvPaymentPlanAck,
    IsPayerAndSendCurrencyStat,
    IsReceiver,
}

impl From<String> for TransactionTransition {
    fn from(t: String) -> TransactionTransition {
        let t: &str = &t;
        match t {
            "Starting" => TransactionTransition::Starting,
            "PaymentPlanSyn" => TransactionTransition::PaymentPlanSyn,
            "PaymentPlanAck" => TransactionTransition::PaymentPlanAck,
            "RecvPaymentPlanSyn" => TransactionTransition::RecvPaymentPlanSyn,
            "RecvPaymentPlanAck" => TransactionTransition::RecvPaymentPlanAck,
            "IsPayerAndSendCurrencyStat" => TransactionTransition::IsPayerAndSendCurrencyStat,
            "IsReceiver" => TransactionTransition::IsReceiver,
            _ => TransactionTransition::PaymentPlanSyn,
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
        "transaction".to_string()
    }

    fn to_string(&self) -> String {
        match self.state {
            TransactionState::Begin => "Begin".to_string(),
            TransactionState::Start => "Start".to_string(),
            TransactionState::PaymentPlanSend => "PaymentPlanSend".to_string(),
            TransactionState::PaymentPlanRecv => "PaymentPlanRecv".to_string(),
            TransactionState::PaymentPlanDone => "PaymentPlanDone".to_string(),
            TransactionState::WaitCurrencyStat => "WaitCurrencyStat".to_string(),
            TransactionState::ComputePlan => "ComputePlan".to_string(),
            TransactionState::ExchangeCurrency => "ExchangeCurrency".to_string(),
            TransactionState::SendCurrencyPlan => "SendCurrencyPlan".to_string(),
            TransactionState::SendExchangeCurrency => "SendExchangeCurrency".to_string(),
            TransactionState::SendCurrencyStat => "SendCurrencyStat".to_string(),
            TransactionState::CurrencyPlanDone => "CurrencyPlanDone".to_string(),
            TransactionState::SenderExchangeCurrency => "SenderExchangeCurrency".to_string(),
            TransactionState::HalfTransaction => "HalfTransaction".to_string(),
            TransactionState::EndTransaction => "EndTransaction".to_string(),
        }
    }

    fn transition(&mut self, t: String) -> Result<String, Error> {
        let ti: TransactionTransition = t.into();
        match (&self.state, ti) {
            (TransactionState::Begin, TransactionTransition::Starting) => {
                self.state = TransactionState::Start;
                Ok(self.to_string())
            }
            (TransactionState::Start, TransactionTransition::PaymentPlanSyn) => {
                self.state = TransactionState::PaymentPlanSend;
                Ok(self.to_string())
            }
            (TransactionState::Start, TransactionTransition::RecvPaymentPlanSyn) => {
                self.state = TransactionState::PaymentPlanRecv;
                Ok(self.to_string())
            }
            (TransactionState::PaymentPlanSend, TransactionTransition::RecvPaymentPlanAck) => {
                self.state = TransactionState::PaymentPlanDone;
                Ok(self.to_string())
            }
            (TransactionState::PaymentPlanRecv, TransactionTransition::PaymentPlanAck) => {
                self.state = TransactionState::PaymentPlanDone;
                Ok(self.to_string())
            }
            (
                TransactionState::PaymentPlanDone,
                TransactionTransition::IsPayerAndSendCurrencyStat,
            ) => {
                self.state = TransactionState::SendCurrencyPlan;
                Ok(self.to_string())
            }
            (TransactionState::PaymentPlanDone, TransactionTransition::IsReceiver) => {
                self.state = TransactionState::WaitCurrencyStat;
                Ok(self.to_string())
            }
            _ => Err(Error::TransitionNotFound),
        }
    }
}
