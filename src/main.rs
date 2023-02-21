extern crate core;

mod model;
mod reader;
mod tests;

fn main() {
    let sm = reader::read_from_xml(r"
    <scxml initial='Main' datamodel='ecmascript'>
      <state id='Main'>
        <initial><transition event='a ab abc' cond='true' type='internal' ></transition></initial>
      </state>
    </scxml>");
    println!("The SM: {:?}", sm)

}


