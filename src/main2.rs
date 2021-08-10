use super::*;

use one_to_many_map::OneToManyMap;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

pub trait Represents<T>: Eq + Clone + Hash {
    fn compute_from(t: &T) -> Self;
}

// A tree representing a
#[derive(Debug)]
pub enum Expr<Di, Ei> {
    ExprId(Ei), // base case 1: expression equivalent to the one stored with ID...
    DataId(Di), // base case 2: expression evaluates to the data stored with ID...
    Compute(Vec<Expr<Di, Ei>>), // recursive case: evaluated by applying args[1..] to function arg0.
}

#[derive(Debug, Default)]
pub struct Store<D, Di, Ei>
where
    Di: Represents<D>,
    Ei: Represents<Expr<Di, Ei>>,
{
    data: HashMap<Di, D>,
    depends_on: HashMap<Ei, Vec<Ei>>,
    data_classes: OneToManyMap<Di, Ei>, // data-equivalence classes for exprs
}

impl<D, Di, Ei> Store<D, Di, Ei>
where
    Di: Represents<D>,
    Ei: Represents<Expr<Di, Ei>>,
{
    pub fn store_data(&mut self, data: &[u8]) -> Di {
        let id = Di::new(data);
        self.data.entry(id).or_insert_with(|| data.to_vec().into_boxed_slice());
        id
    }
    pub fn store_expr(&mut self, expr: &Expr<Di, Ei>) -> Ei {
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
