use std::{
    cell::Cell,
    collections::{HashMap, LinkedList},
};

use derive_visitor::{Drive, DriveMut, Visitor, VisitorMut};

#[derive(Default, Drive, DriveMut)]
struct Top {
    tuple_field: (CountMe1, CountMe2, CountMe1, CountMe2, CountMe1, CountMe2),
    array_field: Box<[CountMe1; 5]>,
    vec_field: Vec<CountMe2>,
    map_field: HashMap<CountMe1, CountMe2>,
    option_field: Option<CountMe2>,
    list_field: LinkedList<CountMe1>,
    cell_field: Cell<CountMe1>,
}

#[derive(Default, Drive, DriveMut, PartialEq, Eq, Hash, Copy, Clone)]
struct CountMe1;

#[derive(Default, Drive, DriveMut, Clone, Debug, PartialEq)]
struct CountMe2(#[drive(skip)] String);

#[derive(Debug, Default, PartialEq, Eq, Visitor)]
#[visitor(CountMe1(enter), CountMe2(enter))]
struct TestVisitor {
    count1: usize,
    count2: usize,
}

impl TestVisitor {
    fn enter_count_me_1(&mut self, _: &CountMe1) {
        self.count1 += 1;
    }
    fn enter_count_me_2(&mut self, _: &CountMe2) {
        self.count2 += 1;
    }
}

#[derive(Debug, Default, PartialEq, Eq, Visitor)]
#[visitor(CountMe1(enter))]
struct TestVisitor2 {
    count1: usize,
}

impl TestVisitor2 {
    fn enter_count_me_1(&mut self, _: &CountMe1) {
        self.count1 += 1;
    }
}

#[test]
fn test_containers() {
    let mut top = Top::default();
    top.vec_field.push(CountMe2("hello".to_string()));
    top.vec_field.push(CountMe2("you".to_string()));
    top.map_field.insert(CountMe1, CountMe2("are".to_string()));
    top.list_field.push_back(CountMe1);
    top.option_field = Some(CountMe2("beautiful".to_string()));
    let mut test_visitor = TestVisitor::default();
    top.drive(&mut test_visitor);

    // Count1:
    //   tuple: 3
    //   array: 5
    //   map: 1
    //   list: 1
    //   cell: 1
    //   SUM: 11
    // Count2:
    //   tuple: 3
    //   vec: 2
    //   map: 1
    //   option: 1
    //   SUM: 7
    assert_eq!(
        test_visitor,
        TestVisitor {
            count1: 11,
            count2: 7,
        }
    );
}

#[test]
fn test_containers_mut() {
    let mut top = Top::default();
    top.map_field
        .insert(CountMe1, CountMe2("bad word".to_string()));
    top.option_field = Some(CountMe2("worst".to_string()));

    #[derive(VisitorMut)]
    #[visitor(CountMe2(enter))]
    struct Censor;

    impl Censor {
        fn enter_count_me_2(&mut self, me2: &mut CountMe2) {
            me2.0 = "censored".to_string();
        }
    }

    top.drive_mut(&mut Censor);
    assert_eq!(top.map_field.get(&CountMe1).unwrap().0, "censored");
    assert_eq!(top.option_field, Some(CountMe2("censored".to_string())));
}
