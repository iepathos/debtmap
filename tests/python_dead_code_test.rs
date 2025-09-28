use debtmap::analysis::python_call_graph::PythonCallGraphAnalyzer;
use debtmap::analysis::{PythonDeadCodeDetector, RemovalConfidence};
use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::scoring::classification::{
    classify_debt_type_with_exclusions, is_dead_code_with_exclusions,
};
use debtmap::priority::DebtType;
use im::HashSet;
use std::path::{Path, PathBuf};

#[test]
fn test_python_instance_method_not_dead_code() {
    // Test that private methods called by other methods are not marked as dead code
    let python_code = r#"
class FileManager:
    """Manages file operations over network."""
    
    def _get_connection(self, server_name: str):
        """Create connection to remote server."""
        server = server_registry.get_server_by_name(server_name)
        if not server:
            raise ValueError(f"Server {server_name} not found")
        
        config_path = server_registry.get_config_path()
        
        # Get credentials from config.yml for Worker servers
        credentials = None
        if server.server_type == "Worker":
            try:
                config_data = utils.load_yaml("config.yml")
                if server_name in config_data:
                    server_config = config_data[server_name]
                    if isinstance(server_config, dict) and "auth" in server_config:
                        auth_data = server_config["auth"]
                        if isinstance(auth_data, dict) and "admin" in auth_data:
                            credentials = auth_data["admin"]
                            logger.info(f"Found credentials for admin user in config.yml")
            except Exception as e:
                logger.warning(f"Could not load credentials from config.yml: {str(e)}")
        
        # For Controller, use config with default user
        if server.server_type == "Controller":
            return create_connection(
                server_name=server_name,
                username="default",
                config_path=config_path,
                server_type=server.server_type,
            )
        else:
            # For Worker, use admin user
            username = server.username or "admin"
            return create_connection(
                server_name=server_name,
                username=username,
                credentials=credentials,
                config_path=config_path,
                server_type=server.server_type,
            )
    
    def list_files(self, server_name: str):
        """
        List all files and their metadata.
        """
        files = {}
        file_paths = self._get_file_paths(server_name)
        
        with self._get_connection(server_name) as connection:  # CALLS _get_connection
            for file_type, file_path in file_paths.items():
                # ... rest of implementation
                pass
        return files
    
    def read_file(self, server_name: str, file_type: str):
        """
        Read file content.
        """
        file_paths = self._get_file_paths(server_name)
        
        if file_type not in file_paths:
            raise ValueError(f"Invalid file type: {file_type}")
        
        file_path = file_paths[file_type]
        
        with self._get_connection(server_name) as connection:  # CALLS _get_connection
            # ... rest of implementation
            pass
    
    def update_file(self, server_name: str, file_type: str, content: str):
        """
        Update file content.
        """
        file_paths = self._get_file_paths(server_name)
        
        if file_type not in file_paths:
            return False, f"Invalid file type: {file_type}"
        
        file_path = file_paths[file_type]
        
        try:
            with self._get_connection(server_name) as connection:  # CALLS _get_connection
                # ... rest of implementation
                pass
        except Exception as e:
            return False, str(e)
"#;

    // Parse and analyze the Python code
    let analyzer = PythonAnalyzer::new();
    let path = PathBuf::from("file_manager.py");
    let ast = analyzer.parse(python_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Build call graph with Python method calls
    let mut call_graph = CallGraph::new();

    // Parse the Python code and extract method calls
    let module =
        rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();

    // Use Python call graph analyzer to populate the call graph
    let mut python_analyzer = PythonCallGraphAnalyzer::new();
    python_analyzer
        .analyze_module(&module, &path, &mut call_graph)
        .unwrap();

    // Find the _get_connection function in metrics
    let connection_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("_get_connection"))
        .expect("Should find _get_connection function");

    // Create function ID for the method
    let func_id = FunctionId {
        file: path.clone(),
        name: "FileManager._get_connection".to_string(),
        line: 0, // We're not tracking exact line numbers yet
    };

    // Check if it's marked as dead code
    let framework_exclusions_im = HashSet::new();
    let framework_exclusions_std: std::collections::HashSet<FunctionId> =
        framework_exclusions_im.clone().into_iter().collect();
    let is_dead = is_dead_code_with_exclusions(
        connection_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
    );

    // Should NOT be marked as dead code because it has 3 callers
    assert!(
        !is_dead,
        "_get_connection should NOT be marked as dead code because it has 3 callers"
    );

    // Also check the debt type classification
    let debt_type = classify_debt_type_with_exclusions(
        connection_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
        None,
    );

    // It should NOT be classified as DeadCode
    match debt_type {
        DebtType::DeadCode { .. } => {
            panic!("_get_connection should not be classified as DeadCode!");
        }
        _ => {
            // Good - it's not dead code
        }
    }
}

