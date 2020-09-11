use crate::error::Error;
use std::cmp::{max, min};
use wallet_common::currencies::StatisticsItem;
use wallet_common::transaction::CurrencyPlanItem;

pub static CURRENCY_VALUE: [u64; 8] = [10000, 5000, 2000, 1000, 500, 100, 10, 1];

pub trait ComputeCurrencyPlan {
    fn new() -> Self;

    fn find_currency_plan(
        self,
        payer: Vec<StatisticsItem>,
        recv: Vec<StatisticsItem>,
        amount: u64,
    ) -> Result<CurrencyPlanItem, Error>;
}

///  收款方提供列表用以找零
///
///
pub fn get_currenics_for_change(mut currencies: Vec<StatisticsItem>) -> Vec<StatisticsItem> {
    let mut ret = Vec::<StatisticsItem>::new();

    currencies.sort();

    let mut last = 10000u64;
    for each in currencies {
        let need_num = max(0, min(last / each.value - 1, each.num));

        if need_num > 0 {
            ret.push(StatisticsItem {
                value: each.value,
                num: need_num,
            });
            last -= each.value * need_num;
        }
    }

    ret
}

/// 单方挑选大于所需金额的货币
/// 输入可用序列，avail_cnts: [(value, num)]中应以value从大到小排序
///  输出贪心尽量拿最大面值的可选方案
///     有缺陷
pub fn find_cover_currency_plan(
    avail_cnts: &Vec<StatisticsItem>,
    cur_index: usize,
    amount: u64,
) -> Result<(u64, Vec<StatisticsItem>), Error> {
    if amount == 0 {
        return Ok((0, Vec::<StatisticsItem>::new()));
    }

    if cur_index >= avail_cnts.len() {
        return Err(Error::TXPayBalanceNotEnough);
    }
    let mut ret = Vec::<StatisticsItem>::new();
    let mut actual_amount = 0u64;

    let cur_value = avail_cnts[cur_index].value;
    let cur_num = avail_cnts[cur_index].num;

    let need_use = min(amount / cur_value, cur_num);

    actual_amount += cur_value * need_use;

    match find_cover_currency_plan(avail_cnts, cur_index + 1, amount - actual_amount) {
        Ok((sub_ans_cnt, sub_ans_plan)) => {
            if need_use > 0 {
                ret.push(StatisticsItem {
                    value: cur_value,
                    num: need_use,
                });
            }
            ret.extend_from_slice(&sub_ans_plan[..]);
            return Ok((actual_amount + sub_ans_cnt, ret));
        }
        Err(_) => {
            if cur_num > need_use {
                ret.push(StatisticsItem {
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

/// 单方挑选大于所需金额的货币
/// 解决(50,2) (20,4)要取60，80，110时贪心失败的缺陷
pub fn find_cover_currency_plan_patch(
    avail_cnts: &Vec<StatisticsItem>,
    amount: u64,
) -> Result<(u64, Vec<StatisticsItem>), Error> {
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
    let mut new_avail_cnts = Vec::<StatisticsItem>::new();
    for each in avail_cnts {
        if each.value == 2000 && each.num >= 3 {
            new_avail_cnts.push(StatisticsItem {
                value: each.value,
                num: each.num - 3,
            });
            delete_flag = true;
        } else {
            new_avail_cnts.push(each.clone());
        }
    }

    if !delete_flag {
        return Ok((ans_amount, ans_plan));
    }

    match find_cover_currency_plan(&new_avail_cnts, 0, amount - 6000) {
        Ok((new_amount, mut new_plan)) => {
            if let Some(it) = new_plan.iter().position(|each| each.value == 2000) {
                new_plan[it].num += 3;
            } else {
                new_plan.push(StatisticsItem {
                    value: 2000,
                    num: 3,
                });
                new_plan.sort();
            }
            Ok((new_amount + 6000, new_plan))
        }
        Err(err) => Err(err),
    }
}

/// 双方找零策略A
/// 考虑付款方((20,1) (2,1)) 收款方(5,1) 要付17的情况
/// 结合双方零钱情况可以找到更好的方案，使用贪心策略
pub struct ComputeCurrencyPlanA {}

impl ComputeCurrencyPlan for ComputeCurrencyPlanA {
    fn new() -> Self {
        Self {}
    }

    fn find_currency_plan(
        self,
        mut payer: Vec<StatisticsItem>,
        mut recv: Vec<StatisticsItem>,
        amount: u64,
    ) -> Result<CurrencyPlanItem, Error> {
        payer.sort();
        recv.sort();

        let base_amount = match find_cover_currency_plan_patch(&payer, amount) {
            Ok((ans_amount, ans_plan)) => {
                if ans_amount == amount {
                    return Ok(CurrencyPlanItem {
                        pay_amount: ans_amount,
                        pay_plan: ans_plan,
                        recv_amount: 0,
                        recv_plan: Vec::<StatisticsItem>::new(),
                    });
                }
                ans_amount
            }
            Err(err) => return Err(err),
        };

        for addition_amount in 0..10000 {
            let cur_amount = base_amount + addition_amount;

            match (
                find_cover_currency_plan_patch(&payer, cur_amount),
                find_cover_currency_plan_patch(&recv, cur_amount - amount),
            ) {
                (Ok((pay_amount, pay_plan)), Ok((recv_amount, recv_plan))) => {
                    if pay_amount == cur_amount && recv_amount == cur_amount - amount {
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
}

/// 双方找零策略B
/// 考虑付款方((20,1) (2,1)) 收款方(5,1) 要付17的情况
/// 结合双方零钱情况可以找到更好的方案，使用背包策略
///     
pub struct ComputeCurrencyPlanB {
    pay_bucket: [i32; 20001],
    recv_bucket: [i32; 20001],

    pay_stuff: Vec<(u64, u64)>,
    recv_stuff: Vec<(u64, u64)>,
}

impl ComputeCurrencyPlanB {
    /// 最好是面值从大到小地二进制化，这样会优先取大的，否则是混着取的
    pub fn binary_stuff(stuff: &mut Vec<(u64, u64)>, item: &StatisticsItem) {
        let mut last_num = item.num;
        let mut tmp_num = 1;
        loop {
            if last_num >= tmp_num {
                stuff.push((item.value * tmp_num, item.value));
                last_num -= tmp_num;
            } else {
                stuff.push((item.value * last_num, item.value));
                break;
            }
            tmp_num *= 2;
        }
    }

    /// 使用compute_bucket计算之前要清空bucket为-1，stuff二进制化
    pub fn compute_bucket(bucket: &mut [i32; 20001], stuff: &Vec<(u64, u64)>) {
        for (i, each_value) in stuff.iter().enumerate() {
            let each_value = each_value.0 as usize;
            let mut loop_j = 20000 - each_value as i64;
            while loop_j >= 0 {
                let j = loop_j as usize;
                if (j == 0 || bucket[j] != -1) && bucket[j + each_value] == -1 {
                    bucket[j + each_value] = i as i32;
                }
                loop_j -= 1;
            }
        }
    }

    pub fn get_path_dep(
        bucket: &[i32; 20001],
        stuff: &Vec<(u64, u64)>,
        amount: usize,
    ) -> Vec<StatisticsItem> {
        if amount == 0 {
            return Vec::<StatisticsItem>::new();
        }
        let index = bucket[amount] as usize;
        let mut recv_plan = Self::get_path_dep(bucket, stuff, amount - (stuff[index].0 as usize));
        recv_plan.push(StatisticsItem {
            value: stuff[index].1,
            num: stuff[index].0 / stuff[index].1,
        });

        return recv_plan;
    }

    /// 合并get_path_dep返回的二进制结果
    pub fn get_path(
        bucket: &[i32; 20001],
        stuff: &Vec<(u64, u64)>,
        amount: usize,
    ) -> Vec<StatisticsItem> {
        let mut ret = Self::get_path_dep(bucket, stuff, amount);

        if ret.len() == 0 {
            return ret;
        }

        let mut i = 0usize;
        let mut j = 0usize;
        while i < ret.len() {
            if ret[j].value == ret[i].value {
                if i == j {
                    i += 1;
                    continue;
                }
                ret[j].num += ret[i].num;
                i += 1;
            } else {
                j += 1;
                ret[j].value = ret[i].value;
                ret[j].num = ret[i].num;

                i += 1;
            }
        }
        ret.truncate(j + 1);

        ret
    }
}

impl ComputeCurrencyPlan for ComputeCurrencyPlanB {
    fn new() -> ComputeCurrencyPlanB {
        Self {
            pay_bucket: [-1i32; 20001],
            recv_bucket: [-1i32; 20001],
            pay_stuff: Vec::<(u64, u64)>::new(),
            recv_stuff: Vec::<(u64, u64)>::new(),
        }
    }

    /// 输入可用序列，avail_cnts: [(value, num)]中应以value从大到小排序
    /// 将大于20000即200元的部分贪心取掉，剩下的二进制背包
    fn find_currency_plan(
        mut self,
        mut payer: Vec<StatisticsItem>,
        mut recv: Vec<StatisticsItem>,
        amount: u64,
    ) -> Result<CurrencyPlanItem, Error> {
        payer.sort();
        recv.sort();

        let right_amount = amount % 10000;
        let left_amount = amount - right_amount;
        // 缺陷不影响整100的取，且因为是整100结果和入参相同
        let (left_amount, left_plan) = match find_cover_currency_plan(&payer, 0, left_amount) {
            Ok((left_amount, left_plan)) => (left_amount, left_plan),
            Err(err) => return Err(err),
        };

        // 将整100取走的货币从列表去掉
        let mut left_index = 0usize;
        let mut pay_index = 0usize;
        while left_index < left_plan.len() && pay_index < payer.len() {
            while pay_index < payer.len() && left_plan[left_index].value != payer[pay_index].value {
                pay_index += 1;
            }

            if pay_index < payer.len() {
                payer[pay_index].num -= left_plan[left_index].num;
                left_index += 1;
            }
        }
        let payer: Vec<StatisticsItem> = payer
            .iter()
            .filter(|x| x.num > 0)
            .map(|x| x.clone())
            .collect();

        for each in payer {
            Self::binary_stuff(&mut self.pay_stuff, &each);
        }
        for each in recv {
            Self::binary_stuff(&mut self.recv_stuff, &each);
        }
        Self::compute_bucket(&mut self.pay_bucket, &self.pay_stuff);
        Self::compute_bucket(&mut self.recv_bucket, &self.recv_stuff);

        for try_amount in right_amount..20000 {
            let try_amount = try_amount as usize;
            let try_amount1 = try_amount - right_amount as usize;
            if self.pay_bucket[try_amount] != -1 && self.recv_bucket[try_amount1] != -1 {
                let mut pay_plan = Self::get_path(&self.pay_bucket, &self.pay_stuff, try_amount);
                // 将整100的方案结果合并回来，因为两个plan都是有序的，直接合并
                let mut left_index = 0usize;
                let mut pay_index = 0usize;
                let old_pay_len = pay_plan.len();
                while pay_index < old_pay_len && left_index < left_plan.len() {
                    while left_index < left_plan.len()
                        && pay_plan[pay_index].value != left_plan[left_index].value
                    {
                        pay_plan.push(left_plan[left_index].clone());
                        left_index += 1;
                    }
                    if left_index < left_plan.len() {
                        pay_plan[pay_index].num += left_plan[left_index].num;
                        pay_index += 1;
                        left_index += 1;
                    }
                }
                while left_index < left_plan.len() {
                    pay_plan.push(left_plan[left_index].clone());
                    left_index += 1;
                }
                pay_plan.sort();

                let recv_plan = Self::get_path(&self.recv_bucket, &self.recv_stuff, try_amount1);

                return Ok(CurrencyPlanItem {
                    pay_amount: left_amount + (try_amount as u64),
                    pay_plan: pay_plan,
                    recv_amount: try_amount1 as u64,
                    recv_plan: recv_plan,
                });
            }
        }

        Err(Error::TXPayNotAvailChangePlan)
    }
}

/// 拆分面值策略
/// 输入要拆分的面值
///  输出拆分方案（要拆分的那张面值，拆分方案）
pub fn find_convert_plan(convert_value: u64, amount: u64) -> Vec<StatisticsItem> {
    let mut ret_plan = Vec::<StatisticsItem>::new();

    let mut src_value = convert_value;
    let mut target_amount = amount;

    // TODO 禁止兑换20元面值的
    for value in &CURRENCY_VALUE {
        if value >= &convert_value || value == &2000u64 {
            continue;
        }

        target_amount = target_amount % value;

        let page_num = if target_amount == 0 {
            src_value / value
        } else {
            src_value / value - 1
        };
        ret_plan.push(StatisticsItem {
            value: value.clone(),
            num: page_num,
        });

        src_value -= value * page_num;

        if src_value == 0 {
            break;
        }
    }

    ret_plan
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
    测试单方挑选大于所需金额的货币

    find_cover_amount_plan
    该函数用以根据可用资金列表产生覆盖需支付金额的支付列表

    策略，能取大额优先取大额，大额取不了再拿小额

    有缺陷，可以看到第三组目标取60时，优先拿一张50，再去尝试拿20和10元的，
    实际上20X3才是最佳方案，可能需要特判60和80取若干20元
    */

    #[test]
    fn test_find_cover_currency_plan() {
        let mut avail_cnts = Vec::<StatisticsItem>::new();

        let ans = find_cover_currency_plan(&avail_cnts, 0, 4000);
        assert_eq!(ans.is_err(), true);
        if let Err(err) = ans {
            assert_eq!(err, Error::TXPayBalanceNotEnough);
        }

        avail_cnts.push(StatisticsItem {
            value: 10000u64,
            num: 2u64,
        });
        avail_cnts.push(StatisticsItem {
            value: 5000u64,
            num: 2u64,
        });
        avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 3u64,
        });
        let ans = find_cover_currency_plan(&avail_cnts, 0, 4000).unwrap();
        assert_eq!(
            (
                4000u64,
                vec![StatisticsItem {
                    value: 2000u64,
                    num: 2u64
                }]
            ),
            ans
        );
        let ans = find_cover_currency_plan(&avail_cnts, 0, 7000).unwrap();
        assert_eq!(
            (
                7000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 1u64
                    }
                ]
            ),
            ans
        );
        let ans = find_cover_currency_plan(&avail_cnts, 0, 6000).unwrap();
        assert_eq!(
            (
                7000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 1u64
                    }
                ]
            ),
            ans
        );
        let ans = find_cover_currency_plan(&avail_cnts, 0, 9000).unwrap();
        assert_eq!(
            (
                9000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 2u64
                    }
                ]
            ),
            ans
        );
    }

    /*
    测试单方挑选大于所需金额的货币patch

        解决(50,1) (20,3)要取60时贪心失败的缺陷
        尝试2000x3的取过后再贪心剩下的

        注意6000 8000 11000几组样例
    */

    #[test]
    fn test_find_cover_currency_plan_patch() {
        let mut avail_cnts = Vec::<StatisticsItem>::new();

        let ans = find_cover_currency_plan_patch(&avail_cnts, 4000);
        assert_eq!(ans.is_err(), true);
        if let Err(err) = ans {
            assert_eq!(err, Error::TXPayBalanceNotEnough);
        }

        avail_cnts.push(StatisticsItem {
            value: 10000u64,
            num: 2u64,
        });
        avail_cnts.push(StatisticsItem {
            value: 5000u64,
            num: 2u64,
        });
        avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 8u64,
        });
        let ans = find_cover_currency_plan_patch(&avail_cnts, 4000).unwrap();
        assert_eq!(
            (
                4000u64,
                vec![StatisticsItem {
                    value: 2000u64,
                    num: 2u64
                }]
            ),
            ans
        );
        let ans = find_cover_currency_plan_patch(&avail_cnts, 7000).unwrap();
        assert_eq!(
            (
                7000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 1u64
                    }
                ]
            ),
            ans
        );
        let ans = find_cover_currency_plan_patch(&avail_cnts, 6000).unwrap();
        assert_eq!(
            (
                6000u64,
                vec![StatisticsItem {
                    value: 2000u64,
                    num: 3u64
                }]
            ),
            ans
        );
        let ans = find_cover_currency_plan_patch(&avail_cnts, 8000).unwrap();
        assert_eq!(
            (
                8000u64,
                vec![StatisticsItem {
                    value: 2000u64,
                    num: 4u64
                }]
            ),
            ans
        );
        let ans = find_cover_currency_plan_patch(&avail_cnts, 9000).unwrap();
        assert_eq!(
            (
                9000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 2u64
                    }
                ]
            ),
            ans
        );
        let ans = find_cover_currency_plan_patch(&avail_cnts, 11000).unwrap();
        assert_eq!(
            (
                11000u64,
                vec![
                    StatisticsItem {
                        value: 5000u64,
                        num: 1u64
                    },
                    StatisticsItem {
                        value: 2000u64,
                        num: 3u64
                    }
                ]
            ),
            ans
        );
    }

    /*
    测试兑换策略

    find_convert_plan
    该函数用以根据一张大额面值和要拆零出的支付金额，来获取兑零方案

    由于 精确找零策略 对20元的缺陷，此处禁止兑换20元的零钱
    以保证 精确找零(失败)->兑换->精确找零 序列中第二次一定成功

    从第三组策略可以看到没有20元出现
    */

    #[test]
    fn test_find_convert_plan() {
        let ans = find_convert_plan(10000, 5001);
        assert_eq!(
            vec![
                StatisticsItem {
                    value: 5000u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 1000u64,
                    num: 4u64
                },
                StatisticsItem {
                    value: 500u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 100u64,
                    num: 4u64
                },
                StatisticsItem {
                    value: 10u64,
                    num: 9u64
                },
                StatisticsItem {
                    value: 1u64,
                    num: 10u64
                }
            ],
            ans
        );
        let ans = find_convert_plan(10000, 5200);
        assert_eq!(
            vec![
                StatisticsItem {
                    value: 5000u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 1000u64,
                    num: 4u64
                },
                StatisticsItem {
                    value: 500u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 100u64,
                    num: 5u64
                }
            ],
            ans
        );
        let ans = find_convert_plan(10000, 2000);
        assert_eq!(
            vec![
                StatisticsItem {
                    value: 5000u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 1000u64,
                    num: 5u64
                }
            ],
            ans
        );
    }

    /*
    测试双方找零策略A
        考虑付款方((20,1) (2,1)) 收款方(5,1) 要付17的情况
    */

    #[test]
    fn test_find_currency_planA() {
        let mut pay_avail_cnts = Vec::<StatisticsItem>::new();
        pay_avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 1u64,
        });
        pay_avail_cnts.push(StatisticsItem {
            value: 200u64,
            num: 1u64,
        });

        let mut recv_avail_cnts = Vec::<StatisticsItem>::new();
        recv_avail_cnts.push(StatisticsItem {
            value: 500u64,
            num: 1u64,
        });

        let ans =
            ComputeCurrencyPlanA::new().find_currency_plan(pay_avail_cnts, recv_avail_cnts, 1700);
        assert_eq!(ans.is_ok(), true);
        assert_eq!(ans.clone().unwrap().pay_amount, 2200);
        assert_eq!(
            vec![
                StatisticsItem {
                    value: 2000u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 200u64,
                    num: 1u64
                }
            ],
            ans.clone().unwrap().pay_plan
        );
        assert_eq!(ans.clone().unwrap().recv_amount, 500);
        assert_eq!(
            vec![StatisticsItem {
                value: 500u64,
                num: 1u64
            }],
            ans.clone().unwrap().recv_plan
        );
    }

    /*
    测试双方找零策略B
        考虑付款方((20,1) (2,1)) 收款方(5,1) 要付17的情况
    */

    #[test]
    fn test_find_currency_planB() {
        let mut pay_avail_cnts = Vec::<StatisticsItem>::new();
        pay_avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 1u64,
        });
        pay_avail_cnts.push(StatisticsItem {
            value: 200u64,
            num: 1u64,
        });

        let mut recv_avail_cnts = Vec::<StatisticsItem>::new();
        recv_avail_cnts.push(StatisticsItem {
            value: 500u64,
            num: 1u64,
        });

        let ans =
            ComputeCurrencyPlanB::new().find_currency_plan(pay_avail_cnts, recv_avail_cnts, 1700);
        assert_eq!(ans.is_ok(), true);
        assert_eq!(ans.clone().unwrap().pay_amount, 2200);
        assert_eq!(
            vec![
                StatisticsItem {
                    value: 2000u64,
                    num: 1u64
                },
                StatisticsItem {
                    value: 200u64,
                    num: 1u64
                }
            ],
            ans.clone().unwrap().pay_plan
        );
        assert_eq!(ans.clone().unwrap().recv_amount, 500);
        assert_eq!(
            vec![StatisticsItem {
                value: 500u64,
                num: 1u64
            }],
            ans.clone().unwrap().recv_plan
        );
    }

    /*
    测试双方找零策略A

    */

    #[test]
    fn test_find_currency_plan_a_multi() {
        let mut pay_avail_cnts = Vec::<StatisticsItem>::new();
        pay_avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 100000000000u64,
        });

        let mut recv_avail_cnts = Vec::<StatisticsItem>::new();
        recv_avail_cnts.push(StatisticsItem {
            value: 1000u64,
            num: 20u64,
        });

        let ans = ComputeCurrencyPlanA::new().find_currency_plan(
            pay_avail_cnts,
            recv_avail_cnts,
            100009000,
        );
        assert_eq!(ans.is_ok(), true);
        assert_eq!(ans.clone().unwrap().pay_amount, 100010000);
        assert_eq!(
            vec![StatisticsItem {
                value: 2000u64,
                num: 50005u64
            }],
            ans.clone().unwrap().pay_plan
        );
        assert_eq!(ans.clone().unwrap().recv_amount, 1000);
        assert_eq!(
            vec![StatisticsItem {
                value: 1000u64,
                num: 1u64
            }],
            ans.clone().unwrap().recv_plan
        );
    }

    /*
    测试双方找零策略B

    */

    #[test]
    fn test_find_currency_plan_b_multi() {
        let mut pay_avail_cnts = Vec::<StatisticsItem>::new();
        pay_avail_cnts.push(StatisticsItem {
            value: 2000u64,
            num: 100000000000u64,
        });

        let mut recv_avail_cnts = Vec::<StatisticsItem>::new();
        recv_avail_cnts.push(StatisticsItem {
            value: 1000u64,
            num: 20u64,
        });

        let ans = ComputeCurrencyPlanB::new().find_currency_plan(
            pay_avail_cnts,
            recv_avail_cnts,
            100009000,
        );
        assert_eq!(ans.is_ok(), true);
        assert_eq!(ans.clone().unwrap().pay_amount, 100010000);
        assert_eq!(
            vec![StatisticsItem {
                value: 2000u64,
                num: 50005u64
            }],
            ans.clone().unwrap().pay_plan
        );
        assert_eq!(ans.clone().unwrap().recv_amount, 1000);
        assert_eq!(
            vec![StatisticsItem {
                value: 1000u64,
                num: 1u64
            }],
            ans.clone().unwrap().recv_plan
        );
    }
}
