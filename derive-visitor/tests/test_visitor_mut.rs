use derive_visitor::{VisitorMut, DriveMut};

#[derive(DriveMut)]
struct Chain {
    next: Option<Box<Chain>>,
}

impl Chain {
    fn depth(&self) -> usize {
        if let Some(child) = &self.next { 1 + child.depth() } else { 0 }
    }
}

#[derive(VisitorMut)]
#[visitor(Chain(enter))]
struct ChainCutter {
    cut_at_depth: usize,
}

impl ChainCutter {
    fn enter_chain(&mut self, item: &mut Chain) {
        if self.cut_at_depth == 0 {
            item.next = None;
        } else {
            self.cut_at_depth -= 1;
        }
    }
}

#[test]
fn test() {
    let mut chain = Chain { next: Some(Box::new(Chain { next: Some(Box::new(Chain { next: None })) })) };
    assert_eq!(chain.depth(), 2);
    let mut cutter = ChainCutter { cut_at_depth: 1 };
    chain.drive_mut(&mut cutter);
    assert_eq!(chain.depth(), 1);
}