//! Added custom actions \
//! As reference each type and method has the w3c description as documentation.\

#![allow(clippy::doc_lazy_continuation)]
#![allow(dead_code)]

use crate::datamodel::GlobalDataAccess;

pub struct ActionContext {
    pub global: GlobalDataAccess,

}

/// Trait to inject custom actions into the datamodel.
pub trait Action {

    /// Executes the action.\
    fn execute(&mut self, context: &mut ActionContext) -> &mut Result<String,String>;
}