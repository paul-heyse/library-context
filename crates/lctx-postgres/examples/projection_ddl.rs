fn main() {
    print!(
        "{}",
        if std::env::args().any(|a| a == "--evidence") {
            lctx_postgres::projection::evidence_ddl()
        } else if std::env::args().any(|a| a == "--specificity") {
            lctx_postgres::projection::specificity_ddl()
        } else if std::env::args().any(|a| a == "--catalog") {
            lctx_postgres::projection::catalog_ddl()
        } else {
            lctx_postgres::projection::relation_ddl()
        }
        .expect("declared PostgreSQL projection")
    );
}
