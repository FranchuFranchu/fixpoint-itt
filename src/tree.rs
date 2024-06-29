use std::{collections::BTreeMap, fmt::Write};

use slotmap::{DefaultKey as SlotKey, SlotMap};

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct NodeLabel(pub u64);

impl NodeLabel {
    pub const CON: Self = Self(0);
    pub const DUP: Self = Self(1);
    pub const ANN: Self = Self(2);
    pub const EQL: Self = Self(3);
}

#[derive(Debug, Clone)]
pub enum Tree {
    Binary {
        label: NodeLabel,
        p1: Box<Tree>,
        p2: Box<Tree>,
    },
    Var {
        id: SlotKey,
    },
}

#[derive(Clone, Debug)]
pub struct Net {
    pub root: Tree,
    pub redexes: Vec<(Tree, Tree)>,
    pub vars: SlotMap<SlotKey, Option<Tree>>,
}

impl Net {
    pub fn recurse_mut(&mut self, f: &mut impl FnMut(&mut Tree)) {
        self.root.recurse_mut(f);
        for (a, b) in &mut self.redexes {
            a.recurse_mut(f);
            b.recurse_mut(f);
        }
        for v in self.vars.values_mut() {
            if let Some(v) = v {
                v.recurse_mut(f)
            }
        }
    }
    pub fn recurse_ref(&self, f: &mut impl FnMut(&Tree)) {
        self.root.recurse_ref(f);
        for (a, b) in &self.redexes {
            a.recurse_ref(f);
            b.recurse_ref(f);
        }
        for v in self.vars.values() {
            if let Some(v) = v {
                v.recurse_ref(f)
            }
        }
    }
    pub fn show<'a>(&'a self) -> NetShow<'a, impl Fn(SlotKey) -> Option<&'a Tree>> {
        NetShow {
            vars: |key| self.vars.get(key).map(|x| x.as_ref()).flatten(),
            scope: Default::default(),
        }
    }
    pub fn display(&self) -> String {
        let mut s = String::new();
        self.show().show_net(&mut s, self).unwrap();
        s
    }
    pub fn validate(&self) {
        for (k, v) in &self.vars {
            if let Some(v) = v {
                v.recurse_ref(&mut |s| match s {
                    Tree::Binary { .. } => (),
                    Tree::Var { id } => assert!(k != *id),
                })
            }
        }
        let mut counts: BTreeMap<SlotKey, u64> = BTreeMap::new();
        self.recurse_ref(&mut |x| match x {
            Tree::Binary { .. } => (),
            Tree::Var { id } => *counts.entry(*id).or_default() += 1,
        });
        for (k, v) in counts {
            let expect = match self.vars.get(k) {
                Some(Some(_)) => 1,
                Some(None) => 2,
                None => 0,
            };
            assert!(v == expect, "var: {k:?} found: {v} != expected: {expect}");
        }
    }
    pub fn resolve_vars(&mut self) {
        self.root.resolve_vars(&mut self.vars);
        for (a, b) in &mut self.redexes {
            a.resolve_vars(&mut self.vars);
            b.resolve_vars(&mut self.vars);
        }
    }
    pub fn is_coherent(&mut self) -> bool {
        self.resolve_vars();
        self.root.is_coherent()
    }
}
impl Tree {
    pub fn recurse_ref(&self, f: &mut impl FnMut(&Tree)) {
        f(self);
        match self {
            Tree::Binary { label: _, p1, p2 } => {
                p1.recurse_ref(f);
                p2.recurse_ref(f);
            }
            Tree::Var { .. } => (),
        }
    }
    pub fn recurse_mut(&mut self, f: &mut impl FnMut(&mut Tree)) {
        f(self);
        match self {
            Tree::Binary { label: _, p1, p2 } => {
                p1.recurse_mut(f);
                p2.recurse_mut(f);
            }
            Tree::Var { .. } => (),
        }
    }
    pub fn map_var_id(&mut self, reassign: impl FnOnce(SlotKey) -> Option<SlotKey>) {
        if let Self::Var { id } = self {
            *id = reassign(*id).unwrap();
        }
    }
    pub fn is_var(&self) -> bool {
        match self {
            Tree::Binary { .. } => false,
            Tree::Var { .. } => true,
        }
    }
    pub fn resolve_vars(&mut self, vars: &mut SlotMap<SlotKey, Option<Tree>>) {
        self.recurse_mut(&mut |term| {
            while term.is_var() {
                let Tree::Var { id } = &term else {
                    unreachable!()
                };
                let id = id.clone();
                if vars.get(id).is_some_and(|x| x.is_some()) {
                    *term = vars.remove(id).unwrap().unwrap()
                } else {
                    break;
                }
            }
        })
    }
}

pub struct NetShow<'a, F: Fn(SlotKey) -> Option<&'a Tree>> {
    vars: F,
    scope: BTreeMap<SlotKey, String>,
}
impl<'a, F: Fn(SlotKey) -> Option<&'a Tree>> NetShow<'a, F> {
    fn get_or_new(&mut self, name: SlotKey) -> String {
        if let Some(e) = self.scope.get(&name) {
            e.clone()
        } else {
            let v = format!("x{:?}", self.scope.len());
            self.scope.insert(name, v.clone());
            v
        }
    }
    fn show_tree(&mut self, f: &mut impl Write, tree: &'a Tree) -> std::fmt::Result {
        match tree {
            Tree::Binary { label, p1, p2 } => {
                {
                    let label = *label;
                    let label_num = format!("{{{} ", label.0);
                    f.write_str(match label {
                        NodeLabel::CON => "(",
                        NodeLabel::EQL => "[",
                        NodeLabel::ANN => "<",
                        _ => &label_num,
                    })?;
                };
                self.show_tree(f, p1)?;
                f.write_str(" ")?;
                self.show_tree(f, p2)?;
                f.write_str(match *label {
                    NodeLabel::CON => ")",
                    NodeLabel::EQL => "]",
                    NodeLabel::ANN => ">",
                    _ => "}",
                })?;
            }
            Tree::Var { id } => {
                if let Some(value) = (self.vars)(*id) {
                    self.show_tree(f, value)?;
                } else {
                    f.write_str(&self.get_or_new(*id))?;
                }
            }
        }
        Ok(())
    }
    fn show_net(&mut self, f: &mut impl Write, net: &'a Net) -> std::fmt::Result {
        self.show_tree(f, &net.root)?;
        for (a, b) in &net.redexes {
            f.write_str(" & ")?;
            self.show_tree(f, a)?;
            f.write_str(" = ")?;
            self.show_tree(f, b)?;
        }
        Ok(())
    }
}
