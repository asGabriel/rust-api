#[macro_export]
macro_rules! push_filter {
    ($builder:expr, $has_where:expr, $condition:expr, $value:expr) => {
        $builder.push(if *$has_where { " AND " } else { " WHERE " });
        $builder.push($condition);
        $builder.push_bind($value);
        *$has_where = true;
    };
}
