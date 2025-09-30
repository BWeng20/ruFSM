//! Implementation of "executable content" elements.\
//! See [W3C:Executable Content](/doc/W3C_SCXML_2024_07_13/index.html#executable).

use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::Ordering;

#[cfg(feature = "Debug")]
use crate::common::debug;
use crate::common::{error, warn};
use crate::datamodel::{str_to_source, Data, Datamodel, ToAny, SCXML_EVENT_PROCESSOR};
use crate::event_io_processor::scxml_event_io_processor::SCXML_TARGET_INTERNAL;
use crate::expression_engine::lexer::ExpressionLexer;
use crate::fsm::{
    vec_to_string, CommonContent, Event, EventType, ExecutableContentId, Fsm, ParamPair, Parameter,
    PLATFORM_ID_COUNTER,
};

pub const TARGET_SCXML_EVENT_PROCESSOR: &str = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

pub const TYPE_IF: u8 = 0;
pub const TYPE_EXPRESSION: u8 = 1;
/// No longer used:
/// pub const TYPE_SCRIPT: u8 = 2;
pub const TYPE_LOG: u8 = 3;
pub const TYPE_FOREACH: u8 = 4;
pub const TYPE_SEND: u8 = 5;
pub const TYPE_RAISE: u8 = 6;
pub const TYPE_CANCEL: u8 = 7;
pub const TYPE_ASSIGN: u8 = 8;

pub const TYPE_NAMES: [&str; 9] = [
    "if",
    "expression",
    "unused",
    "log",
    "foreach",
    "send",
    "raise",
    "cancel",
    "assign",
];

/// Gets the global data store from datamodel.
macro_rules! get_global {
    ($x:expr) => {
        $x.global().lock().unwrap()
    };
}

pub trait ExecutableContent: ToAny + Debug + Send {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) -> bool;
    fn get_type(&self) -> u8;

    fn get_trace(&self) -> HashMap<&str, Data>;
}

pub fn get_safe_executable_content_as<T: 'static>(ec: &mut dyn ExecutableContent) -> &mut T {
    let va = ec.as_any_mut();
    va.downcast_mut::<T>()
        .unwrap_or_else(|| panic!("Failed to cast executable content"))
}

pub fn get_executable_content_as<T: 'static>(ec: &mut dyn ExecutableContent) -> Option<&mut T> {
    let va = ec.as_any_mut();
    match va.downcast_mut::<T>() {
        Some(v) => Some(v),
        None => None,
    }
}

pub fn get_opt_executable_content_as<T: 'static>(
    ec_opt: Option<&mut dyn ExecutableContent>,
) -> Option<&mut T> {
    match ec_opt {
        Some(ec) => get_executable_content_as::<T>(ec),
        None => None,
    }
}

#[derive(Default)]
pub struct Cancel {
    pub send_id: String,
    pub send_id_expr: Data,
}

/// Holds all parameters of a \<send\> call.
#[derive(Default)]
pub struct SendParameters {
    /// SCXML \<send\> attribute 'idlocation'
    pub name_location: String,
    /// SCXML \<send\> attribute 'id'.
    pub name: String,
    /// In case the id is generated, the parent state of the send.
    pub parent_state_name: String,
    /// SCXML \<send\> attribute 'event'.
    pub event: Data,
    /// SCXML \<send\> attribute 'eventexpr'.
    pub event_expr: Data,
    /// SCXML \<send\> attribute 'target'.
    pub target: Data,
    /// SCXML \<send\> attribute 'targetexpr'.
    pub target_expr: Data,
    /// SCXML \<send\> attribute 'type'.
    pub type_value: Data,
    /// SCXML \<send\> attribute 'typeexpr'.
    pub type_expr: Data,
    /// SCXML \<send\> attribute 'delay' in milliseconds.
    pub delay_ms: u64,
    /// SCXML \<send\> attribute 'delayexpr'.
    pub delay_expr: Data,
    /// SCXML \<send\> attribute 'namelist'. Must not be specified in conjunction with 'content'.
    pub name_list: Vec<String>,
    /// \<param\> children
    pub params: Option<Vec<Parameter>>,
    pub content: Option<CommonContent>,
}

