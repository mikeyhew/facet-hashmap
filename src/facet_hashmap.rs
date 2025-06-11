use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash};

use facet::{Facet, PtrConst};

use crate::erased::Erased;
use crate::erased_hashmap::{ErasedHashMap, ErasedKey, ErasedKeyRef, ErasedValue};

#[derive(Default)]
pub struct FacetHashMap<'a, K: Facet<'a>, V: Facet<'a>, S = hashbrown::DefaultHashBuilder> {
    hash_map: ErasedHashMap<S>,
    _marker: std::marker::PhantomData<(K, V, &'a ())>,
}

impl<'a, K, V, S> Drop for FacetHashMap<'a, K, V, S>
where
    K: Facet<'a>,
    V: Facet<'a>,
{
    fn drop(&mut self) {
        unsafe {
            ErasedHashMap::drop_keys_and_values(&mut self.hash_map, K::SHAPE, V::SHAPE);
        }
    }
}

impl<'a, K, V, S> FacetHashMap<'a, K, V, S>
where
    K: Facet<'a>,
    V: Facet<'a>,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
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

    pub fn get<'b, Q: Borrow<K>>(&'b self, key: &Q) -> Option<&'b V>
    where
        K: Hash + Eq,
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
