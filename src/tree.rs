use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter, Write},
};

use slotmap::{DefaultKey as SlotKey, Key, SlotMap};

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct NodeLabel(u64);

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

#[derive(Debug)]
pub struct Book {
    pub defs: BTreeMap<String, Net>,
    pub root: Net,
}

impl Net {
    fn recurse_mut(&mut self, f: &mut impl FnMut(&mut Tree)) {
        self.root.recurse_mut(f);
        for (a, b) in &mut self.redexes {
            a.recurse_mut(f);
            b.recurse_mut(f);
        }
        for (k, v) in &mut self.vars {
            if let Some(v) = v {
                v.recurse_mut(f)
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
        self.show().show_net(&mut s, self);
        s
    }
}
impl Tree {
    fn recurse_mut(&mut self, f: &mut impl FnMut(&mut Tree)) {
        f(self);
        match self {
            Tree::Binary { label, p1, p2 } => {
                p1.recurse_mut(f);
                p2.recurse_mut(f);
            }
            Tree::Var { id } => (),
        }
    }
    fn map_var_id(&mut self, reassign: impl FnOnce(SlotKey) -> Option<SlotKey>) {
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

struct NetShow<'a, F: Fn(SlotKey) -> Option<&'a Tree>> {
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
    fn show_tree(&mut self, f: &mut impl Write, tree: &'a Tree) {
        match tree {
            Tree::Binary { label, p1, p2 } => {
                f.write_str(match *label {
                    NodeLabel::CON => "(",
                    NodeLabel::EQL => "[",
                    NodeLabel::ANN => "<",
                    _ => "{",
                });
                self.show_tree(f, p1);
                f.write_str(" ");
                self.show_tree(f, p2);
                f.write_str(match *label {
                    NodeLabel::CON => ")",
                    NodeLabel::EQL => "]",
                    NodeLabel::ANN => ">",
                    _ => "}",
                });
            }
            Tree::Var { id } => {
                if let Some(value) = (self.vars)(*id) {
                    self.show_tree(f, value);
                } else {
                    f.write_str(&self.get_or_new(*id));
                }
            }
        }
    }
    fn show_net(&mut self, f: &mut impl Write, net: &'a Net) {
        self.show_tree(f, &net.root);
        for (a, b) in &net.redexes {
            f.write_str(" & ");
            self.show_tree(f, a);
            f.write_str(" = ");
            self.show_tree(f, b);
        }
    }
}

use TSPL::Parser;

pub struct TreeParser<'i> {
    input: &'i str,
    index: usize,
    scope: BTreeMap<String, SlotKey>,
    redexes: Vec<(Tree, Tree)>,
    vars: SlotMap<SlotKey, Option<Tree>>,
    defs: BTreeMap<String, Net>,
}

impl<'i> Parser<'i> for TreeParser<'i> {
    fn input(&mut self) -> &'i str {
        &self.input
    }

    fn index(&mut self) -> &mut usize {
        &mut self.index
    }
}

impl<'i> TreeParser<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input,
            index: 0,
            scope: Default::default(),
            vars: Default::default(),
            defs: Default::default(),
            redexes: vec![],
        }
    }
}

impl<'i> TreeParser<'i> {
    fn get_or_new(&mut self, name: String) -> SlotKey {
        if let Some(e) = self.scope.remove(&name) {
            e
        } else {
            let v = self.vars.insert(None);
            self.scope.insert(name, v);
            v
        }
    }
    pub fn parse_tree(&mut self) -> Result<Tree, String> {
        self.skip_trivia();
        match self.peek_one() {
            Some(delim @ ('(' | '[' | '<' | '{')) => {
                self.consume(&delim.to_string())?;
                self.skip_trivia();
                let label = match delim {
                    '(' => NodeLabel::CON,
                    '[' => NodeLabel::EQL,
                    '<' => NodeLabel::ANN,
                    '{' => NodeLabel(self.parse_u64()?),
                    _ => unreachable!(),
                };
                let p1 = self.parse_tree()?;
                let p2 = self.parse_tree()?;
                self.consume(match delim {
                    '(' => ")",
                    '[' => "]",
                    '<' => ">",
                    '{' => "}",
                    _ => unreachable!(),
                })?;
                Ok(Tree::Binary {
                    label,
                    p1: Box::new(p1),
                    p2: Box::new(p2),
                })
            }
            _ => {
                let name = self.parse_name()?;
                if let Some(net) = self.defs.get(&name) {
                    Ok(self.inject(net.clone()))
                } else {
                    Ok(Tree::Var {
                        id: self.get_or_new(name),
                    })
                }
            }
        }
    }
    pub fn parse_net(&mut self) -> Result<Net, String> {
        let mut net = Net {
            root: self.parse_tree()?,
            redexes: vec![],
            vars: Default::default(),
        };
        self.skip_trivia();
        while self.peek_one() == Some('&') {
            self.consume("&")?;
            let a = self.parse_tree()?;
            self.skip_trivia();
            self.consume("=")?;
            let b = self.parse_tree()?;
            self.redexes.push((a, b));
            self.skip_trivia();
        }
        core::mem::swap(&mut net.vars, &mut self.vars);
        core::mem::swap(&mut net.redexes, &mut self.redexes);
        Ok(net)
    }
    pub fn inject(&mut self, mut net: Net) -> Tree {
        let mut remap = BTreeMap::new();
        for (k, v) in core::mem::take(&mut net.vars) {
            remap.insert(k, self.vars.insert(v));
        }
        net.recurse_mut(&mut |tree: &mut Tree| {
            tree.map_var_id(&mut |key| remap.get(&key).cloned())
        });
        self.redexes.extend(net.redexes);
        net.root
    }
    pub fn parse_book(&mut self) -> Result<Book, String> {
        self.skip_trivia();
        while self.peek_many(4) == Some("def ") {
            self.consume("def ")?;
            self.skip_trivia();
            let name = self.parse_name()?;
            self.skip_trivia();
            self.consume("=")?;
            let value = self.parse_net()?;
            self.skip_trivia();
            self.defs.insert(name, value);
        }
        let root = self.parse_net()?;
        Ok(Book {
            root,
            defs: core::mem::take(&mut self.defs),
        })
    }
}
