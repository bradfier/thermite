// Simple Linear Congruential Generator

extern crate num;

use num::{PrimInt,One};
pub static LCG_A: u64 = 22695477;
pub static LCG_C: u64 = 1;

pub struct LCG {
    curr: u64,
    m: u64,
    a: u64,
    c: u64,
}

impl Iterator for LCG {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let this_iter = self.curr;
        
        let next = mod_power2((self.a * self.curr + self.c), self.m);
        self.curr = next;

        Some(this_iter)
    }
}

impl LCG {
    pub fn new(seed: u64, modulus: u64) -> LCG {
        LCG {
            curr: seed,
            m: modulus,
            a: LCG_A,
            c: LCG_C,
        }
    }
}

fn mod_power2<T: PrimInt+One>(n: T, m: T) -> T {
    if m.count_ones() == 1 {
        n & (m - T::one())
    } else {
        n % m
    }
}