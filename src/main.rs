use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

#[derive(Debug)]
struct OneToManyMap<A: Eq + Hash + Clone, B: Eq + Hash + Clone> {
    to_bs: HashMap<A, HashSet<B>>,
    to_a: HashMap<B, A>,
}
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct DataId(u64);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct LineageId(u64);

#[derive(Debug, Default)]
struct Store {
    data: HashMap<DataId, Box<[u8]>>,
    lineage: HashMap<LineageId, Vec<LineageId>>,
    data_classes: OneToManyMap<DataId, LineageId>, // data-equivalence classes for lineages
}

// used externally
#[derive(Debug)]
enum Lineage {
    Inner(Vec<Lineage>), // recursive
    InnerKey(LineageId),
    Leaf(DataId),
}

////////////////////////////////////////////////////////////////

impl<A: Eq + Hash + Clone, B: Eq + Hash + Clone> Default for OneToManyMap<A, B> {
    fn default() -> Self {
        Self { to_bs: Default::default(), to_a: Default::default() }
    }
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

impl std::fmt::Debug for LineageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lk {:X}", self.0)
    }
}
impl std::fmt::Debug for DataId {
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

impl Store {
    fn store_data(&mut self, data: &[u8]) -> DataId {
        let id = DataId::new(data);
        self.data.entry(id).or_insert_with(|| data.to_vec().into_boxed_slice());
        id
    }
    fn store_lineage(&mut self, lineage: &Lineage) -> LineageId {
        match lineage {
            Lineage::InnerKey(lineage_id) => *lineage_id,
            Lineage::Leaf(data_id) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                h.write_u64(data_id.0);
                h.write_u8(b'D');
                let lineage_id = LineageId(h.finish());
                self.data_classes.insert(*data_id, lineage_id);
                lineage_id
            }
            Lineage::Inner(vec) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                let ids: Vec<_> = vec.iter().map(|input| self.store_lineage(input)).collect();
                for id in ids.iter() {
                    h.write_u64(id.0);
                }
                h.write_u8(b'L');
                let lineage_id = LineageId(h.finish());
                self.lineage.entry(lineage_id).or_insert(ids);
                lineage_id
            }
        }
    }
    fn li_to_di(&self, lineage_id: &LineageId) -> Option<&DataId> {
        self.data_classes.to_a.get(lineage_id)
    }
    fn di_to_data(&self, data_id: &DataId) -> Option<&[u8]> {
        self.data.get(data_id).map(AsRef::as_ref)
    }
    fn compute_data(&mut self, lineage_id: &LineageId) -> Result<DataId, LineageId> {
        // try retrieve cached
        if let Some(&data_id) = self.li_to_di(lineage_id) {
            return Ok(data_id);
        }
        let args: Vec<LineageId> = self.lineage.get(lineage_id).ok_or(*lineage_id)?.clone();
        let child_data_ids = args
            .iter()
            .map(|child_li| self.compute_data(child_li))
            .collect::<Result<Vec<DataId>, LineageId>>()?;
        let child_data = child_data_ids
            .iter()
            .map(|di| Ok(self.data.get(&di).expect("SHUDDA").as_ref()))
            .collect::<Result<Vec<&[u8]>, LineageId>>()?;
        let data = pseudo_data_compute(&child_data);
        let data_id = DataId::new(&data);
        self.data_classes.insert(data_id, *lineage_id);
        self.data.insert(data_id, data);
        Ok(data_id)
    }
    fn di_to_li(&self, data_id: &DataId) -> Option<&HashSet<LineageId>> {
        self.data_classes.to_bs.get(data_id)
    }
}

impl DataId {
    fn new(data: &[u8]) -> Self {
        let mut h = std::collections::hash_map::DefaultHasher::default();
        h.write_u8(b'D');
        data.hash(&mut h);
        Self(h.finish())
    }
}

///////////////////////////////////////////////////////
const MY_FUNCTION: &'static [u8] = b"123";
fn main() {
    // Make a new store.
    let mut store = Store::default();

    // Add two data elements to the store without lineages; keep their data-ids.
    let di_f = store.store_data(MY_FUNCTION);
    let di_x = store.store_data(b"x");

    // Store this lineage corresponding to x applied to f (i.e. (f x))
    // keep its lineage-id
    let lineage_fx = Lineage::Inner(vec![Lineage::Leaf(di_f), Lineage::Leaf(di_x)]);
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
}
