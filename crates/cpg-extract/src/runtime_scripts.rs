//! One declaration supplies embedded runtime bytes and producer source capture.
macro_rules! runtime_scripts {
    ($emit:ident) => {
        $emit! {
            DEPLOYMENT_CHECK => "scripts/deployment_check.py",
        }
    };
}
pub(crate) use runtime_scripts;
