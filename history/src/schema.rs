table! {
    history_store (uid, txid) {
        uid -> Text,
        txid -> Text,
        transaction -> Text,
        status -> SmallInt,
        trans_type -> SmallInt,
        oppo_uid -> Text,
        occur_time -> Timestamp,
        amount -> BigInt,
        output -> BigInt,
        balance -> BigInt,
        remark -> Text,
    }
}
