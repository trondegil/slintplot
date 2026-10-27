//! A model for streaming data: the latest values, up to a fixed number.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use slint::{Model, ModelNotify, ModelTracker};

/// The latest `capacity` values of a stream. Appending a value drops the
/// oldest in constant time, where `VecModel::remove(0)` moves every other
/// value: 4 MB per dataset of 1,000,000 floats.
pub struct StreamModel<T> {
    values: RefCell<VecDeque<T>>,
    capacity: Cell<usize>,
    notify: ModelNotify,
}

impl<T: Clone + 'static> StreamModel<T> {
    /// A model holding the last `capacity` of `values`.
    pub fn new(capacity: usize, values: impl IntoIterator<Item = T>) -> Self {
        let mut values: VecDeque<T> = values.into_iter().collect();
        values.drain(..values.len().saturating_sub(capacity));
        StreamModel {
            values: RefCell::new(values),
            capacity: Cell::new(capacity),
            notify: ModelNotify::default(),
        }
    }

    /// Changes how many values the model keeps, dropping the oldest when it
    /// shrinks.
    pub fn set_capacity(&self, capacity: usize) {
        self.capacity.set(capacity);
        let dropped = {
            let mut values = self.values.borrow_mut();
            let dropped = values.len().saturating_sub(capacity);
            values.drain(..dropped);
            dropped
        };
        if dropped > 0 {
            self.notify.row_removed(0, dropped);
        }
    }

    /// Appends values, dropping the oldest ones beyond the capacity.
    pub fn extend(&self, new: impl IntoIterator<Item = T>) {
        let mut values = self.values.borrow_mut();
        let before = values.len();
        values.extend(new);
        let added = values.len() - before;
        let dropped = values.len().saturating_sub(self.capacity.get());
        values.drain(..dropped);
        let len = values.len();
        drop(values);
        if added == 0 {
            return;
        }
        if dropped >= before {
            // Every old value is gone, and some new ones too.
            self.notify.reset();
        } else {
            if dropped > 0 {
                self.notify.row_removed(0, dropped);
            }
            self.notify.row_added(len - added, added);
        }
    }
}

impl<T: Clone + 'static> Model for StreamModel<T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.values.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<T> {
        self.values.borrow().get(row).cloned()
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }
}
