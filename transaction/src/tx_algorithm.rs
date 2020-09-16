use crate::error::Error;
use wallet_common::currencies::CurrencyEntityShort;

/// 单方挑选大于所需金额的货币
/// 输入可用序列，avail_cnts: [(value, num)]中应以value从大到小排序
///  输出贪心尽量拿最大面值的可选方案
///     有缺陷
pub fn find_cover_currency_plan(
    avail_cnts: &Vec<CurrencyEntityShort>,
    cur_index: usize,
    amount: u64,
) -> Result<(u64, Vec<CurrencyEntityShort>), Error> {
    if amount == 0 {
        return Ok((0, Vec::<CurrencyEntityShort>::new()));
    }

    if cur_index >= avail_cnts.len() {
        return Err(Error::TXPayBalanceNotEnough);
    }
    let mut ret = Vec::<CurrencyEntityShort>::new();

    let cur_value = avail_cnts[cur_index].amount;
    let (need_cur, next_amount) = if amount > cur_value {
        (true, amount - cur_value)
    } else {
        (false, amount)
    };

    match find_cover_currency_plan(avail_cnts, cur_index + 1, next_amount) {
        Ok((sub_ans_cnt, sub_ans_plan)) => {
            if need_cur {
                ret.push(avail_cnts[cur_index].clone());
            }
            ret.extend_from_slice(&sub_ans_plan[..]);
            return Ok((cur_value + sub_ans_cnt, ret));
        }
        Err(_) => {
            if !need_cur {
                ret.push(avail_cnts[cur_index].clone());
                return Ok((cur_value, ret));
            } else {
                return Err(Error::TXPayBalanceNotEnough);
            }
        }
    }
}
