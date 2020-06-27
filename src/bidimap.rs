use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

pub struct BidiMap<A, B> {
    key_value: HashMap<Arc<A>, Arc<B>>,
    value_key: HashMap<Arc<B>, Arc<A>>,
}

impl<A, B> BidiMap<A, B>
where
    A: Eq + Hash,
    B: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            key_value: HashMap::with_capacity(64),
            value_key: HashMap::with_capacity(64),
        }
    }

    pub fn insert(&mut self, a: A, b: B) {
        let a = Arc::new(a);
        let b = Arc::new(b);
        self.key_value.insert(a.clone(), b.clone());
        self.value_key.insert(b, a);
    }

    pub fn get(&self, key: &A) -> Option<&B> {
        self.key_value.get(key).map(Deref::deref)
    }

    pub fn get_reverse(&self, value: &B) -> Option<&A> {
        self.value_key.get(value).map(Deref::deref)
    }

    pub fn contain(&self, value: &A) -> bool {
        self.key_value.contains_key(value)
    }

    pub fn contain_reverse(&self, value: &B) -> bool {
        self.value_key.contains_key(value)
    }
}

