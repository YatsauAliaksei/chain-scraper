
// use diesel::__diesel_parse_table;

diesel::table! {
    contracts {
        id -> Integer,
        address -> Nullable<Text>,
        abi_json -> Text,
    }
}