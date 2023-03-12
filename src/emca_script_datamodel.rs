use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use boa_engine::{Context, JsResult, JsValue, object::ObjectInitializer, property::Attribute};

use crate::fsm::{Data, Datamodel, DataStore};

pub const ECMA_SCRIPT: &str = "ECMAScript";
pub const ECMA_SCRIPT_LC: &str = "ecmascript";


pub struct JsonData {
    pub value: String,
}

impl Debug for JsonData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl ToString for JsonData {
    fn to_string(&self) -> String {
        self.value.clone()
    }
}

impl Data for JsonData {
    fn get_copy(&self) -> Box<dyn Data> {
        Box::new(JsonData {
            value: self.value.clone(),
        })
    }
}


#[derive(Debug)]
pub struct ECMAScriptDatamodel {
    pub data: DataStore,
    pub context_id: u32,
}

fn logJS(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    for arg in args {
        print!("{}", arg.to_string(ctx)?);
    }
    println!();
    Ok(JsValue::undefined())
}


impl ECMAScriptDatamodel {
    pub fn new() -> ECMAScriptDatamodel {
        ECMAScriptDatamodel
        {
            data: DataStore::new(),
            context_id: CONTEXT_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn execute_internal(&self, script: &String) -> String {
        println!("Execute: {}", script);
        let mut r: String = "".to_string();
        context_map.with(|c| {
            let mut cb = c.borrow_mut();
            let ctx: &mut Context = cb.get_mut(&self.context_id).unwrap();
            for (name, value) in &self.data.values {
                ctx.register_global_property(name.as_str(), value.to_string(), Attribute::all());
            }
            match ctx.eval(script) {
                Ok(res) => {
                    r = res.to_string(ctx).unwrap().to_string();
                }
                Err(e) => {
                    // Pretty print the error
                    eprintln!("Script Error {}", e.display());
                }
            }
        });
        r
    }
}

static CONTEXT_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

thread_local!(
    static context_map: RefCell<HashMap<u32, Box<Context>>> = RefCell::new(HashMap::new());
);

/**
 * ECMAScript data model
 */
impl Datamodel for ECMAScriptDatamodel {
    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
    }

    fn initializeDataModel(&mut self, data: &DataStore) {
        context_map.with(|c|
            {
                let mut ctx_map = c.borrow_mut();
                ctx_map.insert(self.context_id, Box::new(Context::default()));
                let ctx = ctx_map.get_mut(&self.context_id).unwrap();
                ctx.register_global_builtin_function("log", 1, logJS);

                for (name, data) in &data.values
                {
                    self.data.values.insert(name.clone(), data.get_copy());
                    ctx.register_global_property(name.as_str(), data.to_string(), Attribute::all());
                }
            });
    }

    fn set(self: &mut ECMAScriptDatamodel, name: &String, data: Box<dyn Data>) {
        context_map.with(|c| {
            let mut cb = c.borrow_mut();
            if cb.contains_key(&self.context_id) {
                let ctx: &mut Context = cb.get_mut(&self.context_id).unwrap();
                ctx.register_global_property(name.as_str(), data.to_string(), Attribute::all());
            }
        });
        self.data.set(name, data);
    }

    fn get(self: &ECMAScriptDatamodel, name: &String) -> &dyn Data {
        self.data.get(name).deref()
    }

    fn clear(self: &mut ECMAScriptDatamodel) {}

    fn log(&mut self, msg: &String) {
        println!("Log: {}", msg);
    }

    fn execute(&mut self, script: &String) -> String {
        self.execute_internal(script)
    }


    fn executeCondition(&self, script: &String) -> Result<bool, String> {
        match bool::from_str(self.execute_internal(script).as_str()) {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e.to_string()),
        }
    }
}