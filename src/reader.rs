use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::str;

use quick_xml::events::{BytesStart, Event};
use quick_xml::events::attributes::Attributes;
use quick_xml::Reader;

use crate::fsm::{Fsm, map_transition_type, Name, ScriptConditionalExpression, State, StateId, Transition};

pub type AttributeMap = HashMap<String, String>;

pub fn read_from_xml_file(mut file: File) -> Box<Fsm> {
    let mut contents = String::new();

    let r = file.read_to_string(&mut contents);
    println!("Read {}", r.unwrap());

    let fsm = read_from_xml(contents.as_str());

    fsm
}

pub const TAG_SCXML: &str = "scxml";
pub const TAG_ID: &str = "id";
pub const TAG_DATAMODEL: &str = "datamodel";
pub const TAG_VERSION: &str = "version";
pub const TAG_INITIAL: &str = "initial";
pub const TAG_STATE: &str = "state";
pub const TAG_PARALLEL: &str = "parallel";
pub const TAG_FINAL: &str = "final";
pub const TAG_TRANSITION: &str = "transition";
pub const TAG_COND: &str = "cond";
pub const TAG_EVENT: &str = "event";
pub const TAG_TARGET: &str = "target";
pub const TAG_TYPE: &str = "type";

struct ReaderStackItem {
    current_state: StateId,
    current_tag: String,
}

impl ReaderStackItem {
    pub fn new(o: &ReaderStackItem) -> ReaderStackItem {
        ReaderStackItem {
            current_state: o.current_state.clone(),
            current_tag: o.current_tag.clone(),
        }
    }
}


struct ReaderState {
    // True if reader in inside an scxml element
    in_scxml: bool,
    id_count: i32,

    // The resulting fsm
    fsm: Box<Fsm>,

    current: ReaderStackItem,
    stack: Vec<ReaderStackItem>,
}


impl ReaderState {
    pub fn new() -> ReaderState {
        ReaderState {
            in_scxml: false,
            id_count: 0,
            stack: vec![],
            current: ReaderStackItem {
                current_state: 0,
                current_tag: "".to_string(),
            },
            fsm: Box::new(Fsm::new()),
        }
    }

    pub fn push(&mut self, tag: &str) {
        self.stack.push(ReaderStackItem::new(&self.current));
        self.current.current_tag = tag.to_string();
    }

    pub fn pop(&mut self) {
        let p = self.stack.pop();
        if p.is_some() {
            self.current = p.unwrap();
        }
    }

    pub fn generate_name(&mut self) -> String {
        self.id_count += 1;
        format!("__id{}", self.id_count)
    }

    fn get_state_by_name(&self, name: &Name) -> Option<&State> {
        if self.fsm.statesNames.contains_key(name) {
            Some(self.fsm.get_state_by_name(name))
        } else { None }
    }

    fn get_state_by_name_mut(&mut self, name: &Name) -> Option<&mut State> {
        if self.fsm.statesNames.contains_key(name) {
            Some(self.fsm.get_state_by_name_mut(name))
        } else { None }
    }

    fn get_state_by_id(&self, id: StateId) -> Option<&State> {
        if self.fsm.states.contains_key(&id) {
            Some(self.fsm.get_state_by_id(id))
        } else {
            None
        }
    }

    fn get_state_by_id_mut(&mut self, id: StateId) -> Option<&mut State> {
        if self.fsm.states.contains_key(&id) {
            Some(self.fsm.get_state_by_id_mut(id))
        } else {
            None
        }
    }

    pub fn get_current_state(&mut self) -> &mut State {
        let id = self.current.current_state;
        if id <= 0 {
            panic!("Internal error: Current State is unknown");
        }
        let state = self.get_state_by_id_mut(id);
        if state.is_none() {
            panic!("Internal error: Current State {} is unknown", id);
        }
        state.unwrap()
    }

