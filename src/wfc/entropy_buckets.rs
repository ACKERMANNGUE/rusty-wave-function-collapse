use rand::{ Rng, RngExt };

const NO_BUCKET: usize = usize::MAX;

pub struct EntropyBuckets {
    buckets: Vec<Vec<usize>>,
    cell_bucket: Vec<usize>,
    cell_position: Vec<usize>,
    minimum: usize,
}

impl EntropyBuckets {
    pub fn new(pattern_count: usize, cell_count: usize) -> Self {
        let mut buckets = vec![Vec::new(); pattern_count + 1];
        let mut cell_bucket = vec![NO_BUCKET; cell_count];
        let mut cell_position = vec![0; cell_count];

        let minimum = if pattern_count > 1 {
            buckets[pattern_count].reserve(cell_count);

            for cell_index in 0..cell_count {
                let position = buckets[pattern_count].len();
                buckets[pattern_count].push(cell_index);
                cell_bucket[cell_index] = pattern_count;
                cell_position[cell_index] = position;
            }

            pattern_count
        } else {
            buckets.len()
        };

        Self {
            buckets,
            cell_bucket,
            cell_position,
            minimum,
        }
    }

    pub fn update(&mut self, cell_index: usize, possible_count: usize) {
        assert!(cell_index < self.cell_bucket.len());
        assert!(possible_count < self.buckets.len());

        self.remove_cell(cell_index);

        // collapsed and contradiction cells must not be candidates
        if possible_count <= 1 {
            return;
        }

        let position = self.buckets[possible_count].len();
        self.buckets[possible_count].push(cell_index);
        self.cell_bucket[cell_index] = possible_count;
        self.cell_position[cell_index] = position;

        if possible_count < self.minimum {
            self.minimum = possible_count;
        }
    }

    pub fn pop_lowest<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Option<usize> {
        self.advance_minimum();

        if self.minimum >= self.buckets.len() {
            return None;
        }

        let bucket_index = self.minimum;
        let random_index = rng.random_range(0..self.buckets[bucket_index].len());
        let cell_index = self.buckets[bucket_index][random_index];

        self.remove_cell(cell_index);

        Some(cell_index)
    }

    fn remove_cell(&mut self, cell_index: usize) {
        let bucket_index = self.cell_bucket[cell_index];

        if bucket_index == NO_BUCKET {
            return;
        }

        let position = self.cell_position[cell_index];
        let bucket = &mut self.buckets[bucket_index];
        let removed_cell = bucket.swap_remove(position);

        debug_assert_eq!(removed_cell, cell_index);

        // swap_remove moves the last element into the removed position
        // it's stored position therefore needs to be updated
        if position < bucket.len() {
            let moved_cell = bucket[position];
            self.cell_position[moved_cell] = position;
        }

        self.cell_bucket[cell_index] = NO_BUCKET;
    }

    fn advance_minimum(&mut self) {
        while self.minimum < self.buckets.len() && self.buckets[self.minimum].is_empty() {
            self.minimum += 1;
        }
    }
}
