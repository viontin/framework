//! Fluent collection wrapper — inspired by Laravel Collections.
//!
//! Provides a rich API for working with arrays of data:
//! map, filter, reduce, sort, chunk, unique, merge, sum, avg, etc.

use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    pub fn new(items: Vec<T>) -> Self { Collection { items } }

    pub fn items(&self) -> &[T] { &self.items }
    pub fn into_inner(self) -> Vec<T> { self.items }
    pub fn count(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn first(&self) -> Option<&T> { self.items.first() }
    pub fn last(&self) -> Option<&T> { self.items.last() }
    pub fn get(&self, index: usize) -> Option<&T> { self.items.get(index) }

    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Collection<U> {
        Collection { items: self.items.iter().map(f).collect() }
    }

    pub fn filter(&self, f: impl Fn(&T) -> bool) -> Collection<&T> {
        Collection { items: self.items.iter().filter(|x| f(x)).collect() }
    }

    pub fn reduce<U>(&self, init: U, f: impl Fn(U, &T) -> U) -> U {
        self.items.iter().fold(init, f)
    }

    pub fn each(&self, f: impl Fn(&T)) { self.items.iter().for_each(f); }

    pub fn contains(&self, item: &T) -> bool where T: PartialEq {
        self.items.contains(item)
    }

    pub fn sort_by(&self, f: impl Fn(&T, &T) -> Ordering) -> Collection<T> where T: Clone {
        let mut items = self.items.clone();
        items.sort_by(f);
        Collection { items }
    }

    pub fn unique(&self) -> Collection<T> where T: Clone + PartialEq {
        let mut seen = Vec::new();
        let mut result = Vec::new();
        for item in &self.items {
            if !seen.contains(item) {
                seen.push(item.clone());
                result.push(item.clone());
            }
        }
        Collection { items: result }
    }

    pub fn chunk(&self, size: usize) -> Collection<Vec<&T>> {
        Collection {
            items: self.items.chunks(size).map(|c| c.iter().collect()).collect(),
        }
    }

    pub fn merge(mut self, other: Collection<T>) -> Self {
        self.items.extend(other.items);
        self
    }

    pub fn reverse(&self) -> Collection<T> where T: Clone {
        let mut items = self.items.clone();
        items.reverse();
        Collection { items }
    }

    pub fn take(&self, n: usize) -> Collection<&T> {
        Collection { items: self.items.iter().take(n).collect() }
    }

    pub fn skip(&self, n: usize) -> Collection<&T> {
        Collection { items: self.items.iter().skip(n).collect() }
    }
}

impl<T: Clone> Collection<T> {
    pub fn to_vec(&self) -> Vec<T> { self.items.clone() }
}

impl Collection<i64> {
    pub fn sum(&self) -> i64 { self.items.iter().sum() }
    pub fn avg(&self) -> f64 { if self.items.is_empty() { 0.0 } else { self.items.iter().sum::<i64>() as f64 / self.items.len() as f64 } }
    pub fn min(&self) -> Option<i64> { self.items.iter().min().copied() }
    pub fn max(&self) -> Option<i64> { self.items.iter().max().copied() }
}

impl Collection<f64> {
    pub fn sum(&self) -> f64 { self.items.iter().sum() }
    pub fn avg(&self) -> f64 { if self.items.is_empty() { 0.0 } else { self.items.iter().sum::<f64>() / self.items.len() as f64 } }
    pub fn min(&self) -> Option<f64> { self.items.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied() }
    pub fn max(&self) -> Option<f64> { self.items.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied() }
}

impl<T: serde::Serialize> Collection<T> {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.items).map_err(|e| e.to_string())
    }
}

impl<T> From<Vec<T>> for Collection<T> {
    fn from(items: Vec<T>) -> Self { Collection { items } }
}

impl<T> IntoIterator for Collection<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

impl<'a, T> IntoIterator for &'a Collection<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}
