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
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        address -> Varchar,
        zip_code -> Varchar,
        city -> Varchar,
        iban -> Varchar,
        country -> Varchar,
        fk_principal -> Int4,
    }
}

table! {
    refresh_token (pk) {
        pk -> Int4,
        uuid -> Uuid,
        expiry -> Timestamptz,
        invalidated -> Bool,
        fk_principal -> Int4,
    }
}

joinable!(qr_user -> principal (fk_principal));
joinable!(refresh_token -> principal (fk_principal));

allow_tables_to_appear_in_same_query!(
    principal,
    qr_user,
    refresh_token,
);
