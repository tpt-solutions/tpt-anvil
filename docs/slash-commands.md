# Slash Commands

All commands work in the chat panel and can optionally operate on the current editor selection.

## `/generate`

Generate code from a natural-language description. Anvil uses your codebase context to produce idiomatic code that fits your style.

```
/generate a function that validates an email address using regex
/generate REST API endpoint for user login with JWT
/generate unit tests for the UserService class
```

**Output:** Code block that you can apply as a diff with one click.

## `/test`

Generate unit tests for the selected code. Select a function or class, then run `/test`.

```
/test
/test using pytest fixtures
/test with edge cases for null input
```

**Output:** Test file content ready to apply.

## `/explain`

Explain selected code in plain language. Great for understanding unfamiliar code or preparing documentation.

```
/explain
/explain focus on the concurrency model
/explain what security implications does this have?
```

**Output:** Plain text explanation.

## `/fix`

Diagnose and fix the selected code. Paste the error message for best results.

```
/fix
/fix TypeError: cannot read property 'length' of undefined
/fix this function has an off-by-one error
```

**Output:** Fixed code block with explanation of what was wrong.

## `/docs`

Generate docstrings and inline documentation for selected code.

```
/docs
/docs include parameter types and examples
/docs in JSDoc format
```

**Output:** Documentation-annotated code ready to apply.

## Keyboard Shortcuts (VS Code)

| Action | Shortcut |
|--------|---------|
| Open chat panel | `Ctrl+Shift+A` |
| Explain selection | Right-click → Anvil → Explain |
| Fix selection | Right-click → Anvil → Fix |

## Multi-turn Chat

After any command, you can continue the conversation:

```
/explain

You: what does the memoize decorator do?
Anvil: The memoize decorator caches return values...

You: can you show me an example without the decorator?
Anvil: Sure, here's the equivalent code...
```
