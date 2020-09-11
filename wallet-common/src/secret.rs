use common_structure::digital_currency::DigitalCurrencyWrapper;
use common_structure::transaction::TransactionWrapper;
use kv_object::sm2::{CertificateSm2, KeyPairSm2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyPairEntity {
    Sm2(KeyPairSm2),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateEntity {
    Sm2(CertificateSm2),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntity {
    pub uid: String,
    pub secret_type: String,
    pub keypair: KeyPairEntity,
    pub cert: CertificateEntity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterParam {
    pub url: String,
    pub timeout: u64,
    pub info: RegisterUserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserInfo {
    pub account: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub cert: String,
    pub info: RegisterUserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub uid: String,
    pub cert: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionRequest {
    pub uid: String,
    pub oppo_cert: CertificateSm2,
    pub datas: Vec<DigitalCurrencyWrapper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionResponse {
    pub datas: Vec<String>,
}
