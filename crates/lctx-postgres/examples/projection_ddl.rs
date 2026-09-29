fn main() {
    print!(
        "{}",
        lctx_postgres::projection::relation_ddl().expect("current declared PostgreSQL projection")
    );
}
