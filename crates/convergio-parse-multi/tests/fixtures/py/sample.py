"""Module docstring for sample.py."""


def greet(name: str) -> str:
    """Return a greeting for *name*."""
    return f"Hello, {name}!"


class Animal:
    """Base class for all animals."""

    def __init__(self, species: str) -> None:
        """Initialise with *species*."""
        self.species = species

    def speak(self) -> str:
        """Return the animal's sound."""
        return ""


class Dog(Animal):
    def bark(self) -> str:
        return "Woof!"


@staticmethod
def standalone() -> None:
    pass
