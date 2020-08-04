table! {
    secret_store (uid) {
        uid -> Text,
        secret_type -> Text,
        seed -> Text,
        keypair -> Text,
        cert -> Text,
    }
}
