table! {
    principal (pk) {
        pk -> Int4,
        user_name -> Varchar,
        password -> Varchar,
        fk_user -> Int4,
    }
}

table! {
    qr_user (pk) {
        pk -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        address -> Varchar,
        zip_code -> Varchar,
        city -> Varchar,
        iban -> Varchar,
        country -> Varchar,
    }
}

joinable!(principal -> qr_user (fk_user));

allow_tables_to_appear_in_same_query!(
    principal,
    qr_user,
);
