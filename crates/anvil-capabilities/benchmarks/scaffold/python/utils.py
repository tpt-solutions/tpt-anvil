def sum_list(numbers):
    total = 0
    for n in numbers:
        total += n
    return total


def is_palindrome(s):
    return s == s[::-1]


def fibonacci(n):
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b
