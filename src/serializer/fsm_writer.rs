//! Module to write a persistent binary version of a Fsm.\
//! The format is independent of the platform byte-order

use std::io::Write;

use crate::datamodel::DataStore;
use crate::fsm::{
    CommonContent, DocumentId, DoneData, ExecutableContentId, Fsm, Invoke, Parameter, State,
    StateId, TransitionId,
};
use crate::serializer::protocol_writer::ProtocolWriter;

pub const FSM_WRITER_VERSION: &str = "fsmW1.0";

pub struct FsmWriter<'a, W>
where
    W: Write + 'a,
{
    writer: Box<dyn ProtocolWriter<W> + 'a>,
}

impl<'a, W> FsmWriter<'a, W>
where
    W: Write + 'a,
{
    pub fn new(writer: Box<dyn ProtocolWriter<W>>) -> FsmWriter<'a, W> {
        FsmWriter { writer }
    }

    pub fn write(&mut self, fsm: &Fsm) {
        self.writer.write_str(FSM_WRITER_VERSION);
        self.writer.write_str(fsm.name.as_str());
        self.write_state_id(fsm.pseudo_root);
        self.writer.write_usize(fsm.states.len());
        for state in &fsm.states {
            self.write_state(state)
        }
    }

    pub fn close(&mut self) {
        self.writer.close();
    }

    pub fn write_state_id(&mut self, value: StateId) {
        self.writer.write_uint(value as u64);
    }

    pub fn write_doc_id(&mut self, value: DocumentId) {
        self.writer.write_uint(value as u64);
    }

    pub fn write_transition_id(&mut self, value: TransitionId) {
        self.writer.write_uint(value as u64);
    }

    pub fn write_executable_content_id(&mut self, value: ExecutableContentId) {
        self.writer.write_uint(value as u64);
    }

    pub fn write_data_store(&mut self, value: &DataStore) {
        self.writer.write_usize(value.values.len());
        for (key, data) in &value.values {
            self.writer.write_str(key.as_str());
            self.writer.write_option_string(&data.value);
        }
    }

    pub fn write_common_content(&mut self, value: &CommonContent) {
        self.writer.write_option_string(&value.content);
        self.writer.write_option_string(&value.content_expr);
    }

    pub fn write_parameter(&mut self, value: &Parameter) {
        self.writer.write_str(value.name.as_str());
        self.writer.write_str(value.expr.as_str());
        self.writer.write_str(value.location.as_str());
    }

    pub fn write_done_data(&mut self, value: &DoneData) {
        self.writer.write_boolean(value.content.is_some());
        if value.content.is_some() {
            self.write_common_content(value.content.as_ref().unwrap());
        }

        if value.params.is_some() {
            let params = value.params.as_ref().unwrap();
            self.writer.write_usize(params.len());
            for p in params {
                self.write_parameter(p);
            }
        } else {
            self.writer.write_usize(0usize);
        }
    }

    pub fn write_invoke(&mut self, _value: &Invoke) {
        todo!()
    }

    pub fn write_state(&mut self, state: &State) {
        self.write_state_id(state.id);
        self.write_doc_id(state.doc_id);
        self.writer.write_str(state.name.as_str());
        self.write_transition_id(state.initial);

        self.writer.write_usize(state.states.len());
        for state_id in &state.states {
            self.write_state_id(*state_id);
        }

        self.writer.write_boolean(state.is_parallel);
        self.writer.write_boolean(state.is_final);
        self.writer.write_uint(state.history_type.ordinal() as u64);
        self.write_executable_content_id(state.onentry);
        self.write_executable_content_id(state.onexit);

        self.writer.write_usize(state.transitions.size());
        for transition_id in state.transitions.iterator() {
            self.write_transition_id(*transition_id);
        }

        self.writer.write_usize(state.invoke.size());
        for invoke in state.invoke.iterator() {
            self.write_invoke(invoke);
        }

        self.writer.write_usize(state.history.size());
        for history in state.history.iterator() {
            self.write_state_id(*history);
        }

        self.write_data_store(&state.data);
        self.write_state_id(state.parent);

        self.writer.write_boolean(state.donedata.is_some());
        if state.donedata.is_some() {
            self.write_done_data(state.donedata.as_ref().unwrap())
        }
    }
}
