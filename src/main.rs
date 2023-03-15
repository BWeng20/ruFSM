extern crate core;

use std::{thread, time};

use crate::fsm::{Event, EventType, Trace};

mod reader;
mod fsm;

mod tests;
#[cfg(feature = "ECMAScript")]
mod emca_script_datamodel;

fn main() {
    println!("Creating The SM:");
    let mut sm = reader::read_from_xml(
        r"<scxml initial='Main' datamodel='ecmascript'>
      <script>
        log('Hello World', ' again ');
        log('Hello Again');
      </script>
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
        <transition event='exit' cond='true' type='internal' target='OuterFinal'/>
      </state>
      <final id='OuterFinal'>
      </final>
    </scxml>");
    println!("The SM: {}", sm);

    sm.tracer.enableTrace(Trace::ALL);

    let (threadHandle, sender) = fsm::start_fsm(sm);

    let ten_millis = time::Duration::from_millis(1000);
    thread::sleep(ten_millis);

    println!("Send Event");

    sender.send(Box::new(Event { name: "ab".to_string(), etype: EventType::platform, sendid: 0, origin: "".to_string(), origintype: "".to_string(), invokeid: 1, data: None }));
    sender.send(Box::new(Event { name: "exit".to_string(), etype: EventType::platform, sendid: 0, origin: "".to_string(), origintype: "".to_string(), invokeid: 2, data: None }));

    threadHandle.join();
}


