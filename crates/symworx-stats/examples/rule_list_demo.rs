// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Threshold rules and ordered rule-list classifiers (simple).
//!
//! ```bash
//! cargo run -p symworx-stats --example rule_list_demo
//! ```

use ndarray::array;
use symworx_stats::{
    ClassificationRule,
    Comparison,
    RuleHit,
    RuleListClassifier,
    ThresholdCondition,
    classification_report,
    fit_decision_stump,
};

fn main() {
    println!("=== symworx-stats: rule list / thresholds (simple) ===\n");

    // Features: [heart_rate, rmssd_ms] (illustrative units)
    // Classes: 0 = normal, 1 = high_load, 2 = recovery_concern
    println!("1) Hand-authored rule list (first match wins)");
    let rules = vec![
        ClassificationRule {
            conditions: vec![
                ThresholdCondition::new(0, Comparison::Ge, 160.0), // high HR
                ThresholdCondition::new(1, Comparison::Lt, 20.0),  // low RMSSD
            ],
            label: 1,
            name: Some("high_load".into()),
        },
        ClassificationRule {
            conditions: vec![
                ThresholdCondition::new(0, Comparison::Lt, 55.0),
                ThresholdCondition::new(1, Comparison::Lt, 25.0),
            ],
            label: 2,
            name: Some("recovery_concern".into()),
        },
        ClassificationRule::threshold(0, Comparison::Ge, 140.0, 1).with_name("elevated_hr"),
    ];
    let clf = RuleListClassifier::new(rules, 0, [0, 1, 2]);
    println!("{clf}");

    let x = array![
        [72.0, 45.0],  // normal
        [165.0, 15.0], // high_load (rule 0)
        [50.0, 18.0],  // recovery_concern
        [145.0, 40.0], // elevated_hr only
        [100.0, 50.0], // default normal
    ];
    let y_true = vec![0, 1, 2, 1, 0];

    let (pred, hits) = clf.predict_with_hits(&x);
    println!("\n2) Predictions");
    for i in 0..x.nrows() {
        let hit = match hits[i] {
            RuleHit::Rule(r) => format!("rule[{r}]"),
            RuleHit::Default => "default".into(),
        };
        println!(
            "   HR={:.0} RMSSD={:.0}  y={}  ŷ={}  ({hit})",
            x[[i, 0]],
            x[[i, 1]],
            y_true[i],
            pred[i]
        );
    }
    let rep = classification_report(&y_true, &pred, Some(3));
    println!("\n   {rep}");

    // Learned stump on a simple 1D problem
    println!("\n3) Learned decision stump (Gini) on synthetic 1D data");
    let xs = array![[0.0], [0.2], [0.4], [0.7], [0.9], [1.0]];
    let ys = vec![0, 0, 0, 1, 1, 1];
    let stump = fit_decision_stump(&xs, &ys).expect("stump");
    println!(
        "   feature={}  threshold={:.3}  left→{}  right→{}  impurity={:.4}",
        stump.feature, stump.threshold, stump.left_label, stump.right_label, stump.impurity
    );
    let list = stump.to_rule_list();
    println!("   as rule list:\n{list}");
    println!("   stump predictions = {:?}", stump.predict(&xs));

    println!("\nDone. Rules are comparison-only at inference — embed-friendly.");
}
