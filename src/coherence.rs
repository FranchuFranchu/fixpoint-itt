use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Index, Range},
    slice::SliceIndex,
};

use slotmap::DefaultKey;

use crate::tree::{NodeLabel, Tree};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PathItem {
    label: NodeLabel,
    first: bool,
    enter: bool,
}

impl std::ops::Not for PathItem {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self {
            label: self.label,
            first: self.first,
            enter: !self.enter,
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathStack(Vec<PathItem>);

impl PathStack {
    fn push(&mut self, item: PathItem) {
        self.0.push(item)
    }
    fn normal(mut self) -> NormalPathStack {
        let tail = if let Some(i) = self.0.iter().position(|x| !x.enter) {
            self.0.split_off(i)
        } else {
            vec![]
        };
        NormalPathStack(
            self.0.into_iter().map(|x| x.first).collect(),
            tail.into_iter().map(|x| x.first).collect(),
        )
    }
    fn reverse(&mut self) {
        self.0.reverse();
        for i in &mut self.0 {
            i.enter = !i.enter;
        }
    }
    fn extend_by(&mut self, by: usize) {}
    /*fn normal(&mut self) {
        let mut interact_position = self.0.iter().position(|x| !x.enter).unwrap_or(self.0.len());
        while self.0.get(interact_position).is_some() {
            if self.0[interact_position] == !self.0[interact_position - 1] {
                self.0.remove(interact_position - 1);
                self.0.remove(interact_position - 1);
                interact_position -= 1;
            } else {
                break;
            }
        }
    }*/
    fn starts_with(&self, v: &Self) -> bool {
        self.0.starts_with(&v.0)
    }
}
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalPathStack(VecDeque<bool>, VecDeque<bool>);

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PathStackSet(BTreeMap<NodeLabel, PathStack>);
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalPathStackSet(BTreeMap<NodeLabel, NormalPathStack>);

impl PathStackSet {
    fn push(&mut self, item: PathItem) {
        self.0.entry(item.label).or_default().push(item);
    }
    // Split into "input" and "output" parts.
    fn normal(self) -> NormalPathStackSet {
        NormalPathStackSet(self.0.into_iter().map(|(k, v)| (k, v.normal())).collect())
    }
    fn reverse(&mut self) {
        for (k, v) in &mut self.0 {
            v.reverse();
        }
    }
    fn starts_with(&self, other: &Self) -> bool {
        let empty_stack = PathStack::default();
        for k in self.0.keys().chain(other.0.keys()) {
            let v1 = self.0.get(k).unwrap_or(&empty_stack);
            let v2 = other.0.get(k).unwrap_or(&empty_stack);
            if !v1.starts_with(v2) {
                return false;
            }
        }
        return true;
    }
    fn empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl NormalPathStack {
    fn extend_by(self, depth: usize) -> Vec<Self> {
        if depth == 0 {
            vec![self]
        } else {
            let mut c1 = self.extend_by(depth - 1);
            let mut res = vec![];
            for mut i in c1 {
                let mut j = i.clone();
                i.0.push_back(true);
                i.1.push_front(true);
                j.0.push_back(false);
                j.1.push_front(false);
                res.push(i);
                res.push(j);
            }
            res
        }
    }
    fn neg_len(&self) -> usize {
        self.0.len()
    }
}

impl NormalPathStackSet {
    fn neg_len(&self) -> BTreeMap<NodeLabel, usize> {
        self.0
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v.neg_len()))
            .collect()
    }
    fn extend_by(mut self, max_len: &BTreeMap<NodeLabel, usize>) -> Vec<Self> {
        let mut r = BTreeMap::new();
        let it = max_len
            .keys()
            .chain(self.0.keys())
            .map(|x| *x)
            .collect::<Vec<_>>();
        for i in it {
            let max_len = max_len.get(&i).cloned().unwrap_or_default();
            let curr_len = self.0.entry(i).or_default().neg_len();;
            let vals = self
                .0
                .entry(i)
                .or_default()
                .clone()
                .extend_by(max_len - curr_len);
            r.insert(i, vals);
        }
        let folded = r.into_iter().fold(vec![vec![]], |acc: Vec<Vec<(NodeLabel, _)>>, (k, vals)| {
            vals.into_iter().flat_map(|x| {
                let mut a = acc.clone();
                for i in &mut a {
                    i.push((k, x.clone()));
                }
                a.into_iter()
            }).collect()
        });
        folded.into_iter().map(|x| NormalPathStackSet(x.into_iter().collect())).collect()
    }
    fn key(self) -> BTreeMap<NodeLabel, VecDeque<bool>> {
        self.0.into_iter().map(|x| (x.0, x.1.0)).collect()
    }
}

impl std::fmt::Debug for NormalPathStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for v in &self.0 {
            f.write_str(match v {
                false => "r",
                true => "l",
            })?;
        }
        f.write_str(" => ");
        for v in &self.1 {
            f.write_str(match v {
                false => "r",
                true => "l",
            })?;
        }
        Ok(())
    }
}
impl std::fmt::Debug for NormalPathStackSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for (k, stack) in &self.0 {
            map.key(k).value(&stack);
        }
        map.finish()?;
        Ok(())
    }
}

