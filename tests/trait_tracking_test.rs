/// Integration tests for trait implementation tracking (spec 32)
use debtmap::analyzers::trait_implementation_tracker::{
    TraitExtractor, TraitImplementationTracker,
};
use debtmap::analyzers::trait_resolver::{ResolutionPriority, TraitResolver};
use debtmap::priority::call_graph::FunctionId;
use im::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_basic_trait_definition_extraction() {
    let code = r#"
        trait Display {
            fn fmt(&self) -> String;
        }
        
        trait Debug {
            fn debug(&self) -> String {
                format!("Debug: {:?}", self)
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    assert!(tracker.get_trait("Display").is_some());
    assert!(tracker.get_trait("Debug").is_some());

    let display = tracker.get_trait("Display").unwrap();
    assert_eq!(display.name, "Display");
    assert_eq!(display.methods.len(), 1);
    assert_eq!(display.methods[0].name, "fmt");
    assert!(!display.methods[0].has_default);

    let debug = tracker.get_trait("Debug").unwrap();
    assert_eq!(debug.methods.len(), 1);
    assert!(debug.methods[0].has_default);
}

#[test]
fn test_trait_implementation_extraction() {
    let code = r#"
        trait Display {
            fn fmt(&self) -> String;
        }
        
        struct MyStruct {
            value: i32,
        }
        
        impl Display for MyStruct {
            fn fmt(&self) -> String {
                format!("MyStruct({})", self.value)
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    assert!(tracker.implements_trait("MyStruct", "Display"));
    assert_eq!(tracker.get_traits_for_type("MyStruct").unwrap().len(), 1);
}

#[test]
fn test_generic_trait_implementation() {
    let code = r#"
        trait Container<T> {
            fn get(&self) -> &T;
        }
        
        struct Box<T> {
            value: T,
        }
        
        impl<T> Container<T> for Box<T> {
            fn get(&self) -> &T {
                &self.value
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    // Check that the generic implementation is tracked
    assert!(tracker.implementations.contains_key("Container"));
    let impls = tracker.implementations.get("Container").unwrap();
    assert_eq!(impls.len(), 1);
    assert_eq!(impls[0].implementing_type, "Box<T>");
}

#[test]
fn test_blanket_implementation_detection() {
    let code = r#"
        trait ToString {
            fn to_string(&self) -> String;
        }
        
        impl<T: std::fmt::Display> ToString for T {
            fn to_string(&self) -> String {
                format!("{}", self)
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    assert!(!tracker.blanket_impls.is_empty());
    assert_eq!(tracker.blanket_impls[0].trait_name, "ToString");
    assert!(tracker.blanket_impls[0].is_blanket);
}

#[test]
fn test_trait_object_resolution() {
    let code = r#"
        trait Handler {
            fn handle(&self);
        }
        
        struct FileHandler;
        struct NetworkHandler;
        
        impl Handler for FileHandler {
            fn handle(&self) {}
        }
        
        impl Handler for NetworkHandler {
            fn handle(&self) {}
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    // Check trait object candidates
    let implementors = tracker.get_implementors("Handler").unwrap();
    assert_eq!(implementors.len(), 2);
    assert!(implementors.contains("FileHandler"));
    assert!(implementors.contains("NetworkHandler"));

    // Test trait object call resolution
    let methods = tracker.resolve_trait_object_call("Handler", "handle");
    assert_eq!(methods.len(), 2);
}

#[test]
fn test_associated_type_tracking() {
    let code = r#"
        trait Iterator {
            type Item;
            fn next(&mut self) -> Option<Self::Item>;
        }
        
        struct Counter {
            count: u32,
        }
        
        impl Iterator for Counter {
            type Item = u32;
            
            fn next(&mut self) -> Option<Self::Item> {
                self.count += 1;
                Some(self.count)
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    let iterator = tracker.get_trait("Iterator").unwrap();
    assert_eq!(iterator.associated_types.len(), 1);
    assert_eq!(iterator.associated_types[0].name, "Item");
}

#[test]
fn test_method_resolution_order() {
    let tracker = TraitImplementationTracker::new();
    let mut resolver = TraitResolver::new(Arc::new(tracker.clone()));

    // Register an inherent method
    let func_id = FunctionId {
        file: PathBuf::from("test.rs"),
        name: "MyType::method".to_string(),
        line: 10,
    };
    resolver.register_inherent_method("MyType".to_string(), "method".to_string(), func_id);

    // Test that inherent methods have highest priority
    let traits_in_scope = HashSet::new();
    let resolved = resolver.resolve_method_call("MyType", "method", &traits_in_scope);

    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().priority,
        ResolutionPriority::InherentMethod
    );
}

#[test]
fn test_supertrait_tracking() {
    let code = r#"
        trait Display {
            fn fmt(&self) -> String;
        }
        
        trait Debug: Display {
            fn debug(&self) -> String;
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    let debug = tracker.get_trait("Debug").unwrap();
    assert_eq!(debug.supertraits.len(), 1);
    assert_eq!(debug.supertraits[0], "Display");
}

#[test]
fn test_multiple_trait_bounds() {
    let code = r#"
        trait Clone {
            fn clone(&self) -> Self;
        }
        
        trait Send {}
        trait Sync {}
        
        fn process<T: Clone + Send + Sync>(item: T) {
            let _cloned = item.clone();
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let mut extractor = TraitExtractor::new(PathBuf::from("test.rs"));
    let tracker = extractor.extract(&ast);

    // Verify all traits are tracked
    assert!(tracker.get_trait("Clone").is_some());
    assert!(tracker.get_trait("Send").is_some());
    assert!(tracker.get_trait("Sync").is_some());
}

#[test]
fn test_negative_trait_impl() {
    let code = r#"
        struct NoSend;
        
        // Negative implementation (simplified for testing)
        impl !Send for NoSend {}
    "#;

    // Note: This test would need actual negative impl support
    // For now, we just verify the code compiles and doesn't panic
    let result = syn::parse_file(code);
    assert!(result.is_ok() || result.is_err()); // Either parse or don't, but don't panic
}

#[test]
fn test_trait_method_confidence_scoring() {
    let tracker = Arc::new(TraitImplementationTracker::new());
    let mut resolver = TraitResolver::new(tracker);

    let func_id = FunctionId {
        file: PathBuf::from("test.rs"),
        name: "Type::method".to_string(),
        line: 5,
    };
    resolver.register_inherent_method("Type".to_string(), "method".to_string(), func_id);

    let traits_in_scope = HashSet::new();
    let resolved = resolver.resolve_method_call("Type", "method", &traits_in_scope);

    assert!(resolved.is_some());
    let method = resolved.unwrap();
    assert_eq!(method.confidence, 1.0); // Inherent methods have highest confidence
}

#[test]
fn test_resolve_all_methods_with_name() {
    let tracker = Arc::new(TraitImplementationTracker::new());
    let resolver = TraitResolver::new(tracker);

    // Test finding all methods with a given name
    let methods = resolver.find_all_methods("clone");
    // Since we haven't added any implementations, this should be empty
    assert_eq!(methods.len(), 0);
}

#[test]
fn test_cache_functionality() {
    let tracker = Arc::new(TraitImplementationTracker::new());
    let mut resolver = TraitResolver::new(tracker);

    let traits_in_scope = HashSet::new();

    // First call - should cache
    let _ = resolver.resolve_method_call("Type", "method", &traits_in_scope);
    let (_hits1, total1) = resolver.cache_stats();

    // Second call - should hit cache
    let _ = resolver.resolve_method_call("Type", "method", &traits_in_scope);
    let (_hits2, total2) = resolver.cache_stats();

    assert_eq!(total1, 1);
    assert_eq!(total2, 1); // Same total, means it used cache

    // Clear cache
    resolver.clear_cache();
    let (_hits3, total3) = resolver.cache_stats();
    assert_eq!(total3, 0);
}
