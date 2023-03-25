use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use boa_engine::{Context, JsResult, JsValue, property::Attribute};

use crate::fsm::{Data, Datamodel, DataStore, ExecutableContentId, GlobalData};

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

static CONTEXT_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

struct ECMAScriptContext {
    pub global_data: Rc<RefCell<GlobalData>>,
    pub context: Context,
}

impl ECMAScriptContext {
    fn new() -> ECMAScriptContext {
        ECMAScriptContext {
            global_data: Rc::new(RefCell::new(GlobalData::new())),
            context: Context::default(),
        }
    }
}

thread_local!(
    static context_map: RefCell<HashMap<u32, Rc<RefCell<ECMAScriptContext>>>> = RefCell::new(HashMap::new());
);

#[derive(Debug)]
pub struct ECMAScriptDatamodel {
    pub data: DataStore,
    pub context_id: u32,
}

fn str(js: &JsValue, ctx: &mut Context) -> String {
    js.to_string(ctx).unwrap().to_string()
}


fn log_js(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    for arg in args {
        print!("{}", arg.to_string(ctx)?);
    }
    println!();
    Ok(JsValue::undefined())
}


impl ECMAScriptDatamodel {
    pub fn new() -> ECMAScriptDatamodel {
        let e = ECMAScriptDatamodel
        {
            data: DataStore::new(),
            context_id: CONTEXT_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
        };
        e
    }

    fn get_context(&self) -> Rc<RefCell<ECMAScriptContext>> {
        context_map.with(|c| {
            let mut ch = c.borrow_mut();
            if !ch.contains_key(&self.context_id) {
                ch.insert(self.context_id, Rc::new(RefCell::new(ECMAScriptContext::new())));
            }
            ch.get(&self.context_id).unwrap().clone()
        })
    }

    fn execute_internal(&mut self, script: &String) -> String {
        println!("Execute: {}", script);
        let mut r: String = "".to_string();
        context_map.with(|c| {
            let ctx: Rc<RefCell<ECMAScriptContext>> = c.borrow().get(&self.context_id).unwrap().clone();


            for (name, value) in &self.data.values {
                ctx.deref().borrow_mut().context.register_global_property(name.as_str(), value.to_string(), Attribute::all());
            }
            let result = ctx.deref().borrow_mut().context.eval(script);
            match result {
                Ok(res) => {
                    r = res.to_string(&mut ctx.deref().borrow_mut().context).unwrap().to_string()
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


/**
 * ECMAScript data model
 */
impl Datamodel for ECMAScriptDatamodel {
    fn global(&self) -> Rc<RefCell<GlobalData>> {
        context_map.with(|c| {
            let mut ch = c.borrow_mut();
            if !ch.contains_key(&self.context_id) {
                ch.insert(self.context_id, Rc::new(RefCell::new(ECMAScriptContext::new())));
            }

            let ext = ch.get(&self.context_id).unwrap().clone();
            let mut eeee = ext.borrow() as &RefCell<ECMAScriptContext>;
            let x = eeee.borrow().global_data.clone();
            x
        })
    }

    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
    }

    fn initializeDataModel(&mut self, data: &DataStore) {
        context_map.with(|c|
            {
                let mut ch = c.borrow_mut();
                let mut ecms_ctx = (ch.get(&self.context_id).unwrap().borrow() as &RefCell<ECMAScriptContext>).borrow_mut();

                ecms_ctx.context.register_global_builtin_function("log", 1, log_js);
                let cid = self.context_id;
                ecms_ctx.context.register_global_closure("In", 1, move |_this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    if args.len() > 0 {
                        context_map.with(|c| {
                            let cid2 = cid;
                            let ch = c.borrow();
                            let ecms_ctx = (**ch.get(&cid2).unwrap()).borrow();
                            let gd_rc: Rc<RefCell<GlobalData>> = ecms_ctx.global_data.clone();
                            let m = (gd_rc.borrow() as &RefCell<GlobalData>).borrow().statesNames.get(&str(&args[0], ctx)).cloned();
                            match m
                            {
                                Some(sid) => {
                                    Ok(JsValue::from(false))
                                }
                                None => {
                                    Ok(JsValue::from(false))
                                }
                            }
                        })
                    } else {
                        Err(JsValue::from("Missing argument"))
                    }
                });

                for (name, data) in &data.values
                {
                    self.data.values.insert(name.clone(), data.get_copy());
                    ecms_ctx.context.register_global_property(name.as_str(), data.to_string(), Attribute::all());
                }
            }
        )
    }

    fn set(self: &mut ECMAScriptDatamodel, name: &String, data: Box<dyn Data>) {
        self.data.set(name, data);
        // TODO: Set data also in the Context
    }

    fn get(self: &ECMAScriptDatamodel, name: &String) -> Option<&dyn Data> {
        match self.data.get(name) {
            Some(D) => {
                Some(&**D)
            }
            None => {
                None
            }
        }
    }

    fn clear(self: &mut ECMAScriptDatamodel) {}

    fn log(&mut self, msg: &String) {
        println!("Log: {}", msg);
    }

    fn execute(&mut self, script: &String) -> String {
        self.execute_internal(script)
    }

    fn executeForEach(&mut self, arrayExpression: &String, item: &String, index: &String, executeBody: &dyn FnOnce(&mut dyn Datamodel)) {
        todo!()
    }

    fn executeCondition(&mut self, script: &String) -> Result<bool, String> {
        match bool::from_str(self.execute_internal(script).as_str()) {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e.to_string()),
        }
    }

    fn executeContent(&mut self, content_id: ExecutableContentId) {
        let mut global = self.global();
        let mut ex = global.deref().borrow_mut();

        ex.executableContent.get_mut(&content_id).unwrap().execute(self);
    }
}