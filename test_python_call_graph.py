#!/usr/bin/env python3
"""Comprehensive test file for Python call graph extraction.

This file tests all supported call patterns to ensure the call graph
extractor correctly identifies function calls and method invocations.
"""


# Simple function calls
def helper():
    """Helper function that gets called."""
    print("Helper called")


def main():
    """Main function that calls helper."""
    helper()  # Direct function call
    print("Main executed")


# Method calls within classes
class Calculator:
    """Test class for method calls."""

    def __init__(self):
        """Initialize calculator."""
        self.value = 0
        self.reset()  # Call another method

    def reset(self):
        """Reset calculator value."""
        self.value = 0

    def add(self, x):
        """Add a value."""
        self.value += x
        self.log_operation("add", x)  # Call internal method

    def log_operation(self, operation, value):
        """Log an operation."""
        print(f"{operation}: {value}")

    @staticmethod
    def static_method():
        """Static method example."""
        print("Static method called")

    @classmethod
    def class_method(cls):
        """Class method example."""
        cls.static_method()  # Call static method from class method


# Cross-class method calls
class UserService:
    """Service for user operations."""

    def __init__(self):
        """Initialize user service."""
        self.db = DatabaseConnection()

    def create_user(self, name):
        """Create a new user."""
        self.db.connect()  # Cross-class method call
        self.db.save({"name": name})
        self.db.disconnect()


class DatabaseConnection:
    """Mock database connection."""

    def connect(self):
        """Connect to database."""
        print("Connected to database")

    def save(self, data):
        """Save data to database."""
        print(f"Saving: {data}")

    def disconnect(self):
        """Disconnect from database."""
        print("Disconnected from database")


# Nested functions
def outer_function():
    """Outer function with nested function."""

    def inner_function():
        """Inner nested function."""
        print("Inner function called")

    inner_function()  # Call nested function
    return inner_function


# Lambda and comprehensions
def process_list(items):
    """Process a list with lambdas and comprehensions."""
    # Lambda expression
    transform = lambda x: x * 2

    # List comprehension with function call
    result = [transform(x) for x in items]

    # Direct lambda call
    filtered = filter(lambda x: x > 5, result)

    return list(filtered)


# Callback patterns
def execute_callback(callback, value):
    """Execute a callback function."""
    return callback(value)


def my_callback(value):
    """Example callback function."""
    return value * 10


def test_callbacks():
    """Test callback patterns."""
    result = execute_callback(my_callback, 5)  # Pass function as argument
    print(f"Callback result: {result}")


# Decorator patterns
def log_decorator(func):
    """Decorator that logs function calls."""

    def wrapper(*args, **kwargs):
        print(f"Calling {func.__name__}")
        return func(*args, **kwargs)

    return wrapper


@log_decorator
def decorated_function():
    """Function with decorator."""
    print("Decorated function executed")


# Dynamic calls
def dynamic_call_example():
    """Example of dynamic function calls."""
    calc = Calculator()

    # getattr for dynamic method access
    method = getattr(calc, "add")
    method(10)

    # Direct dynamic call
    operation = "reset"
    getattr(calc, operation)()


# Entry point
if __name__ == "__main__":
    # Test all patterns
    main()

    # Test class methods
    calc = Calculator()
    calc.add(5)
    Calculator.static_method()
    Calculator.class_method()

    # Test cross-class calls
    user_service = UserService()
    user_service.create_user("Alice")

    # Test nested functions
    func = outer_function()

    # Test callbacks
    test_callbacks()

    # Test decorated function
    decorated_function()

    # Test dynamic calls
    dynamic_call_example()

    # Test lambda and comprehensions
    result = process_list([1, 2, 3, 4, 5])
    print(f"Processed list: {result}")