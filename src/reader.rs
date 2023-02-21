use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::rc::Rc;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use crate::model::{Fsm, Id, State, StateRef};
use std::str;

type AttributeMap = HashMap<String,String>;

pub fn read_from_xml_file(mut file: File ) -> Box<Fsm> {

    let mut contents = String::new();

    let r = file.read_to_string(&mut contents);
    println!("Read {}", r.unwrap());

    let fsm = read_from_xml(contents.as_str());

    fsm
}

pub const TAG_SCXML: &str = "scxml";
pub const TAG_DATAMODEL: &str = "datamodel";
pub const TAG_VERSION: &str = "version";
pub const TAG_INITIAL: &str = "initial";
pub const TAG_STATE: &str = "state";
pub const TAG_PARALLEL: &str = "parallel";
pub const TAG_TRANSITION: &str = "transition";

struct ReaderStackItem {
    current_state: Option<Id>,
    current_tag: String,
}

impl ReaderStackItem {
    pub fn new( o : &ReaderStackItem ) -> ReaderStackItem {
        ReaderStackItem {
            current_state: o.current_state.clone(),
            current_tag: o.current_tag.clone()         }
    }
}


struct ReaderState {

    // True if reader in inside an scxml element
    in_scxml : bool,
    id_count : i32,

    // The resulting fsm
    fsm : Box<Fsm>,

    current : ReaderStackItem,
    stack : Vec<ReaderStackItem>

}


impl ReaderState {

    pub fn new() -> ReaderState {
        ReaderState {
            in_scxml: false,
            id_count: 0,
            stack: vec![],
            current: ReaderStackItem {
                current_state: None,
                current_tag: "".to_string() },
            fsm: Box::new(Fsm::new() )
        }
    }

    pub fn push(&mut self, tag : &str) {

        self.stack.push( ReaderStackItem::new(&self.current));
        self.current.current_tag = tag.to_string();
    }

    pub fn pop(&mut self) {
        let p = self.stack.pop();
        if p.is_some() {
            self.current = p.unwrap();
        }
    }

    pub fn generate_id(&mut self) -> String {
        self.id_count += 1;
        format!("__id{}", self.id_count)
    }

    fn get_state(&self, id : &Id) -> Option<StateRef> {
        self.fsm.states.get(id).cloned()
    }

    pub fn get_current_state(&self) -> StateRef {
        {
            if self.current.current_state.is_none() {
                panic!("Internal error: Current State is unknown");
            }
        }
        let state = self.get_state(
                &self.current.current_state.as_ref().unwrap() );
        if state.is_none() {
            panic!("Internal error: Current State {} is unknown", self.current.current_state.as_ref().unwrap());
        }
        state.unwrap()
    }

    pub fn get_parent_tag(&self) -> &str {
        let mut r = "";
        if  !self.stack.is_empty() {
            r = self.stack.get(self.stack.len()-1).as_ref().unwrap().current_tag.as_str();
        }
        r
    }

    fn create_state( &mut self, attr : &AttributeMap )-> Id {
        let mut id = attr.get("id").cloned();
        if id.is_none() {
            id = Some(self.generate_id());
        }
        let sid = id.unwrap();
        let s = State::new( sid.as_str() );
        self.fsm.states.insert(s.id.clone(), Rc::new( RefCell::new(s)) ); // s.id, s);
        sid
    }



    // A new "parallel" element started
    fn start_parallel(&mut self, attr : &AttributeMap) -> Id {
        if  !self.in_scxml {
            panic!("<parallel> needed to be below <scxml>");
        }
        let mut id = attr.get("id").cloned();
        if id.is_none() {
            id = Some(self.generate_id());
        }

        let s = self.create_state(attr);

        let s = self.current.current_state.as_ref();
        if  s.is_some() {
            let parent_state = self.get_current_state();

            (*parent_state).borrow_mut().parallel.push(s.unwrap().clone());
        }

        id.unwrap()

    }