#[test]
fn test_python_truly_dead_private_method() {
    // This method is actually dead code - no callers
    let python_code = r#"
class MyClass:
    def _unused_method(self):
        """This method is never called."""
        return 42
    
    def public_method(self):
        """This method doesn't call _unused_method."""
        return 100
"#;

    let analyzer = PythonAnalyzer::new();
    let path = PathBuf::from("test.py");
    let ast = analyzer.parse(python_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    let mut call_graph = CallGraph::new();

    // Parse and analyze Python method calls
    let module =
        rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut python_analyzer = PythonCallGraphAnalyzer::new();
    python_analyzer
        .analyze_module(&module, &path, &mut call_graph)
        .unwrap();

    let unused_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("_unused_method"))
        .expect("Should find _unused_method function");

    let func_id = FunctionId {
        file: path.clone(),
        name: "MyClass._unused_method".to_string(),
        line: 0,
    };

    let framework_exclusions_std: std::collections::HashSet<FunctionId> =
        HashSet::new().into_iter().collect();
    let is_dead = is_dead_code_with_exclusions(
        unused_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
    );

    // This SHOULD be marked as dead code
    assert!(
        is_dead,
        "_unused_method should be marked as dead code because it has no callers"
    );
}

#[test]
fn test_python_context_manager_pattern() {
    let python_code = r#"
class ResourceManager:
    def _get_resource(self):
        """Get a resource for use in context manager."""
        return Resource()
    
    def process_data(self):
        with self._get_resource() as resource:
            resource.do_work()
"#;

    let analyzer = PythonAnalyzer::new();
    let path = PathBuf::from("resource.py");
    let ast = analyzer.parse(python_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    let mut call_graph = CallGraph::new();

    // Parse and analyze Python method calls
    let module =
        rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut python_analyzer = PythonCallGraphAnalyzer::new();
    python_analyzer
        .analyze_module(&module, &path, &mut call_graph)
        .unwrap();

    let resource_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("_get_resource"))
        .expect("Should find _get_resource function");

    let func_id = FunctionId {
        file: path.clone(),
        name: "ResourceManager._get_resource".to_string(),
        line: 0,
    };

    let framework_exclusions_std: std::collections::HashSet<FunctionId> =
        HashSet::new().into_iter().collect();
    let is_dead = is_dead_code_with_exclusions(
        resource_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
    );

    assert!(
        !is_dead,
        "_get_resource should NOT be dead code - it's used in a context manager"
    );
}

