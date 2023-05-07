//! Implements the SCXML Datamodel for ECMA with Boa Engine.
//! See [W3C:The ECMAScript Data Model](https://www.w3.org/TR/scxml/#ecma-profile).
//! See [Github:Boa Engine](https://github.com/boa-dev/boa).

#![allow(non_snake_case)]

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use boa_engine::{Context, JsResult, JsValue, property::Attribute};
use boa_engine::object::FunctionBuilder;
use boa_engine::value::Type;
use log::{debug, error, info, warn};

use crate::executable_content::{DefaultExecutableContentTracer, ExecutableContentTracer};
use crate::fsm::{Data, Datamodel, DataStore, ExecutableContentId, Fsm, GlobalData, State, StateId};

pub const ECMA_SCRIPT: &str = "ECMAScript";
pub const ECMA_SCRIPT_LC: &str = "ecmascript";


static CONTEXT_ID_COUNTER: AtomicU32 = AtomicU32::new(1);


pub struct ECMAScriptDatamodel {
    pub data: DataStore,
    pub context_id: u32,
    pub global_data: GlobalData,
    pub context: Context,
    pub tracer: Option<Box<dyn ExecutableContentTracer>>,
}

fn js_to_string(jv: &JsValue, ctx: &mut Context) -> String {
    match jv.to_string(ctx) {
        Ok(s) => {
            s.to_string()
        }
        Err(_e) => {
            jv.display().to_string()
        }
    }
}


fn log_js(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let mut msg = String::new();
    for arg in args {
        msg.push_str(js_to_string(arg, ctx).as_str());
    }
    info!("{}", msg);
    Ok(JsValue::from(msg))
}


impl ECMAScriptDatamodel {
    pub fn new() -> ECMAScriptDatamodel {
        let e = ECMAScriptDatamodel
        {
            data: DataStore::new(),
            context_id: CONTEXT_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            global_data: GlobalData::new(),
            context: Context::default(),
            tracer: Some(Box::new(DefaultExecutableContentTracer::new())),
        };
        e
    }


    fn execute_internal(&mut self, _fsm: &Fsm, script: &String) -> String {
        let mut r: String = "".to_string();
        for (name, value) in &self.data.values {
            self.context.register_global_property(name.as_str(), value.to_string(), Attribute::all());
        }
        let result = self.context.eval(script);
        match result {
            Ok(res) => {
                r = res.to_string(&mut self.context).unwrap().to_string();
                debug!("Execute: {} => {}", script, r);
            }
            Err(e) => {
                // Pretty print the error
                error!("Script Error {}", e.display());
            }
        }
        r
    }
}

/**
 * ECMAScript data model
 */
impl Datamodel for ECMAScriptDatamodel {
    fn global(&mut self) -> &mut GlobalData {
        &mut self.global_data
    }
    fn global_s(&self) -> &GlobalData {
        &self.global_data
    }

    fn get_name(self: &Self) -> &str {
        return ECMA_SCRIPT;
    }

    fn initializeDataModel(&mut self, fsm: &mut Fsm, data_state: StateId) {
        let mut s = Vec::new();
        for (sn, _sid) in &fsm.statesNames {
            s.push(sn.clone());
        }

        let state_obj: &mut State = fsm.get_state_by_id_mut(data_state);

        self.context.register_global_builtin_function("log", 1, log_js);

        FunctionBuilder::closure_with_captures(&mut self.context,
                                               move |_this: &JsValue, args: &[JsValue], names: &mut Vec<String>, ctx: &mut Context| {
                                                   if args.len() > 0 {
                                                       let name = &js_to_string(&args[0], ctx);
                                                       let m = names.contains(name);
                                                       Ok(JsValue::from(m))
                                                   } else {
                                                       Err(JsValue::from("Missing argument"))
                                                   }
                                               }, s).name("In").length(1).build();

        for (name, data) in &state_obj.data.values
        {
            self.data.values.insert(name.clone(), data.get_copy());
            self.context.register_global_property(name.as_str(), data.to_string(), Attribute::all());
        }
    }

    fn set(self: &mut ECMAScriptDatamodel, name: &String, data: Box<dyn Data>) {
        self.data.set(name, data);
        // TODO: Set data also in the Context
    }

    fn get(self: &ECMAScriptDatamodel, name: &String) -> Option<&dyn Data> {
        match self.data.get(name) {
            Some(data) => {
                Some(&**data)
            }
            None => {
                None
            }
        }
    }

    fn clear(self: &mut ECMAScriptDatamodel) {}

    fn log(&mut self, msg: &String) {
        info!("Log: {}", msg);
    }

    fn execute(&mut self, fsm: &Fsm, script: &String) -> String {
        self.execute_internal(fsm, script)
    }

    fn executeForEach(&mut self, _fsm: &Fsm, array_expression: &String, item_name: &String, index: &String,
                      execute_body: &mut dyn FnMut(&mut dyn Datamodel)) {
        debug!("ForEach: array: {}", array_expression );
        match self.context.eval(array_expression) {
            Ok(r) => {
                match r.get_type() {
                    Type::Object => {
                        let obj = r.as_object().unwrap();
                        // Iterate through all members
                        let ob = obj.borrow();
                        let p = ob.properties();
                        let mut idx: i64 = 0;
                        for item_prop in p.values() {
                            match item_prop.value() {
                                Some(item) => {
                                    debug!("ForEach: #{} {}", idx, &js_to_string(&item, &mut self.context) );
                                    self.context.register_global_property(item_name.as_str(), item, Attribute::all());
                                    if !index.is_empty() {
                                        self.context.register_global_property(index.as_str(), idx, Attribute::all());
                                    }
                                    execute_body(self);
                                }
                                None => {
                                    warn!("ForEach: #{} - failed to get value", idx, );
                                }
                            }
                            idx = idx + 1;
                        }
                    }
                    _ => {
                        self.log(&"Resulting value is not a supported collection.".to_string());
                    }
                }
            }
            Err(e) => {
                self.log(&e.display().to_string());
            }
        }
    }


    fn executeCondition(&mut self, fsm: &Fsm, script: &String) -> Result<bool, String> {
        match bool::from_str(self.execute_internal(fsm, script).as_str()) {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e.to_string()),
        }
    }

    fn executeContent(&mut self, fsm: &Fsm, content_id: ExecutableContentId) {
        for (_idx, e) in fsm.executableContent.get(&content_id).unwrap().iter().enumerate() {
            match &mut self.tracer {
                Some(t) => {
                    e.trace(t.as_mut(), fsm);
                }
                None => {}
            }
            e.execute(self, fsm);
        }
    }
}



