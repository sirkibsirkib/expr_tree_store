# lineage_tree_store

A little couple-hour project, experimenting with a store of data that allows you to add arbitrary data values (byte sequences), and separately, describe other data that can be computed by way of its 'lineage', a declarative description of function application, where the function and its input arguments are lineages OR concrete data. The idea is that the store lets you add data without any known lineage, on the other hand describe data which can be computed from a given lineage without necessarily computing it (yet). At any time, a user can attempt to translate a lineage into data (by performing computation), with the store doing everything it can to cache intermediate results all the way up the lineage to minimize the effort required for later computations.

Example:
```rust
// Make a new store.
let mut store = Store::default();

// Add two data elements to the store without lineages; keep their data-keys.
let dk_f = store.store_data(MY_FUNCTION);
let dk_x = store.store_data(b"x");

// Store this lineage corresponding to x applied to f
// keep its lineage-key
let lk_fx = store.store_lineage(&Lineage::Inner(vec![
    Lineage::Leaf(dk_f),
    Lineage::Leaf(dk_x),
]));

// This lineage has no known data key (yet!) because (f x) has not yet been computed.
assert!(store.lk_to_dk(&lk_fx).is_none());
// Instruct the store to compute the data corresponding to (f x).
let dk_fx = store.compute_data(&lk_fx).unwrap();
assert!(store.dk_to_data(&dk_fx).is_some());
```
