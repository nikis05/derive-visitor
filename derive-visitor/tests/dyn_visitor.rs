use derive_visitor::{Drive, Visitor};

#[derive(Drive)]
struct Example {
    heads: Heads,
    tails: Tails,
}

#[derive(Drive)]
struct Heads;

#[derive(Drive)]
struct Tails;

trait CounterVisitor: Visitor {
    fn count(&self) -> usize;
}

#[derive(Debug, Default, PartialEq, Eq, Visitor)]
#[visitor(Heads(enter))]
struct HeadsVisitor {
    count: usize,
}

impl CounterVisitor for HeadsVisitor {
    fn count(&self) -> usize {
        self.count
    }
}

impl HeadsVisitor {
    fn enter_heads(&mut self, _heads: &Heads) {
        self.count += 1;
    }
}

#[derive(Debug, Default, PartialEq, Eq, Visitor)]
#[visitor(Tails(enter))]
struct TailsVisitor {
    count: usize,
}

impl TailsVisitor {
    fn enter_tails(&mut self, _tails: &Tails) {
        self.count += 1;
    }
}

impl CounterVisitor for TailsVisitor {
    fn count(&self) -> usize {
        self.count
    }
}

#[test]
fn dyn_visitor() {
    let example = Example {
        heads: Heads,
        tails: Tails,
    };

    let mut visitors = vec![
        Box::new(HeadsVisitor { count: 0 }) as Box<dyn CounterVisitor>,
        Box::new(TailsVisitor { count: 0 }) as Box<dyn CounterVisitor>,
    ];

    for visitor in visitors.iter_mut() {
        example.drive(visitor.as_mut());
    }

    assert_eq!(visitors[0].count(), 1);
    assert_eq!(visitors[1].count(), 1);
}
