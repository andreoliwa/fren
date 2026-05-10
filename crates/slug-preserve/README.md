# slug-preserve

Case-preserving slugifier with Unicode support.

Unlike most Rust slug crates (including the popular [`slug`](https://crates.io/crates/slug) crate) which always
lowercase, `slug-preserve` lets you choose how case is handled via `CaseMode`.

## Usage

```rust
use slug_preserve::{slugify, SlugOpts, CaseMode};

// Preserve original case (default)
let opts = SlugOpts::default();
assert_eq!(slugify("Hello World", &opts), "Hello-World");

// Lowercase
let opts = SlugOpts { separator: '_', case: CaseMode::Lower, ..Default::default() };
assert_eq!(slugify("Hello World", &opts), "hello_world");

// Title case
let opts = SlugOpts { case: CaseMode::Title, ..Default::default() };
assert_eq!(slugify("hello world", &opts), "Hello-World");
```

## Case modes

| `CaseMode`   | Effect                                    |
| ------------ | ----------------------------------------- |
| `Preserve`   | Keep original character case (default)    |
| `Lower`      | Lowercase everything                      |
| `Upper`      | Uppercase everything                      |
| `Title`      | First char of each word upper, rest lower |
| `Capitalize` | Alias for `Title`                         |

## Notes

- Input is NFKC-normalized before slugifying.
- Non-alphanumeric characters become the separator; runs collapse to one.
- Leading/trailing separators are trimmed from output.
- `split_camel` on `SlugOpts` is carried through but not acted on by this
  crate - callers (like [`fren-date`](https://crates.io/crates/fren-date)) split CamelCase before calling `slugify`.

## License

MIT - see [LICENSE](../../LICENSE) in the workspace root.
