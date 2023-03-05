use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::str;

use quick_xml::events::{BytesStart, Event};
use quick_xml::events::attributes::Attributes;
use quick_xml::Reader;

use crate::fsm::{ExecutableContent, Fsm, HistoryType, map_history_type, map_transition_type, Name, ScriptConditionalExpression, State, StateId, Transition};

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
pub const TAG_HISTORY: &str = "history";
pub const TAG_PARALLEL: &str = "parallel";
pub const TAG_FINAL: &str = "final";
pub const TAG_TRANSITION: &str = "transition";
pub const TAG_COND: &str = "cond";
pub const TAG_EVENT: &str = "event";
pub const TAG_TARGET: &str = "target";
pub const TAG_TYPE: &str = "type";
pub const TAG_ON_ENTRY: &str = "onentry";
pub const TAG_ON_EXIT: &str = "onexit";

/// Executable content
pub const TAG_RAISE: &str = "raise";
pub const TAG_SEND: &str = "send";
pub const TAG_LOG: &str = "log";
pub const TAG_SCRIPT: &str = "script";
pub const TAG_ASSIGN: &str = "assign";
pub const TAG_IF: &str = "if";
pub const TAG_FOR_EACH: &str = "foreach";
pub const TAG_CANCEL: &str = "cancel";
pub const TAG_ELSE: &str = "else";
pub const TAG_ELSEIF: &str = "elseif";


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

    fn push(&mut self, tag: &str) {
        self.stack.push(ReaderStackItem::new(&self.current));
        self.current.current_tag = tag.to_string();
    }

    fn pop(&mut self) {
        let p = self.stack.pop();
        if p.is_some() {
            self.current = p.unwrap();
        }
    }

    fn generate_name(&mut self) -> String {
        self.id_count += 1;
        format!("__id{}", self.id_count)
    }

    fn parseStateSpecification(&mut self, target_name: &str, targets: &mut Vec<StateId>) {
        target_name.split_ascii_whitespace().for_each(|target| {
            targets.push(self.get_or_create_state(&target.to_string(), false))
        });
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

    fn get_state_by_id(&self, id: StateId) -> &State {
        self.fsm.get_state_by_id(id)
    }

    fn get_state_by_id_mut(&mut self, id: StateId) -> &mut State {
        self.fsm.get_state_by_id_mut(id)
    }

    fn get_current_state(&mut self) -> &mut State {
        let id = self.current.current_state;
        if id <= 0 {
            panic!("Internal error: Current State is unknown");
        }
        self.get_state_by_id_mut(id)
    }

    fn get_parent_tag(&self) -> &str {
        let mut r = "";
        if !self.stack.is_empty() {
            r = self.stack.get(self.stack.len() - 1).as_ref().unwrap().current_tag.as_str();
        }
        r
    }

    pub fn verify_parent_tag(&self, name: &str, allowed_parents: &[&str]) -> &str {
        let parent_tag = self.get_parent_tag();
        if !allowed_parents.contains(&parent_tag) {
            let mut allowed_parents_s = "".to_string();
            let len = allowed_parents.len();
            for i in 0..allowed_parents.len() {
                allowed_parents_s += format!("{}<{}>",
                                             if i > 0 {
                                                 if i < (len - 1) {
                                                     ", "
                                                 } else {
                                                     " or "
                                                 }
                                             } else {
                                                 ""
                                             }, allowed_parents[i]).as_str();
            }
            panic!("<{}> inside <{}>. Only allowed inside {}", name, parent_tag,
                   allowed_parents_s);
        }
        parent_tag
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

    fn get_or_create_state_with_attributes(&mut self, attr: &AttributeMap, parallel: bool, parent: StateId) -> StateId {
        let sname: String;
        match attr.get("id") {
            None => sname = self.generate_name(),
            Some(id) => sname = id.clone()
        }
        let id = self.get_or_create_state(&sname, parallel);
        if parent != 0 {
            let state = self.get_state_by_id_mut(id);
            state.parent = parent;
            let parent_state = self.get_state_by_id_mut(parent);
            if !parent_state.states.contains(&id) {
                parent_state.states.push(id);
            }
        }
        id
    }


    // A new "parallel" element started
    fn start_parallel(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_PARALLEL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, true, self.current.current_state);
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
        let state_id = self.get_or_create_state_with_attributes(attr, true, self.current.current_state);

        self.fsm.get_state_by_id_mut(state_id).is_final = true;
        state_id
    }

    // A new "history" element started
    fn start_history(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_FINAL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, true, self.current.current_state);
        if self.current.current_state > 0 {
            let parent_state = self.get_current_state();
            parent_state.history.push(state_id);
        }
        let mut hstate = self.fsm.get_state_by_id_mut(state_id);

        match attr.get(TAG_TYPE) {
            None => hstate.history_type = HistoryType::Shallow,
            Some(type_name) => hstate.history_type = map_history_type(type_name)
        }
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
        let sid = self.get_or_create_state(&name, false);

        let initial;
        match attr.get(TAG_INITIAL) {
            None => initial = 0,
            Some(state_name) => initial = self.get_or_create_state(state_name, false)
        }

        if self.current.current_state > 0 {
            self.get_current_state().states.push(sid);
        }
        self.current.current_state = sid;

        let s = self.get_state_by_id_mut(sid);
        s.initial = initial;

        s.id
    }

    // A "initial" element started (node, not attribute)
    fn start_initial(&mut self) {
        self.verify_parent_tag(TAG_INITIAL, &[TAG_STATE, TAG_PARALLEL]);
        if self.get_current_state().initial > 0 {
            panic!("<{}> must not be specified if initial-attribute was given", TAG_INITIAL)
        }
    }

    fn start_transition(&mut self, attr: &AttributeMap) {
        let parent_tag = self.verify_parent_tag(TAG_TRANSITION,
                                                &[TAG_HISTORY, TAG_INITIAL, TAG_STATE, TAG_PARALLEL]).to_string();

        let mut t = Transition::new();
        let event = attr.get(TAG_EVENT);
        if event.is_some() {
            t.events = event.unwrap().split_whitespace().map(|s| { s.to_string() }).collect();
        }

        let cond = attr.get(TAG_COND);
        if cond.is_some() {
            t.cond = Some(Box::new(ScriptConditionalExpression::new(cond.unwrap())));
        }

        let mut target = attr.get(TAG_TARGET);
        match target {
            None => (),
            // TODO: Parse the state specification! (it can be a list)
            Some(target_name) => {
                self.parseStateSpecification(target_name, &mut t.target);
            }
        }

        let trans_type = attr.get(TAG_TYPE);
        if trans_type.is_some() {
            t.transition_type = map_transition_type(trans_type.unwrap())
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
        t.source = state.id;
        self.fsm.transitions.insert(t.id, t);
    }

    fn start_executable_content(&mut self, name: &str, attr: &AttributeMap) {
        let parent_tag = self.verify_parent_tag(name, &[TAG_ON_ENTRY, TAG_ON_EXIT, TAG_TRANSITION, TAG_FOR_EACH, TAG_IF]).to_string();
        // TODO
    }

    fn start_else(&mut self, name: &str, attr: &AttributeMap) {
        self.verify_parent_tag(name, &[TAG_IF]);
    }

    fn start_element(&mut self, reader: &Reader<&[u8]>, e: &BytesStart) {
        let n = e.name();
        let name = str::from_utf8(n.as_ref()).unwrap();
        self.push(name);

        println!("Start Element {}", name);

        let attr = &decode_attributes(&reader, &mut e.attributes());

        match name {
            TAG_SCXML => {
                if self.in_scxml {
                    panic!("Only one <{}> allowed", TAG_SCXML);
                }
                self.in_scxml = true;
                let datamodel = attr.get(TAG_DATAMODEL);
                if datamodel.is_some() {
                    self.fsm.datamodel = crate::fsm::createDatamodel(datamodel.unwrap());
                }
                let version = attr.get(TAG_VERSION);
                if version.is_some() {
                    self.fsm.version = version.unwrap().clone();
                }
                self.fsm.pseudo_root = self.get_or_create_state_with_attributes(&attr, false, 0);
                self.current.current_state = self.fsm.pseudo_root;
                let initial = attr.get(TAG_INITIAL);
                if initial.is_some() {
                    let mut t = Transition::new();
                    self.parseStateSpecification(initial.unwrap(), &mut t.target);
                    self.fsm.get_state_by_id_mut(self.fsm.pseudo_root).initial = t.id;
                    t.source = self.fsm.pseudo_root;
                    self.fsm.transitions.insert(t.id, t);
                }
            }
            TAG_STATE => {
                self.start_state(attr);
            }
            TAG_PARALLEL => {
                self.start_parallel(attr);
            }
            TAG_FINAL => {
                self.start_final(attr);
            }
            TAG_HISTORY => {
                self.start_history(attr);
            }
            TAG_INITIAL => {
                self.start_initial();
            }
            TAG_TRANSITION => {
                self.start_transition(attr);
            }
            TAG_RAISE | TAG_SEND | TAG_LOG | TAG_SCRIPT | TAG_ASSIGN | TAG_IF | TAG_FOR_EACH | TAG_CANCEL => {
                self.start_executable_content(&name, attr);
            }
            TAG_ELSE | TAG_ELSEIF => {
                self.start_else(&name, attr);
            }
            _ => {
                println!("Ignored tag {}", name)
            }
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
            Ok(Event::Empty(e)) => {
                // Element without content.
                rs.start_element(&mut reader, &e);
                rs.end_element(str::from_utf8(e.name().as_ref()).unwrap());
            }
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),

            // Ignore other
            Ok(e) => println!("Ignored SAX Event {:?}", e),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    rs.fsm
}
