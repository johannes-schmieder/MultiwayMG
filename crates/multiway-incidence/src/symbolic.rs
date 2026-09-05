//! Owner-bound symbolic maps; numerical weights and quality gates are separate.

mod coarse;
mod groups;
mod pair;

pub use coarse::PreparedCoarseTupleMap;
pub use groups::TupleMergeGroups;
pub use pair::PreparedPairEdgeMap;

use crate::IncidenceError;

fn admit(context: &'static str, required: usize, budget: usize) -> Result<(), IncidenceError> {
    if required > budget {
        return Err(IncidenceError::SymbolicSetupBudgetExceeded {
            context,
            required,
            budget,
        });
    }
    Ok(())
}

#[cfg(test)]
mod failure_tests;