    // A new "state" element started
    fn start_state(&mut self, attr : &AttributeMap) -> Id {
        if  !self.in_scxml {
            panic!("<state> needed to be below <scxml>");
        }
        let mut id = attr.get("id").cloned();
        if id.is_none() {
            id = Some(self.generate_id());
        }
        let mut s = State::new(id.unwrap().as_str() );

        s.initial = attr.get(TAG_INITIAL).cloned();

        let sr = s.id.clone();

        self.current.current_state = Some(s.id.clone());
        self.fsm.states.insert(s.id.clone(), Rc::new( RefCell::new(s) )); // s.id, s);

        sr
    }

    // A "initial" element startet (node not attribute)
    fn start_initial(&mut self) {
        if [TAG_STATE, TAG_PARALLEL].contains(&self.get_parent_tag() ) {
            if (self.get_current_state().borrow()).initial.is_some() {
                panic!("<initial> must not be specified if initial-attribute was given")
            }
            // Next a "<transition>" must follow

        } else {
            panic!("<initial> only allowed inside <state> or <parallel>");
        }
    }

    fn start_transition(&mut self) {
        let parent_tag = &self.get_parent_tag();
        if [TAG_INITIAL, TAG_STATE, TAG_PARALLEL].contains(parent_tag ) {


        } else {
            panic!("<transition> inside <{}>. Only allowed inside <initial>, <state> or <parallel>", parent_tag);
        }
    }


    fn start_element(&mut self, reader : &Reader<&[u8]>, e : &BytesStart ) {
        let n = e.name();
        let name = str::from_utf8(n.as_ref()).unwrap();
        self.push( name );
        match name {
            TAG_SCXML => {
                if self.in_scxml {
                    panic!("Only one <scxml> allowed");
                }
                self.in_scxml = true;
                let map = decode_attributes(&reader, &mut e.attributes());
                println!("scxml attributes : {:?}", map);
                let datamodel = map.get(TAG_DATAMODEL);
                if datamodel.is_some() {
                    self.fsm.datamodel = datamodel.unwrap().clone();
                }
                let version = map.get(TAG_VERSION);
                if version.is_some() {
                    self.fsm.version = version.unwrap().clone();
                }
                let initial = map.get(TAG_INITIAL);
                if initial.is_some() {
                    self.fsm.initial = Some(initial.unwrap().clone());
                }
            },
            TAG_STATE => {
                self.start_state(&decode_attributes(&reader, &mut e.attributes()));
            },
            TAG_PARALLEL => {
                self.start_parallel(&decode_attributes(&reader, &mut e.attributes()));
            },
            TAG_INITIAL => {
                self.start_initial();
            },
            TAG_TRANSITION => {
                self.start_transition();
            },
            _ => (),
        }

    }

    fn end_element(&mut self, name : &str) {
        if  !self.current.current_tag.eq(name ) {
            panic!("Illegal end-tag {:?}, expected {:?}", &name, &self.current.current_tag);
        }
        self.pop();
    }


}

/**
 * Decode attributes into a hash-map
 */
fn decode_attributes(reader : &Reader<&[u8]>, attr : &mut Attributes) -> AttributeMap {
    attr.map(|attr_result| {
            match attr_result {
                Ok(a) => {
                    let key = reader.decoder().decode(a.key.as_ref() );
                    if key.is_err() {
                            panic!("unable to read attribute name {:?}, utf8 error {:?}", &a, key.err());
                    }
                    let value = a.decode_and_unescape_value(&reader);
                    if value.is_err() {
                        panic!("unable to read attribute value  {:?}, utf8 error {:?}", &a, value.err());
                    }
                    (key.unwrap().to_string(), value.unwrap().to_string())
                },
                Err(err) => {
                    panic!("unable to read key in DefaultSettings, err = {:?}", err);
                }
            }
        }).collect()
}

pub  fn read_from_xml(xml : &str) -> Box<Fsm> {

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

            Ok(Event::Start( e)) => {
                rs.start_element(&mut reader, &e );
            }
            Ok(Event::End(e)) => {
                rs.end_element(str::from_utf8( e.name().as_ref() ).unwrap() );
            },
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),

            // There are several other `Event`s we do not consider here
            _ => (),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    rs.fsm
}
