// mod main2;
mod one_to_many_map;

use one_to_many_map::OneToManyMap;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct DataId(u64);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct ExprId(u64);

#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<DataId, Box<[u8]>>,
    depends_on: HashMap<ExprId, Vec<ExprId>>,
    data_classes: OneToManyMap<DataId, ExprId>, // data-equivalence classes for exprs
}

// A tree representing a
#[derive(Debug)]
pub enum Expr {
    ExprId(ExprId),     // base case 1: expression equivalent to the one stored with ID...
    DataId(DataId),     // base case 2: expression evaluates to the data stored with ID...
    Compute(Vec<Expr>), // recursive case: evaluated by applying args[1..] to function arg0.
}

////////////////////////////////////////////////////////////////

impl std::fmt::Debug for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L_ID {:X}", self.0)
    }
}
impl std::fmt::Debug for DataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "D_ID {:X}", self.0)
    }
}

fn pseudo_data_compute(inputs: &[&[u8]]) -> Box<[u8]> {
    // This is placeholder!
    let mut h = std::collections::hash_map::DefaultHasher::default();
    for input in inputs {
        input.hash(&mut h);
    }
    h.write_u8(b'C');
    Box::new(h.finish().to_ne_bytes())
}

impl DataId {
    fn new(data: &[u8]) -> Self {
        let mut h = std::collections::hash_map::DefaultHasher::default();
        h.write_u8(b'D');
        data.hash(&mut h);
        Self(h.finish())
    }
}

impl Store {
    pub fn store_data(&mut self, data: &[u8]) -> DataId {
        let id = DataId::new(data);
        self.data.entry(id).or_insert_with(|| data.to_vec().into_boxed_slice());
        id
    }
    pub fn store_expr(&mut self, expr: &Expr) -> ExprId {
        match expr {
            Expr::ExprId(expr_id) => *expr_id,
            Expr::DataId(data_id) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                h.write_u64(data_id.0);
                h.write_u8(b'D');
                let expr_id = ExprId(h.finish());
                self.data_classes.insert(*data_id, expr_id);
                expr_id
            }
            Expr::Compute(vec) => {
                let mut h = std::collections::hash_map::DefaultHasher::default();
                let ids: Vec<_> = vec.iter().map(|input| self.store_expr(input)).collect();
                for id in ids.iter() {
                    h.write_u64(id.0);
                }
                h.write_u8(b'L');
                let expr_id = ExprId(h.finish());
                self.depends_on.entry(expr_id).or_insert(ids);
                expr_id
            }
        }
    }
    pub fn data_to_expr(&self, data_id: &DataId) -> Option<&HashSet<ExprId>> {
        self.data_classes.get_many(data_id)
    }
    pub fn expr_to_data(&self, expr_id: &ExprId) -> Option<&DataId> {
        self.data_classes.get_one(expr_id)
    }
    pub fn data_id_to_data(&self, data_id: &DataId) -> Option<&[u8]> {
        self.data.get(data_id).map(AsRef::as_ref)
    }
    pub fn compute_data(&mut self, expr_id: &ExprId) -> Result<DataId, ExprId> {
        // try retrieve cached
        if let Some(&data_id) = self.expr_to_data(expr_id) {
            return Ok(data_id);
        }
        let args: Vec<ExprId> = self.depends_on.get(expr_id).ok_or(*expr_id)?.clone();
        let child_data_ids = args
            .iter()
            .map(|child_li| self.compute_data(child_li))
            .collect::<Result<Vec<DataId>, ExprId>>()?;
        let child_data = child_data_ids
            .iter()
            .map(|di| Ok(self.data.get(&di).expect("SHUDDA").as_ref()))
            .collect::<Result<Vec<&[u8]>, ExprId>>()?;
        let data = pseudo_data_compute(&child_data);
        let data_id = DataId::new(&data);
        self.data_classes.insert(data_id, *expr_id);
        self.data.insert(data_id, data);
        Ok(data_id)
    }
    pub fn remove_data(&mut self, data_id: &DataId) -> Option<Box<[u8]>> {
        self.data.remove(data_id)
    }
}

///////////////////////////////////////////////////////
fn main() {
    const MY_FUNCTION: &'static [u8] = b"123";
    // Make a new store.
    let mut store = Store::default();

    // Add two data elements to the store without exprs; keep their data-ids.
    let data_id_f = store.store_data(MY_FUNCTION);
    let data_id_x = store.store_data(b"x");

    // Store this expr corresponding to x applied to f (i.e. (f x))
    // keep its expr-id
    let expr_fx = Expr::Compute(vec![Expr::DataId(data_id_f), Expr::DataId(data_id_x)]);
    let expr_id_fx = store.store_expr(&expr_fx);

    {
        // note that expr/data ids are identifiers. They will always refer to the same thing
        let mut store2 = Store::default();
        let data_id_f2 = store.store_data(MY_FUNCTION);
        let expr_id_fx2 = store2.store_expr(&expr_fx);
        assert!(data_id_f == data_id_f2 && expr_id_fx == expr_id_fx2);
    }

    // This expr has no known data id (yet!) because (f x) has not yet been computed.
    assert!(store.expr_to_data(&expr_id_fx).is_none());

    // Instruct the store to compute the data corresponding to (f x).
    let data_id_fx = store.compute_data(&expr_id_fx).unwrap();
    assert!(store.data_id_to_data(&data_id_fx).is_some());
}
