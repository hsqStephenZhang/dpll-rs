use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ops::Not,
};

use petgraph::prelude::NodeIndex;
use rand::seq::IteratorRandom;

use crate::{Clause, Clauses, Lit, Strategy};

#[derive(Debug, Clone, Default)]
pub struct FakeHashSet {
    inner: HashMap<Lit, bool>,
    num: usize,
}

impl FakeHashSet {
    pub fn new() -> Self {
        FakeHashSet {
            inner: HashMap::new(),
            num: 0,
        }
    }

    pub fn insert(&mut self, lit: Lit) {
        self.inner.insert(lit, true);
        self.num += 1;
    }

    pub fn remove(&mut self, lit: Lit) {
        self.inner.insert(lit, false);
        self.num -= 1;
    }

    pub fn contains(&self, lit: Lit) -> bool {
        self.inner.contains_key(&lit) && self.inner[&lit]
    }

    pub fn len(&self) -> usize {
        self.num
    }

    pub fn is_empty(&self) -> bool {
        self.num == 0
    }

    pub fn from_set(set: &HashSet<Lit>) -> Self {
        let mut fake_set = FakeHashSet::new();
        for lit in set {
            fake_set.insert(*lit);
        }
        fake_set
    }

    pub fn iter(&self) -> impl Iterator<Item = &Lit> {
        self.inner
            .iter()
            .filter(|(_, &valid)| valid)
            .map(|(k, _)| k)
    }

    pub fn all(&self) -> impl Iterator<Item = &Lit> {
        self.inner.iter().map(|(k, _)| k)
    }
}

impl From<FakeHashSet> for HashSet<Lit> {
    fn from(value: FakeHashSet) -> Self {
        value
            .inner
            .iter()
            .filter(|(_, &valid)| valid)
            .map(|(k, _)| k.clone())
            .collect()
    }
}

use petgraph::graph::DiGraph;

// record the cnf clauses and the state of propagation
#[derive(Debug, Clone)]
pub struct CnfGraph {
    // count of all lit
    pub n_lit: usize,
    pub max_lit: usize,
    // count of clauses
    pub n_clause: usize,
    // set of lits && is_valid
    pub clauses: HashMap<usize, (FakeHashSet, bool)>,
    pub occurrences: HashMap<Lit, HashSet<usize>>,
    // units is a subset of clauses.keys()
    // and the clause in units should also be in clauses
    pub units: HashSet<usize>,

    // update the graph when add unit clause into the units
    // but ignore the buildup process
    pub graph: DiGraph<Lit, usize>,
    // lit.index() -> node
    pub nodes: Vec<NodeIndex>,
    pub guessed: Vec<Lit>,
}

impl From<Clauses> for CnfGraph {
    fn from(value: Clauses) -> Self {
        let mut cnf = CnfGraph::new(value.1, value.2, value.0.len());
        for clause in value.0 {
            cnf.add_clause(clause);
        }
        cnf
    }
}

impl CnfGraph {
    pub fn new(n_lit: usize, max_lit: usize, n_clause: usize) -> CnfGraph {
        CnfGraph {
            n_lit,
            max_lit,
            n_clause,
            clauses: Default::default(),
            occurrences: Default::default(),
            units: Default::default(),
            graph: DiGraph::new(),
            nodes: vec![NodeIndex::end(); 2 * max_lit + 2],
            guessed: Default::default(),
        }
    }
    pub fn num_clause(&self) -> usize {
        self.clauses.values().filter(|(_, valid)| *valid).count()
    }

    pub fn add_clause(&mut self, clause: Clause) {
        let clause = clause.inner().iter().cloned().collect::<HashSet<_>>();
        let clause_id = self.clauses.len();

        // 1. occurrences
        for lit in clause.iter() {
            self.occurrences
                .entry(*lit)
                .or_insert_with(Default::default)
                .insert(clause_id);
        }
        // 2. units
        if clause.len() == 1 {
            self.units.insert(clause_id);
        }
        // 3. clauses
        self.clauses
            .insert(clause_id, (FakeHashSet::from_set(&clause), true));
    }

