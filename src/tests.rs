
#[test]
#[should_panic]
fn initial_attribute_should_panic() {
    let sm = crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'>\
    <initial><transition></transition></initial></state></scxml>");
}

#[test]
fn initial_attribute() {
    let sm = crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></state></scxml>");
}

#[test]
#[should_panic]
fn wrong_end_tag_should_panic() {
    let sm = crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></parallel></scxml>");
}
