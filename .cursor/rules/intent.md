# 🧠 nano-cvm AI Behavior Rules

These rules guide Cursor's behavior when writing, editing, or explaining code in this project.

---

## ✳️ General Philosophy

- Use **idiomatic, safe Rust**
- Code should be **modular, testable, and explicit**
- Prioritize **readability and maintainability** over cleverness
- Use **pattern matching**, **enums**, and **expression trees**
- Avoid unnecessary `unsafe`, `unwrap()`, or `expect()` unless handled clearly
- Keep logic **side-effect free**, unless explicitly for `Emit`, IO, or test output

---

## 🧱 Virtual Machine Principles

- The VM is a **stack-based**, recursive interpreter for a cooperative DSL
- All execution happens via `VM::execute(&[Op])`
- Execution is **deterministic**, operates on an internal stack and memory store
- Op codes may be nested (e.g. `IfZero`, `Loop`)
- New opcodes must:
  - Derive `Debug`, `Serialize`, and `Deserialize`
  - Be handled via `match` in `VM::execute`
  - Be covered by **unit tests**

---

## 🧪 Testing Standards

- All new logic must have **at least one test**
- Tests should live in `#[cfg(test)] mod tests` within the same file
- Prefer descriptive test names like `test_loop_basic`, `test_if_zero_false`
- Use `assert_eq!`, `assert!`, or matching against `Err(...)`

---

## 🧾 JSON Program Expectations

- Support human-readable JSON scripts using:
  - `{"Push": 5.0}`, `{"Store": "x"}`, `{"Emit": "msg"}`
  - Nested ops: `{"IfZero": { "then": [...], "else_": [...] }}`
  - Looping: `{"Loop": { "count": 3, "body": [...] }}`
- JSON input must deserialize to `Vec<Op>`
- Programs must be executable via `cargo run`

---

## 🧠 Prompting Behavior

When adding features, Cursor should:
1. Ask if the opcode needs to support recursion, memory access, or nested blocks
2. Always add serde support (`Serialize`, `Deserialize`)
3. Suggest at least 2 test cases (including one edge case)
4. Offer example JSON usage for the new feature
5. Remind to update `program.json` with a demo
6. Use existing architectural patterns for consistency

---

## 🔥 Advanced

- Avoid speculative opcodes unless requested
- Do not introduce token systems, networking, or file I/O without explicit instruction
- Treat this VM as a potential backend for cooperative contracts or task automation
- Future features may include: vote logic, threshold evaluation, REPL interface, or inter-VM messaging

---

## 💬 Commit Practices (Guidance Only)

- Feature branches must follow `feature/xyz` format
- Commit messages should be:
  - Present-tense: "Add Loop opcode"
  - Descriptive, but under 80 chars
  - One feature per commit unless refactoring

