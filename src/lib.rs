mod erased;
mod erased_hashmap;
mod facet_hashmap;

pub use facet_hashmap::FacetHashMap;

#[test]
fn test_facet_hashmap() {
    let mut facet_hashmap = FacetHashMap::<&str, &str>::default();
    facet_hashmap.insert("key1", "value1");
    facet_hashmap.insert("key2", "value2");

    assert_eq!(facet_hashmap.get(&"key1"), Some(&"value1"));
    assert_eq!(facet_hashmap.get(&"key2"), Some(&"value2"));
    assert_eq!(facet_hashmap.get(&"key3"), None);
}