    pub fn get_parent_tag(&mut self) -> &str {
        let mut r = "";
        if !self.stack.is_empty() {
            r = self.stack.get(self.stack.len() - 1).as_ref().unwrap().current_tag.as_str();
        }
        r
    }

    fn get_or_create_state(&mut self, name: &String, parallel: bool) -> StateId {
        match self.fsm.statesNames.get(name) {
            None => {
                let mut s = State::new(name);
                s.is_parallel = parallel;
                let sid = s.id;
                self.fsm.statesNames.insert(s.name.clone(), s.id); // s.id, s);
                self.fsm.states.insert(s.id, s);
                sid
            }
            Some(id) => {
                if parallel {
                    self.fsm.states.get_mut(id).unwrap().is_parallel = true;
                }
                *id
            }
        }
    }

    fn create_state(&mut self, attr: &AttributeMap, parallel: bool) -> StateId {
        let sname: String;
        match attr.get("id") {
            None => sname = self.generate_name(),
            Some(id) => sname = id.clone()
        }
        let id = self.get_or_create_state(&sname, parallel);
        id
    }


    // A new "parallel" element started
    fn start_parallel(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_PARALLEL, TAG_SCXML);
        }
        let state_id = self.create_state(attr, true);
        if self.current.current_state > 0 {
            let parent_state = self.get_current_state();
            parent_state.states.push(state_id);
        }
        self.fsm.get_state_by_id_mut(state_id).is_parallel = true;
        state_id
    }

    // A new "final" element started
    fn start_final(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_FINAL, TAG_SCXML);
        }
        let state_id = self.create_state(attr, true);
        if self.current.current_state > 0 {
            let parent_state = self.get_current_state();
            parent_state.states.push(state_id);
        }
        self.fsm.get_state_by_id_mut(state_id).is_final = true;
        state_id
    }


    // A new "state" element started
    fn start_state(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_STATE, TAG_SCXML);
        }
        let name: String;
        match attr.get(TAG_ID) {
            None => name = self.generate_name(),
            Some(id) => name = id.clone(),
        }
        let mut s = State::new(&name);

        match attr.get(TAG_INITIAL) {
            None => s.initial = 0,
            Some(state_name) => s.initial = self.get_or_create_state(state_name, false)
        }

        let sr = s.id;

        if self.current.current_state > 0 {
            self.get_current_state().states.push(sr);
        }
        self.current.current_state = s.id.clone();
        self.fsm.statesNames.insert(s.name.clone(), s.id.clone());
        self.fsm.states.insert(s.id.clone(), s); // s.id, s);

        sr
    }

    // A "initial" element started (node, not attribute)
    fn start_initial(&mut self) {
        if [TAG_STATE, TAG_PARALLEL].contains(&self.get_parent_tag()) {
            if self.get_current_state().initial > 0 {
                panic!("<{}> must not be specified if initial-attribute was given", TAG_INITIAL)
            }
            // Next a "<transition>" must follow
        } else {
            panic!("<{}> only allowed inside <{}> or <{}>", TAG_INITIAL, TAG_STATE, TAG_PARALLEL);
        }
    }

    fn start_transition(&mut self, attr: &AttributeMap) {
        let parent_tag =
            {
                self.get_parent_tag().to_string()
            };
        if ![TAG_INITIAL, TAG_STATE, TAG_PARALLEL].contains(&parent_tag.as_str()) {
            panic!("<{}> inside <{}>. Only allowed inside <{}>, <{}> or <{}>", TAG_TRANSITION, parent_tag,
                   TAG_INITIAL, TAG_STATE, TAG_PARALLEL);
        }
        let mut t = Transition::new();

        {
            let event = attr.get(TAG_EVENT);
            if event.is_some() {
                t.events = event.unwrap().split_whitespace().map(|s| { s.to_string() }).collect();
            }
        }
        {
            let cond = attr.get(TAG_COND);
            if cond.is_some() {
                t.cond = Some(Box::new(ScriptConditionalExpression::new(cond.unwrap())));
            }
        }
        {
            let target = attr.get(TAG_TARGET);
            match target {
                None => (),
                // TODO: Parse the state specification! (it can be a list)
                Some(target_name) => t.target.push(self.get_or_create_state(target_name, false)),
            }
        }
        {
            let trans_type = attr.get(TAG_TYPE);
            if trans_type.is_some() {
                t.transition_type = map_transition_type(trans_type.unwrap())
            }
        }

        let state = self.get_current_state();

        if parent_tag.eq(TAG_INITIAL) {
            if state.initial > 0 {
                panic!("<initial> must not be specified if initial-attribute was given")
            }
            state.initial = t.id;
        } else {
            state.transitions.push(t.id);
        }
        self.fsm.transitions.insert(t.id, t);
    }

    fn start_element(&mut self, reader: &Reader<&[u8]>, e: &BytesStart) {
        let n = e.name();
        let name = str::from_utf8(n.as_ref()).unwrap();
        self.push(name);
        match name {
            TAG_SCXML => {
                if self.in_scxml {
                    panic!("Only one <{}> allowed", TAG_SCXML);
                }
                self.in_scxml = true;
                let map = decode_attributes(&reader, &mut e.attributes());
                let datamodel = map.get(TAG_DATAMODEL);
                if datamodel.is_some() {
                    self.fsm.datamodel = crate::fsm::createDatamodel(datamodel.unwrap());
                }
                let version = map.get(TAG_VERSION);
                if version.is_some() {
                    self.fsm.version = version.unwrap().clone();
                }
                self.fsm.pseudo_root = self.create_state(&map, false);
                let initial = map.get(TAG_INITIAL);
                if initial.is_some() {
                    let t = Transition::new();
                    self.fsm.get_state_by_id_mut(self.fsm.pseudo_root).initial = t.id;
                    self.fsm.transitions.insert(t.id, t);
                }
            }
            TAG_STATE => {
                self.start_state(&decode_attributes(&reader, &mut e.attributes()));
            }
            TAG_PARALLEL => {
                self.start_parallel(&decode_attributes(&reader, &mut e.attributes()));
            }
            TAG_FINAL => {
                self.start_final(&decode_attributes(&reader, &mut e.attributes()));
            }
            TAG_INITIAL => {
                self.start_initial();
            }
            TAG_TRANSITION => {
                self.start_transition(&decode_attributes(&reader, &mut e.attributes()));
            }
            _ => (),
        }
    }

    fn end_element(&mut self, name: &str) {
        if !self.current.current_tag.eq(name) {
            panic!("Illegal end-tag {:?}, expected {:?}", &name, &self.current.current_tag);
        }
        self.pop();
    }
}

/**
 * Decode attributes into a hash-map
 */
fn decode_attributes(reader: &Reader<&[u8]>, attr: &mut Attributes) -> AttributeMap {
    attr.map(|attr_result| {
        match attr_result {
            Ok(a) => {
                let key = reader.decoder().decode(a.key.as_ref());
                if key.is_err() {
                    panic!("unable to read attribute name {:?}, utf8 error {:?}", &a, key.err());
                }
                let value = a.decode_and_unescape_value(&reader);
                if value.is_err() {
                    panic!("unable to read attribute value  {:?}, utf8 error {:?}", &a, value.err());
                }
                (key.unwrap().to_string(), value.unwrap().to_string())
            }
            Err(err) => {
                panic!("unable to read key in DefaultSettings, err = {:?}", err);
            }
        }
    }).collect()
}

pub fn read_from_xml(xml: &str) -> Box<Fsm> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    let mut rs = ReaderState::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                rs.start_element(&mut reader, &e);
            }
            Ok(Event::End(e)) => {
                rs.end_element(str::from_utf8(e.name().as_ref()).unwrap());
            }
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),

            // There are several other `Event`s we do not consider here
            _ => (),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    rs.fsm
}
