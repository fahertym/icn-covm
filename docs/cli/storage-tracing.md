# Storage Tracing

The ICN Cooperative VM includes a feature to trace storage operations in detail. This is useful for debugging programs that use persistent storage to understand what data is being stored and loaded.

## Enabling Storage Tracing

Use the `--verbose-storage-trace` flag with the `run` command:

```bash
icn-covm run --program your_program.dsl --verbose-storage-trace
```

## What Gets Traced

When enabled, this feature will log details about:

1. `StoreP` operations - storing values to persistent storage
2. `LoadP` operations - loading values from persistent storage

Each operation will be shown in the console output with the format:

```
[STORAGE] <operation> key: '<key>', value: <value>
```

For example:

```
[STORAGE] StoreP key: 'counter', value: 42
[STORAGE] LoadP key: 'counter', value: 42
```

## Combining with Other Tracing

You can combine this flag with other diagnostic flags:

```bash
icn-covm run --program your_program.dsl --trace --explain --verbose-storage-trace
```

This will show:
- Execution tracing (with `--trace`)
- Operation explanations (with `--explain`)
- Detailed storage operations (with `--verbose-storage-trace`)

## Interactive Mode

In interactive mode, you can toggle storage tracing on and off with these commands:

```
storage-trace on   # Enable storage tracing
storage-trace off  # Disable storage tracing
```

## Use Cases

This feature is particularly useful when:

1. Debugging programs that use persistent storage
2. Verifying that storage operations are occurring as expected
3. Teaching how the VM's storage system works
4. Understanding the data flow in complex programs 