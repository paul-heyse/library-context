//! `store check` (cutover plan P1.6): the live catalog against a shadow install of this binary's
//! lowering, the provisioning contract (roles and database privileges) and the owner's inventory.
//!
//! The check holds the installation lock exclusively. Every lifecycle transition, pin and copy
//! takes it shared, so nothing changes a generation while the check reads. A REPEATABLE READ
//! snapshot would not suffice: `pg_get_*def` and `format_type` read the latest catalog, not the
//! transaction snapshot. Shadows are created inside the check's transaction, which always rolls
//! back.
use std::collections::{BTreeMap, BTreeSet};
use lctx_model::domain::ValidatedModel;
use sqlx::{Connection, PgConnection};
use super::{Error, GenerationId, ddl::{self, CONTROL, STATES}};
use crate::{OwnerPool, RUNTIME_ROLES};

const SHADOW_CONTROL: &str = "lctx_check_control";
const SHADOW_GENERATION: GenerationId = GenerationId([0x5a; 16]);
const SERVICE_SCHEMAS: [&str; 2] = ["lctx_cache", "lctx_ops"];
/// The provisioning contract's database privileges, beyond the owner's own.
const DATABASE_PRIVILEGES: [(&str, &str); 4] =
    [("lctx_app", "CONNECT"), ("lctx_importer", "CONNECT"), ("lctx_importer", "TEMPORARY"), ("lctx_serving", "CONNECT")];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingKind {
    /// The store or the service baseline is absent, or was installed by a different binary.
    Installation,
    /// A provisioned role is missing, elevated or joined to another role, or its settings drifted.
    Role,
    /// The database's owner or privileges differ from the provisioning contract.
    Database,
    /// An object the lowering declares is absent.
    Missing,
    /// An object nothing declares is present.
    Unexpected,
    /// An object's definition, owner or privileges differ from the lowering.
    Differs,
    /// A registry row without its schema, or a generation schema without its registry row.
    Orphan,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding { pub kind: FindingKind, pub subject: String, pub detail: String }
impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?} {}: {}", self.kind, self.subject, self.detail) }
}
#[derive(Debug, Default)]
pub struct CheckReport { pub generations: usize, pub findings: Vec<Finding> }
impl CheckReport {
    pub fn clean(&self) -> bool { self.findings.is_empty() }
}

struct Findings(Vec<Finding>);
impl Findings {
    fn push(&mut self, kind: FindingKind, subject: impl Into<String>, detail: impl Into<String>) {
        self.0.push(Finding { kind, subject: subject.into(), detail: detail.into() });
    }
}

pub(super) async fn check(owner: &OwnerPool, model: &ValidatedModel) -> Result<CheckReport, Error> {
    let mut connection = owner.pool().acquire().await?;
    let mut tx = connection.begin().await?;
    let report = inspect(&mut tx, model).await;
    tx.rollback().await?;
    report
}

