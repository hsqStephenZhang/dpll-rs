use dpll_rs::Clauses;
use sudoku::Sudoku;

// 1-base dimac
// (row-1, col-1)'s value equals to num
fn var_num(row: i32, col: i32, num: i32) -> i32 {
    (row - 1) * 81 + (col - 1) * 9 + num
}

// notice that, since the dimac is 1-base, so we need to -1 at the beginning
// of this function
fn unpack_var_num(var: i32) -> (i32, i32, i32) {
    let var = var - 1;
    let num = var % 9;
    let col = (var - num) / 9 % 9;
    let row = (var - num - col * 9) / 81;
    (row, col, num + 1)
}

fn sudoku_to_cnf(grid: [u8; 81]) -> Vec<Vec<i32>> {
    let mut clauses = Vec::new();
    // 生成单元格规则、行规则、列规则和宫格规则
    for i in 1..=9 {
        for j in 1..=9 {
            // 单元格规则
            let mut cell_clause = Vec::new();
            for k in 1..=9 {
                cell_clause.push(var_num(i, j, k));
            }
            clauses.push(cell_clause);
        }
    }

    // 行规则
    for row in 1..=9 {
        for num in 1..=9 {
            // 确保数字num在行row中出现
            let mut row_clause = Vec::new();
            for col in 1..=9 {
                row_clause.push(var_num(row, col, num));
            }
            clauses.push(row_clause);

            // 确保数字num在行row中不会在多个位置出现
            for col1 in 1..9 {
                for col2 in (col1 + 1)..=9 {
                    clauses.push(vec![-var_num(row, col1, num), -var_num(row, col2, num)]);
                }
            }
        }
    }

    // 列规则
    for col in 1..=9 {
        for num in 1..=9 {
            // 确保数字num在列col中出现
            let mut col_clause = Vec::new();
            for row in 1..=9 {
                col_clause.push(var_num(row, col, num));
            }
            clauses.push(col_clause);

            // 确保数字num在列col中不会在多个位置出现
            for row1 in 1..9 {
                for row2 in (row1 + 1)..=9 {
                    clauses.push(vec![-var_num(row1, col, num), -var_num(row2, col, num)]);
                }
            }
        }
    }

    // 宫格规则
    for block_row in 0..3 {
        for block_col in 0..3 {
            for num in 1..=9 {
                let mut block_clause = Vec::new();
                for row in 1..=3 {
                    for col in 1..=3 {
                        block_clause.push(var_num(block_row * 3 + row, block_col * 3 + col, num));
                    }
                }
                clauses.push(block_clause);

                // 确保数字num在宫格中不会在多个位置出现
                for pos1 in 0..8 {
                    for pos2 in (pos1 + 1)..9 {
                        let row1 = block_row * 3 + pos1 / 3 + 1;
                        let col1 = block_col * 3 + pos1 % 3 + 1;
                        let row2 = block_row * 3 + pos2 / 3 + 1;
                        let col2 = block_col * 3 + pos2 % 3 + 1;
                        clauses.push(vec![-var_num(row1, col1, num), -var_num(row2, col2, num)]);
                    }
                }
            }
        }
    }

    // 添加已知单元格值
    for i in 1..=9 {
        for j in 1..=9 {
            let index = (i - 1) * 9 + j - 1;
            if grid[index] != 0 {
                clauses.push(vec![var_num(i as _, j as _, grid[index] as i32)]);
            }
        }
    }

    clauses
}

fn main() {
    let sudoku = Sudoku::generate();
    let rules = sudoku_to_cnf(sudoku.to_bytes());

    let clauses = Clauses::from(rules.as_slice());
    let mut cnf = dpll_rs::Cnf::from(clauses);

    for i in 0..100 {
        println!("{:?}", &cnf.clauses[&i]);
    }

    println!("n_lit: {:?}", cnf.n_lit);

    let (p_solution, _cnf) = dpll_rs::dpll(&mut cnf).unwrap();
    assert!(p_solution.is_solved());
    let true_lits = p_solution.true_lits();
    assert_eq!(true_lits.len(), 81);

    let mut grid = [0; 81];
    for lit in true_lits {
        let (row, col, num) = unpack_var_num(lit as i32 + 1);
        let index = (row * 9 + col) as usize;
        grid[index] = num as u8;
    }

    let solution = Sudoku::from_bytes(grid).unwrap();
    println!("{}", sudoku.display_block());
    println!("{}", solution.display_block());
}
