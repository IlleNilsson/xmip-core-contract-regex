# xmip-core-contract-regex

The regular-expression content contract, a technology of
[xmip-core-contract](https://github.com/IlleNilsson/xmip-core-contract).

Two claims. **Well-formedness is a given**: the Stream is UTF-8 text, and one
that is not fails at the byte where decoding stopped. **Conformance is a given
once the contract is named**: a Receive or Send Location that refers to this
contract with a pattern bound has every Stream held to it.

A pattern binds whole, the entire text anchored at both ends, or per line with
the `lines:` prefix on the reference, where every non-empty line must match and
each departure names its line. The syntax is the `regex` crate's: finite
automata, linear in the input, so an operator's pattern cannot stall a Location.

## Toolchain

`rust-toolchain.toml` pins the toolchain for the whole estate. Do not change it
here.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
