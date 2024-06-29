#[macro_export]
macro_rules! get_and_insert_if_needed {
    ($connection:ident, $table_name:ident, $model:ident, $record_address:expr, $record_fields:expr) => {{
        let records = $table_name::dsl::$table_name
            .filter($table_name::address.eq($record_address))
            .select($model::as_select())
            .limit(1)
            .load($connection)?;

        let record_id = if records.len() == 0 {
            insert!(
                $connection,
                $table_name,
                $model,
                $record_address,
                $record_fields
            )
        } else {
            records.first().unwrap().id
        };

        record_id
    }};
}

#[macro_export]
macro_rules! insert {
    ($connection:ident, $table_name:ident, $model:ident, $record_address:expr, $record_fields:expr) => {{
        let record_id = diesel::insert_into($table_name::table)
            .values(vec![$record_fields])
            .returning($table_name::id)
            .get_result($connection)?;

        record_id
    }};
}
