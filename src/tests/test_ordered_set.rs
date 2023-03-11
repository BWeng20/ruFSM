#[test]
fn ordered_set_can_add_and_delete() {
    let mut os: OrderedSet<String> = OrderedSet::new();

    os.add("Abc".to_string());
    os.add("def".to_string());
    os.add("ghi".to_string());
    os.add("xyz".to_string());
    assert_eq!(os.size(), 4);

    os.delete(&"Abc".to_string());
    os.delete(&"ghi".to_string());
    os.delete(&"xxx".to_string());
    os.delete(&"Abc".to_string()); // should be ignored.

    assert_eq!(os.size(), 2);
}

#[test]
fn ordered_set_can_union() {
    let mut os1: OrderedSet<String> = OrderedSet::new();

    os1.add("Abc".to_string());
    os1.add("def1".to_string());
    os1.add("ghi1".to_string());
    os1.add("xyz1".to_string());

    let mut os2: OrderedSet<String> = OrderedSet::new();
    os2.add("Abc".to_string());
    os2.add("def2".to_string());
    os2.add("ghi2".to_string());
    os2.add("xyz2".to_string());

    os1.union(&os2);

    assert_eq!(os1.size(), 7);
    assert_eq!(os1.isMember(&"def2".to_string()), true);
    assert_eq!(os1.isMember(&"Abc".to_string()), true);
}

#[test]
fn ordered_set_can_toList() {
    let mut os: OrderedSet<String> = OrderedSet::new();
    os.add("Abc".to_string());
    os.add("def".to_string());
    os.add("ghi".to_string());
    os.add("xyz".to_string());

    let l = os.toList();

    assert_eq!(l.size(), os.size());
}

#[test]
fn ordered_set_can_some() {
    let mut os: OrderedSet<String> = OrderedSet::new();
    os.add("Abc".to_string());
    os.add("def".to_string());
    os.add("ghi".to_string());
    os.add("xyz".to_string());

    let m = os.some(&|s| -> bool {
        *s == "Abc".to_string()
    });

    assert_eq!(m, true);
}

#[test]
fn ordered_set_can_every() {
    let mut os: OrderedSet<String> = OrderedSet::new();
    os.add("Abc".to_string());
    os.add("def".to_string());
    os.add("ghi".to_string());
    os.add("xyz".to_string());

    let mut m = os.every(&|_s| -> bool {
        true
    });
    assert_eq!(m, true);

    m = os.every(&|s| -> bool {
        !s.eq(&"ghi".to_string())
    });
    assert_eq!(m, false);
}

#[test]
fn ordered_set_can_hasIntersection() {
    let mut os1: OrderedSet<String> = OrderedSet::new();
    os1.add("Abc".to_string());
    os1.add("def".to_string());
    os1.add("ghi".to_string());
    os1.add("xyz".to_string());

    let mut os2: OrderedSet<String> = OrderedSet::new();

    let mut m = os1.hasIntersection(&os2);
    assert_eq!(m, false);

    // One common elements
    os2.add("Abc".to_string());
    m = os1.hasIntersection(&os2);
    assert_eq!(m, true);

    // Same other un-common elements
    os2.add("Def".to_string());
    os2.add("Ghi".to_string());
    os2.add("Xyz".to_string());
    m = os1.hasIntersection(&os2);
    assert_eq!(m, true);

    // Same with TWO common elements
    os2.add("def".to_string());
    m = os1.hasIntersection(&os2);
    assert_eq!(m, true);

    // Remove common elements from first
    os1.delete(&"Abc".to_string());
    os1.delete(&"def".to_string());
    m = os1.hasIntersection(&os2);
    assert_eq!(m, false);

    // Always common with itself
    m = os1.hasIntersection(&os1);
    assert_eq!(m, true);

    // but not if empty
    os1.clear();
    m = os1.hasIntersection(&os1);
    // Shall return false
    assert_eq!(m, false);
}

#[test]
fn ordered_set_can_isEmpty() {
    let mut os1: OrderedSet<String> = OrderedSet::new();
    assert_eq!(os1.isEmpty(), true);

    os1.add("Abc".to_string());
    assert_eq!(os1.isEmpty(), false);
}

#[test]
fn ordered_set_can_clear() {
    let mut os1: OrderedSet<String> = OrderedSet::new();
    os1.add("Abc".to_string());
    os1.clear();
    assert_eq!(os1.isEmpty(), true);
}

