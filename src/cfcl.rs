use std::ops::Not;

use crate::{Clause, CnfGraph, PartialSolution};

pub fn cfcl(cnf: &mut CnfGraph) -> Result<(PartialSolution, &mut CnfGraph), usize> {
    let mut solution = PartialSolution::new(cnf.n_lit);
    let mut learnt = vec![];

    let res = _cfcl(cnf, &mut solution, &mut learnt).map(|res| (res, cnf));

    println!("learned clauses: {:?}", learnt);

    res
}

fn propagate(cnf: &mut CnfGraph, solution: &mut PartialSolution) -> Result<(), usize> {
    if cnf.clauses.is_empty() {
        return Ok(());
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
            return Ok(());
        } else {
            // conflict
            return Err(usize::MAX);
        }
    }
    Ok(())
}

fn _cfcl(
    cnf: &mut CnfGraph,
    solution: &mut PartialSolution,
    learned_clauses: &mut Vec<Clause>,
) -> Result<PartialSolution, usize> {
    propagate(cnf, solution)?;

    // 3. now that we must make a guess
    let guess_lit = match cnf.next_guess(crate::Strategy::Direct) {
        Some(lit) => lit,
        None => return Err(usize::MAX),
    };

    let mut _cnf = cnf.clone();
    let mut _solution = solution.clone();

    cnf.make_guess(guess_lit);
    solution.assign_lit(guess_lit);
    cnf.propagation(guess_lit)?;
    if cnf.clauses.is_empty() && cnf.occurrences.is_empty() {
        return Ok(solution.clone());
    }

    // 3.1. try lit is true
    return match _cfcl(cnf, solution, learned_clauses) {
        Ok(solution) => Ok(solution),
        Err(clause_id) => {
            // TODO: get the conflicted clause id and learn from it
            // 3.2. try lit is false
            if clause_id == usize::MAX {
                return Err(usize::MAX);
            }
            cnf.learn_from_conflict(clause_id)
                .map(|c| learned_clauses.push(c));
            *cnf = _cnf;
            *solution = _solution;
            let guess_not = guess_lit.not();
            cnf.make_guess(guess_not);
            cnf.propagation(guess_not)?;
            solution.assign_lit(guess_not);
            match _cfcl(cnf, solution, learned_clauses) {
                Ok(res) => return Ok(res),
                Err(clause_id) => {
                    if clause_id == usize::MAX {
                        return Err(usize::MAX);
                    }
                    cnf.learn_from_conflict(clause_id)
                        .map(|c| learned_clauses.push(c));
                    return Err(clause_id);
                }
            }
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
        let mut cnf = CnfGraph::from(clauses);
        println!("{:?}", cnf);
        let (solution, cnf) = cfcl(&mut cnf).unwrap();
        println!("{}", solution.is_solved());
        println!("{:?}", solution.true_lits());
        println!("{:?}", cnf);
    }

    #[test]
    fn test_learn() {
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
        let mut cnf = CnfGraph::from(clauses);
        cfcl(&mut cnf);
        // println!("{}", solution.is_solved());
        // println!("{:?}", solution.true_lits());
        // println!("{:?}", solution.false_lits());
    }
}
