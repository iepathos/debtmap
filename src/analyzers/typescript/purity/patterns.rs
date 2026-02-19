//! I/O and mutation pattern constants for TypeScript/JavaScript purity detection
//!
//! These patterns identify operations that indicate impurity in JavaScript code.

/// Browser I/O globals and functions
pub const BROWSER_IO_GLOBALS: &[&str] = &[
    "console",
    "alert",
    "confirm",
    "prompt",
    "fetch",
    "XMLHttpRequest",
    "WebSocket",
    "EventSource",
    "document",
    "window",
    "navigator",
    "location",
    "history",
    "localStorage",
    "sessionStorage",
    "indexedDB",
    "caches",
    "performance",
    "crypto",
];

/// Browser scheduling/timing functions (impure)
pub const BROWSER_TIMING: &[&str] = &[
    "setTimeout",
    "setInterval",
    "setImmediate",
    "requestAnimationFrame",
    "requestIdleCallback",
    "queueMicrotask",
];

/// Node.js I/O modules
pub const NODE_IO_MODULES: &[&str] = &[
    "fs",
    "fs/promises",
    "http",
    "https",
    "http2",
    "net",
    "dgram",
    "dns",
    "child_process",
    "cluster",
    "worker_threads",
    "stream",
    "readline",
    "tty",
];

/// Node.js globals that indicate I/O or environment access
pub const NODE_IO_GLOBALS: &[&str] = &[
    "process",
    "Buffer",
    "__dirname",
    "__filename",
    "require",
    "module",
    "exports",
    "global",
];

/// DOM mutation methods
pub const DOM_MUTATIONS: &[&str] = &[
    "appendChild",
    "removeChild",
    "insertBefore",
    "replaceChild",
    "replaceWith",
    "remove",
    "prepend",
    "append",
    "before",
    "after",
    "setAttribute",
    "removeAttribute",
    "toggleAttribute",
    "setAttributeNS",
    "removeAttributeNS",
    "setProperty",
    "removeProperty",
    "insertAdjacentHTML",
    "insertAdjacentElement",
    "insertAdjacentText",
    "focus",
    "blur",
    "click",
    "submit",
    "reset",
];

/// DOM mutation properties (assignment to these is impure)
pub const DOM_MUTATION_PROPERTIES: &[&str] = &[
    "innerHTML",
    "outerHTML",
    "innerText",
    "outerText",
    "textContent",
    "value",
    "checked",
    "selected",
    "disabled",
    "className",
    "classList",
    "style",
    "href",
    "src",
    "id",
];

/// Collection mutation methods (modifying in place)
pub const COLLECTION_MUTATIONS: &[&str] = &[
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
    "set",    // TypedArray.set
    "clear",  // Map/Set
    "delete", // Map/Set/WeakMap/WeakSet
    "add",    // Set/WeakSet
];

/// Object mutation methods
pub const OBJECT_MUTATIONS: &[&str] = &[
    "defineProperty",
    "defineProperties",
    "freeze",
    "seal",
    "preventExtensions",
    "setPrototypeOf",
    "assign",
];

/// Known pure functions (these don't indicate impurity)
pub const KNOWN_PURE_METHODS: &[&str] = &[
    // Array non-mutating methods
    "map",
    "filter",
    "reduce",
    "reduceRight",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "some",
    "every",
    "includes",
    "indexOf",
    "lastIndexOf",
    "slice",
    "concat",
    "flat",
    "flatMap",
    "join",
    "entries",
    "keys",
    "values",
    "at",
    "with",
    "toReversed",
    "toSorted",
    "toSpliced",
    // String methods
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "trim",
    "trimStart",
    "trimEnd",
    "padStart",
    "padEnd",
    "repeat",
    "split",
    "substring",
    "substr",
    "slice",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "normalize",
    "localeCompare",
    "startsWith",
    "endsWith",
    "includes",
    "match",
    "matchAll",
    "replace",
    "replaceAll",
    "search",
    // Number methods
    "toFixed",
    "toPrecision",
    "toExponential",
    "toString",
    "valueOf",
    // Object methods (non-mutating)
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
];

