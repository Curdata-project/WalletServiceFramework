table! {
    history_store (uid, txid) {
        uid -> Text,
        txid -> Text,
        trans_type -> SmallInt,
        oppo_uid -> Text,
        occur_time -> Timestamp,
        amount -> BigInt,
        balance -> BigInt,
        remark -> Text,
    }
}
