fn main() {
    print!(
        "{}",
        lctx_postgres::projection::relation_ddl().expect("declared PostgreSQL projection")
    );
}
