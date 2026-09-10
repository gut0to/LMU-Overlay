#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    items: Vec<Option<T>>,
    next: usize,
    len: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer capacity must be greater than zero");

        Self {
            items: (0..capacity).map(|_| None).collect(),
            next: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.items.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, item: T) {
        self.items[self.next] = Some(item);
        self.next = (self.next + 1) % self.capacity();
        self.len = self.len.saturating_add(1).min(self.capacity());
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let start = if self.len == self.capacity() { self.next } else { 0 };

        (0..self.len).filter_map(move |index| {
            let physical_index = (start + index) % self.capacity();
            self.items[physical_index].as_ref()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_insertion_order_before_wrap() {
        let mut buffer = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);

        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn overwrites_oldest_item_after_wrap() {
        let mut buffer = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4);

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
    }
}