async fn inspect(tx: &mut PgConnection, model: &ValidatedModel) -> Result<CheckReport, Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(1279476824,0)").execute(&mut *tx).await?;
    let mut findings = Findings(Vec::new());
    roles(tx, &mut findings).await?;
    database(tx, &mut findings).await?;
    if !crate::history_current(tx).await? {
        findings.push(FindingKind::Installation, "service baseline", "the migration history differs from this binary's");
    }
    let physical = ddl::physical_digest(model);
    let installed: bool = sqlx::query_scalar("SELECT to_regclass('lctx_model_store.installation') IS NOT NULL").fetch_one(&mut *tx).await?;
    let mut registered = Vec::new();
    let mut compare = installed;
    if installed {
        let digests: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation WHERE singleton")
            .fetch_optional(&mut *tx).await?;
        if digests != Some((model.digest().0.to_vec(), physical.0.to_vec())) {
            findings.push(FindingKind::Installation, CONTROL, "installed from a different model or lowering; generation shapes were not compared");
            compare = false;
        }
        let rows: Vec<(Vec<u8>, String, Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT id, state, model_digest, physical_digest FROM lctx_model_store.generations ORDER BY id")
            .fetch_all(&mut *tx).await?;
        for (id, state, m, p) in rows {
            let g = GenerationId(id.try_into().map_err(|_| Error::Codec("generation id length".into()))?);
            if m != model.digest().0 || p != physical.0 {
                findings.push(FindingKind::Differs, g.schema(), "registered under a different model or lowering");
            } else if !STATES.contains(&state.as_str()) {
                findings.push(FindingKind::Differs, g.schema(), format!("unknown state {state}"));
            } else {
                registered.push((g, state));
            }
        }
    } else {
        findings.push(FindingKind::Installation, CONTROL, "the generation store is not installed");
    }
    let generations = registered.len();
    let registered = inventory(tx, &mut findings, registered, installed).await?;
    if compare {
        sqlx::raw_sql(sqlx::AssertSqlSafe(ddl::control(SHADOW_CONTROL))).execute(&mut *tx).await?;
        let live = describe(tx, CONTROL, &[(CONTROL, "<control>")]).await?;
        let shadow = describe(tx, SHADOW_CONTROL, &[(SHADOW_CONTROL, "<control>")]).await?;
        compare_objects(&mut findings, CONTROL, &shadow, &live);
        let mut shadows = BTreeMap::new();
        for state in registered.iter().map(|(_, state)| state.as_str()).collect::<BTreeSet<_>>() {
            let schema = format!("lctx_check_{state}");
            let lowering = ddl::lower(model, SHADOW_GENERATION, &schema, SHADOW_CONTROL);
            for sql in lowering.through(state) { sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *tx).await?; }
            let hex = SHADOW_GENERATION.hex();
            shadows.insert(state, describe(tx, &schema, &[(&schema, "<schema>"), (&hex, "<generation>"), (SHADOW_CONTROL, "<control>")]).await?);
        }
        for (g, state) in &registered {
            let (schema, hex) = (g.schema(), g.hex());
            let live = describe(tx, &schema, &[(&schema, "<schema>"), (&hex, "<generation>"), (CONTROL, "<control>")]).await?;
            compare_objects(&mut findings, &schema, &shadows[state.as_str()], &live);
        }
    }
    let mut findings = findings.0;
    findings.sort();
    Ok(CheckReport { generations, findings })
}

/// Every provisioned role logs in, holds no elevated attribute and shares no membership edge;
/// the reader's transactions default to read-only.
async fn roles(tx: &mut PgConnection, findings: &mut Findings) -> Result<(), Error> {
    let owner: String = sqlx::query_scalar("SELECT current_user::text").fetch_one(&mut *tx).await?;
    let names: Vec<String> = RUNTIME_ROLES.iter().map(|r| r.to_string()).chain([owner]).collect();
    let rows: Vec<(String, bool, bool, bool, bool, bool, bool)> = sqlx::query_as("SELECT rolname::text, rolcanlogin, rolsuper, rolcreatedb, rolcreaterole, \
        rolreplication, rolbypassrls FROM pg_roles WHERE rolname = ANY($1)").bind(&names).fetch_all(&mut *tx).await?;
    for name in &names {
        let Some(&(_, login, superuser, createdb, createrole, replication, bypassrls)) = rows.iter().find(|r| &r.0 == name) else {
            findings.push(FindingKind::Role, name.clone(), "not provisioned");
            continue;
        };
        if !login { findings.push(FindingKind::Role, name.clone(), "cannot log in"); }
        for (held, attribute) in [(superuser, "SUPERUSER"), (createdb, "CREATEDB"), (createrole, "CREATEROLE"), (replication, "REPLICATION"), (bypassrls, "BYPASSRLS")] {
            if held { findings.push(FindingKind::Role, name.clone(), format!("holds {attribute}")); }
        }
    }
    let edges: Vec<(String, String)> = sqlx::query_as("SELECT granted.rolname::text, member.rolname::text FROM pg_auth_members m \
        JOIN pg_roles granted ON granted.oid = m.roleid JOIN pg_roles member ON member.oid = m.member \
        WHERE granted.rolname = ANY($1) OR member.rolname = ANY($1) ORDER BY 1, 2").bind(&names).fetch_all(&mut *tx).await?;
    for (granted, member) in edges {
        findings.push(FindingKind::Role, format!("{member} -> {granted}"), "membership edge");
    }
    let read_only: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_db_role_setting s JOIN pg_roles r ON r.oid = s.setrole \
        WHERE r.rolname = 'lctx_serving' AND s.setdatabase IN (0, (SELECT oid FROM pg_database WHERE datname = current_database())) \
        AND 'default_transaction_read_only=on' = ANY(s.setconfig))").fetch_one(&mut *tx).await?;
    if !read_only { findings.push(FindingKind::Role, "lctx_serving", "transactions do not default to read-only"); }
    Ok(())
}

