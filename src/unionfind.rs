use crate::Id;
use indexmap::{IndexMap, IndexSet};
use std::cell::Cell;
use std::fmt::Debug;

// The Key bound on UnionFind is necessary to derive clone. We only
// instantiate UnionFind in one place (EGraph), so this type bound
// isn't intrusive

#[derive(Debug, Clone, Default)]
pub struct UnionFind {
    parents: Vec<Cell<Id>>,
}

impl UnionFind {
    pub fn make_set(&mut self) -> Id {
        let id = self.parents.len() as Id;
        self.parents.push(Cell::new(id));
        id
    }

    #[inline(always)]
    fn parent(&self, query: Id) -> Id {
        self.parents[query as usize].get()
    }

    #[inline(always)]
    fn set_parent(&self, query: Id, new_parent: Id) {
        self.parents[query as usize].set(new_parent)
    }

    pub fn find(&self, mut current: Id) -> Id {
        loop {
            let parent = self.parent(current);
            if current == parent {
                return parent;
            }
            // do path halving and proceed
            let grandparent = self.parent(parent);
            self.set_parent(current, grandparent);
            current = grandparent;
        }
    }

    /// Returns (new_leader, old_leader)
    pub fn union(&mut self, set1: Id, set2: Id) -> (Id, Id) {
        let mut root1 = self.find(set1);
        let mut root2 = self.find(set2);

        if root1 == root2 {
            (root1, root2)
        } else {
            if root1 > root2 {
                // NOTE egg actuallly relied on the returned id being the minimum
                std::mem::swap(&mut root1, &mut root2);
            }
            self.set_parent(root2, root1);
            (root1, root2)
        }
    }

    #[cfg(test)]
    pub fn build_sets(&self) -> IndexMap<Id, IndexSet<Id>> {
        let mut map: IndexMap<Id, IndexSet<Id>> = Default::default();

        for i in 0..self.parents.len() {
            let i = i as Id;
            let leader = self.find(i);
            map.entry(leader).or_default().insert(i);
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indexmap::{indexmap, indexset, IndexMap, IndexSet};

    fn make_union_find(n: u32) -> UnionFind {
        let mut uf = UnionFind::default();
        for _ in 0..n {
            uf.make_set();
        }
        uf
    }

    #[test]
    fn union_find() {
        let n = 10;

        let mut uf = make_union_find(n);

        // test the initial condition of everyone in their own set
        for i in 0..n {
            assert_eq!(uf.find(i), i);
            assert_eq!(uf.find(i), i);
        }

        // make sure build_sets works
        let expected_sets = (0..n)
            .map(|i| (i, indexset!(i)))
            .collect::<IndexMap<_, _>>();
        assert_eq!(uf.build_sets(), expected_sets);

        // build up one set
        assert_eq!(uf.union(0, 1), (0, 1));
        assert_eq!(uf.union(1, 2), (0, 2));
        assert_eq!(uf.union(3, 2), (0, 3));

        // build up another set
        assert_eq!(uf.union(6, 7), (6, 7));
        assert_eq!(uf.union(8, 9), (8, 9));
        assert_eq!(uf.union(7, 9), (6, 8));

        // make sure union on same set returns to == from
        assert_eq!(uf.union(1, 3), (0, 0));
        assert_eq!(uf.union(7, 8), (6, 6));

        // check set structure
        let expected_sets = indexmap!(
            0 => indexset!(0, 1, 2, 3),
            4 => indexset!(4),
            5 => indexset!(5),
            6 => indexset!(6, 7, 8, 9),
        );
        assert_eq!(uf.build_sets(), expected_sets);

        // all paths should be compressed at this point
        for i in 0..n {
            assert_eq!(uf.parent(i), uf.find(i));
        }
    }
}