impl SendParameters {
    pub fn new() -> SendParameters {
        SendParameters {
            name_location: "".to_string(),
            name: "".to_string(),
            parent_state_name: "".to_string(),
            event: Data::None(),
            event_expr: Data::None(),
            target: Data::None(),
            target_expr: Data::None(),
            type_value: Data::None(),
            type_expr: Data::None(),
            delay_ms: 0,
            delay_expr: Data::None(),
            name_list: Vec::new(),
            params: None,
            content: None,
        }
    }
}

impl Debug for SendParameters {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Send").field("name", &self.name).finish()
    }
}

impl Cancel {
    pub fn new() -> Cancel {
        Cancel {
            send_id: String::new(),
            send_id_expr: Data::None(),
        }
    }
}

impl Debug for Cancel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cancel")
            .field("send_id", &self.send_id)
            .field("send_id_expr", &self.send_id_expr)
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct Expression {
    pub content: Data,
}

#[derive(Debug, Default)]
pub struct Log {
    pub label: String,
    pub expression: Data,
}

#[derive(Debug, Default)]
pub struct If {
    pub condition: Data,
    pub content: ExecutableContentId,
    pub else_content: ExecutableContentId,
}

#[derive(Debug, Default)]
pub struct ForEach {
    pub array: Data,
    pub item: String,
    pub index: String,
    pub content: ExecutableContentId,
}

#[derive(Default)]
pub struct Assign {
    pub location: Data,
    pub expr: Data,
}

impl Assign {
    pub fn new() -> Assign {
        Assign {
            location: Data::None(),
            expr: Data::None(),
        }
    }
}

impl Debug for Assign {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Assign")
            .field("location", &self.location)
            .field("expr", &self.expr)
            .finish()
    }
}

impl ExecutableContent for Assign {
    fn execute(&self, datamodel: &mut dyn Datamodel, _fsm: &Fsm) -> bool {
        datamodel.assign(&self.location, &self.expr)
    }

    fn get_type(&self) -> u8 {
        TYPE_ASSIGN
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("location", self.location.clone());
        d.insert("expr", self.expr.clone());
        d
    }
}

/// *W3C says*:
/// The \<raise\> element raises an event in the current SCXML session.\
/// Note that the event will not be processed until the current block of executable content has completed
/// and all events that are already in the internal event queue have been processed. For example, suppose
/// the \<raise\> element occurs first in the \<onentry\> handler of state S followed by executable content
/// elements ec1 and ec2. If event e1 is already in the internal event queue when S is entered, the event
/// generated by \<raise\> will not be processed until ec1 and ec2 have finished execution and e1 has been
/// processed.
///
#[derive(Default)]
pub struct Raise {
    pub event: String,
}

impl Raise {
    pub fn new() -> Raise {
        Raise {
            event: String::new(),
        }
    }
}

impl Debug for Raise {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Raise").field("event", &self.event).finish()
    }
}

impl ExecutableContent for Raise {
    fn execute(&self, datamodel: &mut dyn Datamodel, _fsm: &Fsm) -> bool {
        let event = Event::new("", &self.event, None, None, EventType::internal);
        get_global!(datamodel).enqueue_internal(event);
        true
    }

    fn get_type(&self) -> u8 {
        TYPE_RAISE
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("event", Data::String(self.event.clone()));
        d
    }
}

impl Expression {
    pub fn new() -> Expression {
        Expression {
            content: str_to_source(""),
        }
    }
}

impl ExecutableContent for Expression {
    fn execute(&self, datamodel: &mut dyn Datamodel, _fsm: &Fsm) -> bool {
        let r = datamodel.execute(&self.content);
        r.is_ok()
    }

    fn get_type(&self) -> u8 {
        TYPE_EXPRESSION
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("content", self.content.clone());
        d
    }
}

impl Log {
    pub fn new(label: &Option<&String>, expression: Data) -> Log {
        Log {
            label: label.unwrap_or(&"".to_string()).clone(),
            expression,
        }
    }
}

impl ExecutableContent for Log {
    fn execute(&self, datamodel: &mut dyn Datamodel, _fsm: &Fsm) -> bool {
        match &datamodel.execute(&self.expression) {
            Ok(msg) => {
                datamodel.log(msg.lock().unwrap().to_string().as_str());
                true
            }
            Err(_msg) => false,
        }
    }

