use std::hash::{BuildHasher, Hasher};

use facet::{HashFn, PtrConst, PtrMut, Shape};
use hashbrown::HashTable;

use crate::erased::Erased;

#[derive(Clone, Copy)]
pub struct ErasedKeyRef<'a>(pub(crate) PtrConst<'a>);

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

struct HashTableEntry {
    pub key: ErasedKey,
    pub value: ErasedValue,
}

#[derive(Default)]
pub struct ErasedHashMap<S> {
    hash_table: HashTable<HashTableEntry>,
    hash_builder: S,
}

impl<S> ErasedHashMap<S> {
    #[inline(never)]
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

    #[inline(never)]
    pub unsafe fn get<'a>(
        &'a self,
        key_ref: ErasedKeyRef<'_>,
        key_shape: &Shape,
    ) -> Option<&'a ErasedValue>
    where
        S: BuildHasher,
    {
        let hash = unsafe { make_hash(&self.hash_builder, key_ref.0, key_shape) };
        let eq = unsafe { make_eq(key_ref.0, key_shape) };

        let value = self.hash_table.find(hash, eq);

        value.map(|hash_table_entry| &hash_table_entry.value)
    }

    /// Drops the keys and values in the hash map, which requires the shapes
    /// and cannot be done in the Drop impl for this struct.
    /// Safety: `this` is a valid pointer and `key_shape` and `value_shape` are the
    ///         the correct shapes.
    pub unsafe fn drop_keys_and_values(this: *mut Self, key_shape: &Shape, value_shape: &Shape) {
        let drop_key = Erased::drop_fn(key_shape);
        let drop_value = Erased::drop_fn(value_shape);

        if drop_key.is_some() || drop_value.is_some() {
            for hash_table_entry in unsafe { (*this).hash_table.iter_mut() } {
                if let Some(drop_key) = &drop_key {
                    drop_key(&mut hash_table_entry.key.0);
                }
                if let Some(drop_value) = &drop_value {
                    drop_value(&mut hash_table_entry.value.0);
                }
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
