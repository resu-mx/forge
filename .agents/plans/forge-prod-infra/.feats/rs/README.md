# Rust Production Code

## Questions

- Should I duplicate 'relationships' that would be captured by graph relations, in the rust structs or just HQL?
- I'm trying to understand what data should be configured in Rust Structs vs as Metadata that exists primarily in the Database. 
  - Concerns:
    - Read/Write increase for DB stored vs Rust Struct (in-mem?)
    - in-mem data expansion for Rust Struct vs DB stored
- How do I optimize the structure of the Rust structs for
  - storage efficiency
  - read speed/efficiency
  - memory usage
  - graph-analytics/queries
  -
