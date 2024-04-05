use std::collections::HashSet;

use crate::lit::Lit;

#[derive(Debug, Clone)]
pub struct Clause(pub(crate) Vec<Lit>);

impl Clause {
    pub fn inner(&self) -> &[Lit] {
        &self.0
    }
}

impl From<&[i32]> for Clause {
    fn from(value: &[i32]) -> Self {
        let mut clause = Vec::new();
        for &lit in value {
            clause.push(Lit::from_dimacs(lit as isize));
        }
        Clause(clause)
    }
}

impl From<Vec<i32>> for Clause {
    fn from(value: Vec<i32>) -> Self {
        let mut clause = Vec::new();
        for lit in value {
            clause.push(Lit::from_dimacs(lit as isize));
        }
        Clause(clause)
    }
}

// clauses & num of vars& max var
#[derive(Debug, Clone)]
pub struct Clauses(pub(crate) Vec<Clause>, pub(crate) usize, pub(crate) usize);

impl From<&[Vec<i32>]> for Clauses {
    fn from(value: &[Vec<i32>]) -> Self {
        let mut vars_map = HashSet::new();
        let mut max = 0;
        for clause in value {
            for &lit in clause {
                vars_map.insert(lit.abs());
                max = max.max(lit.abs() as usize);
            }
        }
        let mut clauses = Vec::new();
        for clause in value {
            clauses.push(Clause::from(clause.as_slice()));
        }
        Clauses(clauses, vars_map.len(), max)
    }
}
