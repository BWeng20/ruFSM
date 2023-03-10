use std::ops::Deref;

use boa_engine::{
    Context,
    object::ObjectInitializer,
    property::{Attribute, PropertyDescriptor},
};

use crate::fsm::{Data, Datamodel, DataStore};

pub const ECMA_SCRIPT: &str = "ECMAScript";
pub const ECMA_SCRIPT_LC: &str = "ecmascript";

#[derive(Debug)]
pub struct ECMAScriptDatamodel {
    pub data: DataStore,

}

impl ECMAScriptDatamodel {
    pub fn new() -> ECMAScriptDatamodel {
        ECMAScriptDatamodel { data: DataStore::new() }
    }
}

/**
 * ECMAScript data model
 */
impl Datamodel for ECMAScriptDatamodel {
    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
    }

    fn initializeDataModel(&mut self, data: &DataStore) {
        for (name, data) in &data.values
        {
            self.data.values.insert(name.clone(), data.deref().get_copy());
        }
    }

    fn set(self: &mut ECMAScriptDatamodel, name: &String, data: Box<dyn Data>) {
        self.data.set(name, data);
    }

    fn get(self: &ECMAScriptDatamodel, name: &String) -> &dyn Data {
        self.data.get(name).deref()
    }

    fn clear(self: &mut ECMAScriptDatamodel) {}

    fn log(&mut self, msg: &String) {
        println!("Log: {}", msg);
    }

    fn execute(&mut self, script: &String) -> &str {
        println!("Execute: {}", script);
        ""
    }
}