/// Known pure global functions
pub const KNOWN_PURE_GLOBALS: &[&str] = &[
    // Math
    "Math.abs",
    "Math.acos",
    "Math.acosh",
    "Math.asin",
    "Math.asinh",
    "Math.atan",
    "Math.atan2",
    "Math.atanh",
    "Math.cbrt",
    "Math.ceil",
    "Math.clz32",
    "Math.cos",
    "Math.cosh",
    "Math.exp",
    "Math.expm1",
    "Math.floor",
    "Math.fround",
    "Math.hypot",
    "Math.imul",
    "Math.log",
    "Math.log10",
    "Math.log1p",
    "Math.log2",
    "Math.max",
    "Math.min",
    "Math.pow",
    "Math.round",
    "Math.sign",
    "Math.sin",
    "Math.sinh",
    "Math.sqrt",
    "Math.tan",
    "Math.tanh",
    "Math.trunc",
    // Number
    "Number",
    "Number.isFinite",
    "Number.isInteger",
    "Number.isNaN",
    "Number.isSafeInteger",
    "Number.parseFloat",
    "Number.parseInt",
    // String
    "String",
    "String.fromCharCode",
    "String.fromCodePoint",
    "String.raw",
    // Array (non-mutating)
    "Array.isArray",
    "Array.from",
    "Array.of",
    // Object (non-mutating)
    "Object.keys",
    "Object.values",
    "Object.entries",
    "Object.fromEntries",
    "Object.hasOwn",
    "Object.is",
    "Object.getOwnPropertyNames",
    "Object.getOwnPropertySymbols",
    "Object.getOwnPropertyDescriptor",
    "Object.getOwnPropertyDescriptors",
    "Object.getPrototypeOf",
    "Object.isExtensible",
    "Object.isFrozen",
    "Object.isSealed",
    // JSON
    "JSON.parse",
    "JSON.stringify",
    // Parsing
    "parseInt",
    "parseFloat",
    "encodeURI",
    "encodeURIComponent",
    "decodeURI",
    "decodeURIComponent",
    "isFinite",
    "isNaN",
    // Boolean
    "Boolean",
    // Symbol
    "Symbol",
    "Symbol.for",
    "Symbol.keyFor",
];

/// Dynamic evaluation functions (dangerous, always impure)
pub const DYNAMIC_EVAL: &[&str] = &["eval", "Function"];

/// Non-deterministic functions (impure due to non-reproducible output)
pub const NON_DETERMINISTIC_METHODS: &[&str] = &[
    "random",          // Math.random()
    "now",             // Date.now(), performance.now()
    "getTime",         // date.getTime()
    "getRandomValues", // crypto.getRandomValues()
    "randomUUID",      // crypto.randomUUID()
];

/// Non-deterministic global calls
pub const NON_DETERMINISTIC_GLOBALS: &[&str] = &[
    "Math.random",
    "Date.now",
    "performance.now",
    "crypto.getRandomValues",
    "crypto.randomUUID",
];

/// Date constructor creates time-dependent values
pub const TIME_DEPENDENT_CONSTRUCTORS: &[&str] = &["Date"];

/// Check if a method name is a known pure method
pub fn is_known_pure_method(method: &str) -> bool {
    KNOWN_PURE_METHODS.contains(&method)
}

/// Check if a global call is known to be pure
pub fn is_known_pure_global(global: &str) -> bool {
    KNOWN_PURE_GLOBALS.contains(&global)
}

/// Check if an identifier is a browser I/O global
pub fn is_browser_io_global(name: &str) -> bool {
    BROWSER_IO_GLOBALS.contains(&name) || BROWSER_TIMING.contains(&name)
}

/// Check if an identifier is a Node.js I/O module/global
pub fn is_node_io(name: &str) -> bool {
    NODE_IO_MODULES.contains(&name) || NODE_IO_GLOBALS.contains(&name)
}

