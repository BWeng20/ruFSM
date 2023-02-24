use crate::fsm::List;

#[test]
fn list_can_can_push() {
    let mut l: List<String> = List::new();

    l.push("Abc".to_string());
    l.push("def".to_string());
    l.push("ghi".to_string());
    l.push("xyz".to_string());
    assert_eq!(l.size(), 4);
}

#[test]
fn list_can_head() {
    let mut l1: List<String> = List::new();

    l1.push("Abc".to_string());
    l1.push("def1".to_string());
    l1.push("ghi1".to_string());

    assert_eq!(l1.head(), &"Abc".to_string());
}

#[test]
fn list_can_tail() {
    let mut l1: List<String> = List::new();

    l1.push("Abc".to_string());
    l1.push("def1".to_string());
    l1.push("ghi1".to_string());

    assert_eq!(l1.tail().size(), 2);
    assert_eq!(l1.size(), 3);
}


#[test]
fn list_can_append() {
    let mut l1: List<String> = List::new();

    l1.push("Abc".to_string());
    l1.push("def1".to_string());
    l1.push("ghi1".to_string());
    l1.push("xyz1".to_string());

    let mut l2: List<String> = List::new();
    l2.push("Abc".to_string());
    l2.push("def2".to_string());
    l2.push("ghi2".to_string());
    l2.push("xyz2".to_string());

    let l3 = l1.append(&l2);
    assert_eq!(l3.size(), l1.size() + l2.size());

    let l4 = l1.append(&l1);
    assert_eq!(l3.size(), 2 * l1.size());
}

#[test]
fn list_can_some() {
    let mut l: List<String> = List::new();
    l.push("Abc".to_string());
    l.push("def".to_string());
    l.push("ghi".to_string());
    l.push("xyz".to_string());

    let m = l.some(&|s| -> bool {
        *s == "Abc".to_string()
    });

    assert_eq!(m, true);
}

#[test]
fn list_can_every() {
    let mut l: List<String> = List::new();
    l.push("Abc".to_string());
    l.push("def".to_string());
    l.push("ghi".to_string());
    l.push("xyz".to_string());

    let mut m = l.every(&|_s| -> bool {
        true
    });
    assert_eq!(m, true);

    m = l.every(&|s| -> bool {
        !s.eq(&"ghi".to_string())
    });
    assert_eq!(m, false);
}

#[test]
fn list_can_filter() {
    let mut l1: List<String> = List::new();
    l1.push("Abc".to_string());
    l1.push("def".to_string());
    l1.push("ghi".to_string());
    l1.push("xyz".to_string());

    let l2 = l1.filter(&|_s| -> bool {
        true
    });
    assert_eq!(l2.size(), l1.size());

    let l3 = l2.filter(&|_s| -> bool {
        false
    });
    assert_eq!(l3.size(), 0);
}

