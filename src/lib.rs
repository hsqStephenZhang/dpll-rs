mod cfcl;
mod clause;
#[allow(dead_code)]
mod cnf;
mod cnf_graph;
mod dpll;
#[allow(dead_code)]
mod lit;

pub use cfcl::cfcl;
pub use clause::{Clause, Clauses};
pub use cnf::Cnf;
pub use cnf_graph::*;
pub use dpll::{dpll, PartialSolution};
pub use lit::{Lit, Var};

#[derive(Debug, Clone, Copy)]
pub enum Strategy {
    Direct,
    Random,
}
