use std::{
    cell::Cell,
    collections::{HashMap, LinkedList},
};

use derive_visitor::{Drive, Visitor};

#[derive(Default, Drive)]
struct Top {
    tuple_field: (CountMe1, CountMe2, CountMe1, CountMe2, CountMe1, CountMe2),
    array_field: Box<[CountMe1; 5]>,
    vec_field: Vec<CountMe2>,
    map_field: HashMap<CountMe1, CountMe2>,
    option_field: Option<CountMe2>,
    list_field: LinkedList<CountMe1>,
    cell_field: Cell<CountMe2>,
}

#[derive(Default, Drive, PartialEq, Eq, Hash)]
struct CountMe1;
#[derive(Default, Drive, Clone, Copy)]
struct CountMe2;

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

#[test]
fn test_containers() {
    let mut top = Top::default();
    top.vec_field.push(CountMe2);
    top.vec_field.push(CountMe2);
    top.map_field.insert(CountMe1, CountMe2);
    top.list_field.push_back(CountMe1);
    top.option_field = Some(CountMe2);
    let mut test_visitor = TestVisitor::default();
    top.drive(&mut test_visitor);

    // Count1:
    //   tuple: 3
    //   array: 5
    //   map: 1
    //   list: 1
    //   SUM: 10
    // Count2:
    //   tuple: 3
    //   vec: 2
    //   map: 1
    //   option: 1
    //   cell: 1
    //   SUM: 8
    assert_eq!(
        test_visitor,
        TestVisitor {
            count1: 10,
            count2: 8
        }
    );
}
