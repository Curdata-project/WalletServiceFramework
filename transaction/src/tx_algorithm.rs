use crate::error::Error;
use std::cmp::{min, max};
use wallet_common::currencies::CurrencyStatisticsItem;
use wallet_common::transaction::CurrencyPlanItem;

///  收款方提供列表用以找零
///
///
pub fn get_currenics_for_change(
    mut currencies: Vec<CurrencyStatisticsItem>,
) -> Vec<CurrencyStatisticsItem> {
    let mut ret = Vec::<CurrencyStatisticsItem>::new();

    currencies.sort();

    let mut last = 20000u64;
    for each in currencies {
        let need_num = max(0, min(last / each.value - 1, each.num));

        if need_num > 0 {
            ret.push(CurrencyStatisticsItem {
                value: each.value,
                num: need_num,
            });
            last -= each.value * need_num;
        }
    }

    ret
}

/// 输入可用序列，avail_cnts: [(value, num)]中应以value从大到小排序
///  输出贪心尽量拿最大面值的可选方案
///     
pub fn find_cover_currency_plan(
    avail_cnts: &Vec<CurrencyStatisticsItem>,
    cur_index: usize,
    amount: u64,
) -> Result<(u64, Vec<CurrencyStatisticsItem>), Error> {
    if amount == 0 {
        return Ok((0, Vec::<CurrencyStatisticsItem>::new()));
    }

    if cur_index >= avail_cnts.len() {
        return Err(Error::TXPayBalanceNotEnough);
    }
    let mut ret = Vec::<CurrencyStatisticsItem>::new();
    let mut actual_amount = 0u64;

    let cur_value = avail_cnts[cur_index].value;
    let cur_num = avail_cnts[cur_index].num;

    let need_use = min(amount / cur_value, cur_num);

    actual_amount += cur_value * need_use;

    match find_cover_currency_plan(avail_cnts, cur_index + 1, amount - actual_amount) {
        Ok((sub_ans_cnt, sub_ans_plan)) => {
            if need_use > 0 {
                ret.push(CurrencyStatisticsItem {
                    value: cur_value,
                    num: need_use,
                });
            }
            ret.extend_from_slice(&sub_ans_plan[..]);
            return Ok((actual_amount + sub_ans_cnt, ret));
        }
        Err(_) => {
            if cur_num > need_use {
                ret.push(CurrencyStatisticsItem {
                    value: cur_value,
                    num: need_use + 1,
                });
                return Ok((actual_amount + cur_value, ret));
            } else {
                return Err(Error::TXPayBalanceNotEnough);
            }
        }
    }
}

/// 解决(50,1) (20,3)要取60时贪心失败
pub fn find_cover_currency_plan_patch(
    avail_cnts: &Vec<CurrencyStatisticsItem>,
    amount: u64,
) -> Result<(u64, Vec<CurrencyStatisticsItem>), Error> {
    let (ans_amount, ans_plan) = match find_cover_currency_plan(avail_cnts, 0, amount) {
        Ok((ans_amount, ans_plan)) => {
            if ans_amount == amount {
                return Ok((ans_amount, ans_plan));
            }
            (ans_amount, ans_plan)
        }
        Err(err) => return Err(err),
    };

    let mut delete_flag = false;
    let mut new_avail_cnts = Vec::<CurrencyStatisticsItem>::new();
    for each in avail_cnts {
        if each.value == 2000 && each.num >= 3 {
            new_avail_cnts.push(CurrencyStatisticsItem {
                value: each.value,
                num: each.num - 3,
            });
            delete_flag = true;
        } else {
            new_avail_cnts.push(CurrencyStatisticsItem {
                value: each.value,
                num: each.num,
            });
        }
    }

    if !delete_flag {
        return Ok((ans_amount, ans_plan));
    }

    match find_cover_currency_plan(&new_avail_cnts, 0, amount - 6000) {
        Ok((new_amount, mut new_plan)) => {
            if new_amount == amount - 6000 {
                if let Some(it) = new_plan.iter().position(|each| each.value == 2000) {
                    new_plan[it].num += 3;
                } else {
                    new_plan.push(CurrencyStatisticsItem {
                        value: 2000,
                        num: 3,
                    });
                    new_plan.sort();
                }
                return Ok((amount, new_plan));
            }
        }
        Err(err) => return Err(err),
    };

    Err(Error::TXPayBalanceNotEnough)
}

pub fn find_currency_plan(
    payer: Vec<CurrencyStatisticsItem>,
    recv: Vec<CurrencyStatisticsItem>,
    amount: u64,
) -> Result<CurrencyPlanItem, Error> {
    let mut ret_pay_list = Vec::<CurrencyStatisticsItem>::new();
    let mut ret_recv_list = Vec::<CurrencyStatisticsItem>::new();

    let base_amount = match find_cover_currency_plan_patch(&payer, amount) {
        Ok((ans_amount, ans_plan)) => {
            if ans_amount == amount {
                return Ok(CurrencyPlanItem {
                    pay_amount: ans_amount,
                    pay_plan: ans_plan,
                    recv_amount: 0,
                    recv_plan: Vec::<CurrencyStatisticsItem>::new(),
                });
            }
            ans_amount
        }
        Err(err) => return Err(err),
    };

    for addition_amount in 1..10000 {
        let cur_amount = base_amount + addition_amount;

        match (
            find_cover_currency_plan_patch(&payer, cur_amount),
            find_cover_currency_plan_patch(&recv, addition_amount),
        ) {
            (Ok((pay_amount, pay_plan)), Ok((recv_amount, recv_plan))) => {
                if pay_amount == cur_amount && recv_amount == addition_amount {
                    return Ok(CurrencyPlanItem {
                        pay_amount,
                        pay_plan,
                        recv_amount,
                        recv_plan,
                    });
                }
            }
            (_, _) => return Err(Error::TXPayNotAvailChangePlan),
        };
    }
    Err(Error::TXPayNotAvailChangePlan)
}
