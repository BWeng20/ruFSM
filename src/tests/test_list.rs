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
    let mut l: List<String> = List::new();
    l.push("Abc".to_string());
    l.push("def".to_string());
    l.push("ghi".to_string());
    l.push("xyz".to_string());

    let l2: List<String> = l.filterBy(&|_s: &String| -> bool {
        true
    });
    assert_eq!(l2.size(), l.size());

    let l3 = l2.filterBy(&|_s: &String| -> bool {
        false
    });
    assert_eq!(l3.size(), 0);
}

#[test]
fn list_can_sort() {
    let mut l1: List<String> = List::new();
    l1.push("Xyz".to_string());
    l1.push("Bef".to_string());
    l1.push("Ghi".to_string());
    l1.push("Abc".to_string());

    println!("Unsorted ====");
    let mut l1V: Vec<String> = Vec::new();

    let mut l2 = l1.sort(&|a, b| a.partial_cmp(b).unwrap());

    while l1.size() > 0 {
        let e = l1.head();
        println!(" {}", e);
        l1V.push(e.clone());
        l1 = l1.tail();
    }
    l1V.sort_by(&|a: &String, b: &String| a.partial_cmp(b).unwrap());

    assert_eq!(l1V.len(), l2.size());

    println!("Sorted ======");
    let mut i = 0;
    while l2.size() > 0 {
        let h = l2.head().clone();
        l2 = l2.tail();
        println!(" {}", h);
        assert_eq!(h.eq(l1V.get(i).unwrap()), true);
        i += 1;
    }
    println!("=============");
}

