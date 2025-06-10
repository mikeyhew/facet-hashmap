use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash};

use facet::{Facet, PtrConst};

use crate::erased::Erased;
use crate::erased_hashmap::{ErasedHashMap, ErasedKey, ErasedKeyRef, ErasedValue};

#[derive(Default)]
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

    pub fn get<'a, Q: Borrow<K>>(&'a self, key: &Q) -> Option<&'a V>
    where
        K: Facet<'a> + Hash + Eq,
        V: Facet<'a>,
        S: BuildHasher,
    {
        let key_ref = PtrConst::new(key.borrow());

        unsafe {
            self.hash_map
                .get(ErasedKeyRef(key_ref), K::SHAPE)
                .map(|value| value.0.as_ptr(V::SHAPE).get())
        }
    }
}
