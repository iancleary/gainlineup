//! README example: Node-Level Dynamic Range Summary

use gainlineup::{cascade_vector_return_output, Block, Input};

#[test]
fn node_dynamic_range_summary() {
    let input = Input::new(6.0e9, 1.0e6, -80.0, Some(50.0));
    let blocks = vec![Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        output_p1db_dbm: Some(5.0),
        output_ip3_dbm: Some(20.0),
    }];
    let node = cascade_vector_return_output(input, blocks);

    // Simple linear dynamic range
    let dr = node.dynamic_range_db();
    assert!(dr.is_some());
    assert!(dr.unwrap() > 0.0);

    // Full summary
    let summary = node.dynamic_range_summary();
    assert!(summary.is_some());
    let summary = summary.unwrap();
    assert!(summary.linear_dr_db > 0.0);
    assert!(summary.sfdr_db.is_some());
    assert!(summary.mds_dbm < 0.0);
    assert!(summary.max_input_dbm > summary.mds_dbm);
}
