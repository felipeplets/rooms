#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: (u16, u16),
    pub end: (u16, u16),
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionBounds {
    pub start_row: u16,
    pub start_col: u16,
    pub end_row: u16,
    pub end_col: u16,
}

impl Selection {
    pub fn bounds(&self) -> SelectionBounds {
        let (row_a, col_a) = self.start;
        let (row_b, col_b) = self.end;

        if row_a < row_b || (row_a == row_b && col_a <= col_b) {
            SelectionBounds {
                start_row: row_a,
                start_col: col_a,
                end_row: row_b,
                end_col: col_b,
            }
        } else {
            SelectionBounds {
                start_row: row_b,
                start_col: col_b,
                end_row: row_a,
                end_col: col_a,
            }
        }
    }
}

impl SelectionBounds {
    pub fn contains(&self, row: u16, col: u16) -> bool {
        if row < self.start_row || row > self.end_row {
            return false;
        }
        if self.start_row == self.end_row {
            return col >= self.start_col && col <= self.end_col;
        }
        if row == self.start_row {
            return col >= self.start_col;
        }
        if row == self.end_row {
            return col <= self.end_col;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_bounds_ordering() {
        let selection = Selection {
            start: (3, 5),
            end: (1, 2),
        };
        let bounds = selection.bounds();
        assert_eq!(bounds.start_row, 1);
        assert_eq!(bounds.start_col, 2);
        assert_eq!(bounds.end_row, 3);
        assert_eq!(bounds.end_col, 5);
    }

    #[test]
    fn test_selection_bounds_contains_single_line() {
        let bounds = SelectionBounds {
            start_row: 2,
            start_col: 3,
            end_row: 2,
            end_col: 5,
        };
        assert!(bounds.contains(2, 3));
        assert!(bounds.contains(2, 5));
        assert!(!bounds.contains(2, 6));
        assert!(!bounds.contains(1, 3));
    }

    #[test]
    fn test_selection_bounds_contains_multi_line() {
        let bounds = SelectionBounds {
            start_row: 1,
            start_col: 4,
            end_row: 3,
            end_col: 2,
        };
        assert!(bounds.contains(1, 4));
        assert!(bounds.contains(2, 0));
        assert!(bounds.contains(3, 2));
        assert!(!bounds.contains(1, 3));
        assert!(!bounds.contains(3, 3));
    }
}
