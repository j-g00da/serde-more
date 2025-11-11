# serde_more

A Rust procedural macro to add arbitrary computed fields when serializing structs with [serde](https://serde.rs/).

[![Crate](https://img.shields.io/crates/v/serde-more?logo=rust&style=flat-square)](https://crates.io/crates/serde-more)
[![Docs](https://img.shields.io/docsrs/serde-more?logo=rust&style=flat-square)](https://docs.rs/serde-more)
[![CI](https://img.shields.io/github/actions/workflow/status/j-g00da/serde-more/ci.yml?style=flat-square&logo=github)](https://github.com/j-g00da/serde-more/blob/main/.github/workflows/ci.yml)
[![Deps](https://deps.rs/crate/serde_more/latest/status.svg?style=flat-square)](https://deps.rs/crate/serde_more)

## Examples

### Basic Usage

```rust
use serde_more::SerializeMore;
use serde_json::json;

#[derive(SerializeMore)]
#[more(key="next")]
#[more(key="previous", position="front")]
struct Index {
    current: u32,
}

impl Index {
    fn next(&self) -> u32 {
        self.current.saturating_add(1)
    }
    fn previous(&self) -> u32 {
        self.current.saturating_sub(1)
    }
}

fn main() {
    let idx = Index { current: 5 };
    let value = serde_json::to_value(&idx).unwrap();
    assert_eq!(value, json!({
        "previous": 4,
        "current": 5,
        "next": 6
    }));
}
```

### Multiple Extra Fields

You can add multiple `#[more(...)]` attributes to include several computed fields:

```rust
use serde_more::SerializeMore;
use serde_json::json;

#[derive(SerializeMore)]
#[more(k="next", v="get_next")]
#[more(k="description", v="get_description")]
#[more(k="name")]
struct Index {
    current: u32,
}

impl Index {
    fn get_next(&self) -> u32 {
        self.current + 1
    }
    
    fn get_description(&self) -> &str {
        "Index struct"
    }
    
    fn name(&self) -> &str {
        "Index"
    }
}

fn main() {
    let idx = Index { current: 5 };
    let value = serde_json::to_value(&idx).unwrap();
    assert_eq!(value, json!({
        "current": 5,
        "next": 6,
        "description": "Index struct",
        "name": "Index"
    }));
}
```

### Works with Serde Attributes

The macro is fully compatible with `serde` attributes:

```rust
use serde_more::SerializeMore;
use serde_json::json;

#[derive(SerializeMore)]
#[serde(rename_all = "kebab-case")]
#[more(k="extraVal", v="extra_val")]
struct WithSerdeAttrs {
    field_name: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    opt_value: Option<u8>,
}

impl WithSerdeAttrs {
    fn extra_val(&self) -> &'static str {
        "ok"
    }
}

fn main() {
    let data = WithSerdeAttrs {
        field_name: 1,
        opt_value: None
    };
    let value = serde_json::to_value(&data).unwrap();
    assert_eq!(value, json!({
        "field-name": 1,
        "extraVal": "ok"
    }));
}
```

### Works with serde_as

The macro also works with `serde_with::serde_as`:

```rust
use serde_more::SerializeMore;
use serde_with::serde_as;
use serde_json::json;

#[serde_as]
#[derive(SerializeMore)]
#[more(k="payload_len")]
struct WithSerdeAsAttrs {
    #[serde_as(as = "serde_with::hex::Hex")]
    payload: Vec<u8>,
}

impl WithSerdeAsAttrs {
    fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

fn main() {
    let data = WithSerdeAsAttrs {
        payload: vec![0x0a, 0xff]
    };
    let value = serde_json::to_value(&data).unwrap();
    assert_eq!(value, json!({
        "payload": "0aff",
        "payload_len": 2
    }));
}
```

## Attribute Syntax

The `#[more(...)]` attribute supports the following syntax:

```rust
// Full form
#[more(key="field_name", value="method_name")]

// Shorthand (k and v)
#[more(k="field_name", v="method_name")]

// If value/v is omitted, method name is assumed to be the same as key
#[more(k="field_name")]

// Use position="front" to serialize the computed field before the struct fields
#[more(key="field_name", position="front")]
```

## Limitations

Currently only supports structs with named fields.

## License

[![License MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square&color=8d97b3)](LICENSE-MIT)
[![License Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square&color=8d97b3)](LICENSE-APACHE)

serde-more is dual-licensed under
[Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT) terms.
