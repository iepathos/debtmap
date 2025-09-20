use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::analysis::{PythonTypeTracker, TwoPassExtractor};
use std::hint::black_box;
use std::path::PathBuf;

fn generate_large_python_module(num_classes: usize, methods_per_class: usize) -> String {
    let mut code = String::new();

    // Generate classes with methods
    for i in 0..num_classes {
        code.push_str(&format!("class Class{}:\n", i));
        code.push_str("    def __init__(self):\n");
        code.push_str("        self.value = 0\n\n");

        for j in 0..methods_per_class {
            code.push_str(&format!("    def method_{}(self, x):\n", j));
            code.push_str("        self.value += x\n");
            code.push_str("        return self.value\n\n");
        }
    }

    // Generate function that uses all classes
    code.push_str("def process_all():\n");
    for i in 0..num_classes {
        code.push_str(&format!("    obj_{} = Class{}()\n", i, i));
        for j in 0..methods_per_class {
            code.push_str(&format!("    obj_{}.method_{}({})\n", i, j, j));
        }
    }

    code
}

fn benchmark_type_inference(c: &mut Criterion) {
    let small_code = generate_large_python_module(5, 3);
    let medium_code = generate_large_python_module(20, 5);
    let large_code = generate_large_python_module(50, 10);

    let small_module =
        rustpython_parser::parse(&small_code, rustpython_parser::Mode::Module, "<bench>").unwrap();
    let medium_module =
        rustpython_parser::parse(&medium_code, rustpython_parser::Mode::Module, "<bench>").unwrap();
    let large_module =
        rustpython_parser::parse(&large_code, rustpython_parser::Mode::Module, "<bench>").unwrap();

    c.bench_function("type_inference_small", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("bench.py");
            let tracker = PythonTypeTracker::new(file_path);
            black_box(tracker);
        })
    });

    c.bench_function("two_pass_extraction_small", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("bench.py");
            let mut extractor = TwoPassExtractor::new(file_path);
            let _graph = extractor.extract(&small_module);
        })
    });

    c.bench_function("two_pass_extraction_medium", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("bench.py");
            let mut extractor = TwoPassExtractor::new(file_path);
            let _graph = extractor.extract(&medium_module);
        })
    });

    c.bench_function("two_pass_extraction_large", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("bench.py");
            let mut extractor = TwoPassExtractor::new(file_path);
            let _graph = extractor.extract(&large_module);
        })
    });
}

fn benchmark_type_tracking_with_inheritance(c: &mut Criterion) {
    let inheritance_code = r#"
class Animal:
    def make_sound(self):
        pass
    def move(self):
        pass

class Mammal(Animal):
    def feed_young(self):
        pass

class Bird(Animal):
    def fly(self):
        pass

class Dog(Mammal):
    def make_sound(self):
        return "Woof"
    def fetch(self):
        pass

class Cat(Mammal):
    def make_sound(self):
        return "Meow"
    def scratch(self):
        pass

class Eagle(Bird):
    def make_sound(self):
        return "Screech"
    def hunt(self):
        pass

class Parrot(Bird):
    def make_sound(self):
        return "Squawk"
    def mimic(self):
        pass

def zoo_operations():
    dog = Dog()
    cat = Cat()
    eagle = Eagle()
    parrot = Parrot()

    animals = [dog, cat, eagle, parrot]
    for animal in animals:
        animal.make_sound()
        animal.move()

    mammals = [dog, cat]
    for mammal in mammals:
        mammal.feed_young()

    birds = [eagle, parrot]
    for bird in birds:
        bird.fly()

    dog.fetch()
    cat.scratch()
    eagle.hunt()
    parrot.mimic()
"#;

    let module =
        rustpython_parser::parse(inheritance_code, rustpython_parser::Mode::Module, "<bench>")
            .unwrap();

    c.bench_function("type_tracking_inheritance", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("inheritance_bench.py");
            let mut extractor = TwoPassExtractor::new(file_path);
            let _graph = extractor.extract(&module);
        })
    });
}

fn benchmark_type_resolution(c: &mut Criterion) {
    let complex_code = r#"
from typing import List, Dict, Optional

class DataProcessor:
    def __init__(self):
        self.data: List[int] = []
        self.cache: Dict[str, int] = {}

    def process(self, value: int) -> int:
        result = value * 2
        self.data.append(result)
        return result

    def get_cached(self, key: str) -> Optional[int]:
        return self.cache.get(key)

    def set_cached(self, key: str, value: int):
        self.cache[key] = value

class AdvancedProcessor(DataProcessor):
    def process(self, value: int) -> int:
        result = super().process(value)
        return result ** 2

    def analyze(self) -> float:
        if not self.data:
            return 0.0
        return sum(self.data) / len(self.data)

def complex_workflow():
    processors = []
    for i in range(10):
        if i % 2 == 0:
            proc = DataProcessor()
        else:
            proc = AdvancedProcessor()
        processors.append(proc)

    for i, proc in enumerate(processors):
        value = proc.process(i)
        proc.set_cached(f"key_{i}", value)

        if isinstance(proc, AdvancedProcessor):
            avg = proc.analyze()
            print(f"Average: {avg}")

        cached = proc.get_cached(f"key_{i}")
        if cached:
            print(f"Cached: {cached}")
"#;

    let module =
        rustpython_parser::parse(complex_code, rustpython_parser::Mode::Module, "<bench>").unwrap();

    c.bench_function("complex_type_resolution", |b| {
        b.iter(|| {
            let file_path = PathBuf::from("complex_bench.py");
            let mut extractor = TwoPassExtractor::new(file_path);
            let _graph = extractor.extract(&module);
        })
    });
}

criterion_group!(
    benches,
    benchmark_type_inference,
    benchmark_type_tracking_with_inheritance,
    benchmark_type_resolution
);
criterion_main!(benches);
