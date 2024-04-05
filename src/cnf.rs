use std::collections::{HashMap, HashSet};

use rand::seq::IteratorRandom;

use crate::{Clause, Clauses, Lit, Strategy};

// record the cnf clauses and the state of propagation
#[derive(Debug, Clone)]
pub struct Cnf {
    // count of all lit
    pub n_lit: usize,
    // count of clauses
    pub n_clause: usize,
    pub clauses: HashMap<usize, HashSet<Lit>>,
    pub occurrences: HashMap<Lit, HashSet<usize>>,
    // units is a subset of clauses.keys()
    // and the clause in units should also be in clauses
    pub units: HashSet<usize>,
    // for performance
    // shortest_clause_ids: HashSet<usize>,
}

impl From<Clauses> for Cnf {
    fn from(value: Clauses) -> Self {
        let mut cnf = Cnf::new(value.1, value.0.len());
        for clause in value.0 {
            cnf.add_clause(clause);
        }
        cnf
    }
}

impl Cnf {
    pub fn new(n_lit: usize, n_clause: usize) -> Cnf {
        Cnf {
            n_lit,
            n_clause,
            clauses: Default::default(),
            occurrences: Default::default(),
            units: Default::default(),
            // shortest_clause_ids: Default::default(),
        }
    }

    pub fn num_clause(&self) -> usize {
        self.clauses.len()
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
        self.clauses.insert(clause_id, clause);
    }

    // the clause of clause_id is unit
    // so it must be true, and we can do propagation based on that
    pub fn unit_propagation(&mut self, clause_id: usize) -> Result<Option<Lit>, usize> {
        if let Some(clause) = self.clauses.remove(&clause_id) {
            self.n_clause -= 1;
            assert!(clause.len() == 1, "{:?}", clause);
            let lit: Lit = clause.into_iter().next().unwrap();
            log::debug!(
                "after unit_propagation of clause {}, lit: {}: {:?}",
                clause_id,
                lit,
                self
            );

            return self.propagation(lit).map(|_| Some(lit));
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
        self.remove_positive(lit);
        self.remove_negation(!lit)
    }

    pub fn remove_positive(&mut self, lit: Lit) {
        if let Some(occurs) = self.occurrences.remove(&lit) {
            // the clauses that is useless since the lit is true
            for clause_id in occurs {
                if let Some(clause) = self.clauses.remove(&clause_id) {
                    // update the occurrences for other lits in this clause since this clause is removed
                    for lit in clause {
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

    // based on lit is false
    // if one clause's all lits are false, then the clause is conflict
    pub fn remove_negation(&mut self, lit: Lit) -> Result<(), usize> {
        // 1. occurrences
        if let Some(occurs) = self.occurrences.remove(&lit) {
            // the clauses that is useless since the lit is true
            for clause_id in occurs {
                if let Some(mut clause) = self.clauses.remove(&clause_id) {
                    // update the occurrences for other lits in this clause since this clause is removed
                    // 3.1 clause
                    clause.remove(&lit);
                    // 2. units
                    // one clause is conflict when all the lits in the clause are false
                    if clause.len() == 0 {
                        return Err(clause_id);
                    } else if clause.len() == 1 {
                        self.units.insert(clause_id);
                    }
                    // 3.2 clause
                    self.clauses.insert(clause_id, clause);
                }
            }
        }
        Ok(())
    }

    // update the cache that contains the shortest clauses in the cnf
    // return Some(true): the cache is updated successfully
    // return Some(false): the cache is not updated since it is not empty
    // return None: the cache cannot be updated
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
    pub fn next_guess(&mut self, _strategy: Strategy) -> Option<Lit> {
        // vanilla strategy
        let keys = self.occurrences.keys().cloned().collect::<Vec<_>>();
        return keys.iter().choose(&mut rand::thread_rng()).cloned();
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
        let mut cnf = Cnf::from(clauses);
        cnf.propagation(Lit::from_dimacs(1)).unwrap();
        println!("{:?}", cnf);
        cnf.propagation(Lit::from_dimacs(2)).unwrap();
        println!("{:?}", cnf);
        cnf.unit_propagation(2).unwrap();
        println!("{:?}", cnf);
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
        let mut cnf = Cnf::from(clauses);
        println!("{:?}", cnf);
        let lits = cnf.unit_propagations();
        println!("{:?}", cnf);
        println!("{:?}", lits);
    }
}