    fn get_type(&self) -> u8 {
        TYPE_LOG
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("expression", self.expression.clone());
        d
    }
}

impl If {
    pub fn new(condition: Data) -> If {
        If {
            condition,
            content: 0,
            else_content: 0,
        }
    }
}

impl ExecutableContent for If {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) -> bool {
        let r = datamodel
            .execute_condition(&self.condition)
            .unwrap_or_else(|e| {
                warn!("Condition {} can't be evaluated. {}", self.condition, e);
                false
            });
        if r {
            if self.content != 0 {
                for e in fsm
                    .executableContent
                    .get((self.content - 1) as usize)
                    .unwrap()
                {
                    if !e.execute(datamodel, fsm) {
                        return false;
                    }
                }
            }
        } else if self.else_content != 0 {
            for e in fsm
                .executableContent
                .get((self.else_content - 1) as usize)
                .unwrap()
            {
                if !e.execute(datamodel, fsm) {
                    return false;
                }
            }
        }
        true
    }

    fn get_type(&self) -> u8 {
        TYPE_IF
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("condition", self.condition.clone());
        d.insert("then", Data::Integer(self.content as i64));
        d.insert("else", Data::Integer(self.else_content as i64));
        d
    }
}

pub const INDEX_TEMP: &str = "__$index";

impl ForEach {
    pub fn new() -> ForEach {
        ForEach {
            array: Data::None(),
            item: String::new(),
            index: String::new(),
            content: 0,
        }
    }
}

impl ExecutableContent for ForEach {
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) -> bool {
        let idx = if self.index.is_empty() {
            INDEX_TEMP.to_string()
        } else {
            self.index.clone()
        };
        datamodel.execute_for_each(&self.array, &self.item, &idx, &mut |datamodel| -> bool {
            if self.content != 0 {
                for e in fsm
                    .executableContent
                    .get((self.content - 1) as usize)
                    .unwrap()
                {
                    if !e.execute(datamodel, fsm) {
                        return false;
                    }
                }
            }
            true
        })
    }

    fn get_type(&self) -> u8 {
        TYPE_FOREACH
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("array", self.array.clone());
        d.insert("item", Data::String(self.item.clone()));
        d.insert("else", Data::String(self.index.clone()));
        d
    }
}

impl Parameter {
    pub fn new() -> Parameter {
        Parameter {
            name: "".to_string(),
            expr: "".to_string(),
            location: "".to_string(),
        }
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parameter{{name:{} expr:{} location:{}}}",
            self.name, self.expr, self.location
        )
    }
}

impl ExecutableContent for Cancel {
    /// W3c says:\
    /// The \<cancel> element is used to cancel a delayed \<send> event.\
    /// The SCXML Processor MUST NOT allow \<cancel> to affect events that were not raised in the
    /// same session. The Processor SHOULD make its best attempt to cancel all delayed events with
    /// the specified id. Note, however, that it can not be guaranteed to succeed, for example if
    /// the event has already been delivered by the time the \<cancel> tag executes.
    fn execute(&self, datamodel: &mut dyn Datamodel, _fsm: &Fsm) -> bool {
        if let Ok(send_id) = datamodel.get_expression_alternative_value(
            &str_to_source(self.send_id.as_str()),
            &self.send_id_expr,
        ) {
            get_global!(datamodel)
                .delayed_send
                .remove(&send_id.lock().unwrap().to_string());
        };
        true
    }

    fn get_type(&self) -> u8 {
        TYPE_CANCEL
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("sendid", Data::String(self.send_id.clone()));
        d.insert("sendidexpr", self.send_id_expr.clone());
        d
    }
}