    // the clause of clause_id is unit
    // so it must be true, and we can do propagation based on that
    pub fn unit_propagation(&mut self, clause_id: usize) -> Result<Option<Lit>, usize> {
        if let Some((clause, valid)) = self.clauses.get_mut(&clause_id) {
            if *valid {
                *valid = false;
                self.n_clause -= 1;
                assert!(clause.len() == 1, "{:?}", clause);
                let lit: Lit = clause.iter().next().cloned().unwrap();
                log::debug!(
                    "after unit_propagation of clause {}, lit: {}: {:?}",
                    clause_id,
                    lit,
                    self
                );

                return self.propagation(lit).map(|_| Some(lit));
            }
        }
        Ok(None)
    }

    pub fn unit_propagations(&mut self) -> Result<Vec<Lit>, usize> {
        let mut lits = Vec::new();
        while !self.units.is_empty() {
            let clause_id = self.units.iter().next().unwrap().clone();
            if let Some(lit) = self.unit_propagation(clause_id)? {
                lits.push(lit);
            }
            self.units.remove(&clause_id);
        }
        Ok(lits)
    }

    // based on lit is true
    // simplify the clauses that contains lit or !lit
    pub fn propagation(&mut self, lit: Lit) -> Result<(), usize> {
        if self.nodes[lit.code()] == NodeIndex::end() {
            self.nodes[lit.code()] = self.graph.add_node(lit);
        }
        self.remove_positive(lit);

        let lit = !lit;
        if self.nodes[lit.code()] == NodeIndex::end() {
            self.nodes[lit.code()] = self.graph.add_node(lit);
        }
        self.remove_negation(lit)
    }

    pub fn remove_positive(&mut self, lit: Lit) {
        if let Some(occurs) = self.occurrences.remove(&lit) {
            // the clauses that is useless since the lit is true
            for clause_id in occurs {
                if let Some((clause, valid)) = self.clauses.get_mut(&clause_id) {
                    if *valid {
                        *valid = false;
                        // update the occurrences for other lits in this clause since this clause is removed
                        for &lit in clause.iter() {
                            if let Some(occurs) = self.occurrences.get_mut(&lit) {
                                occurs.remove(&clause_id);
                            }
                        }
                        self.n_clause -= 1;
                        self.units.remove(&clause_id);
                    }
                }
            }
        }
    }

    // based on lit is false
    // if one clause's all lits are false, then the clause is conflict
    pub fn remove_negation(&mut self, lit: Lit) -> Result<(), usize> {
        // 1. occurrences
        if let Some(occurs) = self.occurrences.remove(&lit) {
            // the clauses that is useless since the lit is true
            // println!("update graph, lit: {:?}", lit.not());
            for clause_id in occurs {
                if let Some((mut clause, _)) = self.clauses.remove(&clause_id) {
                    // update the occurrences for other lits in this clause since this clause is removed
                    // 3.1 clause
                    clause.remove(lit);
                    // update the graph
                    let lit_node = self.nodes[lit.not().code()];
                    if lit_node == NodeIndex::end() {
                        panic!("lit_node: {:?}", lit_node);
                    }
                    for &affected in clause.iter() {
                        let mut affected_node = self.nodes[affected.code()];
                        if affected_node == NodeIndex::end() {
                            let n = self.graph.add_node(affected);
                            self.nodes[affected.code()] = n;
                            affected_node = n;
                        }
                        if !self.graph.contains_edge(lit_node, affected_node) {
                            // println!(
                            //     "add edge: {:?} -> {:?}, clause id:{}",
                            //     lit, affected, clause_id
                            // );
                            self.graph.add_edge(lit_node, affected_node, 1);
                        }
                    }

                    // clause
                    self.clauses.insert(clause_id, (clause.clone(), true));

                    // units
                    // one clause is conflict when all the lits in the clause are false
                    if clause.len() == 0 {
                        return Err(clause_id);
                    } else if clause.len() == 1 {
                        self.units.insert(clause_id);
                    }
                }
            }
        }
        Ok(())
    }

    // update the cache that contains the shortest clauses in the cnf
    // return Some(true): the cache is updated successfully
    // return Some(false): the cache is not updated since it is not empty
    // return None: the cache cannot be updated
    #[allow(unused)]
    fn update_guess_cache(&mut self) -> Option<bool> {
        // if !self.shortest_clause_ids.is_empty() {
        //     return Some(false);
        // }

        // if self.clauses.is_empty() {
        //     return None;
        // }
        // let mut min_len = usize::MAX;
        // let mut min_len_clauses = HashSet::new();
        // for (clause_id, clause) in self.clauses.iter() {
        //     if clause.len() < min_len {
        //         min_len = clause.len();
        //         min_len_clauses = HashSet::from([*clause_id]);
        //     } else if clause.len() == min_len {
        //         min_len_clauses.insert(clause_id.clone());
        //     }
        // }
        // assert!(min_len_clauses.len() > 0);
        // self.shortest_clause_ids = min_len_clauses;
        Some(true)
    }

