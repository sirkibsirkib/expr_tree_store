use super::*;

#[derive(Debug)]
pub struct OneToManyMap<A: Eq + Hash + Clone, B: Eq + Hash + Clone> {
    to_bs: HashMap<A, HashSet<B>>,
    to_a: HashMap<B, A>,
}
impl<A: Eq + Hash + Clone, B: Eq + Hash + Clone> Default for OneToManyMap<A, B> {
    fn default() -> Self {
        Self { to_bs: Default::default(), to_a: Default::default() }
    }
}
impl<A: Eq + Hash + Clone, B: Eq + Hash + Clone> OneToManyMap<A, B> {
    pub fn insert(&mut self, a: A, b: B) -> bool {
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
    pub fn get_one(&self, b: &B) -> Option<&A> {
        self.to_a.get(b)
    }
    pub fn get_many(&self, a: &A) -> Option<&HashSet<B>> {
        self.to_bs.get(a)
    }
}
