use super::{
    ResourceDetector, ResourceField, ResourceImpact, ResourceManagementIssue, ResourceSeverity,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use syn::{visit::Visit, ImplItem, ItemImpl, ItemStruct, Type};

/// Classification of resource cleanup patterns
#[derive(Debug, PartialEq, Eq)]
enum ResourceCleanupType {
    AutoClose,     // Resources that are automatically closed (e.g., File, Socket)
    ThreadJoin,    // Thread handles that need explicit joining
    ExplicitClose, // Resources requiring explicit close() calls (e.g., Connection)
    Generic,       // Generic resources requiring custom cleanup
}

pub struct DropDetector {
    #[allow(dead_code)]
    resource_type_patterns: HashMap<String, ResourcePattern>,
    #[allow(dead_code)]
    known_resource_types: HashSet<String>,
}

impl Default for DropDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DropDetector {
    pub fn new() -> Self {
        let mut known_resource_types = HashSet::new();

        // Add known resource types
        for rt in RESOURCE_TYPES {
            known_resource_types.insert(rt.to_string());
        }

        Self {
            resource_type_patterns: HashMap::new(),
            known_resource_types,
        }
    }

    fn analyze_type_for_resources(&self, type_def: &TypeDefinition) -> ResourceAnalysis {
        let mut analysis = ResourceAnalysis {
            has_drop_impl: type_def.has_drop_impl,
            ..Default::default()
        };

        // Analyze fields for resource types
        for field in &type_def.fields {
            if let Some(resource_info) = self.classify_field_as_resource(field) {
                analysis.resource_fields.push(resource_info);
                analysis.needs_drop = true;
            }
        }

        // Check for manual cleanup methods
        analysis.has_manual_cleanup = type_def.has_cleanup_methods;

        analysis
    }

    fn classify_field_as_resource(&self, field: &FieldInfo) -> Option<ResourceField> {
        // Check against known resource types
        if self.is_known_resource_type(&field.type_name) {
            return Some(ResourceField {
                field_name: field.name.clone(),
                field_type: field.type_name.clone(),
                is_owning: self.is_owning_type(&field.type_name),
                cleanup_required: self.requires_cleanup(&field.type_name),
            });
        }

        // Pattern-based detection
        if self.matches_resource_pattern(&field.type_name) {
            return Some(ResourceField {
                field_name: field.name.clone(),
                field_type: field.type_name.clone(),
                is_owning: true,
                cleanup_required: true,
            });
        }

        None
    }

    fn is_known_resource_type(&self, type_name: &str) -> bool {
        RESOURCE_TYPES.iter().any(|rt| type_name.contains(rt))
    }

    fn matches_resource_pattern(&self, type_name: &str) -> bool {
        RESOURCE_PATTERNS
            .iter()
            .any(|pattern| type_name.contains(pattern))
    }

    fn is_owning_type(&self, type_name: &str) -> bool {
        // Most resource types are owning by default
        !type_name.contains("Ref") && !type_name.contains("&")
    }

    fn requires_cleanup(&self, type_name: &str) -> bool {
        // Most resource types require cleanup
        !type_name.contains("Arc") && !type_name.contains("Rc")
    }

    /// Classifies a resource type to determine its cleanup pattern
    fn classify_resource_type(field_type: &str) -> ResourceCleanupType {
        match () {
            _ if field_type.contains("File") => ResourceCleanupType::AutoClose,
            _ if field_type.contains("Thread") || field_type.contains("JoinHandle") => {
                ResourceCleanupType::ThreadJoin
            }
            _ if field_type.contains("Connection") => ResourceCleanupType::ExplicitClose,
            _ if field_type.contains("TcpStream") || field_type.contains("Socket") => {
                ResourceCleanupType::AutoClose
            }
            _ => ResourceCleanupType::Generic,
        }
    }

    /// Generates cleanup code for a specific field based on its resource type
    fn generate_field_cleanup(field: &ResourceField) -> String {
        let cleanup_type = Self::classify_resource_type(&field.field_type);

        match cleanup_type {
            ResourceCleanupType::AutoClose => match field.field_type.as_str() {
                t if t.contains("File") => {
                    "        // File handles are automatically closed\n".to_string()
                }
                _ => "        // Network streams are automatically closed\n".to_string(),
            },
            ResourceCleanupType::ThreadJoin => {
                format!(
                    "        if let Some(handle) = self.{}.take() {{\n            let _ = handle.join();\n        }}\n",
                    field.field_name
                )
            }
            ResourceCleanupType::ExplicitClose => {
                format!(
                    "        self.{}.close().unwrap_or_else(|e| {{\n            eprintln!(\"Failed to close connection: {{}}\", e);\n        }});\n",
                    field.field_name
                )
            }
            ResourceCleanupType::Generic => {
                format!("        // Cleanup {} resource\n", field.field_name)
            }
        }
    }

    fn generate_drop_implementation(&self, analysis: &ResourceAnalysis, type_name: &str) -> String {
        let mut drop_impl = String::new();
        drop_impl.push_str(&format!("impl Drop for {} {{\n", type_name));
        drop_impl.push_str("    fn drop(&mut self) {\n");

        for field in &analysis.resource_fields {
            drop_impl.push_str(&Self::generate_field_cleanup(field));
        }

        drop_impl.push_str("    }\n");
        drop_impl.push_str("}\n");
        drop_impl
    }

    fn assess_resource_severity(&self, analysis: &ResourceAnalysis) -> ResourceSeverity {
        let critical_resources = analysis
            .resource_fields
            .iter()
            .filter(|field| self.is_critical_resource(&field.field_type))
            .count();

        if critical_resources > 0 {
            ResourceSeverity::Critical
        } else if analysis.resource_fields.len() > 3 {
            ResourceSeverity::High
        } else if analysis.resource_fields.len() > 1 {
            ResourceSeverity::Medium
        } else {
            ResourceSeverity::Low
        }
    }

    fn is_critical_resource(&self, type_name: &str) -> bool {
        CRITICAL_TYPES.iter().any(|ct| type_name.contains(ct))
    }
}

impl ResourceDetector for DropDetector {
    fn detect_issues(&self, file: &syn::File, _path: &Path) -> Vec<ResourceManagementIssue> {
        let mut visitor = DropVisitor::new();
        visitor.visit_file(file);

        let mut issues = Vec::new();

        for type_def in visitor.type_definitions {
            let resource_analysis = self.analyze_type_for_resources(&type_def);

            if resource_analysis.needs_drop && !resource_analysis.has_drop_impl {
                let severity = self.assess_resource_severity(&resource_analysis);
                let suggested_drop_impl =
                    self.generate_drop_implementation(&resource_analysis, &type_def.name);
                issues.push(ResourceManagementIssue::MissingDrop {
                    type_name: type_def.name.clone(),
                    resource_fields: resource_analysis.resource_fields,
                    suggested_drop_impl,
                    severity,
                });
            }
        }

        issues
    }

    fn detector_name(&self) -> &'static str {
        "DropDetector"
    }

    fn assess_resource_impact(&self, issue: &ResourceManagementIssue) -> ResourceImpact {
        match issue {
            ResourceManagementIssue::MissingDrop { severity, .. } => match severity {
                ResourceSeverity::Critical => ResourceImpact::Critical,
                ResourceSeverity::High => ResourceImpact::High,
                ResourceSeverity::Medium => ResourceImpact::Medium,
                ResourceSeverity::Low => ResourceImpact::Low,
            },
            _ => ResourceImpact::Medium,
        }
    }
}

