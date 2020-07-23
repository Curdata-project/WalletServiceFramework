table! {
    currency_store (id) {
        id -> Text,
        jcurrency -> Text,
        txid -> Text,
        update_time -> Timestamp,
        last_owner_id -> Text,
        status -> SmallInt,
    }
}
