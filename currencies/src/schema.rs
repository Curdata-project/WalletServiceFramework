table! {
    currency_store (id) {
        id -> Text,
        owner_uid -> Text,
        value -> BigInt,
        currency -> Text,
        txid -> Text,
        update_time -> Timestamp,
        last_owner_id -> Text,
        status -> SmallInt,
    }
}
