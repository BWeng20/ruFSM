use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::str;
use std::sync::atomic::{AtomicU32, Ordering};

use log::debug;
use quick_xml::events::{BytesStart, Event};
use quick_xml::events::attributes::Attributes;
use quick_xml::Reader;

use crate::fsm::{Fsm, HistoryType, map_history_type, map_transition_type, Name, State, StateId, Transition};

pub type AttributeMap = HashMap<String, String>;

static DOC_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

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

pub const TAG_INCLUDE: &str = "include";
pub const TAG_HREF: &str = "href";


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

pub const NS_XINCLUDE: &str = "http://www.w3.org/2001/XInclude";

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
    file: String,

    // The resulting fsm
    fsm: Box<Fsm>,

    current: ReaderStackItem,
    stack: Vec<ReaderStackItem>,
}


impl ReaderState {
    pub fn new(f: &String) -> ReaderState {
        ReaderState {
            in_scxml: false,
            id_count: 0,
            stack: vec![],
            current: ReaderStackItem {
                current_state: 0,
                current_tag: "".to_string(),
            },
            fsm: Box::new(Fsm::new()),
            file: f.clone(),
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

    fn parse_state_specification(&mut self, target_name: &str, targets: &mut Vec<StateId>) {
        target_name.split_ascii_whitespace().for_each(|target| {
            targets.push(self.get_or_create_state(&target.to_string(), false))
        });
    }

    fn get_state_by_name(&self, name: &Name) -> Option<&State> {
        if self.fsm.global().borrow().statesNames.contains_key(name) {
            Some(self.fsm.get_state_by_name(name))
        } else { None }
    }

    fn get_state_by_name_mut(&mut self, name: &Name) -> Option<&mut State> {
        if self.fsm.global().borrow().statesNames.contains_key(name) {
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
        let m = self.fsm.global().borrow().statesNames.get(name).cloned();
        match m {
            None => {
                let mut s = State::new(name);
                s.id = (self.fsm.states.len() + 1) as StateId;
                s.is_parallel = parallel;
                let sid = s.id;
                let gd = self.fsm.global();
                gd.borrow_mut().statesNames.insert(s.name.clone(), s.id); // s.id, s);
                self.fsm.states.push(s);
                sid
            }
            Some(id) => {
                if parallel {
                    self.fsm.states.get_mut((id - 1) as usize).unwrap().is_parallel = true;
                }
                id
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

        let initial;
        match attr.get(TAG_INITIAL) {
            None => initial = 0,
            Some(state_name) => initial = self.get_or_create_state(state_name, false)
        }

        let state = self.get_state_by_id_mut(id);

        state.doc_id = DOC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        if initial != 0 {
            state.initial = initial;
        }
        if parent != 0 {
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
        state_id
    }

    // A new "final" element started
    fn start_final(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_FINAL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, false, self.current.current_state);

        self.fsm.get_state_by_id_mut(state_id).is_final = true;
        state_id
    }

    // A new "history" element started
    fn start_history(&mut self, attr: &AttributeMap) -> StateId {
        if !self.in_scxml {
            panic!("<{}> needed to be inside <{}>", TAG_FINAL, TAG_SCXML);
        }
        let state_id = self.get_or_create_state_with_attributes(attr, false, self.current.current_state);
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
        let sid = self.get_or_create_state_with_attributes(&attr, false, self.current.current_state);
        self.current.current_state = sid;
        sid
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
        t.doc_id = DOC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let event = attr.get(TAG_EVENT);
        if event.is_some() {
            t.events = event.unwrap().split_whitespace().map(|s| { s.to_string() }).collect();
        }

        let cond = attr.get(TAG_COND);
        if cond.is_some() {
            t.cond = Some(cond.unwrap().clone());
        }

        let target = attr.get(TAG_TARGET);
        match target {
            None => (),
            // TODO: Parse the state specification! (it can be a list)
            Some(target_name) => {
                self.parse_state_specification(target_name, &mut t.target);
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

    fn end_script(&mut self, txt: &mut Vec<String>) {
        self.get_current_state().script = txt.concat();
        txt.clear();
    }


    fn start_executable_content(&mut self, name: &str) {
        self.verify_parent_tag(name, &[TAG_SCXML, TAG_ON_ENTRY, TAG_ON_EXIT, TAG_TRANSITION, TAG_FOR_EACH, TAG_IF]).to_string();
        // TODO
    }

    fn start_else(&mut self, name: &str) {
        self.verify_parent_tag(name, &[TAG_IF]);
    }

    fn start_element(&mut self, reader: &Reader<Box<dyn BufRead>>, e: &BytesStart) {
        let n = e.local_name();
        let name = str::from_utf8(n.as_ref()).unwrap();
        self.push(name);

        debug!("Start Element {}", name);

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
                    self.fsm.global().borrow_mut().version = version.unwrap().clone();
                }
                self.fsm.pseudo_root = self.get_or_create_state_with_attributes(&attr, false, 0);
                self.current.current_state = self.fsm.pseudo_root;
                let initial = attr.get(TAG_INITIAL);
                if initial.is_some() {
                    let mut t = Transition::new();
                    self.parse_state_specification(initial.unwrap(), &mut t.target);
                    self.fsm.get_state_by_id_mut(self.fsm.pseudo_root).initial = t.id;
                    t.source = self.fsm.pseudo_root;
                    self.fsm.transitions.insert(t.id, t);
                }
            }
            TAG_INCLUDE => {
                self.include(attr);
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
            TAG_SCRIPT | TAG_RAISE | TAG_SEND | TAG_LOG | TAG_ASSIGN | TAG_IF | TAG_FOR_EACH | TAG_CANCEL => {
                self.start_executable_content(&name);
            }
            TAG_ELSE | TAG_ELSEIF => {
                self.start_else(&name);
            }
            _ => {
                debug!("Ignored tag {}", name)
            }
        }
    }

    fn include(&mut self, attr: &AttributeMap) {
        let href = attr.get(TAG_HREF);
        if href.is_none() {
            panic!("{} in <{}> missing", TAG_HREF, TAG_INCLUDE);
        }
        let mut src = Path::new(href.unwrap()).clone().to_owned();

        let parent = Path::new(&self.file).parent();
        match parent {
            Some(parent_path) => {
                let pp = parent_path.join(src);
                src = pp.to_owned();
            }
            None => {}
        }

        match File::open(src.clone()) {
            Ok(f) => {
                let org_file = self.file.clone();
                self.file = src.to_str().unwrap().to_string();
                read_all_events(self, Box::new(BufReader::new(f)));
                self.file = org_file;
            }
            Err(e) => {
                panic!("Can't read '{}' in <{}>. {}", src.to_str().unwrap(), TAG_INCLUDE, e);
            }
        }
    }

    fn end_element(&mut self, name: &str, txt: &mut Vec<String>) {
        if !self.current.current_tag.eq(name) {
            panic!("Illegal end-tag {:?}, expected {:?}", &name, &self.current.current_tag);
        }
        debug!("End Element {}", name);
        match name {
            TAG_SCRIPT => {
                self.end_script(txt);
            }
            _ => {}
        }
        self.pop();
    }
}

/**
 * Decode attributes into a hash-map
 */
fn decode_attributes(reader: &Reader<Box<dyn BufRead>>, attr: &mut Attributes) -> AttributeMap {
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

/// Reads the FSM from a XML file
pub fn read_from_xml_file(file: String) -> Result<Box<Fsm>, String> {
    match File::open(file.clone()) {
        Ok(f) => {
            read(Box::new(BufReader::new(f)), &file)
        }
        Err(e) => {
            Err(format!("Failed to read {}. {}", file, e))
        }
    }
}

/// Reads the FSM from a XML String
pub fn read_from_xml(xml: String) -> Result<Box<Fsm>, String> {
    let fakeFile = "".to_string();
    read(Box::new(Cursor::new(xml)), &fakeFile)
}

fn read(buf: Box<dyn BufRead>, f: &String) -> Result<Box<Fsm>, String> {
    let mut rs = ReaderState::new(f);
    let r = read_all_events(&mut rs, buf);
    match r {
        Ok(m) => {
            Ok(rs.fsm)
        }
        Err(e) => {
            Err(e)
        }
    }
}

fn read_all_events(rs: &mut ReaderState, buf: Box<dyn BufRead>) -> Result<&str, String> {
    debug!(">>> Reading {}", rs.file);

    let mut reader = Reader::from_reader(buf);
    reader.trim_text(true);
    let mut txt = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => {
                debug!("<<< {}", rs.file);
                return Err(format!("Error at position {}: {:?}", reader.buffer_position(), e));
            }
// exits the loop when reaching end of file
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                rs.start_element(&mut reader, &e);
            }
            Ok(Event::End(e)) => {
                rs.end_element(str::from_utf8(e.local_name().as_ref()).unwrap(), &mut txt);
            }
            Ok(Event::Empty(e)) => {
// Element without content.
                rs.start_element(&mut reader, &e);
                rs.end_element(str::from_utf8(e.local_name().as_ref()).unwrap(), &mut txt);
            }
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),
            Ok(Event::Comment(e)) => debug!("Comment :{}", e.unescape().unwrap()),

// Ignore other
            Ok(e) => debug!("Ignored SAX Event {:?}", e),
        }
// if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    debug!("<<< {}", rs.file);
    Ok("ok")
}

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic]
    fn initial_attribute_should_panic() {
        crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'>\
    <initial><transition></transition></initial></state></scxml>".to_string());
    }

    #[test]
    fn initial_attribute() {
        crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></state></scxml>".to_string());
    }

    #[test]
    fn wrong_end_tag_should_panic() {
        let r = crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></parallel></scxml>".to_string());
        assert!(r.is_err(), "Shall result in error");
    }

    #[test]
    #[should_panic]
    fn wrong_transition_type_should_panic() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='bla'></transition></state></scxml>".to_string());
    }

    #[test]
    fn transition_type_internal() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='internal'></transition></state></scxml>".to_string());
    }

    #[test]
    fn transition_type_external() {
        crate::reader::read_from_xml(
            "<scxml><state><transition type='external'></transition></state></scxml>".to_string());
    }
}