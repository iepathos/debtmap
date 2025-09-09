//! Criticality calculation for functions based on various factors

use super::types::{CallGraph, FunctionId};

impl CallGraph {
    /// Pure function to calculate entry point criticality factor
    #[cfg(test)]
    pub fn entry_point_criticality_factor(is_entry_point: bool) -> f64 {
        if is_entry_point { 2.0 } else { 1.0 }
    }

    /// Pure function to calculate dependency count criticality factor
    #[cfg(test)]
    pub fn dependency_count_criticality_factor(dependency_count: usize) -> f64 {
        match dependency_count {
            count if count > 5 => 1.5,
            count if count > 2 => 1.2,
            _ => 1.0,
        }
    }

    /// Pure function to check if any caller is an entry point
    #[cfg(test)]
    pub fn has_entry_point_caller<F>(callers: &[FunctionId], is_entry_point_fn: F) -> bool
    where
        F: Fn(&FunctionId) -> bool,
    {
        callers.iter().any(is_entry_point_fn)
    }

    /// Pure function to calculate entry point caller criticality factor
    #[cfg(test)]
    pub fn entry_point_caller_criticality_factor(has_entry_point_caller: bool) -> f64 {
        if has_entry_point_caller { 1.3 } else { 1.0 }
    }

    pub fn calculate_criticality(&self, func_id: &FunctionId) -> f64 {
        let base_criticality = 1.0;
        let entry_factor = Self::entry_point_criticality_factor(self.is_entry_point(func_id));
        let dependency_factor = Self::dependency_count_criticality_factor(self.get_dependency_count(func_id));
        
        let callers = self.get_callers(func_id);
        let has_entry_caller = Self::has_entry_point_caller(&callers, |id| self.is_entry_point(id));
        let caller_factor = Self::entry_point_caller_criticality_factor(has_entry_caller);
        
        base_criticality * entry_factor * dependency_factor * caller_factor
    }
}