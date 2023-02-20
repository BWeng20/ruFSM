use std::borrow::{Cow};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::Event;
use quick_xml::Reader;
use crate::model::{Fsm, Id, State};
use std::convert::Infallible;

type AttributeMap = HashMap<String,String>;

pub fn read_from_xml_file(mut file: File ) -> Fsm {

    let mut contents = String::new();

    let r = file.read_to_string(&mut contents);
    println!("Read {}", r.unwrap());

    let fsm = read_from_xml(contents.as_str());

    fsm
}

pub const TAG_DATAMODEL: &str = "datamodel";
pub const TAG_VERSION: &str = "version";
pub const TAG_INITIAL: &str = "initial";

struct ReaderState {
    datamodel : String,
    version : String,

    in_scxml : bool,

    fsm : Fsm,
    current_state: Option<Id>,
    id_count : i32

}

impl ReaderState {

    pub fn new() -> ReaderState {
        ReaderState {
            datamodel: "".to_string(),
            version: "".to_string(),
            in_scxml: false,
            fsm: Fsm::new(),
            current_state: None,
            id_count: 0
        }
    }

    pub fn generate_id(&mut self) -> String {
        self.id_count += 1;
        format!("__id{}", self.id_count)
    }


    // A new "state" element started
    fn start_state(&mut self, attr : AttributeMap) {
            if  !self.in_scxml {
                panic!("<state> needed to be inside <scxml>");
            }
            let mut id = attr.get("id").cloned();
            if id.is_none() {
                id = Some(self.generate_id());
            }
            let s = State::new(id.unwrap().as_str() );

            self.current_state = Some(s.id.clone());
            self.fsm.states.insert(s.id.clone(), s ); // s.id, s);
    }

    // A "state" element ended
    fn end_state(&mut self) {
    }

    fn end_element(&mut self, name : &[u8]) {
        match name {
            b"state" => {
                self.end_state();
            },
            b"scxml" => {
                self.in_scxml = false;
            },
            _ => (),
        }
    }


}

/**
 * Decode attributes into a hash-map
 */
fn decode_attributes(reader : &Reader<&[u8]>, attr : Attributes) -> AttributeMap {
    attr.map(|attr_result| {
            match attr_result {
                Ok(a) => {
                    let key = reader.decoder().decode(a.key.local_name().as_ref())
                        .or_else(|err| {
                            dbg!("unable to read attribute name {:?}, utf8 error {:?}", &a, err);
                            Ok::<Cow<str>, Infallible>(std::borrow::Cow::from(""))
                        })
                        .unwrap().to_string();
                    let value = a.decode_and_unescape_value(&reader).or_else(|err| {
                        dbg!("unable to read attribute value  {:?}, utf8 error {:?}", &a, err);
                        Ok::<Cow<str>, Infallible>(std::borrow::Cow::from(""))
                    }).unwrap().to_string();
                    (key, value)
                },
                Err(err) => {
                    dbg!("unable to read key in DefaultSettings, err = {:?}", err);
                    (String::new(), String::new())
                }
            }
        }).collect()
}

pub  fn read_from_xml(xml : &str) -> Fsm {

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
                match e.name().as_ref() {
                    b"scxml" => {
                        if rs.in_scxml {
                            panic!("Only one <scxml> allowed");
                        }
                        rs.in_scxml = true;
                        let map = decode_attributes(&reader, e.attributes());
                        println!("scxml attributes : {:?}", map );
                        let datamodel = map.get(TAG_DATAMODEL);
                        if datamodel.is_some() {
                            rs.fsm.datamodel = datamodel.unwrap().clone();
                        }
                        let version = map.get(TAG_VERSION);
                        if version.is_some() {
                            rs.fsm.version  = version.unwrap().clone();
                        }
                        let initial = map.get(TAG_INITIAL);
                        if initial.is_some() {
                            rs.fsm.initial  = Some(initial.unwrap().clone());
                        }
                    },
                    b"state" => {
                        rs.start_state(decode_attributes(&reader, e.attributes()));
                    },
                    b"initial" => println!("initial"),
                    _ => (),
                }
            }
            Ok(Event::End(e)) => {
                rs.end_element(e.name().as_ref());
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
