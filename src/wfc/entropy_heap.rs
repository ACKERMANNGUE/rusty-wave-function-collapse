use rand::{ Rng, RngExt };

const NO_POSITION: usize = usize::MAX;
const ENTROPY_NOISE: f64 = 1.0e-9;

pub struct EntropyHeap {
    heap: Vec<usize>,
    positions: Vec<usize>,
    priorities: Vec<f64>,
    noise: Vec<f64>,
}

impl EntropyHeap {
    pub fn new<R: Rng + ?Sized>(
        cell_count: usize,
        pattern_count: usize,
        initial_entropy: f64,
        rng: &mut R
    ) -> Self {
        let mut heap = Vec::with_capacity(cell_count);
        let mut positions = vec![NO_POSITION; cell_count];
        let mut priorities = vec![f64::INFINITY; cell_count];
        let mut noise = vec![0.0; cell_count];

        if pattern_count > 1 {
            for cell_index in 0..cell_count {
                let cell_noise = rng.random_range(0.0..1.0) * ENTROPY_NOISE;
                noise[cell_index] = cell_noise;
                priorities[cell_index] = initial_entropy + cell_noise;
                positions[cell_index] = heap.len();
                heap.push(cell_index);
            }
        }

        let mut result = Self {
            heap,
            positions,
            priorities,
            noise,
        };

        if result.heap.len() > 1 {
            for position in (0..result.heap.len() / 2).rev() {
                result.sift_down(position);
            }
        }

        result
    }

    pub fn update(&mut self, cell_index: usize, entropy: f64, possible_count: usize) {
        assert!(cell_index < self.positions.len());

        if possible_count <= 1 {
            self.remove(cell_index);
            return;
        }

        debug_assert!(entropy.is_finite());

        let new_priority = entropy + self.noise[cell_index];
        let position = self.positions[cell_index];

        if position == NO_POSITION {
            self.priorities[cell_index] = new_priority;
            let new_position = self.heap.len();
            self.heap.push(cell_index);
            self.positions[cell_index] = new_position;
            self.sift_up(new_position);

            return;
        }

        let old_priority = self.priorities[cell_index];
        self.priorities[cell_index] = new_priority;

        match new_priority.total_cmp(&old_priority) {
            std::cmp::Ordering::Less => {
                self.sift_up(position);
            }
            std::cmp::Ordering::Greater => {
                self.sift_down(position);
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    pub fn pop_min(&mut self) -> Option<usize> {
        if self.heap.is_empty() {
            return None;
        }

        let cell_index = self.heap[0];
        self.remove_at(0);

        Some(cell_index)
    }

    fn remove(&mut self, cell_index: usize) {
        let position = self.positions[cell_index];

        if position == NO_POSITION {
            return;
        }

        self.remove_at(position);
    }

    fn remove_at(&mut self, position: usize) {
        let removed_cell = self.heap.swap_remove(position);
        self.positions[removed_cell] = NO_POSITION;

        if position >= self.heap.len() {
            return;
        }

        let moved_cell = self.heap[position];
        self.positions[moved_cell] = position;

        if position > 0 {
            let parent = (position - 1) / 2;

            if self.is_less(position, parent) {
                self.sift_up(position);
                return;
            }
        }

        self.sift_down(position);
    }

    fn sift_up(&mut self, mut position: usize) {
        while position > 0 {
            let parent = (position - 1) / 2;

            if !self.is_less(position, parent) {
                break;
            }

            self.swap_positions(position, parent);
            position = parent;
        }
    }

    fn sift_down(&mut self, mut position: usize) {
        loop {
            let left = position * 2 + 1;

            if left >= self.heap.len() {
                break;
            }

            let right = left + 1;
            let mut smallest = left;

            if right < self.heap.len() && self.is_less(right, left) {
                smallest = right;
            }

            if !self.is_less(smallest, position) {
                break;
            }

            self.swap_positions(position, smallest);
            position = smallest;
        }
    }

    fn is_less(&self, left_position: usize, right_position: usize) -> bool {
        let left_cell = self.heap[left_position];
        let right_cell = self.heap[right_position];

        match self.priorities[left_cell].total_cmp(&self.priorities[right_cell]) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => { left_cell < right_cell }
        }
    }

    fn swap_positions(&mut self, left: usize, right: usize) {
        self.heap.swap(left, right);

        let left_cell = self.heap[left];
        let right_cell = self.heap[right];

        self.positions[left_cell] = left;
        self.positions[right_cell] = right;
    }
}
