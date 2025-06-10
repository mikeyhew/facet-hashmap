use std::hash::{BuildHasher, Hasher};

use facet::{HashFn, PtrConst, PtrMut, Shape};
use hashbrown::HashTable;

use crate::erased::Erased;

#[repr(transparent)]
pub struct ErasedKey(pub Erased);

impl std::ops::Deref for ErasedKey {
    type Target = Erased;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ErasedKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[repr(transparent)]
pub struct ErasedValue(pub Erased);

impl std::ops::Deref for ErasedValue {
    type Target = Erased;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ErasedValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct HashTableEntry {
    pub key: ErasedKey,
    pub value: ErasedValue,
}

#[derive(Default)]
pub struct ErasedHashMap<S> {
    hash_table: HashTable<HashTableEntry>,
    hash_builder: S,
}

impl<S> ErasedHashMap<S> {
    pub unsafe fn insert(
        &mut self,
        key: ErasedKey,
        key_shape: &Shape,
        value: ErasedValue,
    ) -> Option<ErasedValue>
    where
        S: BuildHasher,
    {
        let hash = unsafe { make_hash(&self.hash_builder, key.as_ptr(key_shape), key_shape) };

        match self.hash_table.entry(
            hash,
            unsafe { make_eq(key.as_ptr(key_shape), key_shape) },
            unsafe { make_table_entry_hasher(&self.hash_builder, key_shape) },
        ) {
            hashbrown::hash_table::Entry::Occupied(occupied_entry) => {
                let hash_table_entry = occupied_entry.into_mut();
                Some(std::mem::replace(&mut hash_table_entry.value, value))
            }
            hashbrown::hash_table::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(HashTableEntry { key, value });
                None
            }
        }
    }
}

unsafe fn make_eq<'a>(
    key_ref: PtrConst<'a>,
    key_shape: &'a Shape,
) -> impl FnMut(&HashTableEntry) -> bool + 'a {
    let eq = (key_shape.vtable.partial_eq)().unwrap();
    move |hash_table_entry| unsafe { eq(key_ref, hash_table_entry.key.as_ptr(key_shape)) }
}

unsafe fn make_hash<S>(hash_builder: &S, key_ref: PtrConst, key_shape: &Shape) -> u64
where
    S: BuildHasher,
{
    unsafe { make_key_ref_hasher(hash_builder, key_shape)(key_ref) }
}

unsafe fn make_key_ref_hasher<'a, S>(
    hash_builder: &'a S,
    key_shape: &'a Shape,
) -> impl Fn(PtrConst) -> u64 + 'a
where
    S: BuildHasher,
{
    let hasher_write_fn = |hasher_this: PtrMut<'_>, bytes: &[u8]| {
        let hasher: &mut S::Hasher = unsafe { hasher_this.as_mut() };
        hasher.write(bytes)
    };

    let hash_fn: HashFn = (key_shape.vtable.hash)().unwrap();

    move |key_ref| {
        let mut hasher = hash_builder.build_hasher();

        unsafe {
            hash_fn(key_ref, PtrMut::new(&mut hasher), hasher_write_fn);
        }

        hasher.finish()
    }
}

unsafe fn make_table_entry_hasher<'a, S>(
    hash_builder: &'a S,
    key_shape: &'a Shape,
) -> impl Fn(&HashTableEntry) -> u64 + 'a
where
    S: BuildHasher,
{
    unsafe {
        let key_ref_hasher = make_key_ref_hasher(hash_builder, key_shape);

        move |hash_table_entry| key_ref_hasher(hash_table_entry.key.as_ptr(key_shape))
    }
}