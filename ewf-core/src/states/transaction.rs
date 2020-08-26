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
    ExchangeCurrencyAtReceiver,
    SendCurrencyPlan,
    ExchangeCurrencyAtPayer,
    SendCurrencyStat,
    CurrencyPlanDone,
    HalfTransaction,
    EndTransaction,
}

enum TransactionTransition {
    Starting,
    PaymentPlanSyn,
    PaymentPlanAck,
    RecvPaymentPlanSyn,
    RecvPaymentPlanAck,
    IsReceiver,
    RecvCurrencyStat,
    RecvComputePlanNotNeedExchange,
    RecvComputePlanNeedExchange,
    ExChangeCurrencyDoneAtReceiver,
    SendCurrencyPlanData,
    IsPayerAndSendCurrencyStat,
    RecvCurrencyPlanNotNeedExchange,
    RecvCurrencyPlanNeedExchange,
    ExChangeCurrencyDoneAtPayer,
    RecvTransactionSyncAndSendConfirm,
    RecvTransactionConfirm,
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
            "IsReceiver" => TransactionTransition::IsReceiver,
            "RecvCurrencyStat" => TransactionTransition::RecvCurrencyStat,
            "RecvComputePlanNotNeedExchange" => {
                TransactionTransition::RecvComputePlanNotNeedExchange
            }
            "RecvComputePlanNeedExchange" => TransactionTransition::RecvComputePlanNeedExchange,
            "ExChangeCurrencyDoneAtReceiver" => {
                TransactionTransition::ExChangeCurrencyDoneAtReceiver
            }
            "SendCurrencyPlanData" => TransactionTransition::SendCurrencyPlanData,
            "IsPayerAndSendCurrencyStat" => TransactionTransition::IsPayerAndSendCurrencyStat,
            "RecvCurrencyPlanNotNeedExchange" => {
                TransactionTransition::RecvCurrencyPlanNotNeedExchange
            }
            "RecvCurrencyPlanNeedExchange" => TransactionTransition::RecvCurrencyPlanNeedExchange,
            "ExChangeCurrencyDoneAtPayer" => TransactionTransition::ExChangeCurrencyDoneAtPayer,
            "RecvTransactionSyncAndSendConfirm" => {
                TransactionTransition::RecvTransactionSyncAndSendConfirm
            }
            "RecvTransactionConfirm" => TransactionTransition::RecvTransactionConfirm,
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
            TransactionState::ExchangeCurrencyAtReceiver => {
                "ExchangeCurrencyAtReceiver".to_string()
            }
            TransactionState::SendCurrencyPlan => "SendCurrencyPlan".to_string(),
            TransactionState::ExchangeCurrencyAtPayer => "ExchangeCurrencyAtPayer".to_string(),
            TransactionState::SendCurrencyStat => "SendCurrencyStat".to_string(),
            TransactionState::CurrencyPlanDone => "CurrencyPlanDone".to_string(),
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
            // 收款方
            (TransactionState::PaymentPlanDone, TransactionTransition::IsReceiver) => {
                self.state = TransactionState::WaitCurrencyStat;
                Ok(self.to_string())
            }
            (TransactionState::WaitCurrencyStat, TransactionTransition::RecvCurrencyStat) => {
                self.state = TransactionState::ComputePlan;
                Ok(self.to_string())
            }
            (
                TransactionState::ComputePlan,
                TransactionTransition::RecvComputePlanNotNeedExchange,
            ) => {
                self.state = TransactionState::SendCurrencyPlan;
                Ok(self.to_string())
            }
            (TransactionState::ComputePlan, TransactionTransition::RecvComputePlanNeedExchange) => {
                self.state = TransactionState::ExchangeCurrencyAtReceiver;
                Ok(self.to_string())
            }
            (
                TransactionState::ExchangeCurrencyAtReceiver,
                TransactionTransition::ExChangeCurrencyDoneAtReceiver,
            ) => {
                self.state = TransactionState::SendCurrencyPlan;
                Ok(self.to_string())
            }
            (TransactionState::SendCurrencyPlan, TransactionTransition::SendCurrencyPlanData) => {
                self.state = TransactionState::CurrencyPlanDone;
                Ok(self.to_string())
            }
            // 付款方
            (
                TransactionState::PaymentPlanDone,
                TransactionTransition::IsPayerAndSendCurrencyStat,
            ) => {
                self.state = TransactionState::SendCurrencyStat;
                Ok(self.to_string())
            }
            (
                TransactionState::SendCurrencyStat,
                TransactionTransition::RecvCurrencyPlanNeedExchange,
            ) => {
                self.state = TransactionState::ExchangeCurrencyAtPayer;
                Ok(self.to_string())
            }
            (
                TransactionState::SendCurrencyStat,
                TransactionTransition::RecvCurrencyPlanNotNeedExchange,
            ) => {
                self.state = TransactionState::CurrencyPlanDone;
                Ok(self.to_string())
            }
            (
                TransactionState::ExchangeCurrencyAtPayer,
                TransactionTransition::ExChangeCurrencyDoneAtPayer,
            ) => {
                self.state = TransactionState::CurrencyPlanDone;
                Ok(self.to_string())
            }
            (
                TransactionState::CurrencyPlanDone,
                TransactionTransition::RecvTransactionSyncAndSendConfirm,
            ) => {
                self.state = TransactionState::HalfTransaction;
                Ok(self.to_string())
            }
            (TransactionState::HalfTransaction, TransactionTransition::RecvTransactionConfirm) => {
                self.state = TransactionState::EndTransaction;
                Ok(self.to_string())
            }
            _ => Err(Error::TransitionNotFound),
        }
    }
}
