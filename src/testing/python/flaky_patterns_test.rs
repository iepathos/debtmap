#[cfg(test)]
mod tests {
    use super::super::flaky_patterns::FlakyPatternDetector;
    use super::super::{FlakinessType, TestIssueType};
    use rustpython_parser::ast;

    fn parse_function(code: &str) -> ast::StmtFunctionDef {
        let full_code = format!("def test_func():\n{}", 
            code.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"));
        let module: ast::Mod = rustpython_parser::parse(&full_code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse")
            .into();
        
        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            if let Some(ast::Stmt::FunctionDef(func)) = body.into_iter().next() {
                return func;
            }
        }
        panic!("Failed to extract function");
    }

    #[test]
    fn test_new_detector() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("pass");
        let issues = detector.analyze_test_function(&func);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_timing_dependency_sleep() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("time.sleep(1)\nassert something()");
        let issues = detector.analyze_test_function(&func);
        
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0].issue_type,
            TestIssueType::FlakyPattern(FlakinessType::TimingDependency)
        ));
    }

    #[test]
    fn test_timing_dependency_time_module() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("start = time.time()\ndo_something()\nend = time.time()\nassert end - start < 1");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::TimingDependency)
        )));
    }

    #[test]
    fn test_timing_dependency_datetime() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("now = datetime.now()\nassert now.hour == 12");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::TimingDependency)
        )));
    }

    #[test]
    fn test_timing_dependency_perf_counter() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("start = time.perf_counter()\nprocess()\nassert time.perf_counter() - start < 0.1");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::TimingDependency)
        )));
    }

    #[test]
    fn test_timing_in_nested_blocks() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
if condition:
    time.sleep(0.5)
    assert result()
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::TimingDependency)
        )));
    }

    #[test]
    fn test_random_usage_module() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("value = random.randint(1, 10)\nassert value > 0");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::RandomValues)
        )));
    }

    #[test]
    fn test_random_usage_secrets() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("token = secrets.token_hex()\nassert len(token) == 32");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::RandomValues)
        )));
    }

    #[test]
    fn test_random_usage_uuid() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("id = uuid4()\nassert str(id)");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::RandomValues)
        )));
    }

    #[test]
    fn test_random_in_nested_blocks() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
for i in range(10):
    value = random.choice([1, 2, 3])
    assert value in [1, 2, 3]
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::RandomValues)
        )));
    }

    #[test]
    fn test_external_dependency_requests() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("response = requests.get('http://api.example.com')\nassert response.status_code == 200");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ExternalDependency)
        )));
    }

    #[test]
    fn test_external_dependency_urllib() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("data = urllib.urlopen('http://example.com').read()\nassert data");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ExternalDependency)
        )));
    }

    #[test]
    fn test_external_dependency_httpx() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("response = httpx.post('http://api.example.com', json=data)\nassert response.json()");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ExternalDependency)
        )));
    }

    #[test]
    fn test_filesystem_dependency_open() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
with open('/tmp/test.txt', 'w') as f:
    f.write('test')
assert os.path.exists('/tmp/test.txt')
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency)
        )));
    }

    #[test]
    fn test_filesystem_dependency_os_operations() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("os.mkdir('/tmp/test_dir')\nassert os.path.isdir('/tmp/test_dir')");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency)
        )));
    }

    #[test]
    fn test_filesystem_dependency_shutil() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("shutil.rmtree('/tmp/old_dir')\nassert not os.path.exists('/tmp/old_dir')");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency)
        )));
    }

    #[test]
    fn test_filesystem_with_temp_directory() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
with tempfile.TemporaryDirectory() as tmpdir:
    path = os.path.join(tmpdir, 'test.txt')
    with open(path, 'w') as f:
        f.write('test')
    assert os.path.exists(path)
"#);
        let issues = detector.analyze_test_function(&func);
        
        // Should not flag as flaky when using temporary directory
        assert!(!issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency)
        )));
    }

    #[test]
    fn test_filesystem_with_named_temp_file() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
with tempfile.NamedTemporaryFile() as tmp:
    tmp.write(b'test data')
    tmp.flush()
    assert tmp.name
"#);
        let issues = detector.analyze_test_function(&func);
        
        // Should not flag as flaky when using temporary file
        assert!(!issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency)
        )));
    }

    #[test]
    fn test_network_dependency_socket() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
s = socket.socket()
s.connect(('example.com', 80))
s.send(b'GET / HTTP/1.0\r\n\r\n')
data = s.recv(1024)
assert data
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::NetworkDependency)
        )));
    }

    #[test]
    fn test_network_dependency_asyncio() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("await asyncio.connect('example.com', 80)\nassert True");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::NetworkDependency)
        )));
    }

    #[test]
    fn test_network_dependency_aiohttp() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function("async with aiohttp.ClientSession() as session:\n    response = await session.get('http://example.com')");
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::NetworkDependency)
        )));
    }

    #[test]
    fn test_threading_issue_thread() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
t = threading.Thread(target=worker)
t.start()
t.join()
assert result
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue)
        )));
    }

    #[test]
    fn test_threading_issue_process() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
p = multiprocessing.Process(target=worker)
p.start()
p.join()
assert p.exitcode == 0
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue)
        )));
    }

    #[test]
    fn test_threading_issue_pool() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
with concurrent.futures.ThreadPoolExecutor() as executor:
    result = executor.submit(worker, data)
    assert result.result()
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue)
        )));
    }

    #[test]
    fn test_threading_with_lock() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
lock = threading.Lock()
with lock:
    t = threading.Thread(target=worker)
    t.start()
    t.join()
    assert result
"#);
        let issues = detector.analyze_test_function(&func);
        
        // Should not flag as flaky when proper synchronization is used
        assert!(!issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue)
        )));
    }

    #[test]
    fn test_threading_with_semaphore() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
sem = threading.Semaphore(1)
with sem:
    t = threading.Thread(target=worker)
    t.start()
    t.join()
"#);
        let issues = detector.analyze_test_function(&func);
        
        // Should not flag as flaky with synchronization
        assert!(!issues.iter().any(|i| matches!(
            i.issue_type,
            TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue)
        )));
    }

    #[test]
    fn test_multiple_flaky_patterns() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
time.sleep(1)
value = random.randint(1, 10)
response = requests.get('http://api.example.com')
with open('/tmp/test.txt', 'w') as f:
    f.write(str(value))
assert response.status_code == 200
"#);
        let issues = detector.analyze_test_function(&func);
        
        // Should detect multiple flaky patterns
        assert!(issues.len() >= 4);
        
        let issue_types: Vec<_> = issues.iter().map(|i| &i.issue_type).collect();
        assert!(issue_types.iter().any(|t| matches!(t, TestIssueType::FlakyPattern(FlakinessType::TimingDependency))));
        assert!(issue_types.iter().any(|t| matches!(t, TestIssueType::FlakyPattern(FlakinessType::RandomValues))));
        assert!(issue_types.iter().any(|t| matches!(t, TestIssueType::FlakyPattern(FlakinessType::ExternalDependency))));
        assert!(issue_types.iter().any(|t| matches!(t, TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency))));
    }

    #[test]
    fn test_no_flaky_patterns() {
        let detector = FlakyPatternDetector::new();
        let func = parse_function(r#"
# Deterministic test with no external dependencies
x = 1
y = 2
result = x + y
assert result == 3
assert x < y
assert y > 0
"#);
        let issues = detector.analyze_test_function(&func);
        
        assert!(issues.is_empty());
    }
}