/// The owner owns the database; beyond it, exactly the provisioned CONNECT/TEMPORARY grants.
async fn database(tx: &mut PgConnection, findings: &mut Findings) -> Result<(), Error> {
    let owned: bool = sqlx::query_scalar("SELECT pg_get_userbyid(datdba)::text = current_user::text FROM pg_database WHERE datname = current_database()")
        .fetch_one(&mut *tx).await?;
    if !owned { findings.push(FindingKind::Database, "owner", "the store owner does not own the database"); }
    let granted: BTreeSet<(String, String)> = sqlx::query_as("SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee)::text END, \
        a.privilege_type::text FROM pg_database d, aclexplode(COALESCE(d.datacl, acldefault('d', d.datdba))) a \
        WHERE d.datname = current_database() AND a.grantee <> d.datdba").fetch_all(&mut *tx).await?.into_iter().collect();
    let expected: BTreeSet<(String, String)> = DATABASE_PRIVILEGES.iter().map(|(r, p)| (r.to_string(), p.to_string())).collect();
    for (role, privilege) in expected.difference(&granted) { findings.push(FindingKind::Database, format!("{role} {privilege}"), "privilege missing"); }
    for (role, privilege) in granted.difference(&expected) { findings.push(FindingKind::Database, format!("{role} {privilege}"), "privilege not provisioned"); }
    Ok(())
}

/// Classify every schema the owner owns, and owner objects in `public`. Returns the registered
/// generations whose schemas exist and belong to the owner.
async fn inventory(tx: &mut PgConnection, findings: &mut Findings, registered: Vec<(GenerationId, String)>, installed: bool)
    -> Result<Vec<(GenerationId, String)>, Error> {
    let owned: BTreeSet<String> = sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)")
        .fetch_all(&mut *tx).await?.into_iter().collect();
    let mut present = Vec::new();
    for (g, state) in registered {
        if owned.contains(&g.schema()) { present.push((g, state)); continue; }
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1)").bind(g.schema()).fetch_one(&mut *tx).await?;
        if exists { findings.push(FindingKind::Differs, g.schema(), "the schema belongs to another role"); }
        else { findings.push(FindingKind::Orphan, g.schema(), "registered without its schema"); }
    }
    let known: BTreeSet<String> = present.iter().map(|(g, _)| g.schema()).collect();
    for schema in &owned {
        if known.contains(schema) || SERVICE_SCHEMAS.contains(&schema.as_str()) || (installed && schema == CONTROL) { continue; }
        if GenerationId::from_schema(schema).is_some() { findings.push(FindingKind::Orphan, schema.clone(), "a generation schema without a registry row"); }
        else { findings.push(FindingKind::Unexpected, schema.clone(), "a schema owned by the store owner"); }
    }
    let public: Vec<String> = sqlx::query_scalar("SELECT c.relname::text FROM pg_class c WHERE c.relnamespace = 'public'::regnamespace \
        AND c.relowner = (SELECT oid FROM pg_roles WHERE rolname = current_user) AND c.relname NOT IN ('_sqlx_migrations', '_sqlx_migrations_pkey') ORDER BY 1")
        .fetch_all(&mut *tx).await?;
    for relation in public { findings.push(FindingKind::Unexpected, format!("public.{relation}"), "an object owned by the store owner"); }
    Ok(present)
}

const DESCRIBE: &str = "WITH s AS (SELECT oid, nspowner, nspacl FROM pg_namespace WHERE nspname = $1)
SELECT 'schema'::text, ''::text, pg_get_userbyid(s.nspowner)::text || ' ' || COALESCE(s.nspacl::text, '') FROM s
UNION ALL
SELECT 'relation', c.relname::text, c.relkind::text || ' ' || c.relpersistence::text || ' ' || pg_get_userbyid(c.relowner)::text || ' '
    || COALESCE(c.relacl::text, '') || ' rls=' || c.relrowsecurity::text || '/' || c.relforcerowsecurity::text
  FROM pg_class c JOIN s ON c.relnamespace = s.oid
