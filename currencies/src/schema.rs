table! {
    currency_store (id) {
        id -> Text,
        owner_uid -> Text,
        amount -> BigInt,
        currency -> Text,
        txid -> Text,
        update_time -> Timestamp,
        last_owner_id -> Text,
        status -> SmallInt,
    }
}
