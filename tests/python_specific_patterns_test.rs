use debtmap::complexity::python_specific_patterns::{
    PythonComplexityWeights, PythonSpecificPatternDetector,
};
use rustpython_parser::{parse, Mode};

#[test]
fn test_generator_complexity() {
    let code = r#"
def simple_generator():
    yield 1

def multi_yield_generator():
    for i in range(10):
        if i % 2 == 0:
            yield i
        else:
            yield i * 2
    yield "done"

async def async_generator():
    for i in range(5):
        await some_async_call()
        yield i
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.generators.len(), 2); // regular generators only
    assert_eq!(patterns.generators[0].yield_count, 1);
    assert_eq!(patterns.generators[1].yield_count, 3);

    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 0.0);
}

#[test]
fn test_nested_comprehension_complexity() {
    let code = r#"
# Simple comprehension - low complexity
simple = [x for x in range(10)]

# Nested comprehension - higher complexity
nested = [[x*y for x in range(10)] for y in range(10)]

# Triple nested - exponentially higher
triple = [[[x*y*z for x in range(5)] for y in range(5)] for z in range(5)]

# With conditions
filtered = [x for x in range(100) if x % 2 == 0 if x > 10]
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.comprehensions.len(), 7); // All nested levels counted
    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 10.0); // Should be significant due to nesting
}

#[test]
fn test_decorator_stack_complexity() {
    let code = r#"
@property
def simple_property(self):
    return self._value

@property
@cached
@logged
def complex_property(self):
    return expensive_computation()

@dataclass
@frozen
class MyClass:
    value: int

def decorator_factory(param):
    def decorator(func):
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs)
        return wrapper
    return decorator

@decorator_factory("param")
@another_decorator
def decorated_function():
    pass
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.decorators.len(), 4);
    assert!(patterns.decorators[0].is_property);
    assert!(patterns.decorators[2].is_class_decorator);

    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 0.0);
}

#[test]
fn test_event_handler_detection() {
    let code = r#"
class EventHandler:
    def on_button_click(self, event):
        pass

    def handle_mouse_move(self, event):
        pass

    def process_key_event(self, event):
        pass

    def data_changed_handler(self, data):
        pass

    def window_resize_callback(self, size):
        pass

    def network_listener(self, packet):
        pass
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.event_handlers.len(), 6);
    assert!(patterns
        .event_handlers
        .iter()
        .all(|h| !h.handler_name.is_empty()));
}

#[test]
fn test_context_manager_nesting() {
    let code = r#"
with open('file1.txt') as f1:
    data = f1.read()

with open('file2.txt') as f2:
    with lock:
        with transaction:
            process(f2)

async with session:
    async with connection:
        await query()
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.context_managers.len(), 6);

    // Check nesting depths
    let max_depth = patterns
        .context_managers
        .iter()
        .map(|c| c.nesting_depth)
        .max()
        .unwrap_or(0);
    assert_eq!(max_depth, 3);

    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 5.0); // Should include nested context weight
}

#[test]
fn test_metaclass_and_inheritance() {
    let code = r#"
class Meta(type):
    def __new__(cls, name, bases, attrs):
        return super().__new__(cls, name, bases, attrs)

class Base:
    pass

class Mixin1:
    pass

class Mixin2:
    pass

class Complex(Base, Mixin1, Mixin2, metaclass=Meta):
    def __init__(self):
        pass

class DiamondA:
    pass

class DiamondB(DiamondA):
    pass

class DiamondC(DiamondA):
    pass

class DiamondD(DiamondB, DiamondC):
    pass
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.metaclasses.len(), 1);
    assert_eq!(patterns.metaclasses[0].class_name, "Complex");
    assert!(patterns.metaclasses[0].has_custom_init);

    // Check inheritance patterns
    let complex_inheritance = patterns
        .inheritance
        .iter()
        .find(|i| i.class_name == "Complex")
        .unwrap();
    assert_eq!(complex_inheritance.base_classes.len(), 3);
    assert_eq!(complex_inheritance.mixin_count, 2);

    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 10.0); // Metaclass + multiple inheritance should be high
}

#[test]
fn test_dynamic_access_patterns() {
    let code = r#"
def dynamic_operations():
    # Dynamic attribute access
    value = getattr(obj, 'attribute', default)
    setattr(obj, 'new_attr', 42)
    if hasattr(obj, 'method'):
        pass

    # Dangerous eval/exec
    result = eval('2 + 2')
    exec('x = 10')

    # Compile
    code_obj = compile('print("hello")', '<string>', 'exec')

class DynamicClass:
    def __getattribute__(self, name):
        return super().__getattribute__(name)
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);

    assert_eq!(patterns.dynamic_accesses.len(), 6);

    // Count dangerous operations
    let dangerous_count = patterns
        .dynamic_accesses
        .iter()
        .filter(|d| {
            matches!(
                d.access_type,
                debtmap::complexity::python_specific_patterns::DynamicAccessType::Exec
                    | debtmap::complexity::python_specific_patterns::DynamicAccessType::Eval
            )
        })
        .count();
    assert_eq!(dangerous_count, 2);

    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity > 15.0); // eval/exec should have high weight
}

#[test]
fn test_custom_weights() {
    let code = r#"
def generator():
    yield 1

exec('dangerous_code')
"#;

    let module = parse(code, Mode::Module, "<test>").unwrap();

    // Test with default weights
    let mut detector1 = PythonSpecificPatternDetector::new();
    detector1.detect_patterns(&module);
    let complexity1 = detector1.calculate_pattern_complexity();

    // Test with custom weights
    let mut custom_weights = PythonComplexityWeights::default();
    custom_weights.generator_weight = 10.0;
    custom_weights.exec_eval_weight = 20.0;

    let mut detector2 = PythonSpecificPatternDetector::new().with_weights(custom_weights);
    detector2.detect_patterns(&module);
    let complexity2 = detector2.calculate_pattern_complexity();

    assert!(complexity2 > complexity1);
    assert!(complexity2 >= 30.0); // generator (10) + yield (2) + exec (20)
}

#[test]
fn test_acceptance_criteria() {
    // Test spec requirements are met

    // Generator functions add +2 complexity per yield
    let gen_code = "def gen(): yield 1; yield 2";
    let module = parse(gen_code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    detector.detect_patterns(&module);
    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity >= 6.0); // generator (2) + 2 yields (2*2)

    // Decorator stacks add +1 per decorator beyond first
    let dec_code = r#"
@decorator1
@decorator2
@decorator3
def func(): pass
"#;
    let module = parse(dec_code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    let patterns = detector.detect_patterns(&module);
    assert_eq!(patterns.decorators[0].stack_depth, 3);

    // Metaclass usage adds +5 complexity
    let meta_code = "class C(metaclass=Meta): pass";
    let module = parse(meta_code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    detector.detect_patterns(&module);
    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity >= 5.0);

    // Multiple inheritance adds +3 per additional base
    let inherit_code = "class C(Base1, Base2, Base3): pass";
    let module = parse(inherit_code, Mode::Module, "<test>").unwrap();
    let mut detector = PythonSpecificPatternDetector::new();
    detector.detect_patterns(&module);
    let complexity = detector.calculate_pattern_complexity();
    assert!(complexity >= 6.0); // 2 additional bases * 3
}
