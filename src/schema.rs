table! {
    principal (pk) {
        pk -> Int4,
        user_name -> Varchar,
        password -> Varchar,
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
        fk_principal -> Int4,
    }
}

joinable!(qr_user -> principal (fk_principal));

allow_tables_to_appear_in_same_query!(principal, qr_user,);
