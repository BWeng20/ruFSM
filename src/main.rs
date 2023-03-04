extern crate core;

use std::{thread, time};

use crate::fsm::Event;

mod reader;
mod fsm;

mod tests;

fn main() {
    println!("Creating The SM:");
    let sm = reader::read_from_xml(
        r"<scxml initial='Main' datamodel='ecmascript'>
      <state id='Main'>
        <initial>
          <transition target='MainA'/>
        </initial>
        <state id='MainA'>
          <transition event='a ab abc' cond='true' type='internal' target='finalMe'/>
        </state>
        <state id='MainB'>
        </state>
        <final id='finalMe'>
          <onentry>
            <log label='info' expr='Date.now()'/>
          </onentry>
        </final>
      </state>
    </scxml>");
    println!("The SM: {}", sm);

    let jh = fsm::start_fsm(sm);

    let ten_millis = time::Duration::from_millis(1000);
    thread::sleep(ten_millis);

    println!("Send Event");

    jh.1.send(Box::new(Event { name: "ab".to_string(), invokeid: 1, done_data: None }));
    jh.0.join();
}


