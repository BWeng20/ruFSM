use std::collections::HashSet;

use crate::tracer::{TraceMode, Tracer, TracerFactory};

#[derive(Debug)]
pub struct ThriftTracer {
    pub trace_flags: HashSet<TraceMode>,
}

impl Default for ThriftTracer {
    fn default() -> Self {
        ThriftTracer::new()
    }
}

impl Tracer for ThriftTracer {
    fn trace(&self, msg: &str) {
        todo!()
    }

    fn enter(&self) {
        todo!()
    }

    fn leave(&self) {
        todo!()
    }

    fn enable_trace(&mut self, flag: TraceMode) {
        todo!()
    }

    fn disable_trace(&mut self, flag: TraceMode) {
        todo!()
    }

    fn is_trace(&self, flag: TraceMode) -> bool {
        todo!()
    }

    fn trace_mode(&self) -> TraceMode {
        todo!()
    }
}

impl ThriftTracer {
    pub fn new() -> ThriftTracer {
        ThriftTracer {
            trace_flags: HashSet::new(),
        }
    }
}

pub struct ThriftTracerFactory {}

impl ThriftTracerFactory {
    pub fn new() -> ThriftTracerFactory {
        ThriftTracerFactory {}
    }
}

impl Default for ThriftTracerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl TracerFactory for ThriftTracerFactory {
    fn create(&mut self) -> Box<dyn Tracer> {
        Box::new(ThriftTracer::new())
    }
}
