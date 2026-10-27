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
        let mut values = self.values.borrow_mut();
        let dropped = values.len() >= self.capacity;
        if dropped {
            values.pop_front();
        }
        values.push_back(value);
        let len = values.len();
        drop(values);
        if dropped {
            self.notify.row_removed(0, 1);
        }
        self.notify.row_added(len - 1, 1);
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
