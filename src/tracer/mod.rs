//! The Tracer module, monitoring and remote-control.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::common::info;
use crate::common::ArgOption;
use crate::fsm;
use crate::fsm::{Event, OrderedSet, SessionId, State};

#[cfg(feature = "ThriftTrace")]
pub mod thrift_trace_server;

/// Trace mode for FSM Tracer.
#[derive(Debug, Clone, PartialEq, Copy, Hash, Eq)]
pub enum TraceMode {
    METHODS,
    STATES,
    EVENTS,
    ARGUMENTS,
    RESULTS,
    ALL,
    NONE,
}

pub static TRACE_ARGUMENT_OPTION: ArgOption = ArgOption {
    name: "trace",
    with_value: true,
    required: false,
};

impl TraceMode {
    /// Parse Trace-mode from program arguments.
    pub fn from_arguments(named_arguments: &HashMap<&'static str, String>) -> TraceMode {
        let mut trace = TraceMode::STATES;

        match named_arguments.get("trace") {
            None => {}
            Some(trace_name) => match TraceMode::from_str(trace_name) {
                Ok(opt) => {
                    trace = opt;
                }
                Err(_err) => {
                    panic!("Unknown trace mode '{}'", trace_name)
                }
            },
        }
        trace
    }
}

impl Display for TraceMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl FromStr for TraceMode {
    type Err = ();

    fn from_str(input: &str) -> Result<TraceMode, Self::Err> {
        match input.to_lowercase().as_str() {
            "methods" => Ok(TraceMode::METHODS),
            "states" => Ok(TraceMode::STATES),
            "events" => Ok(TraceMode::EVENTS),
            "arguments" => Ok(TraceMode::ARGUMENTS),
            "results" => Ok(TraceMode::RESULTS),
            "all" => Ok(TraceMode::ALL),
            _ => Err(()),
        }
    }
}

/// Trait used to trace methods and
/// states inside the FSM. What is traced can be controlled by
/// [Tracer::enable_trace] and [Tracer::disable_trace], see [TraceMode].
pub trait Tracer: Send + Debug {
    /// Needed by a minimalistic implementation. Default methods below call this
    /// Method with a textual representation of the trace-event.
    fn trace(&self, session_id: SessionId, msg: &str);

    /// Enable traces for the specified scope.
    fn enable_trace(&mut self, flag: TraceMode);

    /// Disable traces for the specified scope.
    fn disable_trace(&mut self, flag: TraceMode);

    /// Return true if the given scape is enabled.
    fn is_trace(&self, flag: TraceMode) -> bool;

    /// Called by FSM if a method is entered
    fn enter_method(&self, session_id: SessionId, what: &str, arguments: &[(&str, &dyn Display)]);

