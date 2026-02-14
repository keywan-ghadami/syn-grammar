Architecture Decision Record (ADR): Universal Character and Byte Primitives
1. Title
Handling Character, Byte, and Whitespace Primitives across Token-Based and Character-Based Backends
2. Status
Accepted
3. Context
syn-grammar is evolving from a single-backend procedural macro parser (targeting syn) into a universal parser frontend (syn-grammar-model) capable of driving multiple backends, such as the character/byte-oriented winnow-grammar.
Upstream users require granular, low-level primitives like alpha, digit, oct_digit, any_byte, and whitespace. However, a fundamental impedance mismatch exists between the target domains:
 * Character/Byte Streams (winnow): Operate on raw &str or &[u8]. Every character and whitespace is physically present and can be consumed individually.
 * Token Streams (syn): Operate on proc_macro2::TokenStream. The Rust lexer has already destroyed all whitespace and grouped characters into atomic, indivisible tokens (Ident, LitInt, Punct).
We need a way to support these primitives in the EBNF DSL without tying the syntax to a specific backend, while maintaining executability in the native syn backend.
4. Decision
We will introduce a standardized, domain-agnostic set of built-in primitive names (eof, whitespace, alpha, digit, alphanumeric, hex_digit, oct_digit, any_byte).
 * AST Representation: These primitives will not get dedicated EBNF AST nodes (to keep syn-grammar-model's parse_atom lean). They will be parsed as standard ModelPattern::RuleCalls.
 * Validation: syn-grammar-model will expose a constant list of these standard built-ins (UNIVERSAL_CHAR_BUILTINS). Backends will validate RuleCalls against this list.
 * Backend-Specific Semantics: The responsibility of translating these primitives falls entirely to the backend's code generator:
   * Character-stream backends (winnow) will map these to consuming parsers (e.g., winnow::ascii::alpha1).
   * Token-stream backends (syn) will map these to Token Filters and Zero-Width Assertions (Lookarounds) acting upon the already lexed tokens.
5. Alternatives Considered & Rejected
 * Option A: Allow backends to inject proprietary types (e.g., winnow's oct_digit1).
   * Rejected because: It causes "Vendor Leaking." If a user writes oct_digit1 in their grammar, it is permanently locked to the winnow ecosystem. The DSL must remain 100% portable across backends.
 * Option B: Force the syn backend to parse character-by-character.
   * Rejected because: It is technically impossible without breaking macro hygiene and spans. proc_macro2 does not allow partial consumption of an Ident. Furthermore, the original whitespace is irrecoverably lost by the time syn receives the token stream.
 * Option C: Hardcode AST variants for every new primitive in syn-grammar-model.
   * Rejected because: It bloats the core parse_atom function with endless else if branches for specific keywords, making the model rigid. Reusing RuleCall delegates the semantic translation to the code generator, which is the correct architectural boundary.
6. Consequences & Implementation Directives
By adopting this architecture, the EBNF syntax remains clean and portable. The syn-grammar-macros (token backend) must implement the following specific behaviors to emulate character-level concepts:
6.1. The Whitespace Paradox (Span-Gap Detection)
Since whitespace does not exist as a token in syn, the whitespace primitive acts as a zero-width assertion. The syn backend will generate code that compares the Span of the previously parsed token with the Span of the upcoming token.
 * Implementation: If the tokens are not adjacent (i.e., span_end of Token A != span_start of Token B), a gap exists, implying whitespace or comments were present. The rule succeeds without consuming a token.
6.2. Token Filtering for Character Primitives
Instead of consuming single characters, the syn backend will apply string-validation filters to the next available token.
 * alpha / alphanumeric: Matches a syn::Ident. The generated action block converts the ident to a string and asserts .chars().all(...).
 * digit / hex_digit / oct_digit: Matches a syn::LitInt. Validates the numeric base (e.g., checking for 0x or 0o prefixes via base10_digits()).
 * any_byte: Matches a syn::LitByte. If applied repeatedly, the backend must generate complex logic to peek into a syn::LitByteStr and advance an internal cursor.
 * eof: Maps natively to input.is_empty().
