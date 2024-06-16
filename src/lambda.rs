use slotmap::DefaultKey;

use crate::tree::NodeLabel;

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
