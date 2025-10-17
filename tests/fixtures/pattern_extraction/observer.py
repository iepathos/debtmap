"""
Observer pattern implementation with decorators, inheritance, and singletons.
"""
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import List


# Abstract base class with inheritance
class Observer(ABC):
    """Abstract observer interface."""

    @abstractmethod
    def on_event(self, event: str) -> None:
        """Handle event notification."""
        pass


class Subject(ABC):
    """Abstract subject that notifies observers."""

    def __init__(self):
        self._observers: List[Observer] = []

    def attach(self, observer: Observer) -> None:
        """Attach an observer."""
        if observer not in self._observers:
            self._observers.append(observer)

    def detach(self, observer: Observer) -> None:
        """Detach an observer."""
        self._observers.remove(observer)

    @abstractmethod
    def notify(self) -> None:
        """Notify all observers."""
        pass


# Concrete implementation with decorator
@dataclass
class ConcreteObserver(Observer):
    """Concrete observer with dataclass decorator."""
    name: str

    def on_event(self, event: str) -> None:
        """Handle event notification."""
        print(f"{self.name} received event: {event}")


class EventManager(Subject):
    """Concrete subject managing events."""

    def __init__(self):
        super().__init__()
        self._event_queue: List[str] = []

    def add_event(self, event: str) -> None:
        """Add event to queue."""
        self._event_queue.append(event)
        self.notify()

    def notify(self) -> None:
        """Notify all observers of events."""
        for event in self._event_queue:
            for observer in self._observers:
                observer.on_event(event)
        self._event_queue.clear()


# Module-level singleton instance
event_manager = EventManager()


# Factory function
def create_observer(name: str) -> ConcreteObserver:
    """Create and return a new observer."""
    return ConcreteObserver(name=name)


# Configuration object (another singleton pattern)
class Configuration:
    """Application configuration."""

    def __init__(self):
        self.debug_mode = False
        self.max_observers = 100


config = Configuration()
