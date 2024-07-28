//! Module to write a persistent binary version of a Fsm.\
//! The format is independent of the platform byte-order

use std::io::Read;

use crate::datamodel::{Data, DataStore};
use crate::fsm::{
    CommonContent, DocumentId, DoneData, ExecutableContentId, Fsm, HistoryType, Invoke, Parameter,
    State, StateId, TransitionId,
};
use crate::serializer::fsm_writer::FSM_WRITER_VERSION;
use crate::serializer::protocol_reader::ProtocolReader;

/// The reader version, must natch the corresponding writer version
pub const FSM_READER_VERSION: &str = "fsmW1.0";

pub struct FsmReader<'a, R>
where
    R: Read + 'a,
{
    reader: Box<dyn ProtocolReader<R> + 'a>,
}

impl<'a, R> FsmReader<'a, R>
where
    R: Read + 'a,
{
    pub fn new(reader: Box<dyn ProtocolReader<R>>) -> FsmReader<'a, R> {
        FsmReader { reader }
    }

    pub fn read(&mut self) -> Result<Fsm, String> {
        let mut fsm = Fsm::new();
        let version = self.reader.read_string();
        if version.as_str() == FSM_WRITER_VERSION {
            fsm.name = self.reader.read_string();
            fsm.pseudo_root = self.read_state_id();

            let states_len = self.reader.read_usize();
            for _idx in 0..states_len {
                let mut state = State::new(&"".to_string());
                self.read_state(&mut state);
                fsm.states.push(state);
            }
            Ok(fsm)
        } else {
            Err(format!(
                "Version missmatch: {} is not {} as expected",
                version, FSM_WRITER_VERSION
            ))
        }
    }

    pub fn close(&mut self) {
        self.reader.close();
    }

    pub fn read_state_id(&mut self) -> StateId {
        self.reader.read_uint() as StateId
    }

    pub fn read_doc_id(&mut self) -> DocumentId {
        self.reader.read_uint() as DocumentId
    }

    pub fn read_transition_id(&mut self) -> TransitionId {
        self.reader.read_uint() as TransitionId
    }

    pub fn read_executable_content_id(&mut self) -> ExecutableContentId {
        self.reader.read_uint() as ExecutableContentId
    }

    pub fn read_data_store(&mut self, value: &mut DataStore) {
        value.values.clear();
        let len = self.reader.read_usize();
        for _i in 0..len {
            let key = self.reader.read_string();
            let data = Data {
                value: self.reader.read_option_string(),
            };
            value.values.insert(key, data);
        }
    }

    pub fn read_common_content(&mut self, value: &mut CommonContent) {
        value.content = self.reader.read_option_string();
        value.content_expr = self.reader.read_option_string();
    }

    pub fn read_parameter(&mut self, value: &mut Parameter) {
        value.name = self.reader.read_string();
        value.expr = self.reader.read_string();
        value.location = self.reader.read_string();
    }

    pub fn read_done_data(&mut self, value: &mut DoneData) {
        if self.reader.read_boolean() {
            let mut c = CommonContent::new();
            self.read_common_content(&mut c);
            let _ = value.content.insert(c);
        } else {
            value.content = None;
        }

        let param_len = self.reader.read_usize();
        if param_len == 0 {
            value.params = None;
        } else {
            let mut pvec = Vec::new();
            for _pi in 0..param_len {
                let mut param = Parameter::new();
                self.read_parameter(&mut param);
                pvec.push(param);
            }
            let _ = value.params.insert(pvec);
        }
    }

    pub fn read_invoke(&mut self, _value: &mut Invoke) {
        todo!()
    }

    pub fn read_state(&mut self, state: &mut State) {
        state.id = self.read_state_id();
        state.doc_id = self.read_doc_id();
        state.name = self.reader.read_string();
        state.initial = self.read_transition_id();

        let states_len = self.reader.read_usize();
        for _si in 0..states_len {
            state.states.push(self.read_state_id());
        }

        state.is_parallel = self.reader.read_boolean();
        state.is_final = self.reader.read_boolean();
        state.history_type = HistoryType::from_ordinal(self.reader.read_u8());
        state.onentry = self.read_executable_content_id();
        state.onexit = self.read_executable_content_id();

        let transition_len = self.reader.read_usize();
        for _ti in 0..transition_len {
            state.transitions.push(self.read_transition_id());
        }

        let invoke_len = self.reader.read_usize();
        for _ii in 0..invoke_len {
            let mut invoke = Invoke::new();
            self.read_invoke(&mut invoke);
            state.invoke.push(invoke);
        }

        let history_len = self.reader.read_usize();
        for _hi in 0..history_len {
            state.history.push(self.read_state_id());
        }

        self.read_data_store(&mut state.data);
        state.parent = self.read_state_id();

        if self.reader.read_boolean() {
            let mut donedata = DoneData::new();
            self.read_done_data(&mut donedata);
            let _ = state.donedata.insert(donedata);
        } else {
            state.donedata = None;
        }
    }
}