/// Check if a method is a collection mutation
pub fn is_collection_mutation(method: &str) -> bool {
    COLLECTION_MUTATIONS.contains(&method)
}

/// Check if a method is a DOM mutation
pub fn is_dom_mutation(method: &str) -> bool {
    DOM_MUTATIONS.contains(&method)
}

/// Check if a property is a DOM mutation property
pub fn is_dom_mutation_property(prop: &str) -> bool {
    DOM_MUTATION_PROPERTIES.contains(&prop)
}

/// Check if a function is dynamic evaluation
pub fn is_dynamic_eval(name: &str) -> bool {
    DYNAMIC_EVAL.contains(&name)
}

/// Check if a method is an Object mutation method
pub fn is_object_mutation(method: &str) -> bool {
    OBJECT_MUTATIONS.contains(&method)
}

/// Check if a method is non-deterministic (Math.random, Date.now, etc.)
pub fn is_non_deterministic_method(method: &str) -> bool {
    NON_DETERMINISTIC_METHODS.contains(&method)
}

/// Check if a global call is non-deterministic
pub fn is_non_deterministic_global(global: &str) -> bool {
    NON_DETERMINISTIC_GLOBALS.contains(&global)
}

/// Check if a constructor creates time-dependent values
pub fn is_time_dependent_constructor(name: &str) -> bool {
    TIME_DEPENDENT_CONSTRUCTORS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_known_pure_method() {
        assert!(is_known_pure_method("map"));
        assert!(is_known_pure_method("filter"));
        assert!(is_known_pure_method("reduce"));
        assert!(!is_known_pure_method("push"));
        assert!(!is_known_pure_method("splice"));
    }

    #[test]
    fn test_is_non_deterministic_method() {
        assert!(is_non_deterministic_method("random"));
        assert!(is_non_deterministic_method("now"));
        assert!(is_non_deterministic_method("getRandomValues"));
        assert!(!is_non_deterministic_method("map"));
        assert!(!is_non_deterministic_method("sqrt"));
    }

    #[test]
    fn test_is_non_deterministic_global() {
        assert!(is_non_deterministic_global("Math.random"));
        assert!(is_non_deterministic_global("Date.now"));
        assert!(is_non_deterministic_global("performance.now"));
        assert!(!is_non_deterministic_global("Math.sqrt"));
        assert!(!is_non_deterministic_global("Array.isArray"));
    }

    #[test]
    fn test_is_time_dependent_constructor() {
        assert!(is_time_dependent_constructor("Date"));
        assert!(!is_time_dependent_constructor("Array"));
        assert!(!is_time_dependent_constructor("Object"));
    }

    #[test]
    fn test_is_browser_io_global() {
        assert!(is_browser_io_global("console"));
        assert!(is_browser_io_global("fetch"));
        assert!(is_browser_io_global("setTimeout"));
        assert!(!is_browser_io_global("Math"));
        assert!(!is_browser_io_global("Array"));
    }

    #[test]
    fn test_is_node_io() {
        assert!(is_node_io("fs"));
        assert!(is_node_io("process"));
        assert!(is_node_io("require"));
        assert!(!is_node_io("Array"));
        assert!(!is_node_io("Object"));
    }

    #[test]
    fn test_is_collection_mutation() {
        assert!(is_collection_mutation("push"));
        assert!(is_collection_mutation("pop"));
        assert!(is_collection_mutation("splice"));
        assert!(!is_collection_mutation("map"));
        assert!(!is_collection_mutation("filter"));
    }

    #[test]
    fn test_is_dom_mutation() {
        assert!(is_dom_mutation("appendChild"));
        assert!(is_dom_mutation("removeChild"));
        assert!(is_dom_mutation("setAttribute"));
        assert!(!is_dom_mutation("getAttribute"));
        assert!(!is_dom_mutation("querySelector"));
    }

    #[test]
    fn test_is_dynamic_eval() {
        assert!(is_dynamic_eval("eval"));
        assert!(is_dynamic_eval("Function"));
        assert!(!is_dynamic_eval("map"));
        assert!(!is_dynamic_eval("console"));
    }
}