struct DropVisitor {
    type_definitions: Vec<TypeDefinition>,
    drop_implementations: HashSet<String>,
}

impl DropVisitor {
    fn new() -> Self {
        Self {
            type_definitions: Vec::new(),
            drop_implementations: HashSet::new(),
        }
    }
}

impl<'ast> Visit<'ast> for DropVisitor {
    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let mut fields = Vec::new();

        match &node.fields {
            syn::Fields::Named(named) => {
                for field in &named.named {
                    if let Some(ident) = &field.ident {
                        let type_name = extract_type_name(&field.ty);
                        fields.push(FieldInfo {
                            name: ident.to_string(),
                            type_name,
                        });
                    }
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                for (idx, field) in unnamed.unnamed.iter().enumerate() {
                    let type_name = extract_type_name(&field.ty);
                    fields.push(FieldInfo {
                        name: format!("{}", idx),
                        type_name,
                    });
                }
            }
            _ => {}
        }

        let type_name = node.ident.to_string();
        let has_drop_impl = self.drop_implementations.contains(&type_name);

        self.type_definitions.push(TypeDefinition {
            name: type_name,
            fields,
            has_drop_impl,
            has_cleanup_methods: false, // Will be populated later
        });
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Check if this is a Drop implementation
        if let Some((_, path, _)) = &node.trait_ {
            if path.segments.last().is_some_and(|s| s.ident == "Drop") {
                if let Type::Path(type_path) = &*node.self_ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        self.drop_implementations.insert(segment.ident.to_string());
                    }
                }
            }
        }

        // Check for manual cleanup methods
        if let Type::Path(type_path) = &*node.self_ty {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();

                for item in &node.items {
                    if let ImplItem::Fn(method) = item {
                        let method_name = method.sig.ident.to_string();
                        if CLEANUP_METHOD_NAMES.contains(&method_name.as_str()) {
                            // Mark type as having cleanup methods
                            for type_def in &mut self.type_definitions {
                                if type_def.name == type_name {
                                    type_def.has_cleanup_methods = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn extract_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Type::Reference(reference) => extract_type_name(&reference.elem),
        _ => "Unknown".to_string(),
    }
}

#[derive(Debug, Clone)]
struct TypeDefinition {
    name: String,
    fields: Vec<FieldInfo>,
    has_drop_impl: bool,
    has_cleanup_methods: bool,
}

#[derive(Debug, Clone)]
struct FieldInfo {
    name: String,
    type_name: String,
}

#[derive(Debug, Default)]
struct ResourceAnalysis {
    needs_drop: bool,
    has_drop_impl: bool,
    has_manual_cleanup: bool,
    resource_fields: Vec<ResourceField>,
}

#[derive(Debug, Clone)]
struct ResourcePattern {
    #[allow(dead_code)]
    pattern: String,
    #[allow(dead_code)]
    is_critical: bool,
}

const RESOURCE_TYPES: &[&str] = &[
    "File",
    "TcpStream",
    "UdpSocket",
    "TcpListener",
    "Mutex",
    "RwLock",
    "Condvar",
    "Barrier",
    "Thread",
    "JoinHandle",
    "Child",
    "Process",
    "Box",
    "Rc",
    "Arc",
    "RefCell",
    "BufReader",
    "BufWriter",
    "Cursor",
    "Connection",
    "Client",
    "Database",
    "Transaction",
    "Channel",
    "Sender",
    "Receiver",
    "oneshot",
];

const RESOURCE_PATTERNS: &[&str] = &[
    "Handle",
    "Manager",
    "Pool",
    "Connection",
    "Client",
    "Stream",
    "Reader",
    "Writer",
    "Buffer",
    "Cache",
    "Guard",
    "Lock",
    "Session",
    "Context",
    "Resource",
];

const CRITICAL_TYPES: &[&str] = &[
    "File",
    "TcpStream",
    "Process",
    "Thread",
    "Connection",
    "Database",
];

const CLEANUP_METHOD_NAMES: &[&str] = &[
    "cleanup", "close", "shutdown", "dispose", "destroy", "release", "free", "clear",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_resource_type_file() {
        assert_eq!(
            DropDetector::classify_resource_type("std::fs::File"),
            ResourceCleanupType::AutoClose
        );
        assert_eq!(
            DropDetector::classify_resource_type("FileHandle"),
            ResourceCleanupType::AutoClose
        );
    }

    #[test]
    fn test_classify_resource_type_thread() {
        assert_eq!(
            DropDetector::classify_resource_type("std::thread::JoinHandle<()>"),
            ResourceCleanupType::ThreadJoin
        );
        assert_eq!(
            DropDetector::classify_resource_type("Thread"),
            ResourceCleanupType::ThreadJoin
        );
    }

    #[test]
    fn test_classify_resource_type_connection() {
        assert_eq!(
            DropDetector::classify_resource_type("Connection"),
            ResourceCleanupType::ExplicitClose
        );
        assert_eq!(
            DropDetector::classify_resource_type("DatabaseConnection"),
            ResourceCleanupType::ExplicitClose
        );
    }

    #[test]
    fn test_classify_resource_type_network() {
        assert_eq!(
            DropDetector::classify_resource_type("TcpStream"),
            ResourceCleanupType::AutoClose
        );
        assert_eq!(
            DropDetector::classify_resource_type("Socket"),
            ResourceCleanupType::AutoClose
        );
    }

    #[test]
    fn test_classify_resource_type_generic() {
        assert_eq!(
            DropDetector::classify_resource_type("CustomResource"),
            ResourceCleanupType::Generic
        );
        assert_eq!(
            DropDetector::classify_resource_type("Buffer"),
            ResourceCleanupType::Generic
        );
    }

    #[test]
    fn test_generate_field_cleanup() {
        // Test file cleanup
        let file_field = ResourceField {
            field_name: "log_file".to_string(),
            field_type: "std::fs::File".to_string(),
            is_owning: true,
            cleanup_required: true,
        };
        assert_eq!(
            DropDetector::generate_field_cleanup(&file_field),
            "        // File handles are automatically closed\n"
        );

        // Test thread cleanup
        let thread_field = ResourceField {
            field_name: "worker".to_string(),
            field_type: "JoinHandle<()>".to_string(),
            is_owning: true,
            cleanup_required: true,
        };
        assert_eq!(
            DropDetector::generate_field_cleanup(&thread_field),
            "        if let Some(handle) = self.worker.take() {\n            let _ = handle.join();\n        }\n"
        );

        // Test connection cleanup
        let conn_field = ResourceField {
            field_name: "db_conn".to_string(),
            field_type: "Connection".to_string(),
            is_owning: true,
            cleanup_required: true,
        };
        assert_eq!(
            DropDetector::generate_field_cleanup(&conn_field),
            "        self.db_conn.close().unwrap_or_else(|e| {\n            eprintln!(\"Failed to close connection: {}\", e);\n        });\n"
        );

        // Test network stream cleanup
        let stream_field = ResourceField {
            field_name: "tcp".to_string(),
            field_type: "TcpStream".to_string(),
            is_owning: true,
            cleanup_required: true,
        };
        assert_eq!(
            DropDetector::generate_field_cleanup(&stream_field),
            "        // Network streams are automatically closed\n"
        );

        // Test generic resource cleanup
        let generic_field = ResourceField {
            field_name: "buffer".to_string(),
            field_type: "Buffer".to_string(),
            is_owning: true,
            cleanup_required: true,
        };
        assert_eq!(
            DropDetector::generate_field_cleanup(&generic_field),
            "        // Cleanup buffer resource\n"
        );
    }

    #[test]
    fn test_generate_drop_implementation_multiple_resources() {
        let detector = DropDetector::new();
        let analysis = ResourceAnalysis {
            resource_fields: vec![
                ResourceField {
                    field_name: "file".to_string(),
                    field_type: "File".to_string(),
                    is_owning: true,
                    cleanup_required: true,
                },
                ResourceField {
                    field_name: "worker".to_string(),
                    field_type: "JoinHandle<()>".to_string(),
                    is_owning: true,
                    cleanup_required: true,
                },
                ResourceField {
                    field_name: "conn".to_string(),
                    field_type: "Connection".to_string(),
                    is_owning: true,
                    cleanup_required: true,
                },
            ],
            needs_drop: true,
            has_drop_impl: false,
            has_manual_cleanup: false,
        };

        let drop_impl = detector.generate_drop_implementation(&analysis, "MyStruct");

        // Check that the implementation contains expected parts
        assert!(drop_impl.contains("impl Drop for MyStruct"));
        assert!(drop_impl.contains("fn drop(&mut self)"));
        assert!(drop_impl.contains("// File handles are automatically closed"));
        assert!(drop_impl.contains("if let Some(handle) = self.worker.take()"));
        assert!(drop_impl.contains("self.conn.close().unwrap_or_else"));
    }
}
