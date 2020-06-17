use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::ops::Deref;

pub(crate) struct BidiMap<A, B> {
    key_value: HashMap<Rc<A>, Rc<B>>,
    value_key: HashMap<Rc<B>, Rc<A>>,
}

impl<A, B> BidiMap<A, B>
where
    A: Eq + Hash,
    B: Eq + Hash,
{
    pub(crate) fn new() -> Self {
        Self {
            key_value: HashMap::with_capacity(64),
            value_key: HashMap::with_capacity(64),
        }
    }

    pub(crate) fn entry_or_insert(&mut self, a: A, b: B) {
        if !self.key_value.contains_key(&a) {
            let a = Rc::new(a);
            let b = Rc::new(b);
            self.key_value.insert(a.clone(), b.clone());
            self.value_key.insert(b, a);
        }
    }

    pub(crate) fn get(&self, key: &A) -> Option<&B> {
        self.key_value.get(key).map(Deref::deref)
    }

    pub(crate) fn get_reverse(&self, value: &B) -> Option<&A> {
        self.value_key.get(value).map(Deref::deref)
    }

    pub(crate) fn contain_reverse(&self, value: &B) -> bool {
        self.value_key.contains_key(value)
    }
}

