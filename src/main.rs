mod model;
mod reader;

fn main() {

    let sm = reader::read_from_xml("<scxml initial='Main' datamodel='ecmascript'><state id='Main'></state></scxml>");
    println!("The SM: {:?}", sm)

}


