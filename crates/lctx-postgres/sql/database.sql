-- One database's bootstrap, run by a superuser after the four lctx roles exist (cutover plan
-- WP0.6/WP1.5). The single source for tests, `just pg-dev` and the operator's bootstrap.
DO $$ BEGIN
    IF current_setting('server_version_num')::int / 10000 <> 18 THEN
        RAISE EXCEPTION 'PostgreSQL 18 is required';
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname IN ('lctx_app', 'lctx_migrator', 'lctx_importer', 'lctx_serving')
               AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'an lctx role has elevated privileges';
    END IF;
END $$;
CREATE SCHEMA IF NOT EXISTS lctx_ext;
REVOKE ALL ON SCHEMA lctx_ext FROM PUBLIC;
CREATE EXTENSION IF NOT EXISTS vector WITH SCHEMA lctx_ext VERSION '0.8.6';
GRANT USAGE ON SCHEMA lctx_ext TO lctx_app, lctx_migrator, lctx_importer, lctx_serving;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT CREATE ON SCHEMA public TO lctx_migrator;
DO $$ BEGIN
    EXECUTE format('REVOKE ALL ON DATABASE %I FROM PUBLIC', current_database());
    EXECUTE format('GRANT CONNECT ON DATABASE %I TO lctx_app, lctx_migrator, lctx_importer, lctx_serving', current_database());
    EXECUTE format('GRANT CREATE ON DATABASE %I TO lctx_migrator', current_database());
    EXECUTE format('GRANT TEMP ON DATABASE %I TO lctx_importer', current_database());
END $$;
ALTER ROLE lctx_serving SET default_transaction_read_only = on;
