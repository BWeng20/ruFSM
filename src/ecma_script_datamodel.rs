#![allow(non_snake_case)]

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use boa_engine::{Context, JsResult, JsValue, property::Attribute};
use boa_engine::object::{FunctionBuilder, JsArray, JsMap};
use boa_engine::value::Type;
use log::{debug, error, info};

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
        debug!("Execute: {}", script);
        let mut r: String = "".to_string();
        for (name, value) in &self.data.values {
            self.context.register_global_property(name.as_str(), value.to_string(), Attribute::all());
        }
        let result = self.context.eval(script);
        match result {
            Ok(res) => {
                r = res.to_string(&mut self.context).unwrap().to_string()
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
        match self.context.eval(array_expression) {
            Ok(r) => {
                match r.get_type() {
                    Type::String => {
                        // Iterate through all chars
                    }
                    Type::Object => {
                        let obj = r.as_object().unwrap().to_owned();
                        if obj.is_array() {
                            let ja = JsArray::from_object(obj, &mut self.context).unwrap();
                            let N = ja.length(&mut self.context).unwrap() as i64;
                            for idx in 0..N {
                                match ja.at(idx, &mut self.context) {
                                    Ok(item) => {
                                        debug!("ForEach: #{} {}", idx, &js_to_string(&item, &mut self.context) );
                                        self.context.register_global_property(item_name.as_str(), item, Attribute::all());
                                        if !index.is_empty() {
                                            self.context.register_global_property(index.as_str(), idx, Attribute::all());
                                        }
                                        execute_body(self);
                                    }
                                    Err(_e) => {
                                        // @TODO Ignore, abort or log?
                                    }
                                }
                            }
                        } else if obj.is_map() {} else {
                            // Iterate through all members
                            let jm = JsMap::from_object(obj, &mut self.context).unwrap();
                            let mir = jm.values(&mut self.context);
                            match mir {
                                Ok(it) => {
                                    let mut e = it.next(&mut self.context);
                                    while e.is_ok() {
                                        e = it.next(&mut self.context);
                                    }
                                    todo!();
                                }
                                Err(e) => {
                                    let msg = format!("Failed to extract iterator. {}", js_to_string(&e, &mut self.context));
                                    self.log(&msg);
                                }
                            }
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

        todo!()
    }

    fn executeCondition(&mut self, fsm: &Fsm, script: &String) -> Result<bool, String> {
        match bool::from_str(self.execute_internal(fsm, script).as_str()) {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e.to_string()),
        }
    }

    fn executeContent(&mut self, fsm: &Fsm, content_id: ExecutableContentId) {
        for (idx, e) in fsm.executableContent.get(&content_id).unwrap().iter().enumerate() {
            info!("executeContent #{}/{}:", content_id, idx);
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



