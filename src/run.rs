use slotmap::DefaultKey as SlotKey;

use crate::tree::{Net, Tree};

impl Net {
    fn wire(&mut self) -> (SlotKey, SlotKey) {
        let key = self.vars.insert(None);
        (key, key)
    }
    fn wire_tree(&mut self) -> (Tree, Tree) {
        let (a, b) = self.wire();
        (Tree::Var { id: a }, Tree::Var { id: b })
    }
    fn link(&mut self, a: Tree, b: Tree) {
        self.redexes.push((a, b));
    }
    fn interact(&mut self, a: Tree, b: Tree) {
        use Tree::*;
        match (a, b) {
            (Var { id }, b) => {
                let entry = self.vars.get_mut(id).unwrap();
                if entry.is_some() {
                    let a = self.vars.remove(id).unwrap().unwrap();
                    self.interact(a, b)
                } else {
                    *entry = Some(b);
                }
            }
            (a, Var { id }) => {
                let entry = self.vars.get_mut(id).unwrap();
                if entry.is_some() {
                    let b = self.vars.remove(id).unwrap().unwrap();
                    self.interact(a, b)
                } else {
                    *entry = Some(a);
                }
            }
            (
                Binary {
                    label: a0,
                    p1: box a1,
                    p2: box a2,
                },
                Binary {
                    label: b0,
                    p1: box b1,
                    p2: box b2,
                },
            ) => {
                if a0 == b0 {
                    self.link(a1, b1);
                    self.link(a2, b2);
                } else {
                    let (a11, b11) = self.wire_tree();
                    let (a12, b12) = self.wire_tree();
                    let (a21, b21) = self.wire_tree();
                    let (a22, b22) = self.wire_tree();

                    self.link(
                        a1,
                        Binary {
                            label: b0.clone(),
                            p1: Box::new(a11),
                            p2: Box::new(a12),
                        },
                    );
                    self.link(
                        a2,
                        Binary {
                            label: b0.clone(),
                            p1: Box::new(a21),
                            p2: Box::new(a22),
                        },
                    );
                    self.link(
                        b1,
                        Binary {
                            label: a0.clone(),
                            p1: Box::new(b11),
                            p2: Box::new(b21),
                        },
                    );
                    self.link(
                        b2,
                        Binary {
                            label: a0.clone(),
                            p1: Box::new(b12),
                            p2: Box::new(b22),
                        },
                    );
                }
            }
        }
    }
    pub fn normal(&mut self, hook: impl Fn(&mut Self)) {
        hook(self);
        while let Some((a, b)) = self.redexes.pop() {
            self.interact(a, b);
            hook(self);
        }
    }
}
