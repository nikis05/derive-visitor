#![cfg(feature = "std-types-drive")]

use std::ops::Range;

use derive_visitor::{Drive, Visitor};

#[derive(Default, Drive)]
struct Top {
    s: String,
    inner: Inner,
    s2: String,
    vec_field: Vec<u32>,
}

#[derive(Default, Drive)]
struct Inner {
    start: u32,
    end: u32,
    rng: Range<u32>,
    inner_s: String,
}

#[derive(Debug, Default, PartialEq, Eq, Visitor)]
#[visitor(String, u32)]
struct TestVisitor {
    all_strings: Vec<String>,
    sum_u32: u32,
    enter_leave_check: u32,
}

impl TestVisitor {
    fn enter_string(&mut self, s: &str) {
        self.all_strings.push(s.to_owned());
    }
    fn exit_string(&mut self, s: &str) {
        assert_eq!(self.all_strings.last().unwrap(), s);
    }
    fn enter_u_32(&mut self, n: &u32) {
        self.sum_u32 += n;
        self.enter_leave_check += n;
    }
    fn exit_u_32(&mut self, n: &u32) {
        self.enter_leave_check -= n;
        assert_eq!(self.enter_leave_check, 0);
    }
}

#[test]
fn test_std_types() {
    let top = Top {
        s: "String1".into(),
        inner: Inner {
            start: 12,
            end: 24,
            rng: 4..6,
            inner_s: "x".into(),
        },
        s2: "zzz".into(),
        vec_field: vec![1, 2, 3],
    };
    let mut test_visitor = TestVisitor::default();
    top.drive(&mut test_visitor);
    assert_eq!(
        test_visitor,
        TestVisitor {
            all_strings: vec!["String1".into(), "x".into(), "zzz".into()],
            sum_u32: 52,
            enter_leave_check: 0,
        }
    );
}
