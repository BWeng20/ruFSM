#![allow(non_snake_case)]

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use boa_engine::{Context, JsResult, JsValue, property::Attribute};
use boa_engine::object::FunctionBuilder;
use log::{debug, info};

use crate::executable_content::{DefaultExecutableContentTracer, ExecutableContent, ExecutableContentTracer};
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
            global_data: GlobalData::new(),
            context: Context::default(),
            tracer: Some(Box::new(DefaultExecutableContentTracer::new())),
        };
        e
    }


    fn execute_internal(&mut self, _fsm: &Fsm, script: &String) -> String {
        println!("Execute: {}", script);
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
                eprintln!("Script Error {}", e.display());
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
                                                       let name = &str(&args[0], ctx);
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
        println!("Log: {}", msg);
    }

    fn execute(&mut self, fsm: &Fsm, script: &String) -> String {
        self.execute_internal(fsm, script)
    }

    fn executeForEach(&mut self, _fsm: &Fsm, array_expression: &String, item: &String, index: &String, _execute_body: &dyn FnOnce(&mut Fsm, &mut dyn Datamodel)) {
        // todo!()
    }

    fn executeCondition(&mut self, fsm: &Fsm, script: &String) -> Result<bool, String> {
        match bool::from_str(self.execute_internal(fsm, script).as_str()) {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e.to_string()),
        }
    }

    fn executeContent(&mut self, fsm: &Fsm, content_id: ExecutableContentId) {
        for e in fsm.executableContent.get(&content_id).unwrap() {
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


