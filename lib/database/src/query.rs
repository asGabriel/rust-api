#[macro_export]
macro_rules! push_filter {
    // For arrays/vectors with IN clause: push_filter!(query, &mut has_where, "id", " IN", ids);
    ($builder:expr, $has_where:expr, $column:expr, " IN", $values:expr) => {{
        let values_iter = $values.iter();
        let vec: Vec<_> = values_iter.collect();
        if !vec.is_empty() {
            $builder.push(if *$has_where { " AND " } else { " WHERE " });
            $builder.push($column);
            $builder.push(" IN (");
            let mut separated = $builder.separated(", ");
            for value in $values {
                separated.push_bind(value);
            }
            separated.push_unseparated(")");
            *$has_where = true;
        }
    }};
    // For single values with operators (>=, <=, =, etc.)
    ($builder:expr, $has_where:expr, $condition:expr, $value:expr) => {
        $builder.push(if *$has_where { " AND " } else { " WHERE " });
        $builder.push($condition);
        $builder.push_bind($value);
        *$has_where = true;
    };
}
