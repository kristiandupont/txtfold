use super::types::{AlgorithmResults, AnalysisOutput};

/// Truncate output to the N largest groups across all result variants.
///
/// Groups already appear in descending-count order (the builders guarantee
/// this). The excess groups are dropped; `budget_applied` is set accordingly.
pub fn apply_top(output: &mut AnalysisOutput, n: usize) {
    let trimmed = match &mut output.results {
        AlgorithmResults::Grouped { groups, .. } => {
            let was = groups.len();
            groups.truncate(n);
            groups.len() < was
        }
        AlgorithmResults::SchemaGrouped { schemas, .. } => {
            let was = schemas.len();
            schemas.truncate(n);
            schemas.len() < was
        }
        AlgorithmResults::PathGrouped { patterns, .. } => {
            let was = patterns.len();
            patterns.truncate(n);
            patterns.len() < was
        }
        AlgorithmResults::OutlierFocused { outliers, .. } => {
            let was = outliers.len();
            outliers.truncate(n);
            outliers.len() < was
        }
    };

    if trimmed {
        output.metadata.budget_applied = Some(true);
    }
}

/// Relabel groups using the value of a field from each group's sample data.
///
/// For `SchemaGrouped`: sets `name` to the first sample value of `field`.
/// For `PathGrouped`: sets the first path as the name (field acts as a hint).
/// For `Grouped` and `OutlierFocused`: no-op (insufficient per-group field data).
pub fn apply_label(output: &mut AnalysisOutput, field: &str) {
    match &mut output.results {
        AlgorithmResults::SchemaGrouped { schemas, .. } => {
            for schema in schemas.iter_mut() {
                if let Some(first_val) = schema
                    .sample_values
                    .get(field)
                    .and_then(|vals| vals.first())
                {
                    schema.name = first_val.clone();
                }
            }
        }
        AlgorithmResults::PathGrouped { patterns, .. } => {
            for pattern in patterns.iter_mut() {
                if let Some(first_val) = pattern
                    .sample_values
                    .get(field)
                    .and_then(|vals| vals.first())
                {
                    pattern.schema_description = first_val.clone();
                }
            }
        }
        // Grouped and OutlierFocused don't carry per-group field-level samples
        // in a form that supports relabelling — no-op for now.
        AlgorithmResults::Grouped { .. } | AlgorithmResults::OutlierFocused { .. } => {}
    }
}
