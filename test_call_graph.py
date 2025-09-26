#!/usr/bin/env python3
"""Test file to validate call graph extraction for spec 103."""

def main():
    """Entry point that calls other functions."""
    process_data()
    result = calculate_metrics()
    display_results(result)
    orphan_function()  # This calls orphan but main has no callers

def process_data():
    """Process data - called by main."""
    data = fetch_data()
    cleaned = clean_data(data)
    return cleaned

def fetch_data():
    """Fetch data - called by process_data."""
    return [1, 2, 3, 4, 5]

def clean_data(data):
    """Clean data - called by process_data."""
    return [x for x in data if x > 2]

def calculate_metrics():
    """Calculate metrics - called by main."""
    values = get_values()
    return sum(values) / len(values) if values else 0

def get_values():
    """Get values - called by calculate_metrics."""
    return [10, 20, 30, 40, 50]

def display_results(result):
    """Display results - called by main."""
    format_output(result)
    print(f"Result: {result}")

def format_output(value):
    """Format output - called by display_results."""
    print(f"Formatted: {value:.2f}")

def orphan_function():
    """Function with no callers - should be detected as dead code."""
    do_nothing()

def do_nothing():
    """Called only by orphan_function."""
    pass

def completely_unused():
    """Never called by anyone - clear dead code."""
    x = 1
    y = 2
    return x + y

class EventHandler:
    """Event handler class with implicit calls."""

    def on_click(self):
        """Event handler - implicitly called."""
        self.handle_click_event()

    def on_hover(self):
        """Event handler - implicitly called."""
        pass

    def handle_click_event(self):
        """Called by on_click."""
        pass

def test_example():
    """Test function - should not be flagged as dead code."""
    assert calculate_metrics() > 0

if __name__ == "__main__":
    main()