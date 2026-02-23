# ADR 10: Typed Numeric Patterns with Static Width Guarantees

## 1. Context

The current implementation of numeric primitives (`hex_literal`, `oct_literal`, `bin_literal`) is problematic for several reasons:

1.  **Redundancy:** They are effectively aliases for standard Rust types (e.g., `u64`) and use `syn::LitInt` internally. This means `hex_literal` accepts decimal `123` or binary `0b101`, which contradicts the name.
2.  **Ambiguity:** There is no distinction between "Token Parsing" (parsing a valid Rust literal like `0xFF_u8`) and "Pattern Matching" (parsing a raw sequence of hex digits like `FF`).
3.  **Safety:** Parsing arbitrary digit sequences into fixed-size integers (`u32`, `u64`) creates risks of runtime overflows or requires awkward manual checking.
4.  **Type Safety:** The current system does not leverage the grammar's knowledge of constraints (e.g., "max 4 digits") to assist the user in choosing the correct data type in their Rust code.

## 2. Decision

We will replace the existing "literal" primitives with a new set of **Pattern-Based Numeric Primitives** that leverage Rust's type system (Const Generics) to enforce safety constraints defined in the grammar.

### 2.1. Remove Old Primitives
The following primitives will be removed:
*   `hex_literal`
*   `oct_literal`
*   `bin_literal`

*Note: Standard Rust types (`u8`, `i32`, `f64`, etc.) remain available for parsing standard Rust literals.*

### 2.2. Introduce Pattern Primitives
We introduce four new primitives that match raw digit sequences (without Rust-specific prefixes like `0x` or suffixes):

*   `hex(min, max)`: Matches `[0-9a-fA-F]`
*   `oct(min, max)`: Matches `[0-7]`
*   `bin(min, max)`: Matches `[0-1]`
*   `dec(min, max)`: Matches `[0-9]`

### 2.3. Return Types with Static Guarantees
These primitives will return custom wrapper types parameterized by their maximum length (`MAX`).

*   `hex` -> `HexValue<MAX>`
*   `oct` -> `OctValue<MAX>`
*   `bin` -> `BinValue<MAX>`
*   `dec` -> `DecValue<MAX>`

### 2.4. The "Guaranteed Fit" API
The wrapper types will strictly limit available conversion methods based on the `MAX` constant. If a value is too large for a type, the conversion method **will not exist** on the struct, preventing compilation.

**Conceptual API:**

```rust
pub struct HexValue<const MAX: usize> {
    raw: String,
    span: Span,
}

impl<const MAX: usize> HexValue<MAX> {
    /// Always available: access the raw string
    pub fn as_str(&self) -> &str { &self.raw }
}

// Methods available ONLY if they are guaranteed to fit
impl<const MAX: usize> HexValue<MAX> 
where Condition: FitsInU8 
{
    pub fn u8(&self) -> u8 { ... }
}

// ... similar for u16, u32, u64
```

**Examples:**
*   `hex(max=2)` -> Returns `HexValue<2>`. Has `.u8()`.
*   `hex(max=4)` -> Returns `HexValue<4>`. Has `.u16()`.
*   `hex(max=8)` -> Returns `HexValue<8>`. Has `.u32()`.
*   `hex(max=5000)` -> Returns `HexValue<5000>`. Has **only** `.as_str()`.

## 3. Technical Implementation

1.  **Model:** Add `HexValue`, `OctValue`, `BinValue`, `DecValue` to `syn-grammar-model/src/model/types.rs`. Use `const generics` to store the `MAX` width.
2.  **Traits:** Implement sealed traits or specific `impl` blocks to define which `MAX` values support which integer conversions.
    *   *Constraint:* Since `const_generic_exprs` is unstable, we may need to implement this via blanket implementations for specific ranges or discrete explicit implementations for common sizes.
3.  **Codegen:** Update `syn-grammar-macros` to extract the `max` argument from the rule definition and inject it into the return type generic parameter (e.g., generating `HexValue<4>` instead of just `HexValue`).
4.  **Backend:** Update `BuiltIn` definitions to reflect that these types are generic.

## 4. Consequences

### Positive
*   **Compile-Time Safety:** Overflows due to size mismatches are caught by the compiler (method not found).
*   **Clarity:** The grammar explicitly defines the data shape, and the code reflects that reality.
*   **Flexibility:** Users can handle arbitrarily large numbers (keys, hashes) as strings without forced conversion penalties.

### Negative
*   **Breaking Change:** Existing grammars using `*_literal` must be updated to use standard types (e.g., `u64`) or the new patterns.
*   **Complexity:** The backend/codegen logic becomes slightly more complex to handle the transfer of the `max` argument into the Type System.

## 5. Alternatives Considered

*   **Runtime Result:** Returning a type that has a `.to::<T>() -> Result<T>` method.
    *   *Rejected:* Delays errors to runtime. The user explicitly requested preventing users from even trying to cast `hex(max=5000)` to a standard integer.
*   **BigInt Dependency:** Returning a `BigUint` for all values.
    *   *Rejected:* Too heavy for a parser library; we want to remain lightweight.