#[test]
fn test_wxpython_event_handler_not_dead_code() {
    // Test that wxPython event handlers bound with Bind() are not marked as dead code
    let python_code = r#"
import wx

class ConversationPanel:
    def on_paint(self, event):
        """
        Handle paint events to draw the drag-and-drop indicator line.
        
        Custom paint handler that draws a blue horizontal line to indicate where
        a dragged message will be inserted. The line position is determined by
        the drop_indicator_pos value and appears only during active drag operations.
        
        :param event: The paint event
        :type event: wx.PaintEvent
        """
        dc = wx.PaintDC(self.message_container)

        # If we're showing a drop indicator
        if self.drop_indicator_pos is not None:
            # Draw a horizontal line at the insertion point
            width, _ = self.message_container.GetSize()
            y_pos = 0

            if self.drop_indicator_pos == 0:
                # At the beginning
                y_pos = 0
            elif self.drop_indicator_pos >= len(self.messages):
                # At the end
                if self.messages:
                    last_msg = self.messages[-1]
                    y_pos = last_msg.GetPosition().y + last_msg.GetSize().height
            else:
                # Between two messages
                y_pos = self.messages[self.drop_indicator_pos].GetPosition().y

            pen = wx.Pen(wx.BLUE, 3)
            dc.SetPen(pen)
            dc.DrawLine(0, y_pos, width, y_pos)

    def __init__(self):
        # Event handler binding
        self.message_container.Bind(wx.EVT_PAINT, self.on_paint)
"#;

    let analyzer = PythonAnalyzer::new();
    let path = PathBuf::from("conversation_panel.py");
    let ast = analyzer.parse(python_code, path.clone()).unwrap();
    let metrics = analyzer.analyze(&ast);

    let mut call_graph = CallGraph::new();

    // Parse and analyze Python method calls
    let module =
        rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
    let mut python_analyzer = PythonCallGraphAnalyzer::new();
    python_analyzer
        .analyze_module(&module, &path, &mut call_graph)
        .unwrap();

    let on_paint_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("on_paint"))
        .expect("Should find on_paint function");

    let func_id = FunctionId {
        file: path.clone(),
        name: "ConversationPanel.on_paint".to_string(),
        line: 0,
    };

    let framework_exclusions_std: std::collections::HashSet<FunctionId> =
        HashSet::new().into_iter().collect();
    let is_dead = is_dead_code_with_exclusions(
        on_paint_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
    );

    // Should NOT be marked as dead code because it's bound as an event handler
    assert!(
        !is_dead,
        "on_paint should NOT be marked as dead code because it's bound with Bind()"
    );

    // Also check the debt type classification
    let debt_type = classify_debt_type_with_exclusions(
        on_paint_func,
        &call_graph,
        &func_id,
        &framework_exclusions_std,
        None,
        None,
    );

    // It should NOT be classified as DeadCode
    match debt_type {
        DebtType::DeadCode { .. } => {
            panic!("on_paint should not be classified as DeadCode!");
        }
        _ => {
            // Good - it's not dead code
        }
    }
}

#[test]
fn test_python_magic_methods_not_flagged() {
    let detector = PythonDeadCodeDetector::new();

    // Test __init__ method
    let init_func = FunctionMetrics::new(
        "MyClass.__init__".to_string(),
        Path::new("test.py").to_path_buf(),
        10,
    );
    assert!(detector.is_implicitly_called(&init_func));

    // Test __str__ method
    let str_func = FunctionMetrics::new(
        "MyClass.__str__".to_string(),
        Path::new("test.py").to_path_buf(),
        20,
    );
    assert!(detector.is_implicitly_called(&str_func));

    // Test __getitem__ method
    let getitem_func = FunctionMetrics::new(
        "Container.__getitem__".to_string(),
        Path::new("container.py").to_path_buf(),
        15,
    );
    assert!(detector.is_implicitly_called(&getitem_func));
}

#[test]
fn test_python_framework_methods_not_flagged() {
    let detector = PythonDeadCodeDetector::new()
        .with_frameworks(vec!["wxpython".to_string(), "django".to_string()]);

    // Test OnInit method (wxPython)
    let on_init = FunctionMetrics::new(
        "MyApp.OnInit".to_string(),
        Path::new("app.py").to_path_buf(),
        10,
    );
    assert!(detector.is_implicitly_called(&on_init));

    // Test save method (Django)
    let save_method = FunctionMetrics::new(
        "Model.save".to_string(),
        Path::new("models.py").to_path_buf(),
        30,
    );
    assert!(detector.is_implicitly_called(&save_method));
}

#[test]
fn test_python_confidence_levels() {
    let detector = PythonDeadCodeDetector::new();

    // Magic method should have Magic confidence
    let magic_func = FunctionMetrics::new(
        "Cls.__init__".to_string(),
        Path::new("test.py").to_path_buf(),
        0,
    );
    assert_eq!(
        detector.get_removal_confidence(&magic_func),
        RemovalConfidence::Magic
    );

    // Event handler should have Framework confidence (matches wxPython event pattern)
    let event_func = FunctionMetrics::new(
        "Panel.on_click".to_string(),
        Path::new("panel.py").to_path_buf(),
        0,
    );
    assert_eq!(
        detector.get_removal_confidence(&event_func),
        RemovalConfidence::Framework
    );

    // Private method should have Safe confidence
    let mut private_func = FunctionMetrics::new(
        "Cls._helper".to_string(),
        Path::new("module.py").to_path_buf(),
        0,
    );
    private_func.visibility = None;
    assert_eq!(
        detector.get_removal_confidence(&private_func),
        RemovalConfidence::Safe
    );
}
