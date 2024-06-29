//! Generates trees in this order:
// ((a a) (b b))
// ((a b) (a b))

// 00 -> 00 -> 00 a
// 01 -> 01 -> 01 a
// 10 -> 10 -> 11 b
// 11 -> 11 -> 10 b
//
// 00 -> 00 -> 00 a 
// 01 -> 10 -> 11 b
// 10 -> 01 -> 01 a
// 11 -> 11 -> 10 b

// Odd amount of 1s: +
// Even amount of 1s: -

use slotmap::{DefaultKey, SlotMap};

use crate::tree::{Net, NodeLabel, Tree};

pub struct Enumerator {
	depth: u64,
	index: u64,
}

impl Enumerator {
	fn factorial(num: u64) -> u64 {
		if num == 0 {
			1
		} else {
			Self::factorial(num - 1) * num
		}
	}
	fn bits_amount(&self) -> u64 {
		self.depth
	}
	fn wire_amount(&self) -> u64 {
		2u64.pow(self.depth as u32 - 1)
	}
	fn index_amount(&self) -> u64 {
		Self::factorial(self.wire_amount())
	}
	fn permute(&self, mut bits: u64) -> u64 {
		let mut index = self.index;
		let mut result = 0;
		for i in (0..self.bits_amount()).rev() {
			let fact = Self::factorial(i);
			let q = index.div_floor(fact);
			index = index.rem_euclid(fact);
			result |= (bits >> q) & 1;
			let left_bits = bits >> (q + 1);
			let right_bits = bits & ((1 << q) - 1);
			let new_bits = (left_bits << q) | right_bits;
			//println!("{q} {bits:0>2b} {left_bits:0>2b} {right_bits:0>2b} {new_bits:0>2b}", );
			bits = new_bits;
			result <<= 1;
		}
		result >> 1
	}
	fn generate(&mut self, vars: &[DefaultKey], depth: u64, path: u64) -> Tree {
		if depth == 0 {
			let var_id = self.permute(path);
			let var_id = if var_id.count_ones() & 1 != 0 {
				((var_id >> 1) << 1) | 1
			} else {
				((var_id >> 1) << 1) | 0
			} >> 1;
			Tree::Var {
				id: vars[var_id as usize].clone(),
			}
		} else {
			Tree::Binary {
				label: NodeLabel::CON,
				p1: Box::new(self.generate(vars, depth - 1, path << 1)),
				p2: Box::new(self.generate(vars, depth - 1, path << 1 | 1)),
			}
		}
	}
	fn gen_all(&mut self) {
		for i in 0..self.index_amount() {
			println!("{}", self.next().display());
		}
	}
	fn next(&mut self) -> Net {
		let mut vars = SlotMap::new();
		let mut var_idxs = vec![];
		for i in 0..self.wire_amount() {
			var_idxs.push(vars.insert(None));
		}
		let root = self.generate(&var_idxs, self.depth, 0);
		self.index += 1;
		Net {
		    root: root,
		    redexes: vec![],
		    vars,
		}
	}
}

#[test]
fn test() {
	let mut a = Enumerator {
	    depth: 3,
	    index: 0,
	};
	a.gen_all();
}