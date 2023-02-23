extern crate core;

use crate::fsm::Event;

mod reader;
mod tests;
mod fsm;

fn main() {
    let jh =
        fsm::start_fsm(r"
    <scxml initial='Main' datamodel='ecmascript'>
      <state id='Main'>
        <initial>
          <transition event='a ab abc' cond='true' type='internal'></transition>
        </initial>
        <state id='MainA'>
        </state>
        <state id='MainB'>
        </state>
      </state>
    </scxml>".to_string());

    jh.1.send(Event { name: "Name".to_string() });
    jh.0.join();
}