/// Implements the execution of \<send\> element.
impl ExecutableContent for SendParameters {
    /// If unable to dispatch, place "error.communication" in internal queue
    /// If target is not supported, place "error.execution" in internal queue
    fn execute(&self, datamodel: &mut dyn Datamodel, fsm: &Fsm) -> bool {
        let target =
            match datamodel.get_expression_alternative_value(&self.target, &self.target_expr) {
                Ok(value) => value,
                Err(_) => {
                    // Error -> abort
                    return false;
                }
            };

        let event_name =
            match datamodel.get_expression_alternative_value(&self.event, &self.event_expr) {
                Ok(value) => value,
                Err(_) => {
                    // Error -> abort
                    return false;
                }
            };

        let send_id = if self.name_location.is_empty() {
            if self.name.is_empty() {
                None
            } else {
                Some(self.name.clone())
            }
        } else {
            // W3c says:
            // If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent
            // <send> element is evaluated and store it in this location.
            // note that the automatically generated id for <invoke> has a special format.
            // See 6.4.1 Attribute Details for details.
            // The SCXML processor MAY generate all other ids in any format, as long as they are unique.
            //
            // Implementation: we do it the same as for invoke

            let generated_id = format!(
                "{}.{}",
                &self.parent_state_name,
                PLATFORM_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
            );

            datamodel.set(
                self.name_location.as_str(),
                Data::String(generated_id.clone()),
                true,
            );
            Some(generated_id)
        };

        let mut data_vec = Vec::new();

        let mut content = None;

        // A conformant document MUST NOT specify "namelist" or <param> with <content>.
        if self.content.is_some() {
            if let Some(content_data) = datamodel.evaluate_content(&self.content) {
                content = Some(content_data.lock().unwrap().clone())
            }
        } else {
            datamodel.evaluate_params(&self.params, &mut data_vec);
            for name in self.name_list.as_slice() {
                match datamodel.get_by_location(name) {
                    Err(_msg) => {
                        // Error -> Abort
                        return false;
                    }
                    Ok(value) => {
                        data_vec.push(ParamPair::new(name.as_str(), &value.lock().unwrap()));
                    }
                }
            }
        }

        let delay_ms = if !self.delay_expr.is_empty() {
            match datamodel.execute(&self.delay_expr) {
                Err(_msg) => {
                    // Error -> Abort
                    return false;
                }
                Ok(delay) => parse_duration_to_milliseconds(&delay.lock().unwrap().to_string()),
            }
        } else {
            self.delay_ms as i64
        };

        if delay_ms < 0 {
            // Delay is invalid -> Abort
            error!("Send: delay {} is negative", self.delay_expr);
            datamodel.internal_error_execution_for_event(&send_id, &fsm.caller_invoke_id);
            return false;
        }

        let target_guard = target.lock().unwrap();
        if delay_ms > 0 && target_guard.to_string().eq(SCXML_TARGET_INTERNAL) {
            // Can't send via internal queue
            error!("Send: illegal delay for target {}", target_guard);
            datamodel.internal_error_execution_for_event(&send_id, &fsm.caller_invoke_id);
            return false;
        }
        let type_result =
            datamodel.get_expression_alternative_value(&self.type_value, &self.type_expr);

        let type_val = match type_result {
            Ok(val) => val,
            Err(err) => {
                error!("Failed to evaluate send type: {}", err);
                datamodel.internal_error_execution_for_event(&send_id, &fsm.caller_invoke_id);
                return false;
            }
        };

        let type_val_string = if type_val.lock().unwrap().is_empty() {
            SCXML_EVENT_PROCESSOR.to_string()
        } else {
            type_val.lock().unwrap().to_string()
        };
        let type_val_str = type_val_string.as_str();

        let event = Event {
            name: event_name.lock().unwrap().to_string(),
            etype: EventType::external,
            sendid: send_id.clone(),
            origin: None,
            origin_type: None,
            invoke_id: fsm.caller_invoke_id.clone(),
            param_values: if data_vec.is_empty() {
                None
            } else {
                Some(data_vec.clone())
            },
            content,
        };

        let result = if delay_ms > 0 {
            let iop_opt = datamodel.get_io_processor(type_val_str);
            if let Some(iop) = iop_opt {
                let iopc = iop.clone();
                #[cfg(feature = "Debug")]
                debug!("schedule '{}' for {}", event, delay_ms);
                let global_clone = datamodel.global_s().clone();
                let send_id_clone = send_id.clone();
                let target_str = target_guard.to_string();
                let tg = fsm.schedule(delay_ms, move || {
                    if let Some(sid) = &send_id_clone {
                        global_clone.lock().unwrap().delayed_send.remove(sid);
                    }
                    iopc.lock()
                        .unwrap()
                        .send(&global_clone, target_str.as_str(), event.clone());
                });
                if let Some(g) = tg {
                    if let Some(sid) = &send_id {
                        datamodel
                            .global()
                            .lock()
                            .unwrap()
                            .delayed_send
                            .insert(sid.clone(), g);
                    } else {
                        g.ignore();
                    }
                };
                true
            } else {
                error!("Unknown io-processor {}", type_val_str);
                false
            }
        } else {
            #[cfg(feature = "Debug")]
            debug!("send '{}' to '{}'", event, target_guard);
            datamodel.send(type_val_str, &target_guard, event.clone())
        };

        if !result {
            // W3C:  If the SCXML Processor does not support the type that is specified,
            // it must place the event error.execution on the internal event queue.
            datamodel.internal_error_execution_for_event(&send_id, &fsm.caller_invoke_id);
        };
        result
    }

