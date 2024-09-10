use crate::emulator::signal::SigSet;

pub struct UnixSigSet {
    pub mask: u64
}

impl UnixSigSet {
    pub fn new(mask: u64) -> Self {
        Self {
            mask
        }
    }

    pub fn iter(&self) -> SigSetIterator {
        SigSetIterator::new(self)
    }
}

impl SigSet for UnixSigSet {
    fn get_mask(&self) -> u64 {
        self.mask
    }

    fn set_mask(&mut self, mask: u64) {
        self.mask = mask;
    }

    fn block_sig_set(&mut self, mask: u64) {
        self.mask |= mask;
    }

    fn unblock_sig_set(&mut self, mask: u64) {
        self.mask &= !mask;
    }

    fn contains_sig_number(&self, signum: i32) -> bool {
        let bit = signum - 1;
        return (self.mask & (1u64 << bit)) != 0;
    }

    fn remove_sig_number(&mut self, signum: i32) {
        let bit = signum - 1;
        self.mask &= 1u64 << bit;
    }

    fn add_sig_number(&mut self, signum: i32) {
        let bit = signum - 1;
        self.mask |= 1u64 << bit;
    }
}

struct SigSetIterator {
    mask: u64,
    bit: u64,
    next_bit: u64,
}

impl SigSetIterator {
    fn new(sig_set: &UnixSigSet) -> Self {
        SigSetIterator {
            mask: sig_set.mask,
            bit: 0,
            next_bit: 0,
        }
    }
}

impl Iterator for SigSetIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.bit..64 {
            if (self.mask & (1 << i)) != 0 {
                self.next_bit = i;
                self.bit = self.next_bit;
                self.mask &= !(1u64 << self.bit);
                return Some(self.next_bit + 1);
            }
        }
        None
    }
}
