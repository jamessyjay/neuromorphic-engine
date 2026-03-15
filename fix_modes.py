import re

with open('src/runner/modes.rs', 'r') as f:
    code = f.read()

code = re.sub(
    r'        if n_source\.column_id == n_target\.column_id \{\n            if n_source\.minicolumn_id == n_target\.minicolumn_id \{\n                prob = 0\.90; // Minicolumnar vertical stitch\n            \} else \{\n                let layer_diff = \(n_source\.layer_id as i32 - n_target\.layer_id as i32\)\.abs\(\);\n                if layer_diff == 1 \|\| layer_diff == 2 \{\n                    prob = 0\.80; // Vertical intra-columnar connections\n                \} else \{\n                    prob = 0\.10; // Sparse random inside column\n                \}\n            \}\n        \} else if n_source\.layer_id == 2 && n_target\.layer_id == 2 \{\n            prob = 0\.05; // Horizontal L1/L2/3 voting\n        \} else \{\n            prob = 0\.01; // Ultra sparse random long-range\n        \}\n\n        // Ban: L6 cannot directly fire to L5\n        if n_source\.layer_id == 6 && n_target\.layer_id == 5 \{\n            prob = 0\.0;\n        \}\n\n        if rand::RngExt::random_bool\(&mut rng, prob as f64\) \{\n            let base_weight = crate::runner::modes::get_gaussian\(&mut rng, DEFAULT_WEIGHT \* BASE_WEIGHT_MODIFIER, WEIGHT_SIGMA\)\.max\(0\.5\)\.min\(10\.0\);\n            let is_inhib = n_source\.type_ == NeuronType::Interneuron as u8;\n            let weight = if is_inhib \{ -base_weight \} else \{ base_weight \};\n            let is_myelin = rand::RngExt::random_bool\(&mut rng, MYELIN_PROB\);\n            let branch_id = rand::RngExt::random_range\(&mut rng, 0\.\.DENDRITIC_BRANCH_COUNT\);',
    '''        let mut target_distal = false;
        if n_source.column_id == n_target.column_id {
            if n_source.minicolumn_id == n_target.minicolumn_id {
                prob = 0.90; // Minicolumnar vertical stitch
            } else {
                let layer_diff = (n_source.layer_id as i32 - n_target.layer_id as i32).abs();
                if layer_diff == 1 || layer_diff == 2 {
                    prob = 0.80; // Vertical intra-columnar connections
                } else {
                    prob = 0.10; // Sparse random inside column
                }
            }
        } else if n_source.layer_id == 2 && n_target.layer_id == 2 {
            prob = 0.05; // Horizontal L2/3 voting
            target_distal = true; // Voting targets distal dendrites
        } else {
            prob = 0.01; // Ultra sparse random long-range
            target_distal = true; // Long-range context targets distal
        }

        // Ban: L6 cannot directly fire to L5
        if n_source.layer_id == 6 && n_target.layer_id == 5 {
            prob = 0.0;
        }

        if rand::RngExt::random_bool(&mut rng, prob as f64) {
            let base_weight = crate::runner::modes::get_gaussian(&mut rng, DEFAULT_WEIGHT * BASE_WEIGHT_MODIFIER, WEIGHT_SIGMA).max(0.5).min(10.0);
            let is_inhib = n_source.type_ == NeuronType::Interneuron as u8;
            let weight = if is_inhib { -base_weight } else { base_weight };
            let is_myelin = rand::RngExt::random_bool(&mut rng, MYELIN_PROB);
            
            let branch_id = if target_distal {
                rand::RngExt::random_range(&mut rng, 4..12) // Distal branches (4-11)
            } else {
                rand::RngExt::random_range(&mut rng, 0..4)  // Proximal branches (0-3)
            };''',
    code
)

with open('src/runner/modes.rs', 'w') as f:
    f.write(code)

