use crate::error::Error;
use wallet_common::currencies::CurrencyEntityShort;

/// 单方挑选大于所需金额的货币
/// 输入可用序列，avail_cnts 应以value从大到小排序
///  输出贪心尽量拿刚好最小面值的可选方案
pub fn find_cover_currency_plan(
    avail_cnts: &Vec<CurrencyEntityShort>,
    amount: u64,
) -> Result<(u64, Vec<CurrencyEntityShort>), Error> {
    if amount == 0 {
        return Ok((0, Vec::<CurrencyEntityShort>::new()));
    }

    let mut left_amount = amount;
    let mut ret = Vec::<CurrencyEntityShort>::new();
    // 最后一张cover支付金额的现场
    let mut last = None;

    for each in avail_cnts {
        if left_amount >= each.amount {
            ret.push(each.clone());
            left_amount -= each.amount;
        } else {
            last = Some((each, left_amount, ret.len()));
        }
    }

    if left_amount > 0 {
        if let Some(last) = last {
            while ret.len() > last.2 {
                ret.pop();
            }
            ret.push(last.0.clone());
            return Ok((amount - last.1 + last.0.amount, ret));
        } else {
            return Err(Error::TXPayBalanceNotEnough);
        }
    } else {
        return Ok((amount, ret));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet_common::currencies::{CurrencyEntityShort, CurrencyStatus};

    #[test]
    fn test_pay() {
        let mut avails = Vec::<CurrencyEntityShort>::new();
        avails.push(CurrencyEntityShort {
            id: "abcdefabcdef001".to_string(),
            amount: 101u64,
            status: CurrencyStatus::Avail,
        });
        avails.push(CurrencyEntityShort {
            id: "abcdefabcdef002".to_string(),
            amount: 51u64,
            status: CurrencyStatus::Avail,
        });
        avails.push(CurrencyEntityShort {
            id: "abcdefabcdef003".to_string(),
            amount: 50u64,
            status: CurrencyStatus::Avail,
        });
        avails.push(CurrencyEntityShort {
            id: "abcdefabcdef003".to_string(),
            amount: 20u64,
            status: CurrencyStatus::Avail,
        });

        let ret = find_cover_currency_plan(&avails, 101);
        assert_eq!(
            (
                101,
                vec![CurrencyEntityShort {
                    id: "abcdefabcdef001".to_string(),
                    amount: 101,
                    status: CurrencyStatus::Avail
                }]
            ),
            ret.unwrap()
        );

        let ret = find_cover_currency_plan(&avails, 151);
        assert_eq!(
            (
                151,
                vec![
                    CurrencyEntityShort {
                        id: "abcdefabcdef001".to_string(),
                        amount: 101,
                        status: CurrencyStatus::Avail
                    },
                    CurrencyEntityShort {
                        id: "abcdefabcdef003".to_string(),
                        amount: 50,
                        status: CurrencyStatus::Avail
                    }
                ]
            ),
            ret.unwrap()
        );

        let ret = find_cover_currency_plan(&avails, 122);
        assert_eq!(
            (
                151,
                vec![
                    CurrencyEntityShort {
                        id: "abcdefabcdef001".to_string(),
                        amount: 101,
                        status: CurrencyStatus::Avail
                    },
                    CurrencyEntityShort {
                        id: "abcdefabcdef003".to_string(),
                        amount: 50,
                        status: CurrencyStatus::Avail
                    }
                ]
            ),
            ret.unwrap()
        );
    }
}