    // random choose a lit according to the strategy:
    // 1. the lit occurs the most
    // 2. after choose the lit, we can do more unit propagation
    pub fn next_guess(&mut self, strategy: Strategy) -> Option<Lit> {
        match strategy {
            // always return the true lit first
            Strategy::Direct => {
                let keys = self.occurrences.keys().cloned().collect::<BTreeSet<_>>();
                let res = keys
                    .iter()
                    .next()
                    .map(|lit| if lit.is_negative() { lit.not() } else { *lit });

                return res;
            }
            Strategy::Random => {
                let keys = self.occurrences.keys().cloned().collect::<Vec<_>>();
                return keys.iter().choose(&mut rand::thread_rng()).cloned();
            }
        }
    }

    pub fn make_guess(&mut self, lit: Lit) {
        self.guessed.push(lit);
    }

    pub fn learn_from_conflict(&mut self, clause_id: usize) -> Option<Clause> {
        // let mut clause = Vec::new();
        println!("clause id:{}", clause_id);
        println!("graph: {:?}", self.graph);
        println!(
            "guessed:{:?}, conflict clause: {:?}",
            self.guessed,
            self.clauses[&clause_id].0.all().collect::<Vec<_>>()
        );

        let root = self.guessed[0];
        let mut special = self.guessed.iter().cloned().collect::<HashSet<_>>();
        special.remove(&root);
        let mut queue = self.clauses[&clause_id]
            .0
            .all()
            .map(|x| x.not())
            .collect::<HashSet<_>>();
        queue.remove(&root);

        // must have a root
        let mut learned = HashSet::from([root]);
        while !queue.is_empty() {
            let lit = queue.iter().next().unwrap().clone();
            queue.remove(&lit);
            if learned.contains(&lit) {
                continue;
            }
            let lit_node = self.nodes[lit.code()];
            let parents = self
                .graph
                .neighbors_directed(lit_node, petgraph::Direction::Incoming);
            let parents = parents.map(|parent| self.graph[parent]).collect::<Vec<_>>();
            if parents.iter().any(|x| special.contains(x)) {
                learned.insert(lit);
            } else {
                queue.extend(parents.iter().cloned());
            }
        }
        let learned = learned.into_iter().map(|x| x.not()).collect::<Vec<_>>();
        // println!("learnt clauses: {:?}", learned);

        return Some(Clause(learned));
    }
}

#[cfg(test)]
mod tests {

    use crate::clause::Clauses;

    use super::*;

    #[test]
    fn do_propagation() {
        let clauses = vec![vec![1, -2, -3], vec![-1, 2, -3], vec![-1, -2, 3]];
        let clauses = Clauses::from(clauses.as_slice());
        let mut cnf = CnfGraph::from(clauses);
        cnf.propagation(Lit::from_dimacs(1)).unwrap();
        println!("{:?}", cnf.graph);
        cnf.propagation(Lit::from_dimacs(2)).unwrap();
        println!("{:?}", cnf.graph);
        cnf.unit_propagation(2).unwrap();
        println!("{:?}", cnf.graph);
    }

    #[test]
    fn do_unit_propagation() {
        let clauses = vec![
            vec![1, -2, -3],
            vec![-1, 2, -3],
            vec![-1, -2, 3],
            vec![1],
            vec![2],
        ];
        let clauses = Clauses::from(clauses.as_slice());
        let mut cnf = CnfGraph::from(clauses);
        println!("{:?}", cnf);
        let lits = cnf.unit_propagations();
        println!("{:?}", cnf);
        println!("{:?}", lits);
    }

    #[test]
    fn test_graph() {
        let mut graph = petgraph::graph::DiGraph::<Lit, usize>::new();
        let node1 = graph.add_node(Lit::from_dimacs(1));
        let node2 = graph.add_node(Lit::from_dimacs(2));
        let node3 = graph.add_node(Lit::from_dimacs(3));
        graph.add_edge(node1, node2, 1);
        graph.add_edge(node3, node2, 1);

        let parents = graph.neighbors_directed(node2, petgraph::Direction::Incoming);
        for parent in parents {
            println!("parent: {:?}", parent);
        }
    }
}
