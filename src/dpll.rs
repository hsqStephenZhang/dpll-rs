use std::{collections::HashSet, ops::Not};

use crate::{Cnf, Lit};

#[derive(Debug, Clone)]
pub struct PartialSolution {
    solution: Vec<Option<bool>>,
    un_solved: HashSet<usize>,
}

impl PartialSolution {
    pub fn new(n_lit: usize) -> PartialSolution {
        PartialSolution {
            solution: vec![None; n_lit],
            un_solved: (0..n_lit).collect(),
        }
    }

    pub fn assign_lit(&mut self, lit: Lit) {
        self.solution[lit.index()] = Some(lit.is_positive());
        self.un_solved.remove(&lit.index());
    }

    pub fn is_solved(&self) -> bool {
        self.un_solved.is_empty()
    }

    pub fn false_lits(&self) -> Vec<usize> {
        self.lits(false)
    }

    pub fn true_lits(&self) -> Vec<usize> {
        self.lits(true)
    }

    fn lits(&self, val: bool) -> Vec<usize> {
        self.solution
            .iter()
            .enumerate()
            .filter(|(_, v)| **v == Some(val))
            .map(|(i, _)| i)
            .collect()
    }
}

pub fn dpll(cnf: &mut Cnf) -> Result<(PartialSolution, &mut Cnf), usize> {
    let mut solution = PartialSolution::new(cnf.n_lit);
    _dpll(cnf, &mut solution).map(|res| (res, cnf))
}

fn _dpll(cnf: &mut Cnf, solution: &mut PartialSolution) -> Result<PartialSolution, usize> {
    if cnf.clauses.is_empty() {
        return Ok(solution.clone());
    }

    // 1. try  unit propagation
    let unit_lits = cnf.unit_propagations()?;
    for &lit in &unit_lits {
        solution.assign_lit(lit);
    }

    // 2. try pure literal elimination
    let mut pure = vec![];
    for lit in cnf.occurrences.keys() {
        if cnf.occurrences.get(&lit.not()).is_none() {
            pure.push(*lit);
        }
    }
    for &lit in &pure {
        solution.assign_lit(lit);
        cnf.propagation(lit)?;
    }

    if cnf.occurrences.is_empty() {
        if cnf.num_clause() == 0 {
            return Ok(solution.clone());
        } else {
            // conflict
            return Err(usize::MAX);
        }
    }

    // 3. now that we must make a guess
    let guess_lit = match cnf.next_guess(crate::Strategy::Direct) {
        Some(lit) => lit,
        None => return Err(usize::MAX),
    };

    let mut _cnf = cnf.clone();
    let mut _solution = solution.clone();

    solution.assign_lit(guess_lit);
    cnf.propagation(guess_lit)?;
    if cnf.clauses.is_empty() && cnf.occurrences.is_empty() {
        return Ok(solution.clone());
    }

    // 3.1. try lit is true
    return match _dpll(cnf, solution) {
        Ok(solution) => Ok(solution),
        Err(_clause_id) => {
            // 3.2. try lit is false
            *cnf = _cnf;
            *solution = _solution;
            let guess_not = guess_lit.not();
            cnf.propagation(guess_not)?;
            solution.assign_lit(guess_not);
            _dpll(cnf, solution)
        }
    };
}

#[cfg(test)]
mod tests {

    use crate::*;

    #[test]
    fn test_ok() {
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
        let (solution, cnf) = dpll(&mut cnf).unwrap();
        println!("{}", solution.is_solved());
        println!("{:?}", solution.true_lits());
        println!("{:?}", cnf);
    }

    #[test]
    fn test_conflict() {
        let clauses = vec![
            vec![-2, -3, -4, 5],
            vec![-1, -5, 6],
            vec![-5, 7],
            vec![-1, -6, -7],
            vec![-1, -2, 5],
            vec![-1, -3, 5],
            vec![-1, -4, 5],
            vec![1, 4],
            vec![-1, 2, 3, 4, 5, -6],
        ];
        let clauses = Clauses::from(clauses.as_slice());
        let mut cnf = Cnf::from(clauses);
        let (solution, _cnf) = dpll(&mut cnf).unwrap();
        println!("{}", solution.is_solved());
        println!("{:?}", solution.true_lits());
        println!("{:?}", solution.false_lits());
    }
}
