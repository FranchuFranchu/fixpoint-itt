
use std::collections::BTreeMap;

use slotmap::{DefaultKey, Key, SlotMap};
use TSPL::Parser;

use crate::{lambda::Term, tree::{Net, NodeLabel, Tree}};

fn closing(delim: char) -> Option<char> {
    match delim {
        '(' => Some(')'),
        '[' => Some(']'),
        '<' => Some('>'),
        '{' => Some('}'),
        _ => None
    }
}

pub struct TreeParser<'i> {
    input: &'i str,
    index: usize,
    scope: BTreeMap<String, DefaultKey>,
    back_scope: BTreeMap<DefaultKey, String>,
    redexes: Vec<(Tree, Tree)>,
    vars: SlotMap<DefaultKey, Option<Tree>>,
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
            back_scope: BTreeMap::new(),
            redexes: vec![],
        }
    }
}

impl<'i> TreeParser<'i> {
    fn get_or_new(&mut self, name: String) -> DefaultKey {
        if let Some(e) = self.scope.remove(&name) {
            e
        } else {
            let v = self.vars.insert(None);
            self.back_scope.insert(v, format!("Var({name})"));
            self.scope.insert(name, v);
            v
        }
    }
    pub fn inject(&mut self, mut net: Net) -> Tree {
        let mut remap = BTreeMap::new();
        for (k, v) in core::mem::take(&mut net.vars) {
            let id = self.vars.insert(v);
            remap.insert(k, id);
            assert!(self.back_scope.insert(id, format!("Remapped")).is_none());
        }
        let mut remap_fun = |key: DefaultKey| {
            remap.get(&key).cloned()
        };
        for (k, remap_to) in &remap {
            if let Some(Some(v)) = self.vars.get_mut(*remap_to) {
                v.recurse_mut(&mut |tree: &mut Tree| {
                    tree.map_var_id(&mut remap_fun);
                });
            }
        };
        net.recurse_mut(&mut |tree: &mut Tree| {
            tree.map_var_id(&mut remap_fun);
        });
        self.redexes.extend(net.redexes);
        net.root
    }
    pub fn to_var(&mut self, tree: Tree) -> DefaultKey {
    	let id = self.vars.insert(Some(tree));
        assert!(self.back_scope.insert(id, format!("Created from tree")).is_none());
        id
    }
    pub fn parse_term(&mut self) -> Result<Term, String> {
        self.skip_trivia();
        let label = match self.peek_one() {
        	Some('#') => {
        		self.consume("#")?;
        		Some(self.parse_u64()?)
        	},
        	_ => None
        };
        self.skip_trivia();
        match self.peek_one() {
        	Some(delim @ ('λ' | '@' | 'θ')) => {
        		self.consume(&delim.to_string())?;
        		let pat = self.parse_term()?;
        		let bod = self.parse_term()?;

                let label = match delim {
                    'λ' | '@' => NodeLabel(label.unwrap_or(0) * 2),
                    'θ' => NodeLabel::ANN,
                    _ => todo!(),
                };

                Ok(Term::Binder { label, pat: Box::new(pat), body: Box::new(bod) })
        	},
        	Some(delim @ ('{' | '<' | '(')) => {
        		self.consume(&delim.to_string())?;
                let mut fun = self.parse_term()?;
                self.skip_trivia();
                while closing(delim) != self.peek_one() && self.peek_one().is_some() {
                    // <a : b : c>
                    // <<a: b> : c>
                    let label = if delim == '<' {
                        self.skip_trivia();
                        if self.peek_many(2) == Some("==") {
                            self.consume("==")?;
                            NodeLabel::EQL
                        } else if self.peek_many(1) == Some(":") {
                            self.consume(":")?;
                            NodeLabel::ANN
                        } else {
                            todo!()
                        }
                    } else if delim == '(' {
                        NodeLabel(label.unwrap_or(0) * 2)
                    } else if delim == '{' {
                        NodeLabel(label.unwrap_or(0) * 2 + 1)
                    } else {
                        todo!()
                    };
                    let arg = self.parse_term()?;
                    fun = match label {
                        NodeLabel::ANN => {
                            Term::Apply { label, fun: Box::new(arg), arg: Box::new(fun) }
                        }
                        NodeLabel::CON => {
                            Term::Apply { label, fun: Box::new(fun), arg: Box::new(arg) }
                        }
                        _ => {
                            Term::Sup { label, fst: Box::new(fun), snd: Box::new(arg) }
                        }
                    };
                }
                self.consume(&closing(delim).unwrap().to_string())?;
        		Ok(fun)
        	},
            _ => {
                let name = self.parse_name()?;
                let var_id = if name == "tree" {
                    let tree = self.parse_tree()?;
                    self.to_var(tree)
                } else if let Some(net) = self.defs.get(&name) {
                    let tree = self.inject(net.clone());
                	self.to_var(tree)
                } else {
                    self.get_or_new(name)
                };
                Ok(Term::Var {
                	id: var_id,
                })
            }
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
                if name == "term" {
                    let term = self.parse_term()?;
                    Ok(term.encode(&mut self.vars, &mut self.redexes))
                } else if let Some(net) = self.defs.get(&name) {
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
        net.validate();
        self.back_scope.clear();
        Ok(net)
    }
    pub fn parse_book(&mut self) -> Result<Book, String> {
        self.skip_trivia();
        let mut tests = vec![];
        while self.peek_many(4) == Some("def ") {
            self.consume("def ")?;
            self.skip_trivia();
            let name = self.parse_name()?;
            let name = if name == "test" {
                self.skip_trivia();
                let name = self.parse_name()?;
                tests.push(name.clone());
                name
            } else {
                name
            };
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
            tests
        })
    }
}

#[derive(Debug)]
pub struct Book {
    pub defs: BTreeMap<String, Net>,
    pub tests: Vec<String>,
    pub root: Net,
}
