// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Text,
        displayName -> Text,
        email -> Text,
        accessLevel -> Integer,
        locked -> Bool,
        passcrypt -> Nullable<Text>,
    }
}
