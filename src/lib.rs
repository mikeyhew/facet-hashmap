use std::hash::{BuildHasher, Hash};

use facet::Facet;

mod erased;
mod erased_hashmap;

use erased::Erased;
use erased_hashmap::{ErasedHashMap, ErasedKey, ErasedValue};

pub struct FacetHashMap<K, V, S = hashbrown::DefaultHashBuilder> {
    hash_map: ErasedHashMap<S>,
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V, S> FacetHashMap<K, V, S> {
    pub fn insert<'a>(&mut self, key: K, value: V) -> Option<V>
    where
        K: Facet<'a> + Hash + Eq,
        V: Facet<'a>,
        S: BuildHasher,
    {
        let erased_key = ErasedKey(Erased::new(key));
        let erased_value = ErasedValue(Erased::new(value));
        let old_erased_value = unsafe { self.hash_map.insert(erased_key, K::SHAPE, erased_value) };

        old_erased_value.map(|old_value| unsafe { old_value.0.into_typed() })
    }
}