    fn get_type(&self) -> u8 {
        TYPE_SEND
    }

    fn get_trace(&self) -> HashMap<&str, Data> {
        let mut d = HashMap::new();
        d.insert("name_location", Data::String(self.name_location.clone()));
        d.insert("name", Data::String(self.name.clone()));
        d.insert(
            "parent_state_name",
            Data::String(self.parent_state_name.clone()),
        );
        d.insert("event", self.event.clone());
        d.insert("event_expr", self.event_expr.clone());
        d.insert("target", self.target.clone());
        d.insert("target_expr", self.target_expr.clone());
        d.insert("type_value", self.type_value.clone());
        d.insert("type_expr", self.type_expr.clone());
        d.insert("delay_ms", Data::Integer(self.delay_ms as i64));
        d.insert("delay_expr", self.delay_expr.clone());
        d.insert("name_list", Data::String(vec_to_string(&self.name_list)));
        d.insert(
            "params",
            match &self.params {
                Some(s) => Data::String(vec_to_string(s)),
                None => Data::None(),
            },
        );
        d.insert(
            "content",
            match &self.content {
                Some(s) => Data::String(format!("{:?}", s)),
                None => Data::None(),
            },
        );
        d
    }
}

#[cfg(test)]
mod tests {
    use crate::executable_content::parse_duration_to_milliseconds;

    #[test]
    fn delay_parse() {
        assert_eq!(parse_duration_to_milliseconds("6.7s"), 6700);
        assert_eq!(parse_duration_to_milliseconds("0.5d"), 12 * 60 * 60 * 1000);
        assert_eq!(parse_duration_to_milliseconds("1m"), 60 * 1000);
        assert_eq!(parse_duration_to_milliseconds("0.001s"), 1);
        assert_eq!(parse_duration_to_milliseconds("6.7S"), 6700);
        assert_eq!(parse_duration_to_milliseconds("0.5D"), 12 * 60 * 60 * 1000);
        assert_eq!(parse_duration_to_milliseconds("1M"), 60 * 1000);
        assert_eq!(parse_duration_to_milliseconds("0.001S"), 1);

        assert_eq!(parse_duration_to_milliseconds("x1S"), -1);
        assert_eq!(parse_duration_to_milliseconds("1Sx"), -1);
    }
}

/// a duration.
/// RegExp: "\\d*(\\.\\d+)?(ms|s|m|h|d))").
pub fn parse_duration_to_milliseconds(d: &str) -> i64 {
    if d.is_empty() {
        0
    } else {
        let mut exp = ExpressionLexer::new(d.to_string());
        let value_result = exp.next_number();
        if value_result.is_err() {
            return -1;
        }
        let Ok(unit) = exp.next_name() else {
            return 0;
        };

        let mut v = value_result.unwrap().as_double();
        match unit.as_str() {
            "D" | "d" => {
                v *= 24.0 * 60.0 * 60.0 * 1000.0;
            }
            "H" | "h" => {
                v *= 60.0 * 60.0 * 1000.0;
            }
            "M" | "m" => {
                v *= 60000.0;
            }
            "S" | "s" => {
                v *= 1000.0;
            }
            "MS" | "ms" => {}
            _ => {
                return -1;
            }
        }
        v.round() as i64
    }
}
