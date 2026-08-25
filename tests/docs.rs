//! Guards the documented surface against drift: README's coverage table and
//! the `src/lib.rs` prelude must agree in both directions, and the Count
//! line must stay arithmetically honest against the exclusions.

use std::collections::BTreeSet;

const LIB_RS: &str = include_str!("../src/lib.rs");
const README_MD: &str = include_str!("../README.md");

#[test]
fn readme_coverage_table_matches_prelude_exports_in_both_directions() {
    let exports = prelude_exports(LIB_RS);
    assert!(!exports.is_empty(), "prelude extraction found no exports");

    let rows = coverage_rows(README_MD);
    let documented: BTreeSet<String> = rows.iter().flat_map(|row| row.names()).collect();

    let undocumented: Vec<_> = exports.difference(&documented).collect();
    assert!(
        undocumented.is_empty(),
        "prelude exports missing from the README coverage table: {undocumented:?}"
    );

    for row in &rows {
        match row.support.as_str() {
            "Mirror" => {
                let unwrapped: Vec<_> =
                    row.names().filter(|name| !exports.contains(name)).collect();
                assert!(
                    unwrapped.is_empty(),
                    "README claims Mirror for {unwrapped:?} but the prelude has no such wrapper"
                );
            }
            "In tree" => {
                let leaked: Vec<_> = row.names().filter(|name| exports.contains(name)).collect();
                assert!(
                    leaked.is_empty(),
                    "README claims In tree for {leaked:?} but they are public prelude wrappers"
                );
            }
            "Mirror or in tree" => {}
            other => panic!("unknown Support value in the coverage table: {other}"),
        }
    }
}

#[test]
fn readme_count_line_gap_equals_listed_exclusions() {
    let count_line = README_MD
        .lines()
        .find(|line| line.starts_with("Count:"))
        .expect("README states its coverage count on a Count: line");
    let numbers: Vec<u64> = count_line
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .flat_map(str::parse)
        .collect();
    assert_eq!(
        numbers.len(),
        2,
        "Count line must read \"Count: N of M ...\": {count_line}"
    );

    let excluded: BTreeSet<String> = README_MD
        .split("## Exclusions")
        .nth(1)
        .expect("README has an Exclusions section")
        .lines()
        .filter(|line| line.starts_with("- "))
        .flat_map(backticked)
        .filter(|name| name.starts_with("Create"))
        .collect();
    assert_eq!(
        numbers[1] - numbers[0],
        excluded.len() as u64,
        "the unsupported share in \"{count_line}\" must equal the excluded Create* types"
    );
}

/// Upstream builder names behind every `pub use crate::…De` prelude line,
/// with the wrapper's `De` suffix stripped.
fn prelude_exports(lib: &str) -> BTreeSet<String> {
    lib.lines()
        .filter(|line| line.trim_start().starts_with("pub use crate::"))
        .map(|line| {
            line.trim()
                .trim_start_matches("pub use crate::")
                .trim_end_matches(';')
                .rsplit("::")
                .next()
                .expect("a pub use line always names its item")
                .trim_end_matches("De")
                .to_string()
        })
        .collect()
}

struct TableRow {
    types_cell: &'static str,
    support: String,
}

impl TableRow {
    fn names(&self) -> impl Iterator<Item = String> + '_ {
        backticked(self.types_cell).into_iter()
    }
}

/// Parses the README coverage table's data rows, skipping header and rule.
fn coverage_rows(readme: &'static str) -> Vec<TableRow> {
    readme
        .lines()
        .filter(|line| line.starts_with("| ") && !line.contains("Family"))
        .map(|row| {
            let cells: Vec<&str> = row.split('|').collect();
            let cell = |index: usize| -> &'static str {
                cells.get(index).copied().unwrap_or_else(|| {
                    panic!("README table row lacks cell {index} (need Family|Types|Support): {row}")
                })
            };
            TableRow {
                types_cell: cell(2),
                support: cell(3).trim().to_string(),
            }
        })
        .collect()
}

/// Segments inside backticks of one cell: `` `A`, `B` `` yields A, B.
fn backticked(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}
