# Direct usage

These calls exercise distinct source sites in an official documentation example.

```python
def first(value: int) -> int:
    return value


def second(value: int) -> int:
    return value


first(1)
first(2)
second(3)
alias = first
alias(4)
```
