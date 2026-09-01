use rand::{ Rng, RngExt };

pub struct EntropyBuckets {
    buckets: Vec<Vec<usize>>,
    minimum: usize,
}

impl EntropyBuckets {
    pub fn new(pattern_count: usize, cell_count: usize) -> Self {
        let mut buckets = vec![Vec::new(); pattern_count + 1];

        buckets[pattern_count].reserve(cell_count);

        for cell_index in 0..cell_count {
            buckets[pattern_count].push(cell_index);
        }

        Self {
            buckets,
            minimum: pattern_count,
        }
    }

    pub fn push(&mut self, cell_index: usize, possible_count: usize) {
        if possible_count <= 1 {
            return;
        }

        self.buckets[possible_count].push(cell_index);

        if possible_count < self.minimum {
            self.minimum = possible_count;
        }
    }

    pub fn pop_lowest<R, F>(&mut self, rng: &mut R, mut current_count: F) -> Option<usize>
        where R: Rng + ?Sized, F: FnMut(usize) -> usize
    {
        while self.minimum < self.buckets.len() {
            let bucket = &mut self.buckets[self.minimum];

            while !bucket.is_empty() {
                let random_index = rng.random_range(0..bucket.len());
                let cell_index = bucket.swap_remove(random_index);

                if current_count(cell_index) == self.minimum {
                    return Some(cell_index);
                }
            }
            self.minimum += 1;
        }

        None
    }
}
