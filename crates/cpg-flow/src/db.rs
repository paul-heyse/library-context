//! The salsa database ty's semantic index runs in: an in-memory file system holding only the
//! modules we hand it, ty's vendored typeshed, and empty program settings. Nothing is discovered
//! from the environment (ADR-0022 §The flow provider).

use ruff_db::Db as SourceDb;
use ruff_db::files::Files;
use ruff_db::system::{DbWithTestSystem, System, TestSystem};
use ruff_db::vendored::VendoredFileSystem;

#[salsa::db]
#[derive(Clone)]
pub(crate) struct FlowDb {
    storage: salsa::Storage<Self>,
    files: Files,
    system: TestSystem,
    vendored: VendoredFileSystem,
}

impl FlowDb {
    pub(crate) fn new() -> Self {
        FlowDb {
            storage: salsa::Storage::new(None),
            files: Files::default(),
            system: TestSystem::default(),
            vendored: ty_vendored::file_system().clone(),
        }
    }
}

impl DbWithTestSystem for FlowDb {
    fn test_system(&self) -> &TestSystem {
        &self.system
    }

    fn test_system_mut(&mut self) -> &mut TestSystem {
        &mut self.system
    }
}

#[salsa::db]
impl SourceDb for FlowDb {
    fn vendored(&self) -> &VendoredFileSystem {
        &self.vendored
    }

    fn system(&self) -> &dyn System {
        &self.system
    }

    fn files(&self) -> &Files {
        &self.files
    }
}

#[salsa::db]
impl ty_module_resolver::Db for FlowDb {}

#[salsa::db]
impl ty_python_core::Db for FlowDb {
    fn should_check_file(&self, _file: ruff_db::files::File) -> bool {
        true
    }
}

#[salsa::db]
impl salsa::Database for FlowDb {}
