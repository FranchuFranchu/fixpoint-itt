use slotmap::{DefaultKey, SlotMap};

use crate::tree::{Net, NodeLabel, Tree};

#[derive(Debug)]
pub enum Term {
    Binder {
        label: NodeLabel,
        pat: Box<Term>,
        body: Box<Term>,
    },
    Apply {
        label: NodeLabel,
        fun: Box<Term>,
        arg: Box<Term>,
    },
    Sup {
        label: NodeLabel,
        fst: Box<Term>,
        snd: Box<Term>,
    },
    Let {
        pat: Box<Term>,
        value: Box<Term>,
        next: Box<Term>,
    },
    Var {
        id: DefaultKey,
    },
}
    
impl Term {
    pub fn encode(&self, vars: &mut SlotMap<DefaultKey, Option<Tree>>, redex: &mut Vec<(Tree, Tree)>) -> Tree {
        match self {
            Term::Binder { label, pat, body } => {
                let pat = pat.encode(vars, redex);
                let body = body.encode(vars, redex);
                Tree::Binary { label: *label, p1: Box::new(pat), p2: Box::new(body)}
            },
            Term::Apply { label, fun, arg } => {
                let fun = fun.encode(vars, redex);
                let arg = arg.encode(vars, redex);
                let id = vars.insert(None);
                redex.push((
                    (Tree::Binary { label: *label, p1: Box::new(arg), p2: Box::new(Tree::Var { id: id })}),
                    fun
                ));
                Tree::Var { id: id }
            },
            Term::Sup { label, fst, snd } => {
                let fst = fst.encode(vars, redex);
                let snd = snd.encode(vars, redex);
                Tree::Binary {
                    label: *label,
                    p1: Box::new(fst),
                    p2: Box::new(snd),
                }
            },
            Term::Let { pat, value, next } => {
                let pat = pat.encode(vars, redex);
                let value = value.encode(vars, redex);
                let next = next.encode(vars, redex);
                redex.push((pat, value));
                next
            },
            Term::Var { id } => Tree::Var { id: *id },
        }
    }
} 