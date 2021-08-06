# lineage_tree_store

A little couple-hour project, experimenting with a store of data that allows you to add arbitrary data values (byte sequences), and separately, describe other data that can be computed by way of its 'lineage', a declarative description of function application, where the function and its input arguments are lineages OR concrete data. The idea is that the store lets you add data without any known lineage, on the other hand describe data which can be computed from a given lineage without necessarily computing it (yet). At any time, a user can attempt to translate a lineage into data (by performing computation), with the store doing everything it can to cache intermediate results all the way up the lineage to minimize the effort required for later computations.

Example:
```rust
// Make a new store.
let mut store = Store::default();

// Add two data elements to the store without lineages; keep their data-ids.
let di_f = store.store_data(MY_FUNCTION);
let di_x = store.store_data(b"x");

// Store this lineage corresponding to x applied to f (i.e. (f x))
// keep its lineage-id
let lineage_fx = Lineage::Inner(vec![
    Lineage::Leaf(di_f),
    Lineage::Leaf(di_x),
]);
let li_fx = store.store_lineage(&lineage_fx);

// note that lineage/data ids are identifiers. They will always refer to the same thing
{
    let mut store2 = Store::default();
    let li_fx2 = store2.store_lineage(&lineage_fx);
    assert_eq!(li_fx, li_fx2);
}

// This lineage has no known data id (yet!) because (f x) has not yet been computed.
assert!(store.li_to_di(&li_fx).is_none());
// Instruct the store to compute the data corresponding to (f x).
let di_fx = store.compute_data(&li_fx).unwrap();
assert!(store.di_to_data(&di_fx).is_some());
```