impl Tree {
    pub fn is_coherent(&self) -> bool {
        #[derive(Default)]
        struct State {
            vars: BTreeMap<DefaultKey, PathStackSet>,
        }

        impl State {
            fn traverse(&mut self, tree: &Tree, execution: &PathStackSet) -> Vec<PathStackSet> {
                match tree {
                    Tree::Binary { label, p1, p2 } => {
                        let label = *label;
                        let (mut ls, mut rs) = (execution.clone(), execution.clone());
                        ls.push(PathItem {
                            first: true,
                            enter: true,
                            label,
                        });
                        rs.push(PathItem {
                            first: false,
                            enter: true,
                            label,
                        });
                        let mut ls = self.traverse(p1, &ls);
                        let mut rs = self.traverse(p2, &rs);
                        for ls in &mut ls {
                            ls.push(PathItem {
                                first: true,
                                enter: false,
                                label,
                            });
                        }
                        for rs in &mut rs {
                            rs.push(PathItem {
                                first: false,
                                enter: false,
                                label,
                            });
                        }
                        ls.append(&mut rs);
                        ls
                    }
                    Tree::Var { id } => {
                        if let Some(e) = self.vars.remove(id) {
                            vec![e]
                        } else {
                            self.vars.insert(id.clone(), execution.clone());
                            vec![]
                        }
                    }
                }
            }
        }

        let mut state = State::default();
        let stack = state.traverse(self, &Default::default());
        let stack: Vec<_> = stack.into_iter().map(|x| x.normal()).collect();
        let max_len = stack.iter().map(|x| x.neg_len()).fold(
            BTreeMap::new(),
            |old: BTreeMap<NodeLabel, usize>, new| {
                let mut res = BTreeMap::new();
                for i in old.keys().chain(new.keys()).map(|x| *x).collect::<Vec<_>>() {
                    let n1 = old.get(&i).cloned().unwrap_or_default();
                    let n2 = new.get(&i).cloned().unwrap_or_default();
                    let n = n1.max(n2);
                    res.insert(i, n);
                }
                res
            },
        );
        let stacks: Vec<_> = stack
            .into_iter()
            .map(|mut x| {x.0.remove(&NodeLabel::EQL); x})
            .map(|x| x.extend_by(&max_len).into_iter())
            .flatten()
            .map(|x| (x.clone().key(), x))
            .collect();

        let mut map = BTreeMap::new();
        for (k, v) in stacks {
            if map.insert(k, v.clone()).is_some_and(|x| v != x) {
                return false;
            }
        }
        return true;
    }
}
