extern crate core;

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
          <transition event='a ab abc' cond='true' type='internal'></transition>
        </initial>
        <state id='MainA'>
        </state>
        <state id='MainB'>
        </state>
      </state>
    </scxml>");
    println!("The SM: {}", sm);

    let jh = fsm::start_fsm(sm);

    jh.1.send(Event { name: "Name".to_string(), invokeid: 1, done_data: None });
    jh.0.join();
}


