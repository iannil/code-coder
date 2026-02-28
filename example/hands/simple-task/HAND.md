---
id: "simple-task"
name: "Simple Task Example"
version: "1.0.0"
schedule: "0 * * * * *"
agent: "general"
enabled: false

# No autonomy config - uses simple agent invocation
memory_path: "hands/simple-task/{date}.md"
params:
  greeting: "Hello"
---

# Simple Task Example

A basic hand that demonstrates the Hands system without autonomous mode.

This hand simply calls the general agent with the greeting parameter.

Use this as a template for simple, scheduled tasks.
