# Strings

Strings that are passed directly (i.e. not as indices of interned strings) as format string parameters (`{:str}`) must be prefixed with their LEB128 encoded length.
This behavior is analogous to that of Slices.

``` rust
binfmt::error!("Hello, {:str}!", [b'w', b'o', b'r', b'l', b'd']);
// on the wire: [1, 5, 199, 111, 114, 108, 100]
//  string index ^  ^  ^^^^^^^^^^^^^^^^^^^^^^^ the slice data (byte literals are ascii chars)
//   LEB128(length) ^
```