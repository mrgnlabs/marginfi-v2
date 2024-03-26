#[macro_export]
macro_rules! insert_if_needed {
    ($connection:ident, $table_name:ident, $model:ident, $record_address:expr, $record_fields:expr) => {{
        let records = $table_name::dsl::$table_name
            .filter($table_name::address.eq($record_address))
            .select($model::as_select())
            .limit(1)
            .load($connection)?;

        let record_id = if records.len() == 0 {
            diesel::insert_into($table_name::table)
                .values(vec![$record_fields])
                .on_conflict_do_nothing()
                .returning($table_name::id)
                .get_result($connection)?
        } else {
            records.first().unwrap().id
        };

        record_id
    }};
}
