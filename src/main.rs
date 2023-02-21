extern crate core;

mod reader;
mod tests;
mod fsm;

fn main() {
    let sm = reader::read_from_xml(r"
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
    </scxml>");
    println!("The SM: {}", sm)
}


