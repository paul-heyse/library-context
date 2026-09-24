//! The MDX components Stage F reads, declared once (the holistic assessment's A3; R1 F5) and read
//! by Stage F and by the rules that check it.
//!
//! These are **Mintlify's** documentation components, the convention FastMCP's docs use. A library
//! documented another way (Sphinx, MkDocs, plain Markdown) has none of them, so its briefs carry no
//! component-sourced warning or parameter description: there, a zero means "no such source", not
//! "no warnings".

/// A caution the documentation states: `<Warning>`, a Limits entry (§10.3).
pub const WARNING: &str = "Warning";
/// One parameter's documentation: `<ParamField>`, a Controls description (§10.3).
pub const PARAM_FIELD: &str = "ParamField";
/// The literal attribute naming a `<ParamField>`'s parameter. Fields named by `path`, `query` or
/// `header` document HTTP inputs, not a Python parameter, and are not read (R1-3).
pub const PARAM_NAME: &str = "body";
/// A component's literal title, shown with the warning it heads.
pub const TITLE: &str = "title";
