#[derive(Clone, Debug)]
pub struct BitSet {
    words: Vec<u64>, // represents the bits in chunks of 64 bits
    bit_count: usize, // total number of bits represented by the BitSet
}

impl BitSet {
    pub fn full(bit_count: usize) -> Self {
        let word_count = bit_count.div_ceil(64);
        let mut words = vec![u64::MAX; word_count];

        if let Some(last_word) = words.last_mut() {
            let remaining_bits = bit_count % 64;

            if remaining_bits != 0 {
                *last_word = (1u64 << remaining_bits) - 1;
            }
        }

        Self {
            words,
            bit_count,
        }
    }

    pub fn empty(bit_count: usize) -> Self {
        let word_count = bit_count.div_ceil(64);

        Self {
            words: vec![0; word_count],
            bit_count,
        }
    }

    pub fn contains(&self, bit: usize) -> bool {
        assert!(bit < self.bit_count);

        let word_index = bit / 64;
        let bit_index = bit % 64;

        (self.words[word_index] & (1u64 << bit_index)) != 0
    }

    pub fn remove(&mut self, bit: usize) -> bool {
        assert!(bit < self.bit_count);

        let word_index = bit / 64;
        let bit_index = bit % 64;
        let mask = 1u64 << bit_index;

        let was_set = (self.words[word_index] & mask) != 0;

        self.words[word_index] &= !mask;

        was_set
    }

    pub fn keep_only(&mut self, bit: usize) -> bool {
        assert!(bit < self.bit_count);

        let word_index = bit / 64;
        let bit_index = bit % 64;
        let mask = 1u64 << bit_index;

        let was_set = (self.words[word_index] & mask) != 0;

        for word in &mut self.words {
            *word = 0;
        }

        if was_set {
            self.words[word_index] = mask;
        }

        was_set
    }

    pub fn intersect_with(&mut self, other: &[u64]) -> bool {
        assert_eq!(self.words.len(), other.len());

        let mut changed = false;

        for (word, other_word) in self.words.iter_mut().zip(other.iter()) {
            let new_word = *word & *other_word;

            if new_word != *word {
                *word = new_word;
                changed = true;
            }
        }

        changed
    }

    pub fn words(&self) -> &[u64] {
        &self.words
    }

    pub fn words_mut(&mut self) -> &mut [u64] {
        &mut self.words
    }

    pub fn count_ones(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(word_index, &word)| {
                let mut remaining = word;

                std::iter::from_fn(move || {
                    if remaining == 0 {
                        return None;
                    }

                    let bit_index = remaining.trailing_zeros() as usize;
                    remaining &= remaining - 1;

                    Some(word_index * 64 + bit_index)
                })
            })
    }
}