    /// Called by FSM if a method is exited
    fn exit_method(&self, session_id: SessionId, what: &str, arguments: &[(&str, &dyn Display)]) {
        #[cfg(feature = "Trace_Method")]
        if self.is_trace(TraceMode::METHODS) {
            DefaultTracer::decrease_indent();

            self.trace(
                session_id,
                format!(
                    "<<< {} {}",
                    what,
                    arguments
                        .iter()
                        .map(|(k, v)| format!("{k}:{v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .as_str(),
            )
        }
    }

    /// Called by FSM if an internal event is sent
    fn event_internal_sent(&self, session_id: SessionId, what: &Event) {
        #[cfg(feature = "Trace_Event")]
        if self.is_trace(TraceMode::EVENTS) {
            self.trace(
                session_id,
                format!("Sent Internal Event: {} #{:?}", what.name, what.invoke_id).as_str(),
            );
        }
    }

    /// Called by FSM if an internal event is received
    fn event_internal_received(&self, session_id: SessionId, what: &Event) {
        #[cfg(feature = "Trace_Event")]
        if self.is_trace(TraceMode::EVENTS) {
            self.trace(
                session_id,
                format!(
                    "Received Internal Event: {}, invokeId {:?}, content {:?}, param {:?}",
                    what.name, what.invoke_id, what.content, what.param_values
                )
                .as_str(),
            );
        }
    }

    /// Called by FSM if an external event is sent
    fn event_external_sent(
        &self,
        from_session_id: SessionId,
        to_session_id: SessionId,
        what: &Event,
    ) {
        #[cfg(feature = "Trace_Event")]
        if self.is_trace(TraceMode::EVENTS) {
            self.trace(
                from_session_id,
                format!(
                    "Send External Event: {} #{:?} to {}",
                    what.name, what.invoke_id, to_session_id
                )
                .as_str(),
            );
        }
    }

    /// Called by FSM if an external event is received
    fn event_external_received(&mut self, session_id: SessionId, what: &Event) {
        if what.name.starts_with("trace.") {
            let p = what.name.as_str().split('.').collect::<Vec<&str>>();
            if p.len() == 3 {
                match TraceMode::from_str(p.get(1).unwrap()) {
                    Ok(t) => match *p.get(2).unwrap() {
                        "on" | "ON" | "On" => {
                            self.enable_trace(t);
                        }
                        "off" | "OFF" | "Off" => {
                            self.disable_trace(t);
                        }
                        _ => {
                            self.trace(
                                session_id,
                                format!(
                                    "Trace event '{}' with illegal flag '{}'. Use 'On' or 'Off'.",
                                    what.name,
                                    *p.get(2).unwrap()
                                )
                                .as_str(),
                            );
                        }
                    },
                    Err(_e) => {
                        self.trace(
                            session_id,
                            format!(
                                "Trace event '{}' has unknown trace flag '{}'",
                                what.name,
                                p.get(1).unwrap()
                            )
                            .as_str(),
                        );
                    }
                }
            }
        }
        #[cfg(feature = "Trace_Event")]
        if self.is_trace(TraceMode::EVENTS) {
            self.trace(
                session_id,
                format!(
                    "Received External Event: {} #{:?}",
                    what.name, what.invoke_id
                )
                .as_str(),
            );
        }
    }

    /// Called by FSM if a state is entered or left.
    fn trace_state(&self, session_id: SessionId, what: &str, s: &State) {
        #[cfg(feature = "Trace_State")]
        if self.is_trace(TraceMode::STATES) {
            if s.name.is_empty() {
                self.trace(session_id, format!("{} #{}", what, s.id).as_str());
            } else {
                self.trace(
                    session_id,
                    format!("{} <{}> #{}", what, &s.name, s.id).as_str(),
                );
            }
        }
    }

    /// Called by FSM if a state is entered. Calls [Tracer::trace_state].
    fn trace_enter_state(&self, session_id: SessionId, s: &State) {
        #[cfg(feature = "Trace_State")]
        self.trace_state(session_id, "Enter", s);
    }

    /// Called by FSM if a state is left. Calls [Tracer::trace_state].
    fn trace_exit_state(&self, session_id: SessionId, s: &State) {
        #[cfg(feature = "Trace_State")]
        self.trace_state(session_id, "Exit", s);
    }

    /// Helper method to trace a vector of ids.
    fn trace_id_vec(&self, session_id: SessionId, what: &str, l: &[u32]) {
        self.trace(
            session_id,
            format!("{}=[{}]", what, &fsm::vec_to_string(l)).as_str(),
        );
    }

    /// Helper method to trace a OrderedSet of ids.
    fn trace_id_set(&self, session_id: SessionId, what: &str, l: &OrderedSet<u32>) {
        self.trace(
            session_id,
            format!("{}=({})", what, fsm::vec_to_string(&l.data)).as_str(),
        );
    }

    /// Get trace mode
    fn trace_mode(&self) -> TraceMode;
}

impl Tracer for DefaultTracer {
    fn trace(&self, session_id: SessionId, msg: &str) {
        info!(
            "Trace {}>{:w$}{}",
            session_id,
            " ",
            msg,
            w = DefaultTracer::get_indent() as usize
        );
    }

    fn enable_trace(&mut self, flag: TraceMode) {
        self.trace_flags.insert(flag);
    }

    fn disable_trace(&mut self, flag: TraceMode) {
        self.trace_flags.remove(&flag);
    }

    fn is_trace(&self, flag: TraceMode) -> bool {
        self.trace_flags.contains(&flag) || self.trace_flags.contains(&TraceMode::ALL)
    }

    /// Called by FSM if a method is entered
    fn enter_method(&self, session_id: SessionId, what: &str, arguments: &[(&str, &dyn Display)]) {
        #[cfg(feature = "Trace_Method")]
        if self.is_trace(TraceMode::METHODS) {
            self.trace(
                session_id,
                format!(
                    ">>> {}({})",
                    what,
                    arguments
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .as_str(),
            );
            DefaultTracer::increase_indent();
        }
    }

    fn trace_mode(&self) -> TraceMode {
        if self.is_trace(TraceMode::ALL) {
            TraceMode::ALL
        } else if self.is_trace(TraceMode::EVENTS) {
            TraceMode::EVENTS
        } else if self.is_trace(TraceMode::STATES) {
            TraceMode::STATES
        } else if self.is_trace(TraceMode::METHODS) {
            TraceMode::METHODS
        } else {
            TraceMode::NONE
        }
    }
}

#[derive(Debug)]
pub struct DefaultTracer {
    pub trace_flags: HashSet<TraceMode>,
}

impl Default for DefaultTracer {
    fn default() -> Self {
        DefaultTracer::new()
    }
}

impl DefaultTracer {
    pub fn new() -> DefaultTracer {
        DefaultTracer {
            trace_flags: HashSet::new(),
        }
    }

    fn get_indent() -> u16 {
        TRACE_INDENT.with_borrow(|v| *v)
    }

    fn increase_indent() {
        TRACE_INDENT.with_borrow_mut(|v| *v += 2);
    }

    fn decrease_indent() {
        TRACE_INDENT.with_borrow_mut(|v| {
            if *v > 2 {
                *v -= 2
            }
        });
    }
}

thread_local! {
   /// Trace prefix for [DefaultTracer]
   static TRACE_INDENT:  RefCell<u16> = RefCell::new(1);
}

pub trait TracerFactory: Send {
    fn create(&mut self) -> Box<dyn Tracer>;
}

pub struct DefaultTracerFactory {}

impl DefaultTracerFactory {
    pub fn new() -> DefaultTracerFactory {
        DefaultTracerFactory {}
    }
}

impl Default for DefaultTracerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl TracerFactory for DefaultTracerFactory {
    fn create(&mut self) -> Box<dyn Tracer> {
        Box::new(DefaultTracer::new())
    }
}

lazy_static! {
    static ref tracer_factory_arc: Arc<Mutex<Box<dyn TracerFactory>>> =
        Arc::new(Mutex::new(Box::new(DefaultTracerFactory::new())));
}
pub fn set_tracer_factory(tracer_factory: Box<dyn TracerFactory>) {
    *tracer_factory_arc.lock().unwrap() = tracer_factory;
}

pub fn create_tracer() -> Box<dyn Tracer> {
    tracer_factory_arc.lock().unwrap().create()
}
