use core::hash::Hasher;
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap};
use std::hash::Hash;

/*
hashing some bytes gives you a DATA KEY.


*/

impl<A: Eq + Hash + Clone, B: Eq + Hash + Clone> Default for OneToManyMap<A, B> {
    fn default() -> Self {
        Self {
            to_bs: Default::default(),
            to_a: Default::default(),
        }
    }
}

#[derive(Debug)]
struct OneToManyMap<A: Eq + Hash + Clone, B: Eq + Hash + Clone> {
    to_bs: HashMap<A, HashSet<B>>,
    to_a: HashMap<B, A>,
}
impl<A: Eq + Hash + Clone, B: Eq + Hash + Clone> OneToManyMap<A, B> {
    fn insert(&mut self, a: A, b: B) -> bool {
        let bs = self.to_bs.entry(a.clone()).or_insert_with(HashSet::default);
        match self.to_a.get_mut(&b) {
            Some(a_element) if a_element == &a && bs.contains(&b) => {
                // providing a mapping we already had
                false
            }
            None if !bs.contains(&b) => {
                // new a <-> b mapping that doesn't cause any conflicts
                bs.insert(b.clone());
                self.to_a.insert(b, a);
                true
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct DataKey(u64);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct LineageKey(u64);

impl std::fmt::Debug for LineageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lk {:X}", self.0)
    }
}
impl std::fmt::Debug for DataKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dk {:X}", self.0)
    }
}

fn pseudo_data_compute(inputs: &[&[u8]]) -> Box<[u8]> {
    let mut h = std::collections::hash_map::DefaultHasher::default();
    for input in inputs {
        input.hash(&mut h);
    }
    h.write_u8(b'C');
    Box::new(h.finish().to_ne_bytes())
}

#[derive(Debug, Default)]
struct Store {
    data: HashMap<DataKey, Box<[u8]>>,
    lineage: HashMap<LineageKey, Vec<LineageKey>>,
    data_classes: OneToManyMap<DataKey, LineageKey>, // data-equivalence classes for lineages
}
impl Store {
    fn store_data(&mut self, data: &[u8]) -> DataKey {
        let key = DataKey::new(data);
        self.data
            .entry(key)
            .or_insert_with(|| data.to_vec().into_boxed_slice());
        key
    }
    fn store_lineage(&mut self, lineage: &Lineage) -> LineageKey {
        match lineage {
            Lineage::InnerKey(lineage_key) => *lineage_key,
            Lineage::Leaf(data_key) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                h.write_u64(data_key.0);
                h.write_u8(b'D');
                let lineage_key = LineageKey(h.finish());
                self.data_classes.insert(*data_key, lineage_key);
                lineage_key
            }
            Lineage::Inner(vec) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                let keys: Vec<_> = vec.iter().map(|input| self.store_lineage(input)).collect();
                for key in keys.iter() {
                    h.write_u64(key.0);
                }
                h.write_u8(b'L');
                let lineage_key = LineageKey(h.finish());
                self.lineage.entry(lineage_key).or_insert(keys);
                lineage_key
            }
        }
    }
    fn lk_to_dk(&self, lineage_key: &LineageKey) -> Option<&DataKey> {
        self.data_classes.to_a.get(lineage_key)
    }
    fn dk_to_data(&self, data_key: &DataKey) -> Option<&[u8]> {
        self.data.get(data_key).map(AsRef::as_ref)
    }
    fn compute_data(&mut self, lineage_key: &LineageKey) -> Result<DataKey, LineageKey> {
        // try retrieve cached
        if let Some(&data_key) = self.lk_to_dk(lineage_key) {
            return Ok(data_key);
        }
        let args: Vec<LineageKey> = self.lineage.get(lineage_key).ok_or(*lineage_key)?.clone();
        let child_data_keys = args
            .iter()
            .map(|child_lk| self.compute_data(child_lk))
            .collect::<Result<Vec<DataKey>, LineageKey>>()?;
        let child_data = child_data_keys
            .iter()
            .map(|dk| Ok(self.data.get(&dk).expect("SHUDDA").as_ref()))
            .collect::<Result<Vec<&[u8]>, LineageKey>>()?;
        let data = pseudo_data_compute(&child_data);
        let data_key = DataKey::new(&data);
        self.data_classes.insert(data_key, *lineage_key);
        self.data.insert(data_key, data);
        Ok(data_key)
    }
    fn dk_to_lks(&self, data_key: &DataKey) -> Option<&HashSet<LineageKey>> {
        self.data_classes.to_bs.get(data_key)
    }
}

impl DataKey {
    fn new(data: &[u8]) -> Self {
        let mut h = std::collections::hash_map::DefaultHasher::default();
        h.write_u8(b'D');
        data.hash(&mut h);
        Self(h.finish())
    }
}

// used externally
#[derive(Debug)]
enum Lineage {
    Inner(Vec<Lineage>), // recursive
    InnerKey(LineageKey),
    Leaf(DataKey),
}

fn main() {
    let mut store = Store::default();
    let dk_f = store.store_data(b"f");
    let dk_x = store.store_data(b"x");
    let lk_fx = store.store_lineage(&Lineage::Inner(vec![
        Lineage::Leaf(dk_f),
        Lineage::Leaf(dk_x),
    ]));
    let dk_fx = store.compute_data(&lk_fx).expect("WEH");
    println!("data is {:?}", store.dk_to_data(&dk_fx));
    let lk_f = store.store_lineage(&Lineage::Leaf(dk_f));
    println!("{:#?}", &store);

    println!("Hello, world!");
}
