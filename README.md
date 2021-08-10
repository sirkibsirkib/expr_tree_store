# Expr Tree Store

This demonstrates an approach to distributed pipeline processing that prioritizes caching intermediate results. We assume that any computable value is **data**, a finite-length byte sequences. We introduce **expr** a tree structure with a sequence of children exprs as dependencies. Exprs represent computable functions, where the first child is understood as the function, and all subsequent children are understood as arguments. Each **expr** _evaluates to_ the **data** representing the result of this function application. Note that _evaluates-to_ is a many-to-one relation, as any number of expression trees may compute the same resulting data.

------------


This is a little dummy implementation demonstrating a functional approach to pipeline programming where we assume all compute steps can be treated as pure functions. 

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

{
    // note that lineage/data ids are identifiers. They will always refer to the same thing
    let mut store2 = Store::default();
    let di_f2 = store.store_data(MY_FUNCTION);
    let li_fx2 = store2.store_lineage(&lineage_fx);
    assert!(di_f == di_f2 && li_fx == li_fx2);
}

// This lineage has no known data id (yet!) because (f x) has not yet been computed.
assert!(store.li_to_di(&li_fx).is_none());
// Instruct the store to compute the data corresponding to (f x).
let di_fx = store.compute_data(&li_fx).unwrap();
assert!(store.di_to_data(&di_fx).is_some());
```
