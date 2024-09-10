use anyhow::anyhow;
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::symbol::ElfSymbol;
use crate::elf::symbol_structure::ElfSymbolStructure;

#[derive(Clone)]
pub enum HashTable {
    Gnu(ElfGnuHashTable),
    SysV(ElfHashTable),
}

#[derive(Clone)]
pub struct ElfHashTable {
    pub buckets: Vec<i32>,
    pub chains: Vec<i32>,
}

impl ElfHashTable {
    pub fn new(parser: ElfParser, offset: usize, length: usize) -> Self {
        parser.seek(offset);

        let num_buckets = parser.read_int();
        let num_chains = parser.read_int();

        let mut buckets = vec![0i32; num_buckets as usize];
        let mut chains = vec![0i32; num_chains as usize];

        for i in 0..num_buckets {
            buckets[i as usize] = parser.read_int();
        }

        for i in 0..num_chains {
            chains[i as usize] = parser.read_int();
        }

        let actual_length = (num_buckets + num_chains) * 4 + 8;
        if length != 0 && length != actual_length as usize {
            panic!("Error reading string table (read {} bytes, expected to read {} bytes).", actual_length, length);
        }

        Self {
            buckets,
            chains,
        }
    }

    pub fn get_symbol(&self, symbol_structure: &ElfSymbolStructure, symbol_name: &str, elf_file: &ElfFile) -> anyhow::Result<ElfSymbol> {
        let hash = sysv_hash(symbol_name) as usize;
        let mut index = self.buckets[hash % self.buckets.len()];
        loop {
            if index == 0 {
                return Err(anyhow!("Symbol not found"));
            }
            let symbol = symbol_structure.get_elf_symbol(index)?;
            if symbol.name(elf_file)? == symbol_name {
                return Ok(symbol);
            }
            index = self.chains[index as usize];
        }
    }

    pub fn get_symbol_by_addr(&self, symbol_structure: &ElfSymbolStructure, so_addr: u64) -> anyhow::Result<ElfSymbol> {
        for i in 0..self.buckets.len() {
            let index = self.buckets[i];
            if index == 0 {
                continue;
            }
            let symbol = symbol_structure.get_elf_symbol(index)?;
            if symbol.matches(so_addr) {
                return Ok(symbol);
            }
        }
        Err(anyhow!("Symbol not found"))
    }
}

#[derive(Clone)]
pub struct ElfGnuHashTable {
    nbucket: i32,
    shift2: i32,
    bloom_filter: Vec<i64>,
    buckets: Vec<i32>,
    chain_base: usize,
    parser: ElfParser,
    maskwords: i32,
    bloom_mask_bits: i32,
}

impl ElfGnuHashTable {
    pub fn new(parser: ElfParser, offset: usize) -> Self {
        parser.seek(offset);

        let nbucket = parser.read_int();
        let symndx = parser.read_int();
        let maskwords = parser.read_int();
        let shift2 = parser.read_int();

        let mut bloom_filter = vec![0i64; maskwords as usize];
        for i in 0..maskwords {
            bloom_filter[i as usize] = parser.read_int_or_long();
        }

        let mut buckets = vec![0i32; nbucket as usize];
        for i in 0..nbucket {
            buckets[i as usize] = parser.read_int();
        }

        //let mut chain_base = offset + 16 + maskwords as usize * if parser.object_size == 1 { 4 } else { 8 } + nbucket as usize * 4 - symndx as usize * 4;
        let mut chain_base = offset + 16 + maskwords as usize * 8  + nbucket as usize * 4 - symndx as usize * 4;

        ElfGnuHashTable {
            nbucket,
            shift2,
            bloom_filter,
            buckets,
            chain_base,
            //bloom_mask_bits: if parser.object_size == 1 { 32 } else { 64 },
            bloom_mask_bits: 64,
            maskwords: maskwords - 1,
            parser,
        }
    }

    pub fn chain(&self, index: i32) -> i32 {
        self.parser.seek(self.chain_base + index as usize * 4);
        self.parser.read_int()
    }

    pub fn get_symbol(&self, symbol_structure: &ElfSymbolStructure, symbol_name: &str, elf_file: &ElfFile) -> anyhow::Result<ElfSymbol> {
        let hash = gnu_hash(symbol_name) as usize;
        let h2 = hash >> self.shift2;

        let word_num = (hash / self.bloom_mask_bits as usize) & self.maskwords as usize;
        let bloom_word = self.bloom_filter[word_num];

        if (1 &
            (bloom_word >> (hash % self.bloom_mask_bits as usize)) &
            (bloom_word >> (h2 % self.bloom_mask_bits as usize))
        ) == 0 {
            return Err(anyhow!("Symbol not found"));
        }

        let mut n = self.buckets[hash % self.nbucket as usize];
        if n == 0 {
            return Err(anyhow!("Symbol not found"));
        }

        loop {
            let symbol = symbol_structure.get_elf_symbol(n)?;
            if symbol.name(elf_file)? == symbol_name {
                return Ok(symbol);
            }
            if self.chain(n) & 1 != 0 {
                return Err(anyhow!("Symbol not found"));
            }
            n += 1;
        }
    }

    pub fn get_symbol_by_addr(&self, symbol_structure: &ElfSymbolStructure, so_addr: u64) -> anyhow::Result<ElfSymbol> {
        for i in 0..self.nbucket as usize {
            let mut n = self.buckets[i];
            if n == 0 {
                continue;
            }
            loop {
                let symbol = symbol_structure.get_elf_symbol(n)?;
                if symbol.matches(so_addr) {
                    return Ok(symbol);
                }
                if self.chain(n) & 1 != 0 {
                    break;
                }
                n += 1;
            }
        }

        Err(anyhow!("Symbol not found"))
    }
}

fn gnu_hash(s: &str) -> u32 {
    let mut h: u32 = 5381;
    for c in s.chars() {
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

fn sysv_hash(s: &str) -> u32 {
    let mut h: u32 = 0;
    for c in s.chars() {
        h = (h << 4).wrapping_add(c as u32);
        let g = h & 0xf0000000;
        if g != 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    h
}