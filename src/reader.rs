use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::Event;
use quick_xml::Reader;
use crate::model::{Fsm,State};
use std::convert::Infallible;

pub fn read_from_xml_file(mut file: File ) -> Fsm {

    let mut contents = String::new();

    let r = file.read_to_string(&mut contents);
    println!("Read {}", r.unwrap());

    let fsm = read_from_xml(contents.as_str());

    fsm
}

struct ReaderState {

    datamodel : String,
    version : String,

    fsm : Option<Fsm>,
    state: Option<State>,
}

impl ReaderState {



}

fn decodeAttributes(reader : &Reader<&[u8]>, attr : Attributes) -> HashMap<String,String> {
    attr.map(|attr_result| {
            match attr_result {
                Ok(a) => {
                    let key = reader.decoder().decode(a.key.local_name().as_ref())
                        .or_else(|err| {
                            dbg!("unable to read key in DefaultSettings attribute {:?}, utf8 error {:?}", &a, err);
                            Ok::<Cow<'_, str>, Infallible>(std::borrow::Cow::from(""))
                        })
                        .unwrap().to_string();
                    let value = a.decode_and_unescape_value(&reader).or_else(|err| {
                        dbg!("unable to read key in DefaultSettings attribute {:?}, utf8 error {:?}", &a, err);
                        Ok::<Cow<'_, str>, Infallible>(std::borrow::Cow::from(""))
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

    let mut fsm : Option<Fsm> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"scxml" => {
                        fsm = Some(Fsm::new());
                        let map = decodeAttributes(&reader, e.attributes());
                        println!("scxml attributes : {:?}", map );
                    },
                    b"state" => {
                                 decodeAttributes(&reader, e.attributes());

                    },
                    b"initial" => println!("initial"),
                    _ => (),
                }
            }
            Ok(Event::End(_e)) => println!("End"),
            Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),

            // There are several other `Event`s we do not consider here
            _ => (),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }

    fsm.unwrap()
}
