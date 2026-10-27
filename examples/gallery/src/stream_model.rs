//! A model for streaming data: the latest values, up to a fixed number.

use std::cell::RefCell;
use std::collections::VecDeque;

use slint::{Model, ModelNotify, ModelTracker};

/// The latest `capacity` values of a stream. Appending a value drops the
/// oldest in constant time, where `VecModel::remove(0)` moves every other
/// value: 4 MB per dataset of 1,000,000 floats.
pub struct StreamModel<T> {
    values: RefCell<VecDeque<T>>,
    capacity: usize,
    notify: ModelNotify,
}

impl<T: Clone + 'static> StreamModel<T> {
    /// A model holding the last `capacity` of `values`.
    pub fn new(capacity: usize, values: impl IntoIterator<Item = T>) -> Self {
        let mut values: VecDeque<T> = values.into_iter().collect();
        values.drain(..values.len().saturating_sub(capacity));
        StreamModel {
            values: RefCell::new(values),
            capacity,
            notify: ModelNotify::default(),
        }
    }

    /// Appends a value, dropping the oldest one when the model is full.
    pub fn push(&self, value: T) {
        self.extend([value]);
    }

    /// Appends values, dropping the oldest ones beyond the capacity.
    pub fn extend(&self, new: impl IntoIterator<Item = T>) {
        let mut values = self.values.borrow_mut();
        let before = values.len();
        values.extend(new);
        let added = values.len() - before;
        let dropped = values.len().saturating_sub(self.capacity);
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
