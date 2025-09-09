use debtmap::organization::{
    GodObjectDetector, GodObjectMetrics, TrendDirection,
};
use std::path::{Path, PathBuf};

/// Test that metrics tracking properly records god object detection results
#[test]
fn test_metrics_tracking_integration() {
    let mut metrics = GodObjectMetrics::new();
    
    // Create a file with many functions (god object)
    let god_object_code = r#"
        fn function1() {}
        fn function2() {}
        fn function3() {}
        fn function4() {}
        fn function5() {}
        fn function6() {}
        fn function7() {}
        fn function8() {}
        fn function9() {}
        fn function10() {}
        fn function11() {}
        fn function12() {}
        fn function13() {}
        fn function14() {}
        fn function15() {}
        fn function16() {}
        fn function17() {}
        fn function18() {}
        fn function19() {}
        fn function20() {}
        fn function21() {}
        fn function22() {}
        fn function23() {}
        fn function24() {}
        fn function25() {}
    "#;
    
    let file = syn::parse_file(god_object_code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(god_object_code);
    let analysis = detector.analyze_comprehensive(Path::new("test.rs"), &file);
    
    // Record the snapshot
    metrics.record_snapshot(PathBuf::from("test.rs"), analysis.clone());
    
    // Verify metrics were recorded
    assert_eq!(metrics.snapshots.len(), 1);
    assert_eq!(metrics.summary.total_snapshots, 1);
    assert_eq!(metrics.summary.total_god_objects_detected, 1);
    assert!(metrics.summary.average_god_object_score >= 100.0);
    assert_eq!(metrics.summary.files_tracked, 1);
    
    // Simulate refactoring - file improved
    let improved_code = r#"
        fn function1() {}
        fn function2() {}
        fn function3() {}
        fn function4() {}
        fn function5() {}
        fn function6() {}
        fn function7() {}
        fn function8() {}
        fn function9() {}
        fn function10() {}
    "#;
    
    let improved_file = syn::parse_file(improved_code).expect("Failed to parse");
    let improved_analysis = detector.analyze_comprehensive(Path::new("test.rs"), &improved_file);
    
    metrics.record_snapshot(PathBuf::from("test.rs"), improved_analysis);
    
    // Verify trend tracking
    assert_eq!(metrics.snapshots.len(), 2);
    let trend = metrics.get_file_trend(&PathBuf::from("test.rs")).unwrap();
    assert!(trend.method_count_change < 0); // Methods decreased
    assert!(trend.score_change < 0.0); // Score improved
    assert_eq!(trend.trend_direction, TrendDirection::Improving);
    assert!(trend.improved);
    
    // Verify resolved god objects
    let resolved = metrics.get_resolved_god_objects();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0], PathBuf::from("test.rs"));
}

/// Test tracking multiple files
#[test]
fn test_multi_file_metrics_tracking() {
    let mut metrics = GodObjectMetrics::new();
    
    // File 1: God object
    let code1 = (0..30).map(|i| format!("fn func_{}() {{}}", i)).collect::<Vec<_>>().join("\n");
    let file1 = syn::parse_file(&code1).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(&code1);
    let analysis1 = detector.analyze_comprehensive(Path::new("file1.rs"), &file1);
    
    // File 2: Not a god object
    let code2 = r#"
        struct SmallStruct {
            field1: i32,
        }
        
        impl SmallStruct {
            fn method1(&self) {}
            fn method2(&self) {}
        }
    "#;
    let file2 = syn::parse_file(code2).expect("Failed to parse");
    let analysis2 = detector.analyze_comprehensive(Path::new("file2.rs"), &file2);
    
    // Record snapshots
    metrics.record_snapshot(PathBuf::from("file1.rs"), analysis1);
    metrics.record_snapshot(PathBuf::from("file2.rs"), analysis2);
    
    // Verify summary
    assert_eq!(metrics.summary.total_snapshots, 2);
    assert_eq!(metrics.summary.total_god_objects_detected, 1);
    assert_eq!(metrics.summary.files_tracked, 2);
    
    // Verify file histories
    let file1_history = metrics.summary.file_histories
        .iter()
        .find(|h| h.file_path == PathBuf::from("file1.rs"))
        .unwrap();
    assert!(file1_history.current_is_god_object);
    assert_eq!(file1_history.max_methods, 30);
    
    let file2_history = metrics.summary.file_histories
        .iter()
        .find(|h| h.file_path == PathBuf::from("file2.rs"))
        .unwrap();
    assert!(!file2_history.current_is_god_object);
    assert_eq!(file2_history.max_methods, 2);
}

/// Test detection of new god objects
#[test]
fn test_new_god_object_detection() {
    let mut metrics = GodObjectMetrics::new();
    
    // Start with a small file
    let small_code = r#"
        fn func1() {}
        fn func2() {}
        fn func3() {}
    "#;
    let small_file = syn::parse_file(small_code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(small_code);
    let small_analysis = detector.analyze_comprehensive(Path::new("growing.rs"), &small_file);
    
    metrics.record_snapshot(PathBuf::from("growing.rs"), small_analysis);
    
    // File grows into a god object
    let large_code = (0..30).map(|i| format!("fn func_{}() {{}}", i)).collect::<Vec<_>>().join("\n");
    let large_file = syn::parse_file(&large_code).expect("Failed to parse");
    let large_analysis = detector.analyze_comprehensive(Path::new("growing.rs"), &large_file);
    
    metrics.record_snapshot(PathBuf::from("growing.rs"), large_analysis);
    
    // Check new god objects
    let new_god_objects = metrics.get_new_god_objects();
    assert_eq!(new_god_objects.len(), 1);
    assert_eq!(new_god_objects[0], PathBuf::from("growing.rs"));
    
    // Verify trend shows worsening
    let trend = metrics.get_file_trend(&PathBuf::from("growing.rs")).unwrap();
    assert!(trend.method_count_change > 0);
    assert!(trend.score_change > 0.0);
    assert_eq!(trend.trend_direction, TrendDirection::Worsening);
    assert!(!trend.improved);
}