UNION ALL
SELECT 'column', c.relname || '.' || a.attname, a.attnum::text || ' ' || format_type(a.atttypid, a.atttypmod) || ' notnull=' || a.attnotnull::text
    || ' generated=' || a.attgenerated::text || ' identity=' || a.attidentity::text || ' default=' || COALESCE(pg_get_expr(d.adbin, d.adrelid), '')
    || ' collation=' || a.attcollation::regcollation::text || ' acl=' || COALESCE(a.attacl::text, '')
  FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid JOIN s ON c.relnamespace = s.oid
  LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
  WHERE a.attnum > 0 AND NOT a.attisdropped
UNION ALL
SELECT 'constraint', c.relname || '.' || k.conname, k.contype::text || ' validated=' || k.convalidated::text || ' ' || pg_get_constraintdef(k.oid)
  FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid JOIN s ON c.relnamespace = s.oid
UNION ALL
SELECT 'index', c.relname::text, pg_get_indexdef(c.oid) FROM pg_class c JOIN s ON c.relnamespace = s.oid WHERE c.relkind IN ('i', 'I')
UNION ALL
SELECT 'view', c.relname::text, pg_get_viewdef(c.oid) FROM pg_class c JOIN s ON c.relnamespace = s.oid WHERE c.relkind IN ('v', 'm')
UNION ALL
SELECT 'function', p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
    pg_get_userbyid(p.proowner)::text || ' ' || COALESCE(p.proacl::text, '') || ' ' || pg_get_functiondef(p.oid)
  FROM pg_proc p JOIN s ON p.pronamespace = s.oid
UNION ALL
SELECT 'trigger', c.relname || '.' || t.tgname, pg_get_triggerdef(t.oid)
  FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN s ON c.relnamespace = s.oid WHERE NOT t.tgisinternal
UNION ALL
SELECT 'policy', c.relname || '.' || p.polname, p.polcmd::text || ' ' || COALESCE(pg_get_expr(p.polqual, p.polrelid), '') || ' '
    || COALESCE(pg_get_expr(p.polwithcheck, p.polrelid), '')
  FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid JOIN s ON c.relnamespace = s.oid
UNION ALL
SELECT 'type', t.typname::text, t.typtype::text || ' ' || pg_get_userbyid(t.typowner)::text
  FROM pg_type t JOIN s ON t.typnamespace = s.oid WHERE t.typtype IN ('d', 'e', 'r', 'm')";

/// A schema's catalog descriptors keyed by (kind, name), with schema-specific names replaced by
/// placeholders so a live schema and its shadow compare equal.
async fn describe(tx: &mut PgConnection, schema: &str, replacements: &[(&str, &str)]) -> Result<BTreeMap<(String, String), String>, Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(DESCRIBE).bind(schema).fetch_all(&mut *tx).await?;
    let normalize = |text: String| replacements.iter().fold(text, |text, (from, to)| text.replace(from, to));
    Ok(rows.into_iter().map(|(kind, name, detail)| ((kind, normalize(name)), normalize(detail))).collect())
}

fn compare_objects(findings: &mut Findings, schema: &str, expected: &BTreeMap<(String, String), String>, live: &BTreeMap<(String, String), String>) {
    for ((kind, name), definition) in expected {
        match live.get(&(kind.clone(), name.clone())) {
            None => findings.push(FindingKind::Missing, format!("{schema} {kind} {name}"), definition.clone()),
            Some(actual) if actual != definition => findings.push(FindingKind::Differs, format!("{schema} {kind} {name}"), format!("expected `{definition}`, found `{actual}`")),
            Some(_) => {}
        }
    }
    for ((kind, name), definition) in live {
        if !expected.contains_key(&(kind.clone(), name.clone())) {
            findings.push(FindingKind::Unexpected, format!("{schema} {kind} {name}"), definition.clone());
        }
    }
}
