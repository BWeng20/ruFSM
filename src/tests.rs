#[test]
#[should_panic]
fn initial_attribute_should_panic() {
    crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'>\
    <initial><transition></transition></initial></state></scxml>");
}

#[test]
fn initial_attribute() {
    crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></state></scxml>");
}

#[test]
#[should_panic]
fn wrong_end_tag_should_panic() {
    crate::reader::read_from_xml("<scxml initial='Main'><state id='Main' initial='A'></parallel></scxml>");
}

#[test]
#[should_panic]
fn wrong_transition_type_should_panic() {
    crate::reader::read_from_xml(
        "<scxml><state><transition type='bla'></transition></state></scxml>");
}

#[test]
fn transition_type_internal() {
    crate::reader::read_from_xml(
        "<scxml><state><transition type='internal'></transition></state></scxml>");
}

#[test]
fn transition_type_external() {
    crate::reader::read_from_xml(
        "<scxml><state><transition type='external'></transition></state></scxml>